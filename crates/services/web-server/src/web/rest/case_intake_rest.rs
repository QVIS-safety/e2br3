use axum::extract::State;
use axum::Json;
use lib_core::model::case::{CaseBmc, CaseForCreate as InternalCaseForCreate};
use lib_core::model::case_duplicate::{
	assess_duplicate_basis, CaseDuplicateBmc, CaseDuplicateKey,
	CaseIntakeDuplicateMatch, DuplicateBasisAssessment,
};
use lib_core::model::case_numbering::generate_case_number;
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::patient::{
	PatientIdentifierBmc, PatientIdentifierForCreate, PatientInformationBmc,
	PatientInformationForCreate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SafetyReportIdentificationForCreate,
};
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::ConstraintViolation;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::web::rest::case_export_rest::{
	message_receiver_identifier, message_sender_identifier,
};
use crate::web::rest::case_rest::validate_case_create_payload;

// -- Types

#[derive(Debug, Deserialize)]
pub struct CaseIntakeCheckInput {
	pub safety_report_id: Option<String>,
	pub date_of_most_recent_information: Option<String>,
	pub report_type: Option<String>,
	pub reporter_organization: Option<String>,
	pub reporter_organization_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub investigation_number: Option<String>,
	pub investigation_number_null_flavor: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub sex_d5_null_flavor: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<String>,
	pub ae_start_date_null_flavor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaseIntakeCheckResult {
	pub duplicate: bool,
	pub basis_complete: bool,
	pub warnings: Vec<String>,
	pub matches: Vec<CaseIntakeDuplicateMatch>,
}

#[derive(Debug, Deserialize)]
pub struct CaseFromIntakeInput {
	pub authority: RegulatoryAuthority,
	pub safety_report_id: Option<String>,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub transmission_date: Option<Date>,
	pub date_first_received_from_source: Option<String>,
	pub date_of_most_recent_information: String,
	pub report_type: String,
	pub status: Option<String>,
	pub allow_duplicate_override: Option<bool>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
	pub reporter_organization: Option<String>,
	pub reporter_organization_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub investigation_number: Option<String>,
	pub investigation_number_null_flavor: Option<String>,
	pub age_d2_2a: Option<String>,
	pub sex_d5: Option<String>,
	pub sex_d5_null_flavor: Option<String>,
	pub dg_prd_key: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub ae_start_date: Option<String>,
	pub ae_start_date_null_flavor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaseFromIntakeResult {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
}

// -- Input normalization

fn normalize_optional_text(value: Option<String>) -> Option<String> {
	value.and_then(|raw| {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			return None;
		}
		Some(trimmed.to_string())
	})
}

fn to_duplicate_key(input: &CaseIntakeCheckInput) -> CaseDuplicateKey {
	CaseDuplicateKey {
		report_type: input.report_type.clone(),
		reporter_organization: input.reporter_organization.clone(),
		reporter_organization_null_flavor: input
			.reporter_organization_null_flavor
			.clone(),
		sponsor_study_number: input.sponsor_study_number.clone(),
		sponsor_study_number_null_flavor: input
			.sponsor_study_number_null_flavor
			.clone(),
		patient_initials: input.patient_initials.clone(),
		patient_initials_null_flavor: input.patient_initials_null_flavor.clone(),
		investigation_number: input.investigation_number.clone(),
		investigation_number_null_flavor: input
			.investigation_number_null_flavor
			.clone(),
		age_d2_2a: input.age_d2_2a.clone(),
		sex_d5: input.sex_d5.clone(),
		sex_d5_null_flavor: input.sex_d5_null_flavor.clone(),
		dg_prd_key: input.dg_prd_key.clone(),
		reaction_meddra_version: input.reaction_meddra_version.clone(),
		reaction_meddra_code: input.reaction_meddra_code.clone(),
		ae_start_date: input.ae_start_date.clone(),
		ae_start_date_null_flavor: input.ae_start_date_null_flavor.clone(),
	}
}

fn normalize_intake_check_input(data: CaseIntakeCheckInput) -> CaseIntakeCheckInput {
	CaseIntakeCheckInput {
		safety_report_id: data
			.safety_report_id
			.map(|value| value.trim().to_string()),
		date_of_most_recent_information: normalize_optional_text(
			data.date_of_most_recent_information,
		),
		report_type: normalize_optional_text(data.report_type),
		reporter_organization: normalize_optional_text(data.reporter_organization),
		reporter_organization_null_flavor: normalize_optional_text(
			data.reporter_organization_null_flavor,
		),
		sponsor_study_number: normalize_optional_text(data.sponsor_study_number),
		sponsor_study_number_null_flavor: normalize_optional_text(
			data.sponsor_study_number_null_flavor,
		),
		patient_initials: normalize_optional_text(data.patient_initials),
		patient_initials_null_flavor: normalize_optional_text(
			data.patient_initials_null_flavor,
		),
		investigation_number: normalize_optional_text(data.investigation_number),
		investigation_number_null_flavor: normalize_optional_text(
			data.investigation_number_null_flavor,
		),
		age_d2_2a: normalize_optional_text(data.age_d2_2a),
		sex_d5: normalize_optional_text(data.sex_d5),
		sex_d5_null_flavor: normalize_optional_text(data.sex_d5_null_flavor),
		dg_prd_key: normalize_optional_text(data.dg_prd_key),
		reaction_meddra_version: normalize_optional_text(
			data.reaction_meddra_version,
		),
		reaction_meddra_code: normalize_optional_text(data.reaction_meddra_code),
		ae_start_date: normalize_optional_text(data.ae_start_date),
		ae_start_date_null_flavor: normalize_optional_text(
			data.ae_start_date_null_flavor,
		),
	}
}

fn normalized_from_intake(data: &CaseFromIntakeInput) -> CaseIntakeCheckInput {
	normalize_intake_check_input(CaseIntakeCheckInput {
		safety_report_id: data.safety_report_id.clone(),
		date_of_most_recent_information: Some(
			data.date_of_most_recent_information.clone(),
		),
		report_type: Some(data.report_type.clone()),
		reporter_organization: data.reporter_organization.clone(),
		reporter_organization_null_flavor: data
			.reporter_organization_null_flavor
			.clone(),
		sponsor_study_number: data.sponsor_study_number.clone(),
		sponsor_study_number_null_flavor: data
			.sponsor_study_number_null_flavor
			.clone(),
		patient_initials: data.patient_initials.clone(),
		patient_initials_null_flavor: data.patient_initials_null_flavor.clone(),
		investigation_number: data.investigation_number.clone(),
		investigation_number_null_flavor: data
			.investigation_number_null_flavor
			.clone(),
		age_d2_2a: data.age_d2_2a.clone(),
		sex_d5: data.sex_d5.clone(),
		sex_d5_null_flavor: data.sex_d5_null_flavor.clone(),
		dg_prd_key: data.dg_prd_key.clone(),
		reaction_meddra_version: data.reaction_meddra_version.clone(),
		reaction_meddra_code: data.reaction_meddra_code.clone(),
		ae_start_date: data.ae_start_date.clone(),
		ae_start_date_null_flavor: data.ae_start_date_null_flavor.clone(),
	})
}

fn intake_to_duplicate_key(data: &CaseFromIntakeInput) -> CaseDuplicateKey {
	to_duplicate_key(&normalized_from_intake(data))
}

fn non_empty(input: Option<&str>) -> Option<String> {
	input
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.map(ToOwned::to_owned)
}

fn validate_intake_pair(
	value_present: bool,
	value: Option<&str>,
	null_flavor: Option<&str>,
	null_flavor_path: &str,
	rule_code: &'static str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<()> {
	let null_flavor = null_flavor.map(str::trim).filter(|value| !value.is_empty());
	if value.map(str::trim).is_some_and(|value| {
		check(input_contracts::FieldInput::new(
			input_contracts::InputValue::Missing,
			Some(value),
		))
		.is_empty()
	}) {
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: rule_code.to_string(),
			path: null_flavor_path.trim_end_matches("NullFlavor").to_string(),
			message: "NullFlavor must be sent in its companion field".to_string(),
		}));
	}
	if value_present && null_flavor.is_some() {
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: rule_code.to_string(),
			path: null_flavor_path.to_string(),
			message: "value and NullFlavor cannot both be set".to_string(),
		}));
	}
	let Some(null_flavor) = null_flavor else {
		return Ok(());
	};
	if let Some(issue) = check(input_contracts::FieldInput::new(
		input_contracts::InputValue::Missing,
		Some(null_flavor),
	))
	.into_iter()
	.next()
	{
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: issue.code.to_string(),
			path: null_flavor_path.to_string(),
			message: issue.message,
		}));
	}
	Ok(())
}

