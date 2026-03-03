use crate::web::rest::compliance::{
	capture_e_signature, ComplianceActionInput, ESignatureInput,
};
use axum::http::{header, HeaderMap};
use axum::response::Response;
use lib_core::model::acs::{
	CASE_CREATE, CASE_DELETE, CASE_LIST, CASE_READ, CASE_UPDATE, XML_EXPORT,
};
use lib_core::model::case::{
	Case, CaseBmc, CaseFilter, CaseForCreate, CaseForUpdate,
};
use lib_core::model::drug::DrugInformationBmc;
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::patient::{
	PatientIdentifierBmc, PatientIdentifierFilter, PatientInformationBmc,
};
use lib_core::model::reaction::ReactionBmc;
use lib_core::model::safety_report::{
	PrimarySourceBmc, PrimarySourceFilter, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, StudyInformationBmc,
	StudyInformationFilter,
};
use lib_core::xml::validate::ValidationProfile;
use lib_core::xml::{export_case_xml, validate_e2b_xml, validate_e2b_xml_business};
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{
	ListOptions, OpValString, OpValValue, OpValsString, OpValsValue,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::{Date, Month, OffsetDateTime};
use tokio::runtime::Handle;
use tokio::task;
use uuid::Uuid;

const SYSTEM_VALIDATION_REASON_AUTO: &str =
	"system validation: automatic case status synchronization";
const SYSTEM_VALIDATION_REASON_VALIDATOR: &str =
	"system validation: validator mark-validated endpoint";

// This macro generates all 5 CRUD functions:
// - create_case
// - get_case
// - list_cases
// - update_case
// - delete_case
generate_common_rest_fns! {
	Bmc: CaseBmc,
	Entity: lib_core::model::case::Case,
	ForCreate: CaseForCreate,
	ForUpdate: CaseForUpdate,
	Filter: CaseFilter,
	Suffix: case,
	PermCreate: CASE_CREATE,
	PermRead: CASE_READ,
	PermUpdate: CASE_UPDATE,
	PermDelete: CASE_DELETE,
	PermList: CASE_LIST
}

fn parse_validation_profile_or_bad_request(
	value: &str,
) -> Result<ValidationProfile> {
	ValidationProfile::parse(value).ok_or_else(|| Error::BadRequest {
		message: format!(
			"invalid validation profile '{value}' (expected: ich, fda or mfds)"
		),
	})
}

fn is_valid_case_status(status: &str) -> bool {
	matches!(
		status.trim().to_ascii_lowercase().as_str(),
		"draft" | "checked" | "validated" | "submitted" | "archived" | "nullified"
	)
}

fn validate_case_create_payload(data: &CaseForCreate) -> Result<()> {
	if data.safety_report_id.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "safety_report_id is required".to_string(),
		});
	}

	if let Some(status) = data.status.as_deref() {
		if !is_valid_case_status(status) {
			return Err(Error::BadRequest {
				message: format!("invalid case status '{status}'"),
			});
		}
		if status.eq_ignore_ascii_case("validated") {
			return Err(Error::BadRequest {
				message: "cannot set case to validated manually: status is managed by validator".to_string(),
			});
		}
	}

	if let Some(profile) = data.validation_profile.as_deref() {
		let _ = parse_validation_profile_or_bad_request(profile)?;
	}

	Ok(())
}

fn validate_case_update_payload(data: &CaseForUpdate) -> Result<()> {
	if let Some(safety_report_id) = data.safety_report_id.as_deref() {
		if safety_report_id.trim().is_empty() {
			return Err(Error::BadRequest {
				message: "safety_report_id cannot be empty".to_string(),
			});
		}
	}

	if let Some(status) = data.status.as_deref() {
		if !is_valid_case_status(status) {
			return Err(Error::BadRequest {
				message: format!("invalid case status '{status}'"),
			});
		}
	}

	if let Some(profile) = data.validation_profile.as_deref() {
		let _ = parse_validation_profile_or_bad_request(profile)?;
	}

	Ok(())
}

