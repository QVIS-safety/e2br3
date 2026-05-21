use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorDgListRowDto, CaseEditorDhListRowDto,
	CaseEditorDirectSectionResponse, CaseEditorFieldEnvelope, CaseEditorFieldIssue,
	CaseEditorFieldPatch, CaseEditorLbListRowDto, CaseEditorListResponse,
	CaseEditorPagePatchRequest, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CaseEditorShellDto,
};
use crate::web::rest::case_rest::case_to_read_result;
use axum::extract::{Path, Query, State};
use axum::Json;
use lib_core::model::acs::{
	CASE_IDENTIFIER_LIST, CASE_READ, CASE_SUMMARY_LIST, CASE_UPDATE,
	DEATH_CAUSE_LIST, DRUG_DOSAGE_LIST, DRUG_INDICATION_LIST, DRUG_LIST,
	DRUG_REACTION_ASSESSMENT_LIST, DRUG_READ, DRUG_RECURRENCE_LIST,
	DRUG_SUBSTANCE_LIST, LITERATURE_REFERENCE_LIST, MEDICAL_HISTORY_LIST,
	MESSAGE_HEADER_READ, NARRATIVE_READ, PARENT_INFORMATION_LIST,
	PARENT_MEDICAL_HISTORY_LIST, PARENT_PAST_DRUG_LIST, PAST_DRUG_LIST,
	PAST_DRUG_READ, PATIENT_DEATH_LIST, PATIENT_IDENTIFIER_LIST, PATIENT_READ,
	PRIMARY_SOURCE_LIST, REACTION_LIST, REACTION_READ, RECEIVER_READ,
	SAFETY_REPORT_READ, SAFETY_REPORT_UPDATE, SENDER_DIAGNOSIS_LIST,
	SENDER_INFORMATION_LIST, STUDY_INFORMATION_LIST, STUDY_REGISTRATION_LIST,
	TEST_RESULT_LIST, TEST_RESULT_READ,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberFilter, OtherCaseIdentifierBmc,
	OtherCaseIdentifierFilter,
};
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationFilter, DrugActiveSubstanceBmc,
	DrugActiveSubstanceFilter, DrugIndicationBmc, DrugIndicationFilter,
	DrugInformationBmc,
};
use lib_core::model::drug_reaction_assessment::DrugReactionAssessmentBmc;
use lib_core::model::drug_recurrence::DrugRecurrenceInformationBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	NarrativeInformationBmc, SenderDiagnosisBmc, SenderDiagnosisFilter,
};
use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryFilter, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryFilter,
};
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeFilter, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistoryBmc, PastDrugHistoryFilter, PatientDeathInformationBmc,
	PatientDeathInformationFilter, PatientIdentifierBmc, PatientIdentifierFilter,
	PatientInformationBmc, ReportedCauseOfDeathBmc, ReportedCauseOfDeathFilter,
};
use lib_core::model::reaction::ReactionBmc;
use lib_core::model::receiver::ReceiverInformationBmc;
use lib_core::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter, LiteratureReferenceBmc,
	LiteratureReferenceFilter, PatchValue, PrimarySourceBmc, PrimarySourceFilter,
	SafetyReportIdentificationBmc, SafetyReportIdentificationForUpdate,
	SenderInformationBmc, SenderInformationFilter, StudyInformationBmc,
	StudyInformationFilter, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter,
};
use lib_core::model::test_result::TestResultBmc;
use lib_core::model::ModelManager;
use lib_core::validation::{
	validate_case_for_profiles, ValidationIssue, ValidationProfile,
};
use lib_rest_core::prelude::*;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub async fn get_editor_shell(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorShellDto>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let case = CaseBmc::get(&ctx, &mm, case_id).await?;
	let case = case_to_read_result(&ctx, &mm, case).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorShellDto::from(case)),
	))
}

fn uuid_eq(id: Uuid) -> OpValsValue {
	OpValsValue::from(vec![OpValValue::Eq(json!(id.to_string()))])
}

fn direct_section_response(
	case_id: Uuid,
	data: Value,
) -> (
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
) {
	(
		axum::http::StatusCode::OK,
		Json(CaseEditorDirectSectionResponse { case_id, data }),
	)
}

#[derive(Debug, Deserialize)]
pub struct CaseEditorPageProjectionQuery {
	appendix: Option<String>,
}

