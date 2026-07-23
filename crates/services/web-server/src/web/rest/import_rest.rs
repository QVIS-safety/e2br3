use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::{XML_IMPORT, XML_IMPORT_READ};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case_duplicate::{CaseDuplicateBmc, CaseDuplicateKey};
use lib_core::model::presave::ProductPresaveBmc;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::xml_import_decision::{
	decide_xml_import, XmlImportDecision, XmlImportDecisionAction,
	XmlImportDuplicateMatch, XmlImportExistingCase, XmlImportIncomingKey,
};
use lib_core::model::xml_import_history::XmlImportHistoryBmc;
use lib_core::model::ModelManager;
use lib_core::xml::import_sections::{
	c_safety_report::parse_c_safety_report, d_patient::parse_d_patient,
	e_reaction::parse_e_reactions, g_drug::parse_g_drugs,
};
use lib_core::xml::{
	extract_safety_report_id_from_xml, import_e2b_xml_unvalidated, CImportSettings,
	XmlImportRequest, XmlValidationReport,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_permission::{
	RequirePermission, XmlImport as XmlImportPerm,
};
use serde::Serialize;
use sqlx::FromRow;
use std::io::{Cursor, Read};
use time::format_description::well_known::Rfc3339;
use tracing::warn;
use uuid::Uuid;
use validator::xml::{
	should_skip_xml_validation, validate_e2b_xml, validate_e2b_xml_basic,
};
use zip::ZipArchive;

const MAX_XML_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
const MAX_XML_ZIP_ENTRY_BYTES: usize = 25 * 1024 * 1024;
const SETTINGS_KEY: &str = "system";

struct UploadedImportPayload {
	bytes: Vec<u8>,
	filename: Option<String>,
	product_id: Option<String>,
	product_presave_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCaseSummary {
	case_number: String,
	status: &'static str,
	message: Option<String>,
	case_id: Option<String>,
	case_version: Option<i64>,
	decision: Option<&'static str>,
	source_file_name: Option<String>,
	matched_case_id: Option<String>,
	matched_case_number: Option<String>,
	matched_case_version: Option<i32>,
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
	uploaded_by: Uuid,
	uploader_email: Option<String>,
	uploaded_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlImportHistoryList {
	items: Vec<XmlImportHistoryRecord>,
}

#[derive(Debug, FromRow)]
struct SameSafetyReportRow {
	case_id: Uuid,
	safety_report_id: String,
	version: i32,
	date_of_most_recent_information: Option<time::Date>,
}

async fn read_xml_multipart(
	mut multipart: Multipart,
) -> Result<UploadedImportPayload> {
	let mut file_bytes: Option<Vec<u8>> = None;
	let mut filename: Option<String> = None;
	let mut product_id: Option<String> = None;
	let mut product_presave_id: Option<Uuid> = None;

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
			file_bytes = Some(
				read_field_limited(field, MAX_XML_UPLOAD_BYTES, "xml upload")
					.await?,
			);
			continue;
		}
		if matches!(name.as_deref(), Some("productId") | Some("product_id")) {
			let text = field.text().await.map_err(|err| Error::BadRequest {
				message: format!("multipart productId read error: {err}"),
			})?;
			let trimmed = text.trim();
			if !trimmed.is_empty() {
				product_id = Some(trimmed.to_string());
			}
			continue;
		}
		if matches!(
			name.as_deref(),
			Some("productPresaveId") | Some("product_presave_id")
		) {
			let text = field.text().await.map_err(|err| Error::BadRequest {
				message: format!("multipart productPresaveId read error: {err}"),
			})?;
			let trimmed = text.trim();
			if !trimmed.is_empty() {
				product_presave_id =
					Some(Uuid::parse_str(trimmed).map_err(|_| {
						Error::BadRequest {
							message: "productPresaveId must be a UUID".to_string(),
						}
					})?);
			}
		}
	}

	let bytes = file_bytes.ok_or_else(|| Error::BadRequest {
		message: "missing xml file field".to_string(),
	})?;

	Ok(UploadedImportPayload {
		bytes,
		filename,
		product_id,
		product_presave_id,
	})
}

async fn read_field_limited(
	mut field: Field<'_>,
	max_bytes: usize,
	label: &str,
) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	while let Some(chunk) = field.chunk().await.map_err(|err| Error::BadRequest {
		message: format!("multipart read error: {err}"),
	})? {
		if bytes.len().saturating_add(chunk.len()) > max_bytes {
			return Err(Error::BadRequest {
				message: format!("{label} exceeds {max_bytes} bytes"),
			});
		}
		bytes.extend_from_slice(&chunk);
	}
	Ok(bytes)
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

		let entry_bytes = read_zip_entry_limited(
			&mut entry,
			MAX_XML_ZIP_ENTRY_BYTES,
			"xml zip entry",
		)?;
		entries.push((entry_name, entry_bytes));
	}

	if entries.is_empty() {
		return Err(Error::BadRequest {
			message: "zip archive does not contain any .xml files".to_string(),
		});
	}

	Ok(entries)
}

