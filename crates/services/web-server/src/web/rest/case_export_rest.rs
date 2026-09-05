use crate::runtime_settings;
use crate::submission::export_message_header;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::safety_report::SafetyReportIdentificationBmc;
use lib_core::model::xml_export_history::{
	XmlExportHistoryBmc, XmlExportHistoryRecord,
};
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::prelude::*;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use std::collections::HashSet;
use std::io::{Cursor, Write};
use time::Month;
use uuid::Uuid;
use xml::{export_case_xml_with_options, ExportXmlOptions};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

// -- Types

const MAX_BULK_XML_CASES: usize = 100;
const MAX_BULK_XML_BYTES: usize = 100 * 1024 * 1024;

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

fn required_env_identifier(name: &str) -> Result<String> {
	let value = std::env::var(name).map_err(|_| Error::BadRequest {
		message: format!("{name} must be configured"),
	})?;
	if value.trim().is_empty() {
		return Err(Error::BadRequest {
			message: format!("{name} must not be empty"),
		});
	}
	Ok(value)
}

pub fn message_sender_identifier() -> Result<String> {
	required_env_identifier("E2BR3_DEFAULT_MESSAGE_SENDER")
}

pub fn message_receiver_identifier(
	authority: RegulatoryAuthority,
) -> Result<String> {
	let env_name = match authority {
		RegulatoryAuthority::Fda => "E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA",
		RegulatoryAuthority::Ich => "E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH",
		RegulatoryAuthority::Mfds => "E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS",
	};
	required_env_identifier(env_name)
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
	authority: RegulatoryAuthority,
	outbound_message_header: xml::OutboundMessageHeader,
) -> Result<ExportXmlOptions> {
	let apply_comments = runtime_settings::load(ctx, mm)
		.await?
		.resolve_notation(include_notation);
	Ok(ExportXmlOptions {
		apply_comments,
		authority,
		outbound_message_header,
	})
}

async fn safety_report_id_for_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: Uuid,
) -> Result<String> {
	SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::Model)?
		.safety_report_id
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::BadRequest {
			message: format!("case {case_id} has no safety report ID"),
		})
}

fn export_file_name(
	safety_report_id: &str,
	case_id: Uuid,
	authority: RegulatoryAuthority,
	include_authority_suffix: bool,
) -> String {
	if include_authority_suffix {
		format!("{safety_report_id}-{case_id}-{}.xml", authority.as_str())
	} else {
		format!("{safety_report_id}-{case_id}.xml")
	}
}

async fn generate_case_xml_for_authority(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
	authority: RegulatoryAuthority,
	include_notation: Option<bool>,
) -> Result<String> {
	let mut header = MessageHeaderBmc::get_by_case(ctx, mm, id)
		.await
		.map_err(Error::Model)?;
	header.batch_transmission_date = Some(OffsetDateTime::now_utc());
	let options = export_xml_options(
		ctx,
		mm,
		include_notation,
		authority,
		export_message_header(&header)?,
	)
	.await?;
	export_case_xml_with_options(ctx, mm, id, options)
		.await
		.map_err(|err| Error::BadRequest {
			message: format!("export task failed: {err}"),
		})
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
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Query(query): Query<ExportCaseQuery>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_export(
		&ctx,
		&snapshot,
		&mm,
		&[id],
		move |ctx, mm| {
			Box::pin(async move { export_case_authorized(ctx, mm, id, query).await })
		},
	)
	.await
}