fn normalize_appendix(value: Option<String>) -> Result<Option<String>> {
	value
		.map(|value| {
			let normalized = value.trim().to_ascii_lowercase();
			ValidationProfile::parse(&normalized)
				.map(|_| normalized)
				.ok_or_else(|| Error::BadRequest {
					message: format!("unknown appendix '{value}'"),
				})
		})
		.transpose()
}

async fn load_editor_ci_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let message_header = match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await
	{
		Ok(entity) => Some(entity),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
		Err(err) => return Err(err.into()),
	};
	let receiver_information =
		ReceiverInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;
	let other_case_identifiers = OtherCaseIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![OtherCaseIdentifierFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let linked_reports = LinkedReportNumberBmc::list(
		ctx,
		mm,
		Some(vec![LinkedReportNumberFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let documents_held_by_sender = DocumentsHeldBySenderBmc::list(
		ctx,
		mm,
		Some(vec![DocumentsHeldBySenderFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!({
		"case": case,
		"safetyReportIdentification": safety_report_identification,
		"messageHeader": message_header,
		"receiverInfo": receiver_information,
		"otherCaseIdentifiers": other_case_identifiers,
		"linkedReports": linked_reports,
		"documentsHeldBySender": documents_held_by_sender,
	}))
}

pub async fn get_editor_ci(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	require_permission(&ctx, MESSAGE_HEADER_READ)?;
	require_permission(&ctx, RECEIVER_READ)?;
	require_permission(&ctx, CASE_IDENTIFIER_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_ci_data(&ctx, &mm, case_id).await?,
	))
}

fn validation_profiles_for_appendices(
	appendices: &[String],
) -> Vec<ValidationProfile> {
	let mut profiles = Vec::new();
	for appendix in appendices {
		if let Some(profile) = ValidationProfile::parse(appendix) {
			if !profiles.contains(&profile) {
				profiles.push(profile);
			}
		}
	}
	if profiles.is_empty() {
		vec![ValidationProfile::Ich]
	} else {
		profiles
	}
}

fn report_type_display(value: Option<&str>) -> Option<String> {
	match value.map(str::trim) {
		Some("1") => Some("Spontaneous report".to_string()),
		Some("2") => Some("Report from study".to_string()),
		Some("3") => Some("Other".to_string()),
		Some("4") => Some("Not available to sender (unknown)".to_string()),
		Some(value) if !value.is_empty() => Some(value.to_string()),
		_ => None,
	}
}

fn bool_display(value: Option<bool>) -> Option<String> {
	value.map(|value| value.to_string())
}

fn string_value(value: Option<&str>) -> Value {
	value.map(|value| json!(value)).unwrap_or(Value::Null)
}

fn bool_value(value: Option<bool>) -> Value {
	value.map(|value| json!(value)).unwrap_or(Value::Null)
}

fn issue_field_key(issue: &ValidationIssue) -> Option<&'static str> {
	let path = issue.field_path.as_deref().unwrap_or(issue.path.as_str());
	match path {
		"safetyReportIdentification.reportType" => Some("reportType"),
		"safetyReportIdentification.fulfilExpeditedCriteria" => {
			Some("fulfilExpeditedCriteria")
		}
		"safetyReportIdentification.localCriteriaReportType" => {
			Some("localCriteriaReportType")
		}
		"safetyReportIdentification.combinationProductReportIndicator" => {
			Some("combinationProductReportIndicator")
		}
		_ => None,
	}
}

fn collect_projected_issues(
	issues: &[ValidationIssue],
	visible_fields: &BTreeSet<&'static str>,
) -> BTreeMap<&'static str, Vec<CaseEditorFieldIssue>> {
	let mut grouped: BTreeMap<&'static str, Vec<CaseEditorFieldIssue>> =
		BTreeMap::new();
	for issue in issues {
		let Some(key) = issue_field_key(issue) else {
			continue;
		};
		if !visible_fields.contains(key) {
			continue;
		}
		grouped.entry(key).or_default().push(CaseEditorFieldIssue {
			code: issue.code.clone(),
			message: issue.message.clone(),
			blocking: issue.blocking,
		});
	}
	grouped
}

struct FieldEnvelopeInput {
	field_id: &'static str,
	path: &'static str,
	label: &'static str,
	value: Value,
	display: Option<String>,
	null_flavor: Option<String>,
	visible: bool,
	issues: Vec<CaseEditorFieldIssue>,
}

fn field_envelope(input: FieldEnvelopeInput) -> CaseEditorFieldEnvelope {
	let empty = match &input.value {
		Value::Null => true,
		Value::String(value) => value.trim().is_empty(),
		_ => false,
	};
	let required_empty = input
		.issues
		.iter()
		.any(|issue| issue.code.ends_with(".REQUIRED"));
	CaseEditorFieldEnvelope {
		field_id: input.field_id,
		path: input.path,
		label: input.label,
		value: input.value.clone(),
		display: input.display,
		null_flavor: input.null_flavor.clone(),
		notation: None,
		origin_value: input.value,
		origin_null_flavor: input.null_flavor,
		visible: input.visible,
		editable: input.visible,
		empty,
		required_empty,
		issues: input.issues,
	}
}

async fn build_ci_page_projection(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	focused_appendix: Option<String>,
) -> Result<CaseEditorPageProjectionResponse> {
	let focused_appendix = normalize_appendix(focused_appendix)?;
	let active_appendices = focused_appendix
		.as_ref()
		.map(|appendix| vec![appendix.clone()])
		.unwrap_or_else(|| vec!["ich".to_string()]);
	let profiles = validation_profiles_for_appendices(&active_appendices);
	let has_fda = profiles.contains(&ValidationProfile::Fda);
	let safety_report =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let reports = validate_case_for_profiles(ctx, mm, case_id, &profiles).await?;
	let all_issues = reports
		.iter()
		.flat_map(|report| report.issues.iter().cloned())
		.collect::<Vec<_>>();

	let mut visible_fields = BTreeSet::from([
		"reportType",
		"fulfilExpeditedCriteria",
		"combinationProductReportIndicator",
	]);
	if has_fda {
		visible_fields.insert("localCriteriaReportType");
	}
	let mut issues_by_field = collect_projected_issues(&all_issues, &visible_fields);

	let mut fields = BTreeMap::new();
	let report_type = safety_report
		.as_ref()
		.and_then(|report| report.report_type.as_deref());
	fields.insert(
		"reportType".to_string(),
		field_envelope(FieldEnvelopeInput {
			field_id: "CASE_RPT_TYPE",
			path: "safetyReportIdentification.reportType",
			label: "Type of Report",
			value: string_value(report_type),
			display: report_type_display(report_type),
			null_flavor: None,
			visible: true,
			issues: issues_by_field.remove("reportType").unwrap_or_default(),
		}),
	);

	let expedited = safety_report
		.as_ref()
		.and_then(|report| report.fulfil_expedited_criteria);
	fields.insert(
		"fulfilExpeditedCriteria".to_string(),
		field_envelope(FieldEnvelopeInput {
			field_id: "CASE_EXPEDIT",
			path: "safetyReportIdentification.fulfilExpeditedCriteria",
			label:
				"Does This Case Fulfil the Local Criteria for an Expedited Report?",
			value: bool_value(expedited),
			display: bool_display(expedited),
			null_flavor: None,
			visible: true,
			issues: issues_by_field
				.remove("fulfilExpeditedCriteria")
				.unwrap_or_default(),
		}),
	);

	let local_criteria = safety_report
		.as_ref()
		.and_then(|report| report.local_criteria_report_type.as_deref());
	fields.insert(
		"localCriteriaReportType".to_string(),
		field_envelope(FieldEnvelopeInput {
			field_id: "CASEU_LOC_REPORT_TYPE",
			path: "safetyReportIdentification.localCriteriaReportType",
			label: "Local Criteria Report Type",
			value: string_value(local_criteria),
			display: local_criteria.map(str::to_string),
			null_flavor: None,
			visible: has_fda,
			issues: issues_by_field
				.remove("localCriteriaReportType")
				.unwrap_or_default(),
		}),
	);

	let combination = safety_report
		.as_ref()
		.and_then(|report| report.combination_product_report_indicator.as_deref());
	fields.insert(
		"combinationProductReportIndicator".to_string(),
		field_envelope(FieldEnvelopeInput {
			field_id: "CASEU_PRD_RPT_IND",
			path: "safetyReportIdentification.combinationProductReportIndicator",
			label: "Combination Product Report Indicator",
			value: string_value(combination),
			display: combination.map(str::to_string),
			null_flavor: None,
			visible: has_fda,
			issues: issues_by_field
				.remove("combinationProductReportIndicator")
				.unwrap_or_default(),
		}),
	);

	let required_count = fields
		.values()
		.filter(|field| field.visible && field.required_empty)
		.count();

	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id: "CI",
		focused_appendix,
		saved: safety_report.is_some(),
		required_count,
		fields,
		rows: rows_from_direct_section(load_editor_ci_data(ctx, mm, case_id).await?),
		section_summaries: Vec::new(),
	})
}

