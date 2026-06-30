use axum::extract::State;
use axum::Json;
use lib_core::model::acs::CASE_CREATE;
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
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
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
pub struct CaseIntakeCheckResult {
	pub duplicate: bool,
	pub basis_complete: bool,
	pub warnings: Vec<String>,
	pub matches: Vec<CaseIntakeDuplicateMatch>,
}

#[derive(Debug, Deserialize)]
pub struct CaseFromIntakeInput {
	pub safety_report_id: Option<String>,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub transmission_date: Option<Date>,
	#[serde(
		default,
		deserialize_with = "lib_core::serde::flex_date::deserialize_option_date"
	)]
	pub date_first_received_from_source: Option<Date>,
	#[serde(deserialize_with = "lib_core::serde::flex_date::deserialize_date")]
	pub date_of_most_recent_information: Date,
	pub report_type: String,
	pub status: Option<String>,
	pub allow_duplicate_override: Option<bool>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
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

// -- Input normalization

fn normalize_optional_text(value: Option<String>) -> Option<String> {
	value.and_then(|raw| {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			return None;
		}
		if matches!(
			trimmed.to_ascii_uppercase().as_str(),
			"NI" | "UNK" | "ASKU" | "NASK" | "MSK"
		) {
			return None;
		}
		Some(trimmed.to_string())
	})
}

fn to_duplicate_key(input: &CaseIntakeCheckInput) -> CaseDuplicateKey {
	CaseDuplicateKey {
		report_type: input.report_type.clone(),
		reporter_organization: input.reporter_organization.clone(),
		sponsor_study_number: input.sponsor_study_number.clone(),
		patient_initials: input.patient_initials.clone(),
		investigation_number: input.investigation_number.clone(),
		age_d2_2a: input.age_d2_2a.clone(),
		sex_d5: input.sex_d5.clone(),
		dg_prd_key: input.dg_prd_key.clone(),
		reaction_meddra_version: input.reaction_meddra_version.clone(),
		reaction_meddra_code: input.reaction_meddra_code.clone(),
		ae_start_date: input.ae_start_date,
	}
}

fn normalize_intake_check_input(data: CaseIntakeCheckInput) -> CaseIntakeCheckInput {
	CaseIntakeCheckInput {
		safety_report_id: data
			.safety_report_id
			.map(|value| value.trim().to_string()),
		date_of_most_recent_information: data.date_of_most_recent_information,
		report_type: normalize_optional_text(data.report_type),
		reporter_organization: normalize_optional_text(data.reporter_organization),
		sponsor_study_number: normalize_optional_text(data.sponsor_study_number),
		patient_initials: normalize_optional_text(data.patient_initials),
		investigation_number: normalize_optional_text(data.investigation_number),
		age_d2_2a: normalize_optional_text(data.age_d2_2a),
		sex_d5: normalize_optional_text(data.sex_d5),
		dg_prd_key: normalize_optional_text(data.dg_prd_key),
		reaction_meddra_version: normalize_optional_text(
			data.reaction_meddra_version,
		),
		reaction_meddra_code: normalize_optional_text(data.reaction_meddra_code),
		ae_start_date: data.ae_start_date,
	}
}

fn intake_to_duplicate_key(data: &CaseFromIntakeInput) -> CaseDuplicateKey {
	let normalized = normalize_intake_check_input(CaseIntakeCheckInput {
		safety_report_id: data.safety_report_id.clone(),
		date_of_most_recent_information: Some(data.date_of_most_recent_information),
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
	});
	to_duplicate_key(&normalized)
}

fn non_empty(input: Option<&str>) -> Option<String> {
	input
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.map(ToOwned::to_owned)
}

async fn assess_intake_duplicates(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	key: &CaseDuplicateKey,
) -> Result<(DuplicateBasisAssessment, Vec<CaseIntakeDuplicateMatch>)> {
	let assessment = assess_duplicate_basis(key);
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
	Json(params): Json<ParamsForCreate<CaseIntakeCheckInput>>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseIntakeCheckResult>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;

	let data = normalize_intake_check_input(params.data);
	let key = to_duplicate_key(&data);
	let (assessment, matches) = assess_intake_duplicates(&ctx, &mm, &key).await?;

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
}

