use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::XML_IMPORT;
use lib_core::model::store::{
	set_full_context_dbx, set_full_context_dbx_or_rollback,
};
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
use sqlx::FromRow;
use std::io::{Cursor, Read};
use time::OffsetDateTime;
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

#[derive(Debug, Serialize, FromRow)]
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
	uploaded_at: OffsetDateTime,
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
	let dbx = mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	set_full_context_dbx_or_rollback(
		dbx,
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	.map_err(Error::from)?;

	dbx.execute(
		sqlx::query(
			"INSERT INTO xml_import_history (
				uploaded_file_name,
				source_file_name,
				case_id,
				case_number,
				status,
				error_message,
				validation_profile,
				uploaded_by
			) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
		)
		.bind(uploaded_file_name)
		.bind(source_file_name)
		.bind(case_id)
		.bind(case_number)
		.bind(status)
		.bind(error_message)
		.bind(validation_profile)
		.bind(ctx.user_id()),
	)
	.await
	.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	dbx.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
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

	let dbx = mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(Error::from)?;

	let items = dbx
		.fetch_all(sqlx::query_as::<_, XmlImportHistoryRecord>(
			"SELECT h.id,
			        h.uploaded_file_name,
			        h.source_file_name,
			        h.case_id,
			        h.case_number,
			        h.status,
			        h.error_message,
			        h.validation_profile,
			        h.uploaded_by,
			        u.email AS uploader_email,
			        h.uploaded_at
			   FROM xml_import_history h
			   LEFT JOIN users u ON u.id = h.uploaded_by
			  ORDER BY h.uploaded_at DESC, h.created_at DESC
			  LIMIT 200",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	dbx.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: XmlImportHistoryList { items },
		}),
	))
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