/// GET /api/cases/{case_id}/editor/pages/CI
pub async fn get_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection =
		build_ci_page_projection(&ctx, &mm, case_id, query.appendix).await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

fn patch_string_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<PatchValue<String>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(PatchValue::Missing);
	};
	if value.is_null() {
		return Ok(PatchValue::Null);
	}
	let Some(value) = value.as_str() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a string or null"),
		});
	};
	Ok(PatchValue::Value(value.trim().to_string()))
}

fn patch_bool_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<PatchValue<bool>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(PatchValue::Missing);
	};
	if value.is_null() {
		return Ok(PatchValue::Null);
	}
	let Some(value) = value.as_bool() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a boolean or null"),
		});
	};
	Ok(PatchValue::Value(value))
}

/// PATCH /api/cases/{case_id}/editor/pages/CI
pub async fn patch_editor_ci_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	let mut update = SafetyReportIdentificationForUpdate {
		transmission_date: None,
		transmission_date_null_flavor: None,
		report_type: PatchValue::Missing,
		date_first_received_from_source: None,
		date_first_received_from_source_null_flavor: None,
		date_of_most_recent_information: None,
		date_of_most_recent_information_null_flavor: None,
		fulfil_expedited_criteria: PatchValue::Missing,
		local_criteria_report_type: PatchValue::Missing,
		combination_product_report_indicator: PatchValue::Missing,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
	};

	for (field, patch) in &request.changes {
		match field.as_str() {
			"reportType" => {
				update.report_type = patch_string_value(field, patch)?;
			}
			"fulfilExpeditedCriteria" => {
				update.fulfil_expedited_criteria = patch_bool_value(field, patch)?;
			}
			"localCriteriaReportType" => {
				update.local_criteria_report_type =
					patch_string_value(field, patch)?;
			}
			"combinationProductReportIndicator" => {
				update.combination_product_report_indicator =
					patch_string_value(field, patch)?;
			}
			_ => {
				return Err(Error::BadRequest {
					message: format!("unknown CI field '{field}'"),
				});
			}
		}
	}
	if !request.rows.is_empty() {
		return Err(Error::BadRequest {
			message: "CI row patch operations are not implemented in this slice"
				.to_string(),
		});
	}

	SafetyReportIdentificationBmc::update_by_case(&ctx, &mm, case_id, update)
		.await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let projection =
		build_ci_page_projection(&ctx, &mm, case_id, request.appendix).await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