fn read_zip_entry_limited<R: Read>(
	reader: &mut R,
	max_bytes: usize,
	label: &str,
) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	let mut buffer = [0_u8; 64 * 1024];
	loop {
		let read = reader.read(&mut buffer).map_err(|err| Error::BadRequest {
			message: format!("{label} read error: {err}"),
		})?;
		if read == 0 {
			break;
		}
		if bytes.len().saturating_add(read) > max_bytes {
			return Err(Error::BadRequest {
				message: format!("{label} exceeds {max_bytes} bytes"),
			});
		}
		bytes.extend_from_slice(&buffer[..read]);
	}
	Ok(bytes)
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
	c_settings: CImportSettings,
	decision: XmlImportDecision,
	product_presave_id: Option<Uuid>,
	product_id: Option<String>,
) -> ImportedCaseSummary {
	let validation_report = if should_skip_xml_validation() {
		validate_e2b_xml_basic(&xml, None)
	} else {
		validate_e2b_xml(&xml, None)
	};
	match validation_report {
		Ok(report) if report.ok => {}
		Ok(report) => {
			let message = report
				.errors
				.first()
				.map(|error| error.message.clone())
				.unwrap_or_else(|| "XML validation failed".to_string());
			if let Err(history_err) = record_import_history(
				ctx,
				mm,
				uploaded_file_name,
				&filename,
				None,
				None,
				"error",
				Some(&message),
			)
			.await
			{
				warn!("failed to record xml import validation error: {history_err}");
			}
			return ImportedCaseSummary {
				case_number: filename.clone(),
				status: "error",
				message: Some(message),
				case_id: None,
				case_version: None,
				decision: Some("error"),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
			};
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
			)
			.await
			{
				warn!("failed to record xml import validation error: {history_err}");
			}
			return ImportedCaseSummary {
				case_number: filename.clone(),
				status: "error",
				message: Some(message),
				case_id: None,
				case_version: None,
				decision: Some("error"),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
			};
		}
	}

	let import_result = import_e2b_xml_unvalidated(
		ctx,
		mm,
		XmlImportRequest {
			xml,
			filename: Some(filename.clone()),
			skip_validation: false,
			c_settings,
			product_presave_id,
			product_id,
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
				decision: Some(decision_label(decision.action)),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
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
			)
			.await
			{
				warn!("failed to record xml import history: {history_err}");
			}
			ImportedCaseSummary {
				case_number: filename.clone(),
				status: "error",
				message: Some(message),
				case_id: None,
				case_version: None,
				decision: Some("error"),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
			}
		}
	}
}

fn decision_label(action: XmlImportDecisionAction) -> &'static str {
	match action {
		XmlImportDecisionAction::New => "new",
		XmlImportDecisionAction::FollowUp => "followUp",
		XmlImportDecisionAction::Skip => "skip",
		XmlImportDecisionAction::Error => "error",
	}
}

fn summary_for_skipped_decision(
	_uploaded_file_name: &str,
	source_file_name: &str,
	decision: XmlImportDecision,
) -> ImportedCaseSummary {
	ImportedCaseSummary {
		case_number: decision
			.matched_case_number
			.clone()
			.unwrap_or_else(|| source_file_name.to_string()),
		status: "skipped",
		message: decision.message.clone(),
		case_id: None,
		case_version: None,
		decision: Some("skip"),
		source_file_name: Some(source_file_name.to_string()),
		matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
		matched_case_number: decision.matched_case_number,
		matched_case_version: decision.matched_case_version,
	}
}

fn summary_for_decision_error(
	source_file_name: &str,
	message: String,
) -> ImportedCaseSummary {
	ImportedCaseSummary {
		case_number: source_file_name.to_string(),
		status: "error",
		message: Some(message),
		case_id: None,
		case_version: None,
		decision: Some("error"),
		source_file_name: Some(source_file_name.to_string()),
		matched_case_id: None,
		matched_case_number: None,
		matched_case_version: None,
	}
}

fn decimal_string(value: Option<rust_decimal::Decimal>) -> Option<String> {
	value.map(|value| value.normalize().to_string())
}