fn is_allowed_case_status_transition(from: &str, to: &str) -> bool {
	let from = from.trim().to_ascii_lowercase();
	let to = to.trim().to_ascii_lowercase();
	if from == to {
		return true;
	}
	match from.as_str() {
		"" | "draft" => matches!(
			to.as_str(),
			"checked" | "validated" | "submitted" | "archived" | "nullified"
		),
		"checked" => {
			matches!(
				to.as_str(),
				"draft" | "validated" | "submitted" | "archived" | "nullified"
			)
		}
		"validated" => matches!(
			to.as_str(),
			"draft" | "checked" | "submitted" | "archived" | "nullified"
		),
		"submitted" => matches!(to.as_str(), "archived" | "nullified"),
		"archived" => false,
		"nullified" => to == "archived",
		_ => false,
	}
}

#[derive(Debug, Serialize)]
pub struct CaseLifecycleItem {
	pub case_id: Uuid,
	pub version: i32,
	pub status: String,
	pub created_at: String,
	pub updated_at: String,
	pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct CaseLifecycleResult {
	pub safety_report_id: String,
	pub current_case_id: Uuid,
	pub items: Vec<CaseLifecycleItem>,
}

pub async fn create_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<CaseForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;
	let ParamsForCreate { data } = params;
	validate_case_create_payload(&data)?;

	let id = CaseBmc::create(&ctx, &mm, data).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn update_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<CaseUpdateRequest>,
) -> Result<(StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let CaseUpdateRequest {
		data,
		reason_for_change,
		e_signature,
	} = params;
	validate_case_update_payload(&data)?;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	let requested_status = data.status.clone();
	if let Some(next_status) = data.status.as_deref() {
		if !is_allowed_case_status_transition(&current.status, next_status) {
			return Err(Error::BadRequest {
				message: format!(
					"illegal case status transition: '{}' -> '{}'",
					current.status, next_status
				),
			});
		}
	}

	let requires_compliance = requested_status
		.as_deref()
		.map(|next_status| {
			let prev = current.status.trim().to_ascii_lowercase();
			let next = next_status.trim().to_ascii_lowercase();
			prev != next && matches!(next.as_str(), "submitted" | "nullified")
		})
		.unwrap_or(false);

	let ctx_for_update = if requires_compliance {
		let reason = reason_for_change
			.and_then(|v| {
				let trimmed = v.trim().to_string();
				if trimmed.is_empty() { None } else { Some(trimmed) }
			})
			.ok_or(Error::BadRequest {
				message:
					"reason_for_change is required for submitted/nullified status transitions"
						.to_string(),
			})?;
		let e_signature = e_signature.ok_or(Error::BadRequest {
			message:
				"e_signature is required for submitted/nullified status transitions"
					.to_string(),
		})?;
		let compliance = ComplianceActionInput {
			reason_for_change: reason.clone(),
			e_signature,
		};
		let signature_id = capture_e_signature(
			&ctx,
			&mm,
			Some(id),
			"CASE_STATUS_TRANSITION",
			&compliance,
		)
		.await?;
		ctx.with_compliance(Some(reason), Some(signature_id))
	} else {
		ctx.clone()
	};

	CaseBmc::update(&ctx_for_update, &mm, id, data).await?;
	let mut entity = CaseBmc::get(&ctx, &mm, id).await?;

	let status = entity.status.trim().to_ascii_lowercase();
	let auto_manage_status =
		matches!(status.as_str(), "" | "draft" | "checked" | "validated");
	if auto_manage_status {
		let profile = entity
			.validation_profile
			.as_deref()
			.and_then(ValidationProfile::parse)
			.unwrap_or(ValidationProfile::Fda);
		let report = match profile {
			ValidationProfile::Ich => {
				lib_core::xml::ich::validation::validate_case(&ctx, &mm, id).await?
			}
			ValidationProfile::Fda => {
				lib_core::xml::fda::validation::validate_case(&ctx, &mm, id).await?
			}
			ValidationProfile::Mfds => {
				lib_core::xml::mfds::validation::validate_case(&ctx, &mm, id).await?
			}
		};
		let desired = if report.ok { "validated" } else { "checked" };
		if entity.status.trim().to_ascii_lowercase() != desired {
			let system_validation_ctx = ctx.with_compliance(
				Some(SYSTEM_VALIDATION_REASON_AUTO.to_string()),
				None,
			);
			CaseBmc::update(
				&system_validation_ctx,
				&mm,
				id,
				CaseForUpdate {
					safety_report_id: None,
					dg_prd_key: None,
					status: Some(desired.to_string()),
					validation_profile: None,
					submitted_by: None,
					submitted_at: None,
					raw_xml: None,
					dirty_c: None,
					dirty_d: None,
					dirty_e: None,
					dirty_f: None,
					dirty_g: None,
					dirty_h: None,
				},
			)
			.await?;
			entity = CaseBmc::get(&ctx, &mm, id).await?;
		}
	}

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

#[derive(Deserialize)]
pub struct CaseUpdateRequest {
	pub data: CaseForUpdate,
	pub reason_for_change: Option<String>,
	pub e_signature: Option<ESignatureInput>,
}

pub async fn get_case_lifecycle(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<CaseLifecycleResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let mut versions: Vec<Case> = CaseBmc::list(&ctx, &mm, None, None)
		.await?
		.into_iter()
		.filter(|row| row.safety_report_id == case.safety_report_id)
		.collect();
	versions.sort_by(|a, b| a.version.cmp(&b.version));
	let items = versions
		.into_iter()
		.map(|row| CaseLifecycleItem {
			case_id: row.id,
			version: row.version,
			status: row.status,
			created_at: row.created_at.to_string(),
			updated_at: row.updated_at.to_string(),
			is_current: row.id == id,
		})
		.collect();
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseLifecycleResult {
				safety_report_id: case.safety_report_id,
				current_case_id: id,
				items,
			},
		}),
	))
}

