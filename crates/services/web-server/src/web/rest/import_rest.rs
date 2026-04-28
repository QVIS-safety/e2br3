use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::XML_IMPORT;
use lib_core::model::xml_import_history::XmlImportHistoryBmc;
use lib_core::model::ModelManager;
use lib_core::validation::xml::{
	should_skip_xml_validation, validate_e2b_xml_basic,
};
use lib_core::xml::{
	import_e2b_xml, validate_e2b_xml, XmlImportRequest, XmlValidationReport,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Serialize;
use std::io::{Cursor, Read};
use time::format_description::well_known::Rfc3339;
use tracing::warn;
use uuid::Uuid;
use zip::ZipArchive;

struct UploadedImportPayload {
	bytes: Vec<u8>,
	filename: Option<String>,
	validation_profile: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCaseSummary {
	case_number: String,
	status: &'static str,
	message: Option<String>,
	case_id: Option<String>,
	case_version: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlImportBatchResult {
	imported_cases: Vec<ImportedCaseSummary>,
	case_id: Option<String>,
	case_version: Option<i64>,
	xml_key: Option<String>,
	parsed_json_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlImportHistoryRecord {
	id: Uuid,
	uploaded_file_name: String,
	source_file_name: String,
	case_id: Option<Uuid>,
	case_number: Option<String>,
	status: String,
	error_message: Option<String>,
	validation_profile: Option<String>,
	uploaded_by: Uuid,
	uploader_email: Option<String>,
	uploaded_at: String,
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlImportHistoryList {
	items: Vec<XmlImportHistoryRecord>,
}

fn normalize_validation_profile(value: &str) -> Result<String> {
	let normalized = value.trim().to_ascii_lowercase();
	match normalized.as_str() {
		"" | "auto" => Ok(String::new()),
		"ich" | "fda" | "mfds" => Ok(normalized),
		_ => Err(Error::BadRequest {
			message: format!(
				"invalid validation profile '{value}' (expected: auto, ich, fda or mfds)"
			),
		}),
	}
}

async fn read_xml_multipart(
	mut multipart: Multipart,
) -> Result<UploadedImportPayload> {
	let mut file_bytes: Option<Vec<u8>> = None;
	let mut filename: Option<String> = None;
	let mut validation_profile: Option<String> = None;

	while let Some(field) =
		multipart
			.next_field()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("multipart error: {err}"),
			})? {
		let name = field.name().map(|v| v.to_string());
		if name.as_deref() == Some("file") || name.as_deref() == Some("xml") {
			filename = field.file_name().map(|value| value.to_string());
			let bytes = field.bytes().await.map_err(|err| Error::BadRequest {
				message: format!("multipart read error: {err}"),
			})?;
			file_bytes = Some(bytes.to_vec());
			continue;
		}

		if name.as_deref() == Some("format")
			|| name.as_deref() == Some("validation_profile")
		{
			let value = field.text().await.map_err(|err| Error::BadRequest {
				message: format!("multipart read error: {err}"),
			})?;
			let normalized = normalize_validation_profile(&value)?;
			if !normalized.is_empty() {
				validation_profile = Some(normalized);
			}
		}
	}

	let bytes = file_bytes.ok_or_else(|| Error::BadRequest {
		message: "missing xml file field".to_string(),
	})?;

	Ok(UploadedImportPayload {
		bytes,
		filename,
		validation_profile,
	})
}

fn extract_xml_entries(
	bytes: &[u8],
	filename: Option<&str>,
) -> Result<Vec<(String, Vec<u8>)>> {
	let looks_like_zip = filename
		.map(|name| name.to_ascii_lowercase().ends_with(".zip"))
		.unwrap_or(false);

	if !looks_like_zip {
		if let Ok(zip) = ZipArchive::new(Cursor::new(bytes)) {
			return extract_xml_entries_from_zip(zip);
		}
		return Ok(vec![(
			filename.unwrap_or("import.xml").to_string(),
			bytes.to_vec(),
		)]);
	}

	let zip =
		ZipArchive::new(Cursor::new(bytes)).map_err(|err| Error::BadRequest {
			message: format!("invalid import zip: {err}"),
		})?;
	extract_xml_entries_from_zip(zip)
}

fn extract_xml_entries_from_zip(
	mut zip: ZipArchive<Cursor<&[u8]>>,
) -> Result<Vec<(String, Vec<u8>)>> {
	let mut entries = Vec::new();
	for idx in 0..zip.len() {
		let mut entry = zip.by_index(idx).map_err(|err| Error::BadRequest {
			message: format!("zip read error: {err}"),
		})?;
		if entry.name().ends_with('/') {
			continue;
		}
		let entry_name = entry.name().to_string();
		if !entry_name.to_ascii_lowercase().ends_with(".xml") {
			continue;
		}

		let mut entry_bytes = Vec::new();
		entry
			.read_to_end(&mut entry_bytes)
			.map_err(|err| Error::BadRequest {
				message: format!("zip entry read error: {err}"),
			})?;
		entries.push((entry_name, entry_bytes));
	}

	if entries.is_empty() {
		return Err(Error::BadRequest {
			message: "zip archive does not contain any .xml files".to_string(),
		});
	}

	Ok(entries)
}

async fn record_import_history(
	ctx: &Ctx,
	mm: &ModelManager,
	uploaded_file_name: &str,
	source_file_name: &str,
	case_id: Option<Uuid>,
	case_number: Option<&str>,
	status: &str,
	error_message: Option<&str>,
	validation_profile: Option<&str>,
) -> Result<()> {
	XmlImportHistoryBmc::record(
		mm,
		ctx,
		uploaded_file_name,
		source_file_name,
		case_id,
		case_number,
		status,
		error_message,
		validation_profile,
	)
	.await
	.map_err(Error::Model)
}

async fn import_single_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: Vec<u8>,
	uploaded_file_name: &str,
	filename: String,
	validation_profile: Option<String>,
) -> ImportedCaseSummary {
	let import_result = import_e2b_xml(
		ctx,
		mm,
		XmlImportRequest {
			xml,
			filename: Some(filename.clone()),
			validation_profile: validation_profile.clone(),
			skip_validation: false,
		},
	)
	.await;
	match import_result {
		Ok(result) => {
			let case_number = result
				.case_number
				.clone()
				.unwrap_or_else(|| filename.clone());
			let case_id = result
				.case_id
				.as_deref()
				.and_then(|value| Uuid::parse_str(value).ok());
			if let Err(err) = record_import_history(
				ctx,
				mm,
				uploaded_file_name,
				&filename,
				case_id,
				result.case_number.as_deref(),
				"success",
				None,
				validation_profile.as_deref(),
			)
			.await
			{
				warn!("failed to record xml import history: {err}");
			}
			ImportedCaseSummary {
				case_number,
				status: "success",
				message: Some("Successfully imported".to_string()),
				case_id: result.case_id,
				case_version: result.case_version,
			}
		}
		Err(err) => {
			let message = err.to_string();
			if let Err(history_err) = record_import_history(
				ctx,
				mm,
				uploaded_file_name,
				&filename,
				None,
				None,
				"error",
				Some(&message),
				validation_profile.as_deref(),
			)
			.await
			{
				warn!("failed to record xml import history: {history_err}");
			}
			ImportedCaseSummary {
				case_number: filename,
				status: "error",
				message: Some(message),
				case_id: None,
				case_version: None,
			}
		}
	}
}

pub async fn list_import_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportHistoryList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT)?;

	let rows = XmlImportHistoryBmc::list_all(&mm, &ctx)
		.await
		.map_err(Error::Model)?;
	let mut items = Vec::with_capacity(rows.len());
	for row in rows {
		let allowed = match row.case_id {
			Some(case_id) => {
				lib_rest_core::case_matches_user_scope(&ctx, &mm, case_id).await?
			}
			None => ctx.can_admin_safety_db(),
		};
		if !allowed {
			continue;
		}
		items.push(XmlImportHistoryRecord {
			id: row.id,
			uploaded_file_name: row.uploaded_file_name,
			source_file_name: row.source_file_name,
			case_id: row.case_id,
			case_number: row.case_number,
			status: row.status,
			error_message: row.error_message,
			validation_profile: row.validation_profile,
			uploaded_by: row.uploaded_by,
			uploader_email: row.uploader_email,
			uploaded_at: row
				.uploaded_at
				.format(&Rfc3339)
				.unwrap_or_else(|_| row.uploaded_at.to_string()),
		});
	}

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: XmlImportHistoryList { items },
		}),
	))
}