fn duplicate_key_from_xml(
	xml: &[u8],
	product_id: Option<&str>,
) -> Result<(XmlImportIncomingKey, CaseDuplicateKey)> {
	let safety_report_id =
		extract_safety_report_id_from_xml(xml).map_err(Error::Xml)?;
	let c_report =
		parse_c_safety_report(xml)
			.map_err(Error::Xml)?
			.ok_or_else(|| Error::BadRequest {
				message: "C.1 safety report section missing".to_string(),
			})?;
	let patient = parse_d_patient(xml).map_err(Error::Xml)?;
	let reactions = parse_e_reactions(xml).map_err(Error::Xml)?;
	let drugs = parse_g_drugs(xml).map_err(Error::Xml)?;
	let first_reaction = reactions.first();
	let first_drug = drugs.first();
	let dg_prd_key = product_id
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(ToOwned::to_owned)
		.or_else(|| first_drug.map(|drug| drug.medicinal_product.clone()));

	Ok((
		XmlImportIncomingKey {
			safety_report_id,
			date_of_most_recent_information: c_report
				.date_of_most_recent_information,
		},
		CaseDuplicateKey {
			report_type: Some(c_report.report_type),
			reporter_organization: None,
			sponsor_study_number: None,
			patient_initials: patient
				.as_ref()
				.and_then(|patient| patient.patient_initials.clone()),
			investigation_number: None,
			age_d2_2a: patient
				.as_ref()
				.and_then(|patient| decimal_string(patient.age_at_time_of_onset)),
			sex_d5: patient.as_ref().and_then(|patient| patient.sex.clone()),
			dg_prd_key,
			reaction_meddra_version: first_reaction
				.and_then(|reaction| reaction.reaction_meddra_version.clone()),
			reaction_meddra_code: first_reaction
				.and_then(|reaction| reaction.reaction_meddra_code.clone()),
			ae_start_date: first_reaction.and_then(|reaction| reaction.start_date),
		},
	))
}