fn validate_intake_value(
	value: Option<&str>,
	path: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<()> {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(());
	};
	if let Some(issue) = check(input_contracts::FieldInput::new(
		input_contracts::InputValue::String(value),
		None,
	))
	.into_iter()
	.next()
	{
		return Err(Error::ConstraintViolation(ConstraintViolation {
			rule_code: issue.code.to_string(),
			path: path.to_string(),
			message: issue.message,
		}));
	}
	Ok(())
}

fn validate_intake_pairs(data: &CaseIntakeCheckInput) -> Result<()> {
	validate_intake_value(
		data.date_of_most_recent_information.as_deref(),
		"safetyReportIdentification.dateOfMostRecentInformation",
		input_contracts::generated::c::c_1_5,
	)?;
	validate_intake_pair(
		data.reporter_organization.is_some(),
		data.reporter_organization.as_deref(),
		data.reporter_organization_null_flavor.as_deref(),
		"primarySources[].reporterOrganizationNullFlavor",
		"ICH.C.2.r.2.1.NULLFLAVOR.ALLOWED",
		input_contracts::generated::c::c_2_r_2_1,
	)?;
	validate_intake_pair(
		data.sponsor_study_number.is_some(),
		data.sponsor_study_number.as_deref(),
		data.sponsor_study_number_null_flavor.as_deref(),
		"studyInformation.sponsorStudyNumberNullFlavor",
		"ICH.C.5.3.NULLFLAVOR.ALLOWED",
		input_contracts::generated::c::c_5_3,
	)?;
	validate_intake_pair(
		data.patient_initials.is_some(),
		data.patient_initials.as_deref(),
		data.patient_initials_null_flavor.as_deref(),
		"patientInformation.patientInitialsNullFlavor",
		"ICH.D.1.NULLFLAVOR.ALLOWED",
		input_contracts::generated::d::d_1,
	)?;
	validate_intake_pair(
		data.investigation_number.is_some(),
		data.investigation_number.as_deref(),
		data.investigation_number_null_flavor.as_deref(),
		"patientInformation.investigationNumberNullFlavor",
		"ICH.D.1.1.4.NULLFLAVOR.ALLOWED",
		input_contracts::generated::d::d_1_1_4,
	)?;
	validate_intake_pair(
		data.sex_d5.is_some(),
		data.sex_d5.as_deref(),
		data.sex_d5_null_flavor.as_deref(),
		"patientInformation.patientSexNullFlavor",
		"ICH.D.5.NULLFLAVOR.ALLOWED",
		input_contracts::generated::d::d_5,
	)?;
	validate_intake_pair(
		data.ae_start_date.is_some(),
		data.ae_start_date.as_deref(),
		data.ae_start_date_null_flavor.as_deref(),
		"reactions[].reactionStartDateNullFlavor",
		"ICH.E.i.4.NULLFLAVOR.ALLOWED",
		input_contracts::generated::e::e_i_4,
	)?;
	Ok(())
}

