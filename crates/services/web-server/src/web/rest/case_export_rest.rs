use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use lib_core::model::acs::{CASE_READ, XML_EXPORT};
use lib_core::model::case::CaseBmc;
use lib_core::model::xml_export_history::{
	XmlExportHistoryBmc, XmlExportHistoryRecord,
};
use lib_core::validation::{RegulatoryAuthority, ValidationProfile};
use lib_core::xml::{export_case_xml, validate_e2b_xml, validate_e2b_xml_business};
use lib_rest_core::prelude::*;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
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

// -- Types

#[derive(Debug, Deserialize)]
pub struct BulkXmlExportInput {
	pub case_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ExportCaseQuery {
	pub profile: Option<String>,
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

pub fn message_receiver_identifier(profile: ValidationProfile) -> String {
	let authority = RegulatoryAuthority::from_validation_profile(profile);
	let env_name = match authority {
		RegulatoryAuthority::Fda => "E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA",
		RegulatoryAuthority::Ich => "E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH",
		RegulatoryAuthority::Mfds => "E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS",
	};
	std::env::var(env_name).unwrap_or_else(|_| {
		authority.default_message_receiver_identifier().to_string()
	})
}

fn should_validate_export_xml(profile: ValidationProfile) -> bool {
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
	if matches!(
		RegulatoryAuthority::from_validation_profile(profile),
		RegulatoryAuthority::Fda
	) {
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

fn parse_appendix_profiles(value: &str) -> Vec<ValidationProfile> {
	let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(value) else {
		return Vec::new();
	};
	items
		.iter()
		.filter_map(|item| item.as_str())
		.filter_map(ValidationProfile::parse)
		.fold(Vec::new(), |mut acc, profile| {
			if !acc.contains(&profile) {
				acc.push(profile);
			}
			acc
		})
}

fn selected_export_profiles(
	case: &lib_core::model::case::Case,
) -> Vec<ValidationProfile> {
	if let Some(value) = case.appendices_json.as_deref() {
		let profiles = parse_appendix_profiles(value);
		if !profiles.is_empty() {
			return profiles;
		}
	}
	case.validation_profile
		.as_deref()
		.and_then(ValidationProfile::parse)
		.map(|profile| vec![profile])
		.unwrap_or_else(|| vec![ValidationProfile::Fda])
}

fn resolve_requested_export_profile(
	case: &lib_core::model::case::Case,
	requested_profile: Option<&str>,
) -> Result<ValidationProfile> {
	let selected = selected_export_profiles(case);
	let Some(raw_profile) = requested_profile else {
		return Ok(selected[0]);
	};
	let profile =
		ValidationProfile::parse(raw_profile).ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid validation profile '{raw_profile}' (expected: ich, fda or mfds)"
			),
		})?;
	if !selected.contains(&profile) {
		return Err(Error::BadRequest {
			message: format!(
				"profile '{}' is not selected on this case",
				profile.as_str()
			),
		});
	}
	Ok(profile)
}

fn export_file_name(
	case: &lib_core::model::case::Case,
	case_id: Uuid,
	profile: ValidationProfile,
	include_profile_suffix: bool,
) -> String {
	if include_profile_suffix {
		format!(
			"{}-{}-{}.xml",
			case.safety_report_id.as_str(),
			case_id,
			profile.as_str()
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
	let profile = resolve_requested_export_profile(&case, None)?;
	generate_validated_case_xml_for_profile(ctx, mm, id, case, profile).await
}

pub async fn generate_validated_case_xml_for_profile(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
	case: lib_core::model::case::Case,
	profile: ValidationProfile,
) -> Result<(lib_core::model::case::Case, String)> {
	let ctx_clone = ctx.clone();
	let mm_clone = mm.clone();
	let xml = task::spawn_blocking(move || {
		Handle::current().block_on(export_case_xml(&ctx_clone, &mm_clone, id))
	})
	.await
	.map_err(|err| Error::BadRequest {
		message: format!("export task failed: {err}"),
	})??;

	if should_validate_export_xml(profile) {
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
	validation_profile: Option<&str>,
	status: &str,
	error_message: Option<&str>,
) -> Result<()> {
	XmlExportHistoryBmc::record(
		mm,
		ctx,
		case_id,
		case_number,
		file_name,
		validation_profile,
		status,
		error_message,
	)
	.await
	.map_err(Error::Model)
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
	let profile = resolve_requested_export_profile(&case, query.profile.as_deref())?;
	let include_profile_suffix = query.profile.is_some();
	let (case, xml) =
		generate_validated_case_xml_for_profile(&ctx, &mm, id, case, profile)
			.await?;
	let file_name = export_file_name(&case, id, profile, include_profile_suffix);
	if let Err(err) = record_xml_export(
		&ctx,
		&mm,
		id,
		Some(case.safety_report_id.as_str()),
		&file_name,
		Some(profile.as_str()),
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
	axum::Json(input): axum::Json<BulkXmlExportInput>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	if input.case_ids.is_empty() {
		return Err(Error::BadRequest {
			message: "case_ids is required".to_string(),
		});
	}

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
			let profiles = selected_export_profiles(&case);
			let include_profile_suffix = profiles.len() > 1;
			for profile in profiles {
				let (case, xml) = generate_validated_case_xml_for_profile(
					&ctx,
					&mm,
					case_id,
					case.clone(),
					profile,
				)
				.await?;
				let file_name = export_file_name(
					&case,
					case_id,
					profile,
					include_profile_suffix,
				);
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
					Some(profile.as_str()),
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
	require_permission(&ctx, XML_EXPORT)?;

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
	require_permission(&ctx, CASE_READ)?;
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
	require_permission(&ctx, XML_EXPORT)?;

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
			required_permission: XML_EXPORT.to_string(),
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