pub async fn mark_case_validated_by_validator(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	headers: HeaderMap,
) -> Result<(StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	if !ctx.is_admin() {
		return Err(Error::BadRequest {
			message: "only validator service/admin can mark case validated"
				.to_string(),
		});
	}

	let required_token =
		std::env::var("E2BR3_VALIDATOR_TOKEN").map_err(|_| Error::BadRequest {
			message: "validator token is not configured".to_string(),
		})?;
	let provided_token = headers
		.get("x-validator-token")
		.and_then(|value| value.to_str().ok())
		.unwrap_or_default();
	if provided_token != required_token {
		return Err(Error::BadRequest {
			message: "invalid validator token".to_string(),
		});
	}

	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let profile = case
		.validation_profile
		.as_deref()
		.and_then(ValidationProfile::parse)
		.unwrap_or(ValidationProfile::Fda);
	let report = match profile {
		ValidationProfile::Ich => {
			lib_core::xml::ich::validation::validate_case(&ctx, &mm, id).await?
		}
		ValidationProfile::Fda => {
			lib_core::xml::fda::validation::validate_case(&ctx, &mm, id).await?
		}
		ValidationProfile::Mfds => {
			lib_core::xml::mfds::validation::validate_case(&ctx, &mm, id).await?
		}
	};
	if !report.ok {
		return Err(Error::BadRequest {
			message: format!(
				"validator cannot mark case validated: {} blocking issue(s) remain",
				report.blocking_count
			),
		});
	}

	let validator_ctx = ctx
		.with_compliance(Some(SYSTEM_VALIDATION_REASON_VALIDATOR.to_string()), None);
	CaseBmc::update(
		&validator_ctx,
		&mm,
		id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: Some("validated".to_string()),
			validation_profile: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: None,
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

#[derive(Debug, Deserialize)]
pub struct CaseIntakeCheckInput {
	pub safety_report_id: String,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_most_recent_information: Option<Date>,
	pub report_type: Option<String>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub ae_start_date: Option<Date>,
}

#[derive(Debug, Serialize)]
pub struct CaseIntakeDuplicateMatch {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
	pub status: String,
	pub created_at: String,
	pub report_type: Option<String>,
	pub date_of_most_recent_information: Option<Date>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<Date>,
}

#[derive(Debug, Serialize)]
pub struct CaseIntakeCheckResult {
	pub duplicate: bool,
	pub matches: Vec<CaseIntakeDuplicateMatch>,
}

#[derive(Debug, Deserialize)]
pub struct CaseFromIntakeInput {
	pub safety_report_id: String,
	#[serde(deserialize_with = "lib_core::serde::flex_date::deserialize_date")]
	pub date_of_most_recent_information: Date,
	pub report_type: String,
	pub validation_profile: Option<String>,
	pub status: Option<String>,
	pub allow_duplicate_override: Option<bool>,
	pub reporter_organization: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub patient_initials: Option<String>,
	pub investigation_number: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub ae_start_date: Option<Date>,
}

#[derive(Debug, Serialize)]
pub struct CaseFromIntakeResult {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
}

async fn list_potential_duplicates(
	ctx: &Ctx,
	mm: &ModelManager,
	key: &CaseIntakeCheckInput,
) -> Result<Vec<CaseIntakeDuplicateMatch>> {
	let safety_report_id = key.safety_report_id.trim();
	let cases = cases_by_safety_report_id(ctx, mm, safety_report_id).await?;
	let mut matches = Vec::new();
	for case in cases.into_iter() {
		let safety =
			match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case.id).await
			{
				Ok(data) => Some(data),
				Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
				Err(err) => return Err(err.into()),
			};
		let row_date = safety.as_ref().map(|s| s.date_of_most_recent_information);
		let row_report = safety.as_ref().map(|s| s.report_type.clone());
		let primary_sources = PrimarySourceBmc::list(
			ctx,
			mm,
			Some(vec![PrimarySourceFilter {
				case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case
					.id
					.to_string()))])),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let reporter_organization = primary_sources
			.into_iter()
			.min_by_key(|row| row.sequence_number)
			.and_then(|row| row.organization);
		let study_info = StudyInformationBmc::list(
			ctx,
			mm,
			Some(vec![StudyInformationFilter {
				case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case
					.id
					.to_string()))])),
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let sponsor_study_number = study_info
			.into_iter()
			.next()
			.and_then(|row| row.sponsor_study_number);
		let patient =
			match PatientInformationBmc::get_by_case(ctx, mm, case.id).await {
				Ok(value) => Some(value),
				Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
				Err(err) => return Err(err.into()),
			};
		let patient_initials =
			patient.as_ref().and_then(|p| p.patient_initials.clone());
		let age_d2_2a = patient
			.as_ref()
			.and_then(|p| p.age_at_time_of_onset.map(|v| v.normalize().to_string()));
		let sex_d5 = patient.as_ref().and_then(|p| p.sex.clone());
		let investigation_number = if let Some(patient) = patient.as_ref() {
			let ids = PatientIdentifierBmc::list(
				ctx,
				mm,
				Some(vec![PatientIdentifierFilter {
					patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(
						json!(patient.id.to_string()),
					)])),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			ids.iter()
				.find(|id| id.identifier_type_code.trim() == "4")
				.or_else(|| {
					ids.iter().find(|id| {
						id.identifier_type_code.to_ascii_uppercase().contains("INV")
					})
				})
				.or_else(|| ids.iter().min_by_key(|id| id.sequence_number))
				.map(|id| id.identifier_value.clone())
		} else {
			None
		};
		let dg_prd_key = case.dg_prd_key.clone().or(
			DrugInformationBmc::list_by_case(ctx, mm, case.id)
				.await?
				.into_iter()
				.min_by_key(|row| row.sequence_number)
				.map(|row| row.medicinal_product),
		);
		let reaction = ReactionBmc::list_by_case(ctx, mm, case.id)
			.await?
			.into_iter()
			.min_by_key(|row| row.sequence_number);
		let reaction_meddra_version = reaction
			.as_ref()
			.and_then(|r| r.reaction_meddra_version.clone());
		let reaction_meddra_code = reaction
			.as_ref()
			.and_then(|r| r.reaction_meddra_code.clone());
		let ae_start_date = reaction.as_ref().and_then(|r| r.start_date);

		let date_ok = key
			.date_of_most_recent_information
			.map(|value| row_date == Some(value))
			.unwrap_or(true);
		let report_ok = key
			.report_type
			.as_deref()
			.map(|value| {
				row_report
					.as_deref()
					.map(|v| v.eq_ignore_ascii_case(value))
					.unwrap_or(false)
			})
			.unwrap_or(true);
		let reporter_ok = matches_optional_text(
			key.reporter_organization.as_deref(),
			reporter_organization.as_deref(),
		);
		let study_ok = matches_optional_text(
			key.sponsor_study_number.as_deref(),
			sponsor_study_number.as_deref(),
		);
		let initials_ok = matches_optional_text(
			key.patient_initials.as_deref(),
			patient_initials.as_deref(),
		);
		let investigation_ok = matches_optional_text(
			key.investigation_number.as_deref(),
			investigation_number.as_deref(),
		);
		let age_ok =
			matches_optional_decimal(key.age_d2_2a.as_deref(), age_d2_2a.as_deref());
		let sex_ok = matches_optional_text(key.sex_d5.as_deref(), sex_d5.as_deref());
		let dg_ok =
			matches_optional_text(key.dg_prd_key.as_deref(), dg_prd_key.as_deref());
		let meddra_version_ok = matches_optional_text(
			key.reaction_meddra_version.as_deref(),
			reaction_meddra_version.as_deref(),
		);
		let meddra_code_ok = matches_optional_text(
			key.reaction_meddra_code.as_deref(),
			reaction_meddra_code.as_deref(),
		);
		let ae_start_date_ok = key
			.ae_start_date
			.map(|value| ae_start_date == Some(value))
			.unwrap_or(true);

		if !date_ok
			|| !report_ok
			|| !reporter_ok
			|| !study_ok
			|| !initials_ok
			|| !investigation_ok
			|| !age_ok
			|| !sex_ok
			|| !dg_ok || !meddra_version_ok
			|| !meddra_code_ok
			|| !ae_start_date_ok
		{
			continue;
		}
		matches.push(CaseIntakeDuplicateMatch {
			case_id: case.id,
			safety_report_id: case.safety_report_id,
			version: case.version,
			status: case.status,
			created_at: case.created_at.to_string(),
			report_type: row_report,
			date_of_most_recent_information: row_date,
			reporter_organization,
			sponsor_study_number,
			patient_initials,
			investigation_number,
			age_d2_2a,
			sex_d5,
			dg_prd_key,
			reaction_meddra_version,
			reaction_meddra_code,
			ae_start_date,
		});
	}
	matches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
	matches.truncate(20);

	Ok(matches)
}