async fn assess_intake_duplicates(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	key: &CaseDuplicateKey,
) -> Result<(DuplicateBasisAssessment, Vec<CaseIntakeDuplicateMatch>)> {
	let assessment = assess_duplicate_basis(key);
	if !assessment.basis_complete {
		return Ok((assessment, Vec::new()));
	}
	let matches = CaseDuplicateBmc::list_potential_matches(ctx, mm, key)
		.await
		.map_err(Error::Model)?;
	Ok((assessment, matches))
}

async fn next_case_version(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<i32> {
	Ok(
		SafetyReportIdentificationBmc::max_version_by_safety_report_id(
			ctx,
			mm,
			safety_report_id,
		)
		.await
		.map_err(Error::Model)?
			+ 1,
	)
}

// -- Handlers

/// POST /api/cases/intake-check
pub async fn check_case_intake_duplicate(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<CaseIntakeCheckInput>>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseIntakeCheckResult>>,
)> {
	let ctx = ctx_w.0;
	let product_key = params.data.dg_prd_key.clone();
	lib_rest_core::with_authorized_case_create(
		&ctx,
		&snapshot,
		&mm,
		product_key.as_deref(),
		move |ctx, mm| {
			Box::pin(async move {
				let data = normalize_intake_check_input(params.data);
				validate_intake_pairs(&data)?;
				let key = to_duplicate_key(&data);
				let (assessment, matches) =
					assess_intake_duplicates(ctx, mm, &key).await?;

				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult {
						data: CaseIntakeCheckResult {
							duplicate: !matches.is_empty(),
							basis_complete: assessment.basis_complete,
							warnings: assessment.warnings,
							matches,
						},
					}),
				))
			})
		},
	)
	.await
}

