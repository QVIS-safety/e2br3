use crate::runtime_settings;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::authorization::BuiltInIdentityKind;
use lib_core::ctx::Ctx;
use lib_core::model::case_duplicate::{CaseDuplicateBmc, CaseDuplicateKey};
use lib_core::model::presave::ProductPresaveBmc;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::xml_import_decision::{
	decide_xml_import, XmlImportDecision, XmlImportDecisionAction,
	XmlImportDuplicateMatch, XmlImportExistingCase, XmlImportIncomingKey,
};
use lib_core::model::xml_import_history::{
	XmlImportHistoryBmc, XmlImportHistoryStatus,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	with_authorized_import_history_collection, with_authorized_import_history_read,
	with_authorized_xml_import, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serde::Serialize;
use sqlx::FromRow;
use std::collections::HashSet;
use std::io::{Cursor, Read};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;
use xml::import_sections::g_drug::product_links::ProductLinkMapping;
use xml::import_sections::{
	c_safety_report::parse_c_safety_report, d_patient::parse_d_patient,
	e_reaction::parse_e_reactions,
};
use xml::validation::validate_e2b_xml_for_import;
use xml::{
	extract_safety_report_id_from_xml, import_e2b_xml_in_txn, CImportSettings,
	XmlImportRequest, XmlValidationReport,
};
use zip::ZipArchive;

const MAX_XML_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
const MAX_XML_ZIP_ENTRY_BYTES: usize = 25 * 1024 * 1024;
const MAX_XML_ZIP_ENTRIES: usize = 128;
const MAX_XML_ZIP_EXPANDED_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_XML_REQUEST_BYTES: usize = MAX_XML_UPLOAD_BYTES + 64 * 1024;

struct UploadedImportPayload {
	bytes: Vec<u8>,
	filename: Option<String>,
	product_presave_id: Option<Uuid>,
	product_link_mappings: Vec<ProductLinkMapping>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCaseSummary {
	case_number: Option<String>,
	status: XmlImportHistoryStatus,
	message: Option<String>,
	public_error: Option<serde_json::Value>,
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
	public_error: Option<serde_json::Value>,
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
	transmission_date: Option<String>,
}

async fn read_xml_multipart(
	mut multipart: Multipart,
) -> Result<UploadedImportPayload> {
	let mut file_bytes: Option<Vec<u8>> = None;
	let mut filename: Option<String> = None;
	let mut product_presave_id: Option<Uuid> = None;
	let mut product_link_mappings: Option<Vec<ProductLinkMapping>> = None;

	while let Some(field) =
		multipart
			.next_field()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("multipart error: {err}"),
			})? {
		let name = field.name().map(|v| v.to_string());
		if name.as_deref() == Some("productLinkMappings") {
			if product_link_mappings.is_some() {
				return Err(Error::BadRequest {
					message: "Duplicate productLinkMappings field".to_owned(),
				});
			}
			let bytes =
				read_field_limited(field, 64 * 1024, "productLinkMappings").await?;
			product_link_mappings = Some(serde_json::from_slice(&bytes).map_err(|_| Error::BadRequest { message: "productLinkMappings must be an array of sourceProductPresaveId/targetProductPresaveId UUID pairs".to_owned() })?);
			continue;
		}
		if name.as_deref() == Some("file") || name.as_deref() == Some("xml") {
			filename = field.file_name().map(|value| value.to_string());
			file_bytes = Some(
				read_field_limited(field, MAX_XML_UPLOAD_BYTES, "xml upload")
					.await?,
			);
			continue;
		}
		if matches!(name.as_deref(), Some("productId") | Some("product_id")) {
			return Err(Error::BadRequest {
				message:
					"productId is not accepted; select an authorized productPresaveId"
						.to_string(),
			});
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
			continue;
		}
	}

	let bytes = file_bytes.ok_or_else(|| Error::BadRequest {
		message: "missing xml file field".to_string(),
	})?;

	Ok(UploadedImportPayload {
		bytes,
		filename,
		product_presave_id,
		product_link_mappings: match product_link_mappings {
			Some(mappings) => mappings,
			None => Vec::new(),
		},
	})
}