/// POST /api/cases/from-intake
pub async fn create_case_from_intake(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<CaseFromIntakeInput>>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFromIntakeResult>>,
)> {
	use sqlx::types::time::OffsetDateTime;

	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;

	let data = params.data;
	let mut safety_report_id = data.safety_report_id.clone().unwrap_or_default();
	if data.report_type.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "report_type is required".to_string(),
		});
	}

	let duplicate_key = intake_to_duplicate_key(&data);
	let (duplicate_basis, duplicate_matches) =
		assess_intake_duplicates(&ctx, &mm, &duplicate_key).await?;
	if !duplicate_matches.is_empty() {
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
				"duplicate check basis is incomplete; resubmit with allow_duplicate_override=true after review".to_string()
			});
		return Err(Error::BadRequest { message });
	}

	let profile_enum = RegulatoryAuthority::Fda;

	let generated_case_number = if safety_report_id.trim().is_empty() {
		let generated = generate_case_number(&ctx, &mm)
			.await
			.map_err(Error::Model)?;
		safety_report_id = generated.safety_report_id.clone();
		Some(generated)
	} else {
		safety_report_id = safety_report_id.trim().to_string();
		None
	};
	let next_version = next_case_version(&ctx, &mm, &safety_report_id).await?;
	let case_create = InternalCaseForCreate {
		organization_id: ctx.organization_id(),
		dg_prd_key: data.dg_prd_key.clone(),
		status: Some(data.status.unwrap_or_else(|| "draft".to_string())),
		review_receivers_json: None,
		workflow_routes_json: None,
		mfds_report_type: data.mfds_report_type.clone(),
		fda_report_type: data.fda_report_type.clone(),
		report_year: data.report_year.clone(),
		source_document_name: data.source_document_name.clone(),
		source_document_base64: data.source_document_base64.clone(),
		source_document_media_type: data.source_document_media_type.clone(),
	};
	validate_case_create_payload(&case_create)?;
	let case_id = CaseBmc::create(&ctx, &mm, case_create).await?;
	let transmission_date = data
		.transmission_date
		.unwrap_or(data.date_of_most_recent_information);
	let date_first_received_from_source = data
		.date_first_received_from_source
		.unwrap_or(data.date_of_most_recent_information);

	let now = OffsetDateTime::now_utc();
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number: format!("MSG-{case_id}"),
			message_sender_identifier: message_sender_identifier(),
			message_receiver_identifier: message_receiver_identifier(profile_enum),
			message_date:
				crate::web::rest::case_export_rest::format_message_timestamp_utc_pub(
					now,
				),
		},
	)
	.await?;

	SafetyReportIdentificationBmc::create(
		&ctx,
		&mm,
		SafetyReportIdentificationForCreate {
			case_id,
			safety_report_id: Some(safety_report_id.clone()),
			version: Some(next_version),
			transmission_date: Some(transmission_date),
			transmission_date_null_flavor: None,
			report_type: Some(data.report_type),
			date_first_received_from_source: Some(date_first_received_from_source),
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: Some(
				data.date_of_most_recent_information,
			),
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria: Some(false),
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			first_sender_type: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
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
		|| data
			.investigation_number
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.is_some()
		|| data
			.age_d2_2a
			.as_deref()
			.map(str::trim)
			.filter(|v| !v.is_empty())
			.is_some()
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
			&ctx,
			&mm,
			PatientInformationForCreate {
				case_id,
				patient_initials: non_empty(data.patient_initials.as_deref()),
				patient_given_name: None,
				patient_family_name: None,
				patient_initials_null_flavor: None,
				birth_date: None,
				birth_date_null_flavor: None,
				age_at_time_of_onset,
				age_at_time_of_onset_null_flavor: None,
				age_unit: None,
				gestation_period: None,
				gestation_period_unit: None,
				age_group: None,
				weight_kg: None,
				height_cm: None,
				sex: non_empty(data.sex_d5.as_deref()),
				sex_null_flavor: None,
				race_code: None,
				ethnicity_code: None,
				last_menstrual_period_date: None,
				last_menstrual_period_date_null_flavor: None,
				medical_history_text: None,
				concomitant_therapy: None,
			},
		)
		.await?;
		if let Some(investigation_number) =
			non_empty(data.investigation_number.as_deref())
		{
			PatientIdentifierBmc::create(
				&ctx,
				&mm,
				PatientIdentifierForCreate {
					patient_id,
					sequence_number: 1,
					identifier_type_code: "4".to_string(),
					identifier_value: investigation_number,
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
	{
		ReactionBmc::create(
			&ctx,
			&mm,
			ReactionForCreate {
				case_id,
				sequence_number: 1,
				primary_source_reaction: non_empty(
					data.reaction_meddra_code.as_deref(),
				)
				.unwrap_or_else(|| "Intake reaction".to_string()),
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
				included_in_ema_ime_list: None,
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
				start_date: data.ae_start_date,
				start_date_null_flavor: None,
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