/// POST /api/cases/from-intake
pub async fn create_case_from_intake(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<CaseFromIntakeInput>>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFromIntakeResult>>,
)> {
	let ctx = ctx_w.0;
	let product_key = params.data.dg_prd_key.clone();
	lib_rest_core::with_authorized_case_create(
		&ctx,
		&snapshot,
		&mm,
		product_key.as_deref(),
		move |ctx, mm| {
			Box::pin(create_case_from_intake_authorized(ctx, mm, params.data))
		},
	)
	.await
}

async fn create_case_from_intake_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	data: CaseFromIntakeInput,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFromIntakeResult>>,
)> {
	let tx_mm = mm.new_with_txn().map_err(Error::Model)?;
	let dbx = tx_mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	if let Err(error) = set_full_context_from_ctx_dbx(dbx, ctx).await {
		let _ = dbx.rollback_txn().await;
		return Err(Error::Model(error));
	}

	let result = create_case_from_intake_in_txn(ctx, &tx_mm, data).await;
	match result {
		Ok(response) => {
			dbx.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)
				.map_err(Error::Model)?;
			Ok(response)
		}
		Err(error) => {
			let _ = dbx.rollback_txn().await;
			Err(error)
		}
	}
}

async fn create_case_from_intake_in_txn(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	data: CaseFromIntakeInput,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFromIntakeResult>>,
)> {
	use sqlx::types::time::OffsetDateTime;

	let mut safety_report_id = data.safety_report_id.clone().unwrap_or_default();
	validate_intake_value(
		data.date_first_received_from_source.as_deref(),
		"safetyReportIdentification.dateFirstReceivedFromSource",
		input_contracts::generated::c::c_1_4,
	)?;
	validate_intake_pairs(&normalized_from_intake(&data))?;
	if data.report_type.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "report_type is required".to_string(),
		});
	}
	let dg_prd_key =
		non_empty(data.dg_prd_key.as_deref()).ok_or_else(|| Error::BadRequest {
			message: "Product ID is required".to_string(),
		})?;

	let duplicate_key = intake_to_duplicate_key(&data);
	let (duplicate_basis, duplicate_matches) =
		assess_intake_duplicates(ctx, mm, &duplicate_key).await?;
	if !duplicate_matches.is_empty()
		&& !data.allow_duplicate_override.unwrap_or(false)
	{
		return Err(Error::BadRequest {
			message:
				"duplicate case detected; create is blocked when intake check finds duplicates"
					.to_string(),
		});
	}
	if !duplicate_basis.basis_complete
		&& !data.allow_duplicate_override.unwrap_or(false)
	{
		let message = duplicate_basis
			.warnings
			.first()
			.cloned()
			.unwrap_or_else(|| {
				"Some fields needed for the duplicate check are missing. Review the form before creating this case.".to_string()
			});
		return Err(Error::BadRequest { message });
	}

	let profile_enum = data.authority;
	let message_sender_identifier = message_sender_identifier()?;
	let message_receiver_identifier = message_receiver_identifier(profile_enum)?;
	let batch_sender_identifier = message_sender_identifier.clone();
	let batch_receiver_identifier = message_receiver_identifier.clone();

	let generated_case_number = if safety_report_id.trim().is_empty() {
		let generated = generate_case_number(ctx, mm).await.map_err(Error::Model)?;
		safety_report_id = generated.safety_report_id.clone();
		Some(generated)
	} else {
		safety_report_id = safety_report_id.trim().to_string();
		None
	};
	let next_version = next_case_version(ctx, mm, &safety_report_id).await?;
	let case_create = InternalCaseForCreate {
		organization_id: ctx.organization_id(),
		dg_prd_key: Some(dg_prd_key),
		status: Some(data.status.unwrap_or_else(|| "draft".to_string())),
		review_receivers_json: None,
		workflow_routes_json: None,
		mfds_report_type: data.mfds_report_type.clone(),
		fda_report_type: data.fda_report_type.clone(),
		report_year: data.report_year.clone(),
	};
	validate_case_create_payload(&case_create)?;
	let case_id = CaseBmc::create(ctx, mm, case_create).await?;
	let date_of_most_recent_information =
		non_empty(Some(&data.date_of_most_recent_information)).ok_or_else(|| {
			Error::BadRequest {
				message: "date_of_most_recent_information is required".to_string(),
			}
		})?;
	let transmission_date = data.transmission_date.unwrap_or_else(|| {
		lib_core::serde::flex_date::e2b_datetime_date(
			&date_of_most_recent_information,
		)
		.unwrap_or_else(|| OffsetDateTime::now_utc().date())
	});
	let date_first_received_from_source = data
		.date_first_received_from_source
		.as_deref()
		.and_then(|value| non_empty(Some(value)))
		.unwrap_or_else(|| date_of_most_recent_information.clone());

	let now = OffsetDateTime::now_utc();
	MessageHeaderBmc::create(
		ctx,
		mm,
		MessageHeaderForCreate {
			case_id,
			batch_sender_identifier: Some(batch_sender_identifier),
			batch_receiver_identifier: Some(batch_receiver_identifier),
			batch_transmission_date: Some(now),
			message_number: format!("MSG-{case_id}"),
			message_sender_identifier,
			message_receiver_identifier,
			message_date: lib_utils::time::format_e2b_timestamp(now),
		},
	)
	.await?;

	SafetyReportIdentificationBmc::create(
		ctx,
		mm,
		SafetyReportIdentificationForCreate {
			case_id,
			safety_report_id: Some(safety_report_id.clone()),
			version: Some(next_version),
			transmission_date: Some(format_e2b_datetime(transmission_date)),
			report_type: Some(data.report_type),
			date_first_received_from_source: Some(date_first_received_from_source),
			date_of_most_recent_information: Some(date_of_most_recent_information),
			fulfil_expedited_criteria: Some(false),
			fulfil_expedited_criteria_null_flavor: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			first_sender_type: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			combination_product_report_indicator_null_flavor: None,
			worldwide_unique_id: generated_case_number
				.as_ref()
				.map(|generated| generated.worldwide_unique_id.clone()),
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await?;

	if data
		.patient_initials
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.is_some()
		|| data.patient_initials_null_flavor.is_some()
		|| data
			.investigation_number
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.is_some()
		|| data.investigation_number_null_flavor.is_some()
		|| data
			.age_d2_2a
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.is_some()
		|| data.sex_d5_null_flavor.is_some()
		|| data
			.sex_d5
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.is_some()
	{
		let age_at_time_of_onset = data
			.age_d2_2a
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.and_then(|v| v.parse().ok());
		let patient_id = PatientInformationBmc::create(
			ctx,
			mm,
			PatientInformationForCreate {
				case_id,
				patient_initials: non_empty(data.patient_initials.as_deref()),
				patient_initials_null_flavor: non_empty(
					data.patient_initials_null_flavor.as_deref(),
				),
				birth_date: None,
				birth_date_null_flavor: None,
				age_at_time_of_onset,
				age_unit: None,
				gestation_period: None,
				gestation_period_unit: None,
				age_group: None,
				weight_kg: None,
				height_cm: None,
				sex: non_empty(data.sex_d5.as_deref()),
				sex_null_flavor: non_empty(data.sex_d5_null_flavor.as_deref()),
				race_codes: Vec::new(),
				race_code_null_flavor: None,
				ethnicity_code: None,
				ethnicity_code_null_flavor: None,
				last_menstrual_period_date: None,
				last_menstrual_period_date_null_flavor: None,
				medical_history_text: None,
				medical_history_text_null_flavor: None,
				concomitant_therapy: None,
			},
		)
		.await?;
		let investigation_number = non_empty(data.investigation_number.as_deref());
		let investigation_number_null_flavor =
			non_empty(data.investigation_number_null_flavor.as_deref());
		if investigation_number.is_some()
			|| investigation_number_null_flavor.is_some()
		{
			PatientIdentifierBmc::create(
				ctx,
				mm,
				PatientIdentifierForCreate {
					patient_id,
					sequence_number: 1,
					identifier_type_code: "4".to_string(),
					identifier_value: investigation_number,
					identifier_value_null_flavor: investigation_number_null_flavor,
				},
			)
			.await?;
		}
	}

	if data
		.reaction_meddra_code
		.as_deref()
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.is_some()
		|| data.ae_start_date.is_some()
		|| data.ae_start_date_null_flavor.is_some()
	{
		ReactionBmc::create(
			ctx,
			mm,
			ReactionForCreate {
				case_id,
				sequence_number: 1,
				primary_source_reaction: None,
				primary_source_reaction_translation: None,
				reaction_language: None,
				reaction_meddra_code: non_empty(
					data.reaction_meddra_code.as_deref(),
				),
				reaction_meddra_version: non_empty(
					data.reaction_meddra_version.as_deref(),
				),
				term_highlighted: None,
				serious: None,
				criteria_death: None,
				criteria_death_null_flavor: None,
				criteria_life_threatening: None,
				criteria_life_threatening_null_flavor: None,
				criteria_hospitalization: None,
				criteria_hospitalization_null_flavor: None,
				criteria_disabling: None,
				criteria_disabling_null_flavor: None,
				criteria_congenital_anomaly: None,
				criteria_congenital_anomaly_null_flavor: None,
				criteria_other_medically_important: None,
				criteria_other_medically_important_null_flavor: None,
				required_intervention: None,
				required_intervention_null_flavor: None,
				expectedness: None,
				severity: None,
				mfds_device_ae_classification: None,
				mfds_device_ae_outcome: None,
				mfds_device_cause_medical_device: None,
				mfds_device_cause_procedure_issue: None,
				mfds_device_cause_patient_condition: None,
				mfds_device_cause_unable_to_assess: None,
				mfds_device_cause_other: None,
				mfds_device_action_reason: None,
				mfds_device_action_recall: None,
				mfds_device_action_repair: None,
				mfds_device_action_inspection: None,
				mfds_device_action_replacement: None,
				mfds_device_action_improvement: None,
				mfds_device_action_monitoring: None,
				mfds_device_action_notification: None,
				mfds_device_action_label_change: None,
				mfds_device_action_other: None,
				start_date: data
					.ae_start_date
					.and_then(|value| non_empty(Some(&value))),
				start_date_null_flavor: non_empty(
					data.ae_start_date_null_flavor.as_deref(),
				),
				end_date: None,
				end_date_null_flavor: None,
				duration_value: None,
				duration_unit: None,
				outcome: None,
				medical_confirmation: None,
				country_code: None,
				deleted: Some(false),
			},
		)
		.await?;
	}

	Ok((
		axum::http::StatusCode::CREATED,
		Json(DataRestResult {
			data: CaseFromIntakeResult {
				case_id,
				safety_report_id,
				version: next_version,
			},
		}),
	))
}

fn format_e2b_datetime(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}000000",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

#[cfg(test)]
mod tests {
	use super::{validate_intake_pair, validate_intake_value};

	#[test]
	fn intake_null_flavor_uses_explicit_catalog_companion() {
		let path = "reactions[].reactionStartDateNullFlavor";
		let validate = |present, value, null_flavor| {
			validate_intake_pair(
				present,
				value,
				null_flavor,
				path,
				"ICH.E.i.4.NULLFLAVOR.ALLOWED",
				input_contracts::generated::e::e_i_4,
			)
		};
		assert!(validate(false, None, Some("ASKU")).is_ok());
		assert!(validate(false, None, Some("UNK")).is_err());
		assert!(validate(true, None, Some("ASKU")).is_err());
		assert!(validate(true, Some("ASKU"), None).is_err());
	}

	#[test]
	fn intake_date_contract_keeps_field_specific_precision() {
		assert!(validate_intake_value(
			Some("202206"),
			"reactionStartDate",
			input_contracts::generated::e::e_i_4,
		)
		.is_ok());
		assert!(validate_intake_value(
			Some("202206"),
			"dateOfMostRecentInformation",
			input_contracts::generated::c::c_1_5,
		)
		.is_err());
		assert!(validate_intake_value(
			Some("200509211242-08"),
			"reactionStartDate",
			input_contracts::generated::e::e_i_4,
		)
		.is_ok());
	}
}
