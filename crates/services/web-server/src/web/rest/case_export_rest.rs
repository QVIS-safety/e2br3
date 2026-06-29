use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use lib_core::model::acs::{XML_EXPORT, XML_EXPORT_READ};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::CaseBmc;
use lib_core::model::xml_export_history::{
	XmlExportHistoryBmc, XmlExportHistoryRecord,
};
use lib_core::regulatory::RegulatoryAuthority;
use lib_core::xml::{
	export_case_xml_with_options, validate_e2b_xml, validate_e2b_xml_business,
	ExportXmlOptions,
};
use lib_rest_core::prelude::*;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_permission::{
	RequirePermission, XmlExport as XmlExportPerm,
};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use std::collections::HashSet;
use std::io::{Cursor, Write};
use time::Month;
use tokio::runtime::Handle;
use tokio::task;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const SETTINGS_KEY: &str = "system";

// -- Types

#[derive(Debug, Deserialize)]
pub struct BulkXmlExportInput {
	pub case_ids: Vec<Uuid>,
	pub authority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportCaseQuery {
	pub authority: Option<String>,
	pub include_notation: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlExportHistoryList {
	pub items: Vec<XmlExportHistoryRecord>,
}

// -- Helpers

pub fn format_message_timestamp_utc_pub(now: OffsetDateTime) -> String {
	let month = match now.month() {
		Month::January => 1,
		Month::February => 2,
		Month::March => 3,
		Month::April => 4,
		Month::May => 5,
		Month::June => 6,
		Month::July => 7,
		Month::August => 8,
		Month::September => 9,
		Month::October => 10,
		Month::November => 11,
		Month::December => 12,
	};
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		now.year(),
		month,
		now.day(),
		now.hour(),
		now.minute(),
		now.second()
	)
}

pub fn message_sender_identifier() -> String {
	std::env::var("E2BR3_DEFAULT_MESSAGE_SENDER")
		.unwrap_or_else(|_| "DSJP".to_string())
}

pub fn message_receiver_identifier(authority: RegulatoryAuthority) -> String {
	let env_name = match authority {
		RegulatoryAuthority::Fda => "E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA",
		RegulatoryAuthority::Ich => "E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH",
		RegulatoryAuthority::Mfds => "E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS",
	};
	std::env::var(env_name).unwrap_or_else(|_| {
		authority.default_message_receiver_identifier().to_string()
	})
}

fn should_validate_export_xml(authority: RegulatoryAuthority) -> bool {
	if let Ok(value) = std::env::var("E2BR3_EXPORT_VALIDATE_FDA") {
		if matches!(
			value.trim().to_ascii_lowercase().as_str(),
			"0" | "false" | "no"
		) {
			return false;
		}
		if matches!(
			value.trim().to_ascii_lowercase().as_str(),
			"1" | "true" | "yes"
		) {
			return true;
		}
	}
	if matches!(authority, RegulatoryAuthority::Fda) {
		return true;
	}
	match std::env::var("E2BR3_EXPORT_VALIDATE") {
		Ok(value) => matches!(
			value.trim().to_ascii_lowercase().as_str(),
			"1" | "true" | "yes"
		),
		Err(_) => false,
	}
}

fn resolve_requested_export_authority(
	requested_authority: Option<&str>,
) -> Result<RegulatoryAuthority> {
	let Some(raw_authority) = requested_authority else {
		return Err(Error::BadRequest {
			message: "authority is required for XML export".to_string(),
		});
	};
	RegulatoryAuthority::parse(raw_authority).ok_or_else(|| Error::BadRequest {
		message: format!(
			"invalid export authority '{raw_authority}' (expected: ich, fda or mfds)"
		),
	})
}

async fn export_xml_options(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	include_notation: Option<bool>,
) -> Result<ExportXmlOptions> {
	if let Some(apply_comments) = include_notation {
		return Ok(ExportXmlOptions { apply_comments });
	}
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	let apply_comments = value
		.as_ref()
		.and_then(|value| value.get("apply_comments_on_exported_xml"))
		.and_then(|value| value.as_bool())
		.unwrap_or(false);
	Ok(ExportXmlOptions { apply_comments })
}

fn export_file_name(
	case: &lib_core::model::case::Case,
	case_id: Uuid,
	authority: RegulatoryAuthority,
	include_authority_suffix: bool,
) -> String {
	if include_authority_suffix {
		format!(
			"{}-{}-{}.xml",
			case.safety_report_id.as_str(),
			case_id,
			authority.as_str()
		)
	} else {
		format!("{}-{}.xml", case.safety_report_id.as_str(), case_id)
	}
}

pub async fn generate_validated_case_xml(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
) -> Result<(lib_core::model::case::Case, String)> {
	lib_rest_core::require_case_read_allowed(ctx, mm, id).await?;
	let case = CaseBmc::get(ctx, mm, id).await?;
	let authority = RegulatoryAuthority::Fda;
	generate_validated_case_xml_for_authority(ctx, mm, id, case, authority).await
}

pub async fn generate_validated_case_xml_for_authority(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
	case: lib_core::model::case::Case,
	authority: RegulatoryAuthority,
) -> Result<(lib_core::model::case::Case, String)> {
	generate_validated_case_xml_for_authority_with_notation(
		ctx, mm, id, case, authority, None,
	)
	.await
}

async fn generate_validated_case_xml_for_authority_with_notation(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
	case: lib_core::model::case::Case,
	authority: RegulatoryAuthority,
	include_notation: Option<bool>,
) -> Result<(lib_core::model::case::Case, String)> {
	let ctx_clone = ctx.clone();
	let mm_clone = mm.clone();
	let options = export_xml_options(ctx, mm, include_notation).await?;
	let xml = task::spawn_blocking(move || {
		Handle::current().block_on(export_case_xml_with_options(
			&ctx_clone, &mm_clone, id, options,
		))
	})
	.await
	.map_err(|err| Error::BadRequest {
		message: format!("export task failed: {err}"),
	})??;

	if should_validate_export_xml(authority) {
		let schema_report =
			validate_e2b_xml(xml.as_bytes(), None).map_err(|err| {
				Error::BadRequest {
					message: format!("export XML validation failed: {err}"),
				}
			})?;
		if !schema_report.ok {
			let debug_path =
				std::env::temp_dir().join(format!("e2br3-export-debug-{id}.xml"));
			let _ = std::fs::write(&debug_path, &xml);
			let details = schema_report
				.errors
				.iter()
				.take(8)
				.map(|e| match (e.line, e.column) {
					(Some(line), Some(column)) => {
						format!("{} [line {}, col {}]", e.message, line, column)
					}
					(Some(line), None) => {
						format!("{} [line {}]", e.message, line)
					}
					_ => e.message.clone(),
				})
				.collect::<Vec<_>>()
				.join(" | ");
			return Err(Error::BadRequest {
				message: format!(
					"exported XML failed schema/basic validation ({} issue(s)); details: {}; debug_xml: {}",
					schema_report.errors.len(),
					details,
					debug_path.display()
				),
			});
		}
		let business_report = validate_e2b_xml_business(xml.as_bytes(), None)
			.map_err(|err| Error::BadRequest {
				message: format!("export XML business validation failed: {err}"),
			})?;
		if !business_report.ok {
			let first = business_report
				.errors
				.first()
				.map(|e| e.message.clone())
				.unwrap_or_else(|| "unknown business validation error".to_string());
			return Err(Error::BadRequest {
				message: format!(
					"exported XML failed business validation ({} issue(s)); first: {first}",
					business_report.errors.len()
				),
			});
		}
	}

	Ok((case, xml))
}

pub async fn record_xml_export(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: Uuid,
	case_number: Option<&str>,
	file_name: &str,
	status: &str,
	error_message: Option<&str>,
) -> Result<()> {
	let mut tx = mm.dbx().db().begin().await.map_err(|err| {
		Error::Model(lib_core::model::Error::Store(err.to_string()))
	})?;
	lib_core::model::store::set_user_context(&mut tx, ctx.user_id())
		.await
		.map_err(Error::Model)?;
	lib_core::model::store::set_org_context(
		&mut tx,
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	.map_err(Error::Model)?;
	sqlx::query(
		"INSERT INTO xml_export_history (
			case_id,
			case_number,
			file_name,
			status,
			error_message,
			exported_by
		) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(case_id)
	.bind(case_number)
	.bind(file_name)
	.bind(status)
	.bind(error_message)
	.bind(ctx.user_id())
	.execute(&mut *tx)
	.await
	.map_err(|err| Error::Model(lib_core::model::Error::Store(err.to_string())))?;
	tx.commit().await.map_err(|err| {
		Error::Model(lib_core::model::Error::Store(err.to_string()))
	})?;
	Ok(())
}

// -- Handlers

/// GET /api/cases/{id}/export/xml
pub async fn export_case(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Query(query): Query<ExportCaseQuery>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let authority = resolve_requested_export_authority(query.authority.as_deref())?;
	let include_authority_suffix = true;
	let file_name = export_file_name(&case, id, authority, include_authority_suffix);
	let (case, xml) = match generate_validated_case_xml_for_authority_with_notation(
		&ctx,
		&mm,
		id,
		case.clone(),
		authority,
		query.include_notation,
	)
	.await
	{
		Ok(result) => result,
		Err(err) => {
			let error_message = err.to_string();
			if let Err(record_err) = record_xml_export(
				&ctx,
				&mm,
				id,
				Some(case.safety_report_id.as_str()),
				&file_name,
				"error",
				Some(error_message.as_str()),
			)
			.await
			{
				tracing::warn!(
					"failed to record xml export error history: {record_err}"
				);
			}
			return Err(err);
		}
	};
	if let Err(err) = record_xml_export(
		&ctx,
		&mm,
		id,
		Some(case.safety_report_id.as_str()),
		&file_name,
		"success",
		None,
	)
	.await
	{
		tracing::warn!("failed to record xml export history: {err}");
	}

	let mut response = (axum::http::StatusCode::OK, xml).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("application/xml"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{file_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid export filename header: {err}"),
		})?,
	);
	Ok(response)
}

/// POST /api/cases/export/xml
pub async fn export_cases_zip(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	_perm: RequirePermission<XmlExportPerm>,
	axum::Json(input): axum::Json<BulkXmlExportInput>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	if input.case_ids.is_empty() {
		return Err(Error::BadRequest {
			message: "case_ids is required".to_string(),
		});
	}
	let authority = resolve_requested_export_authority(input.authority.as_deref())?;

	let mut unique_case_ids = Vec::new();
	let mut seen = HashSet::new();
	for case_id in input.case_ids {
		if seen.insert(case_id) {
			unique_case_ids.push(case_id);
		}
	}

	let mut cursor = Cursor::new(Vec::new());
	let options =
		SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
	{
		let mut zip = ZipWriter::new(&mut cursor);
		for case_id in unique_case_ids {
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
			let case = CaseBmc::get(&ctx, &mm, case_id).await?;
			{
				let file_name = export_file_name(&case, case_id, authority, true);
				let (case, xml) = match generate_validated_case_xml_for_authority(
					&ctx,
					&mm,
					case_id,
					case.clone(),
					authority,
				)
				.await
				{
					Ok(result) => result,
					Err(err) => {
						let error_message = err.to_string();
						if let Err(record_err) = record_xml_export(
							&ctx,
							&mm,
							case_id,
							Some(case.safety_report_id.as_str()),
							&file_name,
							"error",
							Some(error_message.as_str()),
						)
						.await
						{
							tracing::warn!(
								"failed to record xml export error history: {record_err}"
							);
						}
						return Err(err);
					}
				};
				zip.start_file(file_name.clone(), options).map_err(|err| {
					Error::BadRequest {
						message: format!("failed to start zip entry: {err}"),
					}
				})?;
				zip.write_all(xml.as_bytes())
					.map_err(|err| Error::BadRequest {
						message: format!("failed to write zip entry: {err}"),
					})?;
				if let Err(err) = record_xml_export(
					&ctx,
					&mm,
					case_id,
					Some(case.safety_report_id.as_str()),
					&file_name,
					"success",
					None,
				)
				.await
				{
					tracing::warn!("failed to record xml export history: {err}");
				}
			}
		}
		zip.finish().map_err(|err| Error::BadRequest {
			message: format!("failed to finalize zip export: {err}"),
		})?;
	}

	let bytes = cursor.into_inner();
	let file_name = format!(
		"e2br3-bulk-export-{}.zip",
		OffsetDateTime::now_utc().unix_timestamp()
	);
	let mut response = (axum::http::StatusCode::OK, bytes).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("application/zip"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{file_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid export filename header: {err}"),
		})?,
	);
	Ok(response)
}