fn matches_optional_text(expected: Option<&str>, actual: Option<&str>) -> bool {
	let Some(expected) = expected.map(str::trim).filter(|v| !v.is_empty()) else {
		return true;
	};
	actual
		.map(str::trim)
		.map(|value| value.eq_ignore_ascii_case(expected))
		.unwrap_or(false)
}

fn matches_optional_decimal(expected: Option<&str>, actual: Option<&str>) -> bool {
	let Some(expected) = expected.map(str::trim).filter(|v| !v.is_empty()) else {
		return true;
	};
	let parsed_expected = match expected.parse::<f64>() {
		Ok(value) => value,
		Err(_) => return false,
	};
	let Some(actual) = actual.map(str::trim).filter(|v| !v.is_empty()) else {
		return false;
	};
	match actual.parse::<f64>() {
		Ok(value) => (value - parsed_expected).abs() < f64::EPSILON,
		Err(_) => false,
	}
}

async fn next_case_version(
	ctx: &Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<i32> {
	let max = cases_by_safety_report_id(ctx, mm, safety_report_id)
		.await?
		.into_iter()
		.map(|case| case.version)
		.max()
		.unwrap_or(0);
	Ok(max + 1)
}

async fn cases_by_safety_report_id(
	ctx: &Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<Vec<Case>> {
	CaseBmc::list(
		ctx,
		mm,
		Some(vec![CaseFilter {
			organization_id: None,
			safety_report_id: Some(OpValsString::from(vec![OpValString::Eq(
				safety_report_id.to_string(),
			)])),
			status: None,
		}]),
		Some(ListOptions {
			limit: Some(100),
			offset: None,
			order_bys: Some("version".into()),
		}),
	)
	.await
	.map_err(Into::into)
}

fn message_sender_identifier() -> String {
	std::env::var("E2BR3_DEFAULT_MESSAGE_SENDER")
		.unwrap_or_else(|_| "DSJP".to_string())
}

fn message_receiver_identifier(profile: ValidationProfile) -> String {
	match profile {
		ValidationProfile::Fda => {
			std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA")
				.unwrap_or_else(|_| "CDER".to_string())
		}
		ValidationProfile::Ich => {
			std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH")
				.unwrap_or_else(|_| "ICHTEST".to_string())
		}
		ValidationProfile::Mfds => {
			std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS")
				.unwrap_or_else(|_| "MFDS".to_string())
		}
	}
}

fn format_message_timestamp_utc(now: OffsetDateTime) -> String {
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

/// POST /api/cases/intake-check
/// Checks whether the base intake fields would duplicate an existing case.
pub async fn check_case_intake_duplicate(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<CaseIntakeCheckInput>>,
) -> Result<(StatusCode, Json<DataRestResult<CaseIntakeCheckResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;

	let data = params.data;
	let safety_report_id = data.safety_report_id.trim();
	if safety_report_id.is_empty() {
		return Err(Error::BadRequest {
			message: "safety_report_id is required".to_string(),
		});
	}

	let normalized = CaseIntakeCheckInput {
		safety_report_id: safety_report_id.to_string(),
		date_of_most_recent_information: data.date_of_most_recent_information,
		report_type: data.report_type,
		reporter_organization: data.reporter_organization,
		sponsor_study_number: data.sponsor_study_number,
		patient_initials: data.patient_initials,
		investigation_number: data.investigation_number,
		age_d2_2a: data.age_d2_2a,
		sex_d5: data.sex_d5,
		dg_prd_key: data.dg_prd_key,
		reaction_meddra_version: data.reaction_meddra_version,
		reaction_meddra_code: data.reaction_meddra_code,
		ae_start_date: data.ae_start_date,
	};
	let matches = list_potential_duplicates(&ctx, &mm, &normalized).await?;

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseIntakeCheckResult {
				duplicate: !matches.is_empty(),
				matches,
			},
		}),
	))
}