async fn read_field_limited(
	mut field: Field<'_>,
	max_bytes: usize,
	label: &str,
) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	let mut exceeded = false;
	while let Some(chunk) = field.chunk().await.map_err(|err| Error::BadRequest {
		message: format!("multipart read error: {err}"),
	})? {
		if bytes.len().saturating_add(chunk.len()) > max_bytes {
			exceeded = true;
			continue;
		}
		if !exceeded {
			bytes.extend_from_slice(&chunk);
		}
	}
	if exceeded {
		return Err(Error::BadRequest {
			message: format!("{label} exceeds {max_bytes} bytes"),
		});
	}
	Ok(bytes)
}

fn extract_xml_entries(
	bytes: &[u8],
	filename: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
	let looks_like_zip = filename.to_ascii_lowercase().ends_with(".zip");

	if !looks_like_zip {
		if let Ok(zip) = ZipArchive::new(Cursor::new(bytes)) {
			return extract_xml_entries_from_zip(zip);
		}
		return Ok(vec![(filename.to_string(), bytes.to_vec())]);
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
	if zip.len() > MAX_XML_ZIP_ENTRIES {
		return Err(Error::BadRequest {
			message: format!(
				"xml import zip contains more than {MAX_XML_ZIP_ENTRIES} entries"
			),
		});
	}
	let mut entries = Vec::new();
	let mut names = HashSet::new();
	let mut expanded_bytes = 0usize;
	for idx in 0..zip.len() {
		let mut entry = zip.by_index(idx).map_err(|err| Error::BadRequest {
			message: format!("zip read error: {err}"),
		})?;
		let entry_path = entry.enclosed_name().ok_or_else(|| Error::BadRequest {
			message: format!("unsafe zip entry path: {}", entry.name()),
		})?;
		if entry.name().ends_with('/') {
			continue;
		}
		let entry_name = entry_path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| Error::BadRequest {
				message: format!("invalid zip entry name: {}", entry.name()),
			})?
			.to_string();
		if !entry_name.to_ascii_lowercase().ends_with(".xml") {
			continue;
		}
		if !names.insert(entry_name.to_ascii_lowercase()) {
			return Err(Error::BadRequest {
				message: format!("duplicate xml file name in zip: {entry_name}"),
			});
		}

		let entry_bytes = read_zip_entry_limited(
			&mut entry,
			MAX_XML_ZIP_ENTRY_BYTES,
			"xml zip entry",
		)?;
		expanded_bytes =
			expanded_bytes
				.checked_add(entry_bytes.len())
				.ok_or_else(|| Error::BadRequest {
					message: "xml import zip expanded size overflow".to_string(),
				})?;
		if expanded_bytes > MAX_XML_ZIP_EXPANDED_BYTES {
			return Err(Error::BadRequest {
				message: format!(
					"xml import zip exceeds {MAX_XML_ZIP_EXPANDED_BYTES} expanded bytes"
				),
			});
		}
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
	status: XmlImportHistoryStatus,
	error_message: Option<&str>,
	public_error: Option<&serde_json::Value>,
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
		public_error,
	)
	.await
	.map_err(Error::Model)
}

fn public_import_error(error: &Error) -> serde_json::Value {
	let (status, code, detail) =
		lib_web::middleware::mw_res_map::map_rest_error_to_client(error);
	let reference = Uuid::new_v4();
	tracing::error!(error_reference = %reference, error_type = %error.diagnostic_type(), "XML import entry failed");
	serde_json::json!({
		"httpStatus": status.as_u16(),
		"error": { "message": code.as_ref(), "data": { "req_uuid": reference, "detail": detail } }
	})
}