async fn export_case_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	id: Uuid,
	query: ExportCaseQuery,
) -> Result<Response> {
	let safety_report_id = safety_report_id_for_case(ctx, mm, id).await?;
	let authority = resolve_requested_export_authority(query.authority.as_deref())?;
	let file_name = export_file_name(&safety_report_id, id, authority, true);
	let xml = match generate_case_xml_for_authority(
		ctx,
		mm,
		id,
		authority,
		query.include_notation,
	)
	.await
	{
		Ok(result) => result,
		Err(err) => {
			let error_message = err.to_string();
			if let Err(record_err) = record_xml_export(
				ctx,
				mm,
				id,
				Some(safety_report_id.as_str()),
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
		ctx,
		mm,
		id,
		Some(safety_report_id.as_str()),
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
	snapshot: AuthorizationSnapshotW,
	axum::Json(input): axum::Json<BulkXmlExportInput>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	let mut unique_case_ids = Vec::new();
	let mut seen = HashSet::new();
	for case_id in &input.case_ids {
		if seen.insert(*case_id) {
			unique_case_ids.push(*case_id);
		}
	}
	if unique_case_ids.len() > MAX_BULK_XML_CASES {
		return Err(Error::BadRequest {
			message: format!(
				"bulk export accepts at most {MAX_BULK_XML_CASES} cases"
			),
		});
	}
	let export_case_ids = unique_case_ids.clone();
	lib_rest_core::with_authorized_case_export(
		&ctx,
		&snapshot,
		&mm,
		&unique_case_ids,
		move |ctx, mm| {
			Box::pin(async move {
				export_cases_zip_authorized(ctx, mm, input, export_case_ids).await
			})
		},
	)
	.await
}

async fn export_cases_zip_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	input: BulkXmlExportInput,
	unique_case_ids: Vec<Uuid>,
) -> Result<Response> {
	if input.case_ids.is_empty() {
		return Err(Error::BadRequest {
			message: "case_ids is required".to_string(),
		});
	}
	let authority = resolve_requested_export_authority(input.authority.as_deref())?;

	let mut cursor = Cursor::new(Vec::new());
	let options =
		SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
	let mut xml_bytes = 0usize;
	{
		let mut zip = ZipWriter::new(&mut cursor);
		for case_id in unique_case_ids {
			let safety_report_id =
				safety_report_id_for_case(ctx, mm, case_id).await?;
			{
				let file_name =
					export_file_name(&safety_report_id, case_id, authority, true);
				let xml = match generate_case_xml_for_authority(
					ctx, mm, case_id, authority, None,
				)
				.await
				{
					Ok(result) => result,
					Err(err) => {
						let error_message = err.to_string();
						if let Err(record_err) = record_xml_export(
							ctx,
							mm,
							case_id,
							Some(safety_report_id.as_str()),
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
				xml_bytes = xml_bytes.checked_add(xml.len()).ok_or_else(|| {
					Error::BadRequest {
						message: "bulk export size overflow".to_string(),
					}
				})?;
				if xml_bytes > MAX_BULK_XML_BYTES {
					return Err(Error::BadRequest {
						message: format!(
							"bulk export exceeds {MAX_BULK_XML_BYTES} uncompressed bytes"
						),
					});
				}
				zip.write_all(xml.as_bytes())
					.map_err(|err| Error::BadRequest {
						message: format!("failed to write zip entry: {err}"),
					})?;
				if let Err(err) = record_xml_export(
					ctx,
					mm,
					case_id,
					Some(safety_report_id.as_str()),
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
	snapshot: AuthorizationSnapshotW,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<XmlExportHistoryList>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_export_history_collection(
		&ctx,
		&snapshot,
		&mm,
		|_ctx, mm, scope| {
			Box::pin(async move {
				let items = XmlExportHistoryBmc::list_all_scoped(mm.dbx(), scope)
					.await
					.map_err(Error::from)?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult {
						data: XmlExportHistoryList { items },
					}),
				))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/exports/history
pub async fn list_case_xml_export_history(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<XmlExportHistoryList>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"case.export.history.read",
		move |_ctx, mm| {
			Box::pin(async move {
				let items = XmlExportHistoryBmc::list_by_case(mm.dbx(), case_id)
					.await
					.map_err(Error::from)?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult {
						data: XmlExportHistoryList { items },
					}),
				))
			})
		},
	)
	.await
}

/// GET /api/exports/history/{id}/error.txt
pub async fn download_xml_export_history_error(
	State(mm): State<lib_core::model::ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_export_history_read(
		&ctx,
		&snapshot,
		&mm,
		id,
		move |_ctx, mm| {
			Box::pin(async move {
				let row = XmlExportHistoryBmc::get_error_row(mm.dbx(), id)
					.await
					.map_err(Error::from)?
					.ok_or_else(|| Error::BadRequest {
						message: format!("xml export history record {id} not found"),
					})?;
				let text = row.error_message.ok_or_else(|| Error::BadRequest {
					message: format!(
						"xml export history record {id} has no error details"
					),
				})?;
				let safe_file_name = row
					.file_name
					.chars()
					.map(|ch| match ch {
						'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
						_ => '_',
					})
					.collect::<String>();
				let download_name =
					format!("export-error-{id}-{safe_file_name}.txt");
				let mut response =
					(axum::http::StatusCode::OK, text).into_response();
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
						message: format!(
							"invalid export error filename header: {err}"
						),
					})?,
				);
				Ok(response)
			})
		},
	)
	.await
}