/// POST /api/cases/from-intake
/// Creates a case from base intake fields after duplicate check passes.
pub async fn create_case_from_intake(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<CaseFromIntakeInput>>,
) -> Result<(StatusCode, Json<DataRestResult<CaseFromIntakeResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;

	let data = params.data;
	let safety_report_id = data.safety_report_id.trim();
	if safety_report_id.is_empty() {
		return Err(Error::BadRequest {
			message: "safety_report_id is required".to_string(),
		});
	}
	if data.report_type.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "report_type is required".to_string(),
		});
	}

	let duplicate_matches = list_potential_duplicates(
		&ctx,
		&mm,
		&CaseIntakeCheckInput {
			safety_report_id: safety_report_id.to_string(),
			date_of_most_recent_information: Some(
				data.date_of_most_recent_information,
			),
			report_type: Some(data.report_type.clone()),
			reporter_organization: data.reporter_organization.clone(),
			sponsor_study_number: data.sponsor_study_number.clone(),
			patient_initials: data.patient_initials.clone(),
			investigation_number: data.investigation_number.clone(),
			age_d2_2a: data.age_d2_2a.clone(),
			sex_d5: data.sex_d5.clone(),
			dg_prd_key: data.dg_prd_key.clone(),
			reaction_meddra_version: data.reaction_meddra_version.clone(),
			reaction_meddra_code: data.reaction_meddra_code.clone(),
			ae_start_date: data.ae_start_date,
		},
	)
	.await?;
	if !duplicate_matches.is_empty() {
		if !data.allow_duplicate_override.unwrap_or(false) {
			return Err(Error::BadRequest {
				message: "duplicate case detected; create is blocked when intake check finds duplicates".to_string(),
			});
		}
	}

	let profile = match data.validation_profile.as_deref() {
		Some(value) => ValidationProfile::parse(value)
			.ok_or_else(|| Error::BadRequest {
				message: format!(
					"invalid validation profile '{value}' (expected: ich, fda or mfds)"
				),
			})?
			.as_str()
			.to_string(),
		None => "fda".to_string(),
	};
	let profile_enum =
		ValidationProfile::parse(&profile).ok_or_else(|| Error::BadRequest {
			message: format!(
			"invalid validation profile '{profile}' (expected: ich, fda or mfds)"
		),
		})?;

	let next_version = next_case_version(&ctx, &mm, safety_report_id).await?;
	let case_create = CaseForCreate {
		organization_id: ctx.organization_id(),
		safety_report_id: safety_report_id.to_string(),
		dg_prd_key: data.dg_prd_key.clone(),
		status: Some(data.status.unwrap_or_else(|| "draft".to_string())),
		validation_profile: Some(profile),
		version: Some(next_version),
	};
	validate_case_create_payload(&case_create)?;
	let case_id = CaseBmc::create(&ctx, &mm, case_create).await?;

	let now = OffsetDateTime::now_utc();
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number: format!("MSG-{case_id}"),
			message_sender_identifier: message_sender_identifier(),
			message_receiver_identifier: message_receiver_identifier(profile_enum),
			message_date: format_message_timestamp_utc(now),
		},
	)
	.await?;

	SafetyReportIdentificationBmc::create(
		&ctx,
		&mm,
		SafetyReportIdentificationForCreate {
			case_id,
			transmission_date: data.date_of_most_recent_information,
			report_type: data.report_type,
			date_first_received_from_source: data.date_of_most_recent_information,
			date_of_most_recent_information: data.date_of_most_recent_information,
			fulfil_expedited_criteria: false,
		},
	)
	.await?;

	Ok((
		StatusCode::CREATED,
		Json(DataRestResult {
			data: CaseFromIntakeResult {
				case_id,
				safety_report_id: safety_report_id.to_string(),
				version: next_version,
			},
		}),
	))
}

pub async fn export_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT)?;
	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let profile = case
		.validation_profile
		.as_deref()
		.and_then(ValidationProfile::parse)
		.unwrap_or(ValidationProfile::Fda);
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
			let first = schema_report
				.errors
				.first()
				.map(|e| e.message.clone())
				.unwrap_or_else(|| "unknown validation error".to_string());
			return Err(Error::BadRequest {
				message: format!(
					"exported XML failed schema/basic validation ({} issue(s)); first: {first}",
					schema_report.errors.len()
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

	let mut response = (StatusCode::OK, xml).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		header::HeaderValue::from_static("application/xml"),
	);
	Ok(response)
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
	if matches!(profile, ValidationProfile::Fda) {
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