/// GET /api/exports/history
pub async fn list_xml_export_history(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<XmlExportHistoryList>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;

	let items = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		Box::pin(async move {
			XmlExportHistoryBmc::list_all(dbx)
				.await
				.map_err(Error::from)
		})
	})
	.await?;

	let mut scoped = Vec::with_capacity(items.len());
	for item in items {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, item.case_id).await? {
			scoped.push(item);
		}
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: XmlExportHistoryList { items: scoped },
		}),
	))
}

/// GET /api/cases/{case_id}/exports/history
pub async fn list_case_xml_export_history(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<XmlExportHistoryList>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let items = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		Box::pin(async move {
			XmlExportHistoryBmc::list_by_case(dbx, case_id)
				.await
				.map_err(Error::from)
		})
	})
	.await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: XmlExportHistoryList { items },
		}),
	))
}

/// GET /api/exports/history/{id}/error.txt
pub async fn download_xml_export_history_error(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;

	let row = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		Box::pin(async move {
			XmlExportHistoryBmc::get_error_row(dbx, id)
				.await
				.map_err(Error::from)
		})
	})
	.await?;

	let row = row.ok_or_else(|| Error::BadRequest {
		message: format!("xml export history record {id} not found"),
	})?;
	if !lib_rest_core::case_matches_user_scope(&ctx, &mm, row.case_id).await? {
		return Err(Error::PermissionDenied {
			required_permission: XML_EXPORT_READ.to_string(),
		});
	}
	let text = row.error_message.ok_or_else(|| Error::BadRequest {
		message: format!("xml export history record {id} has no error details"),
	})?;

	let safe_file_name = row
		.file_name
		.chars()
		.map(|ch| match ch {
			'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
			_ => '_',
		})
		.collect::<String>();
	let download_name = format!("export-error-{id}-{safe_file_name}.txt");

	let mut response = (axum::http::StatusCode::OK, text).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("text/plain; charset=utf-8"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{download_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid export error filename header: {err}"),
		})?,
	);
	Ok(response)
}