async fn import_single_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: Vec<u8>,
	uploaded_file_name: &str,
	filename: String,
	c_settings: CImportSettings,
	decision: XmlImportDecision,
	product_presave_id: Uuid,
	product_id: String,
	product_link_mappings: Vec<ProductLinkMapping>,
) -> Result<ImportedCaseSummary> {
	let import_mm = mm.new_with_txn().map_err(Error::Model)?;
	import_mm
		.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	let import_result = import_e2b_xml_in_txn(
		ctx,
		&import_mm,
		XmlImportRequest {
			xml,
			c_settings,
			product_presave_id,
			product_id,
			product_link_mappings,
		},
	)
	.await;
	match import_result {
		Ok(result) => {
			let case_number = result.case_number.clone();
			let case_id = match result.case_id.as_deref() {
				Some(value) => match Uuid::parse_str(value) {
					Ok(case_id) => case_id,
					Err(_) => {
						let _ = import_mm.dbx().rollback_txn().await;
						return Err(Error::Service {
							operation: "resolve imported case identity",
						});
					}
				},
				None => {
					let _ = import_mm.dbx().rollback_txn().await;
					return Err(Error::Service {
						operation: "resolve imported case identity",
					});
				}
			};
			if result.skipped {
				let message =
					"Existing case has the same C.1.1 and C.1.2; import skipped."
						.to_string();
				if let Err(error) = XmlImportHistoryBmc::record_in_txn(
					import_mm.dbx(),
					ctx,
					uploaded_file_name,
					&filename,
					Some(case_id),
					case_number.as_deref(),
					XmlImportHistoryStatus::Skipped,
					Some(&message),
					None,
				)
				.await
				{
					let _ = import_mm.dbx().rollback_txn().await;
					return Err(Error::Model(error));
				}
				import_mm
					.dbx()
					.commit_txn()
					.await
					.map_err(lib_core::model::Error::from)
					.map_err(Error::Model)?;
				return Ok(ImportedCaseSummary {
					case_number,
					status: XmlImportHistoryStatus::Skipped,
					message: Some(message),
					public_error: None,
					case_id: None,
					case_version: None,
					decision: Some("skip"),
					source_file_name: Some(filename),
					matched_case_id: Some(case_id.to_string()),
					matched_case_number: result.case_number,
					matched_case_version: result
						.case_version
						.map(i32::try_from)
						.transpose()
						.map_err(|error| Error::BadRequest {
							message: format!("invalid case version: {error}"),
						})?,
				});
			}
			let potential_duplicate = decision.action
				== XmlImportDecisionAction::New
				&& decision.matched_case_id.is_some();
			let status = if potential_duplicate {
				XmlImportHistoryStatus::Warning
			} else {
				XmlImportHistoryStatus::Success
			};
			let message = if potential_duplicate {
				decision.message.clone()
			} else {
				None
			};
			if let Err(error) = XmlImportHistoryBmc::record_in_txn(
				import_mm.dbx(),
				ctx,
				uploaded_file_name,
				&filename,
				Some(case_id),
				result.case_number.as_deref(),
				status,
				if potential_duplicate {
					message.as_deref()
				} else {
					None
				},
				None,
			)
			.await
			{
				let _ = import_mm.dbx().rollback_txn().await;
				return Err(Error::Model(error));
			}
			import_mm
				.dbx()
				.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)
				.map_err(Error::Model)?;
			Ok(ImportedCaseSummary {
				case_number,
				status,
				message,
				public_error: None,
				case_id: result.case_id,
				case_version: result.case_version,
				decision: Some(decision_label(decision.action)),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
			})
		}
		Err(err) => {
			let _ = import_mm.dbx().rollback_txn().await;
			let message = err.to_string();
			let public_error = public_import_error(&Error::Xml(err));
			record_import_history(
				ctx,
				mm,
				uploaded_file_name,
				&filename,
				None,
				None,
				XmlImportHistoryStatus::Error,
				Some(&message),
				Some(&public_error),
			)
			.await?;
			Ok(ImportedCaseSummary {
				case_number: None,
				status: XmlImportHistoryStatus::Error,
				message: None,
				public_error: Some(public_error),
				case_id: None,
				case_version: None,
				decision: Some("error"),
				source_file_name: Some(filename),
				matched_case_id: decision.matched_case_id.map(|id| id.to_string()),
				matched_case_number: decision.matched_case_number,
				matched_case_version: decision.matched_case_version,
			})
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
		case_number: decision.matched_case_number.clone(),
		status: XmlImportHistoryStatus::Skipped,
		message: decision.message.clone(),
		public_error: None,
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
	public_error: serde_json::Value,
) -> ImportedCaseSummary {
	ImportedCaseSummary {
		case_number: None,
		status: XmlImportHistoryStatus::Error,
		message: None,
		public_error: Some(public_error),
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
	product_id: &str,
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
	let first_reaction = reactions.first();
	let dg_prd_key = Some(product_id.trim().to_owned());

	Ok((
		XmlImportIncomingKey {
			safety_report_id,
			transmission_date: c_report.transmission_date,
		},
		CaseDuplicateKey {
			report_type: c_report.report_type,
			reporter_organization: None,
			reporter_organization_null_flavor: None,
			sponsor_study_number: None,
			sponsor_study_number_null_flavor: None,
			patient_initials: patient
				.as_ref()
				.and_then(|patient| patient.patient_initials.clone()),
			patient_initials_null_flavor: None,
			investigation_number: None,
			investigation_number_null_flavor: None,
			age_d2_2a: patient
				.as_ref()
				.and_then(|patient| decimal_string(patient.age_at_time_of_onset)),
			sex_d5: patient.as_ref().and_then(|patient| patient.sex.clone()),
			sex_d5_null_flavor: None,
			dg_prd_key,
			reaction_meddra_version: first_reaction
				.and_then(|reaction| reaction.reaction_meddra_version.clone()),
			reaction_meddra_code: first_reaction
				.and_then(|reaction| reaction.reaction_meddra_code.clone()),
			ae_start_date: first_reaction
				.and_then(|reaction| reaction.start_date.clone()),
			ae_start_date_null_flavor: None,
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
				       s.transmission_date
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
			transmission_date: row.transmission_date,
		})
		.collect())
}

async fn decide_import_entry(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	product_id: &str,
) -> Result<XmlImportDecision> {
	let report = validate_e2b_xml_for_import(xml, None)?;
	if !report.ok {
		return Err(Error::Xml(xml::Error::InvalidXml {
			message: report
				.errors
				.into_iter()
				.map(|error| error.message)
				.collect::<Vec<_>>()
				.join("; "),
			line: None,
			column: None,
		}));
	}

	let (incoming, duplicate_key) = duplicate_key_from_xml(xml, product_id)?;
	let same_report_cases = match incoming
		.safety_report_id
		.as_deref()
		.filter(|value| !value.trim().is_empty())
	{
		Some(id) => list_same_safety_report_cases(ctx, mm, id).await?,
		None => Vec::new(),
	};
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
	let settings = runtime_settings::load(ctx, mm).await?;
	Ok(CImportSettings {
		update_date_of_creation: settings.import_dates.update_date_of_creation,
		update_most_recent_info_date: settings
			.import_dates
			.update_most_recent_info_date,
		update_report_first_received_date: settings
			.import_dates
			.update_report_first_received_date,
		apply_sender_info_to_imported_cases: settings
			.apply_sender_info_to_imported_cases,
		selected_sender_presave_id: None,
		import_date: Some(settings.import_date()?),
	})
}

pub async fn list_import_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportHistoryList>>)> {
	let ctx = ctx_w.0;
	let include_unscoped = matches!(
		snapshot.identity().built_in_kind(),
		Some(
			BuiltInIdentityKind::PlatformAdministrator
				| BuiltInIdentityKind::SponsorCroAdministrator
				| BuiltInIdentityKind::SponsorCompanyAdministrator
		)
	);
	with_authorized_import_history_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm, scope| {
			Box::pin(async move {
				let rows = XmlImportHistoryBmc::list_all_scoped(
					mm,
					ctx,
					scope,
					include_unscoped,
				)
				.await
				.map_err(Error::Model)?;
				let items = rows
					.into_iter()
					.map(|row| {
						let uploaded_at =
							row.uploaded_at.format(&Rfc3339).map_err(|err| {
								Error::BadRequest {
									message: format!(
										"invalid import history timestamp: {err}"
									),
								}
							})?;
						Ok(XmlImportHistoryRecord {
							id: row.id,
							uploaded_file_name: row.uploaded_file_name,
							source_file_name: row.source_file_name,
							case_id: row.case_id,
							case_number: row.case_number,
							error_message: if row.status == "error" {
								None
							} else {
								row.error_message
							},
							status: row.status,
							public_error: row.public_error,
							uploaded_by: row.uploaded_by,
							uploader_email: row.uploader_email,
							uploaded_at,
						})
					})
					.collect::<Result<Vec<_>>>()?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: XmlImportHistoryList { items },
					}),
				))
			})
		},
	)
	.await
}