async fn list_same_safety_report_cases(
	ctx: &Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<Vec<XmlImportExistingCase>> {
	let scoped = mm.new_with_txn().map_err(Error::Model)?;
	let dbx = scoped.dbx();
	dbx.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	if let Err(err) =
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await
	{
		let _ = dbx.rollback_txn().await;
		return Err(Error::Model(err));
	}
	let rows = dbx
		.fetch_all(
			sqlx::query_as::<_, SameSafetyReportRow>(
				r#"
				SELECT c.id AS case_id,
				       s.safety_report_id,
				       s.version,
				       s.date_of_most_recent_information
				  FROM safety_report_identification s
				  JOIN cases c ON c.id = s.case_id
				 WHERE s.safety_report_id = $1
				   AND c.organization_id = $2
				 ORDER BY s.version DESC
				"#,
			)
			.bind(safety_report_id)
			.bind(ctx.organization_id()),
		)
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	dbx.commit_txn()
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	Ok(rows
		.into_iter()
		.map(|row| XmlImportExistingCase {
			case_id: row.case_id,
			safety_report_id: row.safety_report_id,
			version: row.version,
			date_of_most_recent_information: row.date_of_most_recent_information,
		})
		.collect())
}

async fn decide_import_entry(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	product_id: Option<&str>,
) -> Result<XmlImportDecision> {
	let (incoming, duplicate_key) = duplicate_key_from_xml(xml, product_id)?;
	let same_report_cases =
		list_same_safety_report_cases(ctx, mm, &incoming.safety_report_id).await?;
	let duplicate_matches =
		CaseDuplicateBmc::list_potential_matches(ctx, mm, &duplicate_key)
			.await
			.map_err(Error::Model)?;
	let duplicate_matches = duplicate_matches
		.into_iter()
		.map(|item| XmlImportDuplicateMatch {
			case_id: item.case_id,
			safety_report_id: item.safety_report_id,
			version: item.version,
			date_of_most_recent_information: item.date_of_most_recent_information,
		})
		.collect::<Vec<_>>();
	Ok(decide_xml_import(
		&incoming,
		&same_report_cases,
		&duplicate_matches,
	))
}

async fn load_import_settings(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<CImportSettings> {
	let Some(value) = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?
	else {
		return Ok(CImportSettings::default());
	};
	let import_date_update =
		value.get("import_date_update").and_then(|v| v.as_object());
	Ok(CImportSettings {
		update_date_of_creation: import_date_update
			.and_then(|v| v.get("date_of_creation"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false),
		update_most_recent_info_date: import_date_update
			.and_then(|v| v.get("most_recent_info_date"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false),
		update_report_first_received_date: import_date_update
			.and_then(|v| v.get("report_first_received_date"))
			.and_then(|v| v.as_bool())
			.unwrap_or(false),
		apply_sender_info_to_imported_cases: value
			.get("apply_sender_info_to_imported_cases")
			.and_then(|v| v.as_bool())
			.unwrap_or(false),
		apply_default_values_to_imported_r2_cases: value
			.get("apply_default_values_to_imported_r2_cases")
			.and_then(|v| v.as_bool())
			.unwrap_or(false),
		selected_sender_presave_id: None,
	})
}

pub async fn list_import_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportHistoryList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT_READ)?;

	let rows = XmlImportHistoryBmc::list_all(&mm, &ctx)
		.await
		.map_err(Error::Model)?;
	let mut items = Vec::with_capacity(rows.len());
	for row in rows {
		let allowed = match row.case_id {
			Some(case_id) => {
				lib_rest_core::case_matches_user_scope(&ctx, &mm, case_id).await?
			}
			None => ctx.is_admin(),
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
	require_permission(&ctx, XML_IMPORT_READ)?;

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
		None if ctx.is_admin() => {}
		None => {
			return Err(Error::PermissionDenied {
				required_permission: XML_IMPORT_READ.to_string(),
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
	_perm: RequirePermission<XmlImportPerm>,
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
	_perm: RequirePermission<XmlImportPerm>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportBatchResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_IMPORT)?;

	let payload = read_xml_multipart(multipart).await?;
	let selected_product = match payload.product_presave_id {
		Some(id) => {
			let product = ProductPresaveBmc::get(&ctx, &mm, id)
				.await
				.map_err(Error::Model)?;
			if product.deleted {
				return Err(Error::BadRequest {
					message: "selected Product is deleted".to_string(),
				});
			}
			super::section_presave_rest::ensure_product_presave_scope(
				&ctx, &mm, &product,
			)
			.await?;
			Some((id, product.product_id, product.sender_presave_id))
		}
		None => None,
	};
	let entries = extract_xml_entries(&payload.bytes, payload.filename.as_deref())?;
	let mut imported_cases = Vec::with_capacity(entries.len());
	let mut c_settings = load_import_settings(&ctx, &mm).await?;
	if c_settings.apply_sender_info_to_imported_cases {
		c_settings.selected_sender_presave_id = selected_product
			.as_ref()
			.and_then(|(_, _, sender_presave_id)| *sender_presave_id);
	}
	let uploaded_file_name = payload
		.filename
		.clone()
		.unwrap_or_else(|| "import.xml".to_string());
	let effective_product_id = selected_product
		.as_ref()
		.and_then(|(_, product_id, _)| product_id.as_deref())
		.or(payload.product_id.as_deref());

	for (entry_name, xml) in entries {
		let decision =
			match decide_import_entry(&ctx, &mm, &xml, effective_product_id).await {
				Ok(decision) => decision,
				Err(err) => {
					let message = err.to_string();
					if let Err(history_err) = record_import_history(
						&ctx,
						&mm,
						&uploaded_file_name,
						&entry_name,
						None,
						None,
						"error",
						Some(&message),
					)
					.await
					{
						warn!(
						"failed to record xml import decision error: {history_err}"
					);
					}
					imported_cases
						.push(summary_for_decision_error(&entry_name, message));
					continue;
				}
			};

		if decision.action == XmlImportDecisionAction::Skip {
			if let Err(history_err) = record_import_history(
				&ctx,
				&mm,
				&uploaded_file_name,
				&entry_name,
				decision.matched_case_id,
				decision.matched_case_number.as_deref(),
				"skipped",
				decision.message.as_deref(),
			)
			.await
			{
				warn!("failed to record skipped xml import history: {history_err}");
			}
			imported_cases.push(summary_for_skipped_decision(
				&uploaded_file_name,
				&entry_name,
				decision,
			));
			continue;
		}

		imported_cases.push(
			import_single_xml(
				&ctx,
				&mm,
				xml,
				&uploaded_file_name,
				entry_name,
				c_settings,
				decision,
				selected_product.as_ref().map(|(id, _, _)| *id),
				selected_product
					.as_ref()
					.and_then(|(_, product_id, _)| product_id.clone()),
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

#[cfg(test)]
mod tests {
	use super::summary_for_skipped_decision;
	use lib_core::model::xml_import_decision::{
		XmlImportDecision, XmlImportDecisionAction,
	};
	use uuid::Uuid;

	#[test]
	fn skipped_decision_summary_exposes_skip_without_case_id() {
		let matched_case_id = Uuid::from_u128(1);
		let summary = summary_for_skipped_decision(
			"batch.zip",
			"case.xml",
			XmlImportDecision {
				action: XmlImportDecisionAction::Skip,
				matched_case_id: Some(matched_case_id),
				matched_case_number: Some("CASE-1".to_string()),
				matched_case_version: Some(2),
				message: Some("same C.1.1/C.1.2".to_string()),
			},
		);

		assert_eq!(summary.case_number, "CASE-1");
		assert_eq!(summary.status, "skipped");
		assert_eq!(summary.decision, Some("skip"));
		assert_eq!(summary.source_file_name.as_deref(), Some("case.xml"));
		assert_eq!(summary.case_id, None);
		assert_eq!(summary.matched_case_id, Some(matched_case_id.to_string()));
		assert_eq!(summary.matched_case_version, Some(2));
	}
}