fn rows_from_direct_section(data: Value) -> BTreeMap<String, Value> {
	match data {
		Value::Object(map) => map.into_iter().collect(),
		value => BTreeMap::from([("data".to_string(), value)]),
	}
}

fn direct_page_saved(page_id: &str, data: &Value) -> bool {
	let Some(map) = data.as_object() else {
		return false;
	};
	match page_id {
		"RP" => map
			.get("primarySources")
			.and_then(Value::as_array)
			.map(|rows| !rows.is_empty())
			.unwrap_or(false),
		"SD" => map
			.get("senderInformation")
			.and_then(Value::as_array)
			.map(|rows| !rows.is_empty())
			.unwrap_or(false),
		"LR" => map
			.get("literatureReferences")
			.and_then(Value::as_array)
			.map(|rows| !rows.is_empty())
			.unwrap_or(false),
		"SI" => map
			.get("studyInformation")
			.map(|value| !value.is_null())
			.unwrap_or(false),
		"DM" => map
			.get("patientInformation")
			.map(|value| !value.is_null())
			.unwrap_or(false),
		"NR" => map
			.get("narrative")
			.map(|value| !value.is_null())
			.unwrap_or(false),
		_ => false,
	}
}

async fn direct_page_projection_response(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	focused_appendix: Option<String>,
	data: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	let saved = direct_page_saved(page_id, &data);
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		focused_appendix: normalize_appendix(focused_appendix)?,
		saved,
		required_count: 0,
		fields: BTreeMap::new(),
		rows: rows_from_direct_section(data),
		section_summaries: Vec::new(),
	})
}