pub async fn download_import_history_error(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	with_authorized_import_history_read(&ctx, &snapshot, &mm, id, move |ctx, mm| {
		Box::pin(async move {
			let row = XmlImportHistoryBmc::get_error_row(mm, ctx, id)
				.await
				.map_err(Error::Model)?
				.ok_or_else(|| Error::BadRequest {
					message: format!("xml import history record {id} not found"),
				})?;
			let text = public_history_error_text(
				&row.status,
				row.public_error.as_ref(),
				row.error_message.as_deref(),
			)?;
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
		})
	})
	.await
}

fn public_history_error_text(
	status: &str,
	public_error: Option<&serde_json::Value>,
	message: Option<&str>,
) -> Result<String> {
	if status == "error" {
		let public_error = public_error.ok_or_else(|| Error::BadRequest {
			message:
				"This historical import error has no structured public details."
					.into(),
		})?;
		return Ok(serde_json::to_string_pretty(public_error)?);
	}
	message.map(str::to_owned).ok_or_else(|| Error::BadRequest {
		message: "This import history record has no error details.".into(),
	})
}

/// POST /api/import/xml/validate
/// Validates E2B(R3) XML payload (XSD-only for now)
pub async fn validate_xml(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlValidationReport>>)> {
	let ctx = ctx_w.0;
	with_authorized_xml_import(
		&ctx,
		&snapshot,
		&mm,
		"import.xml.validate",
		"validate",
		move |_ctx, _mm, _scope| {
			Box::pin(async move {
				let payload = read_xml_multipart(multipart).await?;
				let xml = payload.bytes;
				let report = validate_e2b_xml_for_import(&xml, None)?;
				Ok((StatusCode::OK, Json(DataRestResult { data: report })))
			})
		},
	)
	.await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductLinkPreview {
	source_product_presave_id: Uuid,
	drug_names: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductLinkPreviewResult {
	links: Vec<ProductLinkPreview>,
}

fn preview_product_links(
	bytes: &[u8],
	filename: &str,
) -> Result<ProductLinkPreviewResult> {
	use xml::import_sections::g_drug::{
		parse_g_drugs, product_links::SourceProductLink,
	};
	let mut links: Vec<ProductLinkPreview> = Vec::new();
	for (_, xml) in extract_xml_entries(bytes, filename)? {
		for drug in parse_g_drugs(&xml)? {
			if let SourceProductLink::Linked(source_product_presave_id) =
				drug.source_product_link
			{
				if let Some(link) = links.iter_mut().find(|link| {
					link.source_product_presave_id == source_product_presave_id
				}) {
					link.drug_names.push(drug.medicinal_product);
				} else {
					links.push(ProductLinkPreview {
						source_product_presave_id,
						drug_names: vec![drug.medicinal_product],
					});
				}
			}
		}
	}
	Ok(ProductLinkPreviewResult { links })
}

pub async fn inspect_product_links(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	multipart: Multipart,
) -> Result<Json<DataRestResult<ProductLinkPreviewResult>>> {
	with_authorized_xml_import(
		&ctx_w.0,
		&snapshot,
		&mm,
		"import.xml.execute",
		"inspect-product-links",
		move |_ctx, _mm, _scope| {
			Box::pin(async move {
				let payload = read_xml_multipart(multipart).await?;
				let filename =
					payload.filename.ok_or_else(|| Error::BadRequest {
						message: "uploaded file name is required for XML import"
							.to_owned(),
					})?;
				Ok(Json(DataRestResult {
					data: preview_product_links(&payload.bytes, &filename)?,
				}))
			})
		},
	)
	.await
}

/// POST /api/import/xml
/// Parse + import E2B(R3) XML (pipeline WIP)
pub async fn import_xml(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportBatchResult>>)> {
	let ctx = ctx_w.0;
	with_authorized_xml_import(
		&ctx,
		&snapshot,
		&mm,
		"import.xml.execute",
		"execute",
		move |ctx, mm, scope| {
			Box::pin(async move {
				import_xml_authorized(ctx, mm, scope, multipart).await
			})
		},
	)
	.await
}

async fn import_xml_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	scope: &lib_core::authorization::EnforcedScopeFilter,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<XmlImportBatchResult>>)> {
	let payload = read_xml_multipart(multipart).await?;
	let mut mapped_sources = HashSet::new();
	for mapping in &payload.product_link_mappings {
		if mapping.source_product_presave_id.is_nil()
			|| mapping.target_product_presave_id.is_nil()
			|| !mapped_sources.insert(mapping.source_product_presave_id)
		{
			return Err(Error::BadRequest { message: "Product link mappings require unique non-nil source UUIDs and non-nil target UUIDs".to_owned() });
		}
		let mapped =
			ProductPresaveBmc::get(ctx, mm, mapping.target_product_presave_id)
				.await
				.map_err(Error::Model)?;
		if mapped.deleted || mapped.organization_id != ctx.organization_id() {
			return Err(Error::BadRequest { message: "Mapped products must be active and belong to the importing organization".to_owned() });
		}
		if !super::section_presave_rest::product_presave_allowed(scope, &mapped) {
			return Err(Error::PermissionDenied {
				required_permission: "info.read product scope".to_owned(),
			});
		}
	}
	let product_presave_id =
		payload
			.product_presave_id
			.ok_or_else(|| Error::BadRequest {
				message: "productPresaveId is required for XML import".to_string(),
			})?;
	let product = ProductPresaveBmc::get(ctx, mm, product_presave_id)
		.await
		.map_err(Error::Model)?;
	if product.deleted {
		return Err(Error::BadRequest {
			message: "selected Product is deleted".to_string(),
		});
	}
	if !super::section_presave_rest::product_presave_allowed(scope, &product) {
		return Err(Error::PermissionDenied {
			required_permission: "info.read product scope".to_string(),
		});
	}
	let selected_product_id =
		product.product_id.ok_or_else(|| Error::BadRequest {
			message: "selected Product has no Product ID".to_string(),
		})?;
	let selected_product_id = selected_product_id.trim();
	if selected_product_id.is_empty() {
		return Err(Error::BadRequest {
			message: "selected Product has no Product ID".to_string(),
		});
	}
	let selected_product = (
		product_presave_id,
		selected_product_id.to_string(),
		product.sender_presave_id,
	);
	let uploaded_file_name = payload.filename.ok_or_else(|| Error::BadRequest {
		message: "uploaded file name is required for XML import".to_string(),
	})?;
	if uploaded_file_name.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "uploaded file name is required for XML import".to_string(),
		});
	}
	let entries = extract_xml_entries(&payload.bytes, &uploaded_file_name)?;
	if !payload.product_link_mappings.is_empty() {
		let referenced_sources =
			preview_product_links(&payload.bytes, &uploaded_file_name)?
				.links
				.into_iter()
				.map(|link| link.source_product_presave_id)
				.collect::<HashSet<_>>();
		if mapped_sources != referenced_sources {
			return Err(Error::BadRequest { message: "Product link mappings must exactly cover the uploaded source references".to_owned() });
		}
	}
	let mut imported_cases = Vec::with_capacity(entries.len());
	let mut c_settings = load_import_settings(ctx, mm).await?;
	c_settings.selected_sender_presave_id = selected_product.2;
	let effective_product_id = selected_product.1.as_str();

	for (entry_name, xml) in entries {
		let decision =
			match decide_import_entry(ctx, mm, &xml, effective_product_id).await {
				Ok(decision) => decision,
				Err(err) => {
					let message = err.to_string();
					let public_error = public_import_error(&err);
					record_import_history(
						ctx,
						mm,
						&uploaded_file_name,
						&entry_name,
						None,
						None,
						XmlImportHistoryStatus::Error,
						Some(&message),
						Some(&public_error),
					)
					.await?;
					imported_cases
						.push(summary_for_decision_error(&entry_name, public_error));
					continue;
				}
			};

		if decision.action == XmlImportDecisionAction::Skip {
			record_import_history(
				ctx,
				mm,
				&uploaded_file_name,
				&entry_name,
				decision.matched_case_id,
				decision.matched_case_number.as_deref(),
				XmlImportHistoryStatus::Skipped,
				decision.message.as_deref(),
				None,
			)
			.await?;
			imported_cases.push(summary_for_skipped_decision(
				&uploaded_file_name,
				&entry_name,
				decision,
			));
			continue;
		}

		imported_cases.push(
			import_single_xml(
				ctx,
				mm,
				xml,
				&uploaded_file_name,
				entry_name,
				c_settings,
				decision,
				selected_product.0,
				selected_product.1.clone(),
				payload.product_link_mappings.clone(),
			)
			.await?,
		);
	}

	let first_success = imported_cases.iter().find(|item| {
		matches!(
			item.status,
			XmlImportHistoryStatus::Success | XmlImportHistoryStatus::Warning
		)
	});
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
	#[test]
	fn product_link_preview_groups_real_xml_and_zip_references_without_inference() {
		let source = "00000000-0000-0000-0000-000000000042";
		let drug = "00000000-0000-0000-0000-000000000001";
		let xml = format!(
			r#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><subjectOf2><organizer><code code="4" codeSystem="2.16.840.1.113883.3.989.2.1.1.20"/><component><substanceAdministration><?e2br3-product-link-v1 {source}?><id root="{drug}"/><consumable><instanceOfKind><kindOfProduct><name>Existing fixture</name></kindOfProduct></instanceOfKind></consumable></substanceAdministration></component></organizer></subjectOf2><component><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/><subject2><productUseReference><id root="{drug}"/></productUseReference></subject2></causalityAssessment></component></MCCI_IN200100UV01>"#
		);
		let direct = super::preview_product_links(xml.as_bytes(), "case.xml")
			.expect("XML preview");
		assert_eq!(direct.links.len(), 1);
		assert_eq!(
			direct.links[0].source_product_presave_id.to_string(),
			source
		);
		assert_eq!(direct.links[0].drug_names, vec!["Existing fixture"]);
		let zip = zip_with(&[("a.xml", xml.as_bytes()), ("b.xml", xml.as_bytes())]);
		let batch =
			super::preview_product_links(&zip, "cases.zip").expect("ZIP preview");
		assert_eq!(batch.links.len(), 1);
		assert_eq!(batch.links[0].drug_names.len(), 2);
		assert!(super::preview_product_links(
			xml.replace(&format!("<?e2br3-product-link-v1 {source}?>"), "")
				.as_bytes(),
			"external.xml"
		)
		.expect("legacy XML")
		.links
		.is_empty());
		assert!(super::preview_product_links(
			xml.replace(source, "invalid").as_bytes(),
			"invalid.xml"
		)
		.is_err());
	}

	#[test]
	fn public_import_errors_hide_private_diagnostics_and_keep_classification() {
		use super::{public_history_error_text, public_import_error, Error};
		for (error, status, code) in [
			(
				Error::Xml(xml::Error::InvalidXml {
					message: "PRIVATE_XML_DETAIL".into(),
					line: Some(1),
					column: None,
				}),
				400,
				"INVALID_REQUEST",
			),
			(
				Error::Xml(xml::Error::Io(std::io::Error::other("PRIVATE_PATH"))),
				500,
				"SERVICE_ERROR",
			),
			(
				Error::Xml(xml::Error::Model(
					lib_core::model::Error::CaseStateConflict,
				)),
				409,
				"CASE_STATE_CONFLICT",
			),
		] {
			let public = public_import_error(&error);
			assert_eq!(public["httpStatus"], status);
			assert_eq!(public["error"]["message"], code);
			let reference = public["error"]["data"]["req_uuid"]
				.as_str()
				.expect("reference");
			uuid::Uuid::parse_str(reference).expect("valid reference");
			assert!(!public.to_string().contains("PRIVATE_"));
			let download = public_history_error_text(
				"error",
				Some(&public),
				Some("PRIVATE_STORED_DIAGNOSTIC"),
			)
			.expect("public download");
			assert_eq!(
				serde_json::from_str::<serde_json::Value>(&download).expect("JSON"),
				public
			);
			assert!(!download.contains("PRIVATE_"));
		}
		assert!(public_history_error_text(
			"error",
			None,
			Some("PRIVATE_LEGACY_ERROR")
		)
		.is_err());
	}

	use super::{
		extract_xml_entries, summary_for_skipped_decision, XmlImportHistoryStatus,
	};
	use lib_core::model::xml_import_decision::{
		XmlImportDecision, XmlImportDecisionAction,
	};
	use uuid::Uuid;
	use xml::import_sections::g_drug::parse_g_drugs;
	use zip::write::SimpleFileOptions;
	use zip::ZipWriter;

	fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
		use std::io::{Cursor, Write};

		let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
		for (name, value) in entries {
			writer
				.start_file(*name, SimpleFileOptions::default())
				.expect("start zip entry");
			writer.write_all(value).expect("write zip entry");
		}
		writer.finish().expect("finish zip").into_inner()
	}

	#[test]
	fn zip_import_rejects_unsafe_and_duplicate_xml_names() {
		let unsafe_zip = zip_with(&[("../case.xml", b"<xml/>")]);
		assert!(extract_xml_entries(&unsafe_zip, "cases.zip")
			.unwrap_err()
			.to_string()
			.contains("unsafe zip entry path"));

		let duplicate_zip =
			zip_with(&[("one/case.xml", b"<one/>"), ("two/CASE.XML", b"<two/>")]);
		assert!(extract_xml_entries(&duplicate_zip, "cases.zip")
			.unwrap_err()
			.to_string()
			.contains("duplicate xml file name"));
	}

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

		assert_eq!(summary.case_number.as_deref(), Some("CASE-1"));
		assert_eq!(summary.status, XmlImportHistoryStatus::Skipped);
		assert_eq!(summary.decision, Some("skip"));
		assert_eq!(summary.source_file_name.as_deref(), Some("case.xml"));
		assert_eq!(summary.case_id, None);
		assert_eq!(summary.matched_case_id, Some(matched_case_id.to_string()));
		assert_eq!(summary.matched_case_version, Some(2));
	}

	#[test]
	fn official_fda_scenario_6_contains_blinded_product_data() {
		let xml = include_bytes!(
			"../../../../../../docs/exporter/fda/FAERS2022Scenario6.xml"
		);
		let drugs = parse_g_drugs(xml).expect("parse official FDA scenario 6");
		assert!(drugs
			.iter()
			.any(|drug| drug.investigational_product_blinded == Some(true)));
	}
}