pub async fn download_import_history_error(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT)?;

	let row = XmlImportHistoryBmc::get_error_row(&mm, &ctx, id)
		.await
		.map_err(Error::Model)?;

	let row = row.ok_or_else(|| Error::BadRequest {
		message: format!("xml import history record {id} not found"),
	})?;
	match row.case_id {
		Some(case_id) => {
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
		}
		None if ctx.can_admin_safety_db() => {}
		None => {
			return Err(Error::PermissionDenied {
				required_permission: XML_IMPORT.to_string(),
			});
		}
	}
	let text = row.error_message.ok_or_else(|| Error::BadRequest {
		message: format!("xml import history record {id} has no error details"),
	})?;

	let safe_source_name = row
		.source_file_name
		.chars()
		.map(|ch| match ch {
			'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
			_ => '_',
		})
		.collect::<String>();
	let file_name = format!("import-error-{id}-{safe_source_name}.txt");

	let mut response = (StatusCode::OK, text).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("text/plain; charset=utf-8"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		header::HeaderValue::from_str(&format!(
			"attachment; filename=\"{file_name}\""
		))
		.map_err(|err| Error::BadRequest {
			message: format!("invalid import error filename header: {err}"),
		})?,
	);
	Ok(response)
}

/// POST /api/import/xml/validate
/// Validates E2B(R3) XML payload (XSD-only for now)
pub async fn validate_xml(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlValidationReport>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT)?;

	let payload = read_xml_multipart(multipart).await?;
	let report = if should_skip_xml_validation() {
		// Keep local dev usable even when XSD files are not mounted/available.
		validate_e2b_xml_basic(&payload.bytes, None)?
	} else {
		validate_e2b_xml(&payload.bytes, None)?
	};

	Ok((StatusCode::OK, Json(DataRestResult { data: report })))
}

/// POST /api/import/xml
/// Parse + import E2B(R3) XML (pipeline WIP)
pub async fn import_xml(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportBatchResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT)?;

	let payload = read_xml_multipart(multipart).await?;
	let entries = extract_xml_entries(&payload.bytes, payload.filename.as_deref())?;
	let mut imported_cases = Vec::with_capacity(entries.len());
	let uploaded_file_name = payload
		.filename
		.clone()
		.unwrap_or_else(|| "import.xml".to_string());

	for (entry_name, xml) in entries {
		imported_cases.push(
			import_single_xml(
				&ctx,
				&mm,
				xml,
				&uploaded_file_name,
				entry_name,
				payload.validation_profile.clone(),
			)
			.await,
		);
	}

	let first_success = imported_cases.iter().find(|item| item.status == "success");
	let result = XmlImportBatchResult {
		case_id: first_success.and_then(|item| item.case_id.clone()),
		case_version: first_success.and_then(|item| item.case_version),
		xml_key: None,
		parsed_json_id: None,
		imported_cases,
	};

	Ok((StatusCode::OK, Json(DataRestResult { data: result })))
}