async fn load_editor_rp_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let primary_sources = PrimarySourceBmc::list(
		ctx,
		mm,
		Some(vec![PrimarySourceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!({ "primarySources": primary_sources }))
}

pub async fn get_editor_rp(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PRIMARY_SOURCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_rp_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_rp_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PRIMARY_SOURCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"RP",
		query.appendix,
		load_editor_rp_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_sd_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let sender_information = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let sender = sender_information.first().cloned();

	Ok(json!({
		"safetyReportIdentification": safety_report_identification,
		"senderInformation": sender_information,
		"sender": sender,
	}))
}

pub async fn get_editor_sd(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	require_permission(&ctx, SENDER_INFORMATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_sd_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_sd_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	require_permission(&ctx, SENDER_INFORMATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"SD",
		query.appendix,
		load_editor_sd_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_lr_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let literature_references = LiteratureReferenceBmc::list(
		ctx,
		mm,
		Some(vec![LiteratureReferenceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(json!({ "literatureReferences": literature_references }))
}

pub async fn get_editor_lr(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, LITERATURE_REFERENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_lr_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_lr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, LITERATURE_REFERENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"LR",
		query.appendix,
		load_editor_lr_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_si_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let mut studies = StudyInformationBmc::list(
		ctx,
		mm,
		Some(vec![StudyInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	studies.sort_by_key(|study| study.created_at);
	let study_information = studies.into_iter().next();
	let study_registration_numbers = if let Some(ref study) = study_information {
		StudyRegistrationNumberBmc::list(
			ctx,
			mm,
			Some(vec![StudyRegistrationNumberFilter {
				study_information_id: Some(uuid_eq(study.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?
	} else {
		Vec::new()
	};

	Ok(json!({
		"studyInformation": study_information,
		"studyRegistrationNumbers": study_registration_numbers,
	}))
}

pub async fn get_editor_si(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, STUDY_INFORMATION_LIST)?;
	require_permission(&ctx, STUDY_REGISTRATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_si_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_si_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, STUDY_INFORMATION_LIST)?;
	require_permission(&ctx, STUDY_REGISTRATION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"SI",
		query.appendix,
		load_editor_si_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_dm_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let Some(patient) =
		(match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		})
	else {
		return Ok(json!({
			"patientInformation": null,
			"patientIdentifiers": [],
			"medicalHistoryEpisodes": [],
			"deathInfo": null,
			"reportedCauses": [],
			"autopsyCauses": [],
			"parentInfo": null,
			"parentMedicalHistory": [],
			"parentPastDrugs": [],
		}));
	};

	let patient_id = patient.id;
	let patient_identifiers = PatientIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![PatientIdentifierFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let medical_history_episodes = MedicalHistoryEpisodeBmc::list(
		ctx,
		mm,
		Some(vec![MedicalHistoryEpisodeFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let parent_information_rows = ParentInformationBmc::list(
		ctx,
		mm,
		Some(vec![ParentInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut parents = Vec::new();
	let mut parent_medical_history = Vec::new();
	let mut parent_past_drugs = Vec::new();
	for parent in &parent_information_rows {
		let medical_history = ParentMedicalHistoryBmc::list(
			ctx,
			mm,
			Some(vec![ParentMedicalHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let past_drug_history = ParentPastDrugHistoryBmc::list(
			ctx,
			mm,
			Some(vec![ParentPastDrugHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let mut parent_with_children = json!(parent);
		if let Value::Object(ref mut map) = parent_with_children {
			map.insert("medicalHistory".to_string(), json!(medical_history));
			map.insert("pastDrugHistory".to_string(), json!(past_drug_history));
			map.insert("pastDrugs".to_string(), json!(past_drug_history));
		}
		parent_medical_history.extend(medical_history);
		parent_past_drugs.extend(past_drug_history);
		parents.push(parent_with_children);
	}
	let death_information = PatientDeathInformationBmc::list(
		ctx,
		mm,
		Some(vec![PatientDeathInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut reported_causes = Vec::new();
	let mut autopsy_causes = Vec::new();
	for death_info in &death_information {
		reported_causes.extend(
			ReportedCauseOfDeathBmc::list(
				ctx,
				mm,
				Some(vec![ReportedCauseOfDeathFilter {
					death_info_id: Some(uuid_eq(death_info.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?,
		);
		autopsy_causes.extend(
			AutopsyCauseOfDeathBmc::list(
				ctx,
				mm,
				Some(vec![AutopsyCauseOfDeathFilter {
					death_info_id: Some(uuid_eq(death_info.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?,
		);
	}
	let death_info = death_information.into_iter().next();
	let parent_info = parent_information_rows.into_iter().next();

	Ok(json!({
		"patientInformation": patient,
		"patientIdentifiers": patient_identifiers,
		"medicalHistoryEpisodes": medical_history_episodes,
		"deathInfo": death_info,
		"reportedCauses": reported_causes,
		"autopsyCauses": autopsy_causes,
		"parentInfo": parent_info,
		"parentMedicalHistory": parent_medical_history,
		"parentPastDrugs": parent_past_drugs,
		"parents": parents,
	}))
}

pub async fn get_editor_dm(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PATIENT_READ)?;
	require_permission(&ctx, PATIENT_IDENTIFIER_LIST)?;
	require_permission(&ctx, MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PATIENT_DEATH_LIST)?;
	require_permission(&ctx, DEATH_CAUSE_LIST)?;
	require_permission(&ctx, PARENT_INFORMATION_LIST)?;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PARENT_PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_dm_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_dm_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PATIENT_READ)?;
	require_permission(&ctx, PATIENT_IDENTIFIER_LIST)?;
	require_permission(&ctx, MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PATIENT_DEATH_LIST)?;
	require_permission(&ctx, DEATH_CAUSE_LIST)?;
	require_permission(&ctx, PARENT_INFORMATION_LIST)?;
	require_permission(&ctx, PARENT_MEDICAL_HISTORY_LIST)?;
	require_permission(&ctx, PARENT_PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"DM",
		query.appendix,
		load_editor_dm_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_nr_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let narrative =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;
	let (sender_diagnoses, case_summary_information) =
		if let Some(ref narrative) = narrative {
			let sender_diagnoses = SenderDiagnosisBmc::list(
				ctx,
				mm,
				Some(vec![SenderDiagnosisFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			let case_summary_information = CaseSummaryInformationBmc::list(
				ctx,
				mm,
				Some(vec![CaseSummaryInformationFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			(sender_diagnoses, case_summary_information)
		} else {
			(Vec::new(), Vec::new())
		};

	Ok(json!({
		"narrative": narrative,
		"senderDiagnoses": sender_diagnoses,
		"caseSummaryInformation": case_summary_information,
	}))
}

pub async fn get_editor_nr(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, NARRATIVE_READ)?;
	require_permission(&ctx, SENDER_DIAGNOSIS_LIST)?;
	require_permission(&ctx, CASE_SUMMARY_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	Ok(direct_section_response(
		case_id,
		load_editor_nr_data(&ctx, &mm, case_id).await?,
	))
}

pub async fn get_editor_nr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, NARRATIVE_READ)?;
	require_permission(&ctx, SENDER_DIAGNOSIS_LIST)?;
	require_permission(&ctx, CASE_SUMMARY_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		"NR",
		query.appendix,
		load_editor_nr_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub async fn list_editor_ae(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorAeListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = ReactionBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|reaction| CaseEditorAeListRowDto {
			id: reaction.id,
			sequence_number: reaction.sequence_number,
			reaction_primary_source_native: reaction.primary_source_reaction,
			reaction_primary_source_translation: reaction
				.primary_source_reaction_translation,
			meddra_version: reaction.reaction_meddra_version,
			meddra_code: reaction.reaction_meddra_code,
			seriousness: reaction.serious,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_ae(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, reaction_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let reaction = ReactionBmc::get_in_case(&ctx, &mm, case_id, reaction_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: reaction_id,
			data: json!({ "reactions": [reaction] }),
		}),
	))
}

pub async fn list_editor_lb(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorLbListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = TestResultBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|test| CaseEditorLbListRowDto {
			id: test.id,
			sequence_number: test.sequence_number,
			test_name: test.test_name,
			test_date: test.test_date.map(|date| date.to_string()),
			result_value: test.test_result_value,
			result_unit: test.test_result_unit,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_lb(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, test_result_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let test_result =
		TestResultBmc::get_in_case(&ctx, &mm, case_id, test_result_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: test_result_id,
			data: json!({ "testResults": [test_result] }),
		}),
	))
}

pub async fn list_editor_dg(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorDgListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = DrugInformationBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|drug| CaseEditorDgListRowDto {
			id: drug.id,
			sequence_number: drug.sequence_number,
			drug_role: drug.drug_characterization,
			dg_prd_key: None,
			medicinal_product: drug.medicinal_product,
			action_taken: drug.action_taken,
			warning_count: 0,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

fn drug_id_filter<T>(drug_id: Uuid) -> Option<Vec<T>>
where
	T: Default,
	T: FromDrugIdFilter,
{
	Some(vec![T::from_drug_id(drug_id)])
}

trait FromDrugIdFilter {
	fn from_drug_id(drug_id: Uuid) -> Self;
}

impl FromDrugIdFilter for DrugActiveSubstanceFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

impl FromDrugIdFilter for DosageInformationFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

impl FromDrugIdFilter for DrugIndicationFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

pub async fn get_editor_dg(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_READ)?;
	require_permission(&ctx, DRUG_SUBSTANCE_LIST)?;
	require_permission(&ctx, DRUG_DOSAGE_LIST)?;
	require_permission(&ctx, DRUG_INDICATION_LIST)?;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_LIST)?;
	require_permission(&ctx, DRUG_RECURRENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let drug = DrugInformationBmc::get_in_case(&ctx, &mm, case_id, drug_id).await?;
	let active_substances = DrugActiveSubstanceBmc::list(
		&ctx,
		&mm,
		drug_id_filter::<DrugActiveSubstanceFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let dosage_information = DosageInformationBmc::list(
		&ctx,
		&mm,
		drug_id_filter::<DosageInformationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let indications = DrugIndicationBmc::list(
		&ctx,
		&mm,
		drug_id_filter::<DrugIndicationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let drug_reaction_assessments =
		DrugReactionAssessmentBmc::list_by_drug(&ctx, &mm, drug_id).await?;
	let drug_recurrences =
		DrugRecurrenceInformationBmc::list_by_drug(&ctx, &mm, drug_id).await?;

	let mut drug = json!(drug);
	if let Value::Object(ref mut map) = drug {
		map.insert("activeSubstances".to_string(), json!(active_substances));
		map.insert("dosageInformation".to_string(), json!(dosage_information));
		map.insert("indications".to_string(), json!(indications));
		map.insert(
			"drugReactionAssessments".to_string(),
			json!(drug_reaction_assessments),
		);
		map.insert("drugRecurrences".to_string(), json!(drug_recurrences));
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: drug_id,
			data: json!({ "drugs": [drug] }),
		}),
	))
}

pub async fn list_editor_dh(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorDhListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let patient = match PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await
	{
		Ok(patient) => patient,
		Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "patient_information",
			..
		}) => {
			return Ok((
				axum::http::StatusCode::OK,
				Json(CaseEditorListResponse {
					case_id,
					rows: Vec::new(),
				}),
			));
		}
		Err(err) => return Err(err.into()),
	};
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(patient
			.id
			.to_string()))])),
		..Default::default()
	};
	let rows = PastDrugHistoryBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?
	.into_iter()
	.map(|history| CaseEditorDhListRowDto {
		id: history.id,
		sequence_number: history.sequence_number,
		drug_name: history.drug_name,
		indication: history.indication_meddra_code,
		start_date: history.start_date.map(|date| date.to_string()),
		end_date: history.end_date.map(|date| date.to_string()),
	})
	.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_dh(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, past_drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let history = PastDrugHistoryBmc::get(&ctx, &mm, past_drug_id).await?;
	if history.patient_id != patient.id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "past_drug_history",
			id: past_drug_id,
		}
		.into());
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: past_drug_id,
			data: json!({
				"patientInformation": {
					"pastDrugHistory": [history]
				}
			}),
		}),
	))
}
