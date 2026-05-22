use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorDgListRowDto, CaseEditorDhListRowDto,
	CaseEditorDirectSectionResponse, CaseEditorFieldEnvelope, CaseEditorFieldIssue,
	CaseEditorFieldPatch, CaseEditorLbListRowDto, CaseEditorListResponse,
	CaseEditorPagePatchRequest, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CaseEditorShellDto, FocusedAppendixResponse,
};
use crate::web::rest::case_rest::case_to_read_result;
use axum::extract::{Path, Query, State};
use axum::Json;
use lib_core::model::acs::{
	CASE_IDENTIFIER_LIST, CASE_READ, CASE_SUMMARY_LIST, CASE_UPDATE,
	DEATH_CAUSE_LIST, DRUG_CREATE, DRUG_DELETE, DRUG_DOSAGE_LIST,
	DRUG_INDICATION_LIST, DRUG_LIST, DRUG_REACTION_ASSESSMENT_LIST, DRUG_READ,
	DRUG_RECURRENCE_LIST, DRUG_SUBSTANCE_LIST, DRUG_UPDATE,
	LITERATURE_REFERENCE_LIST, MEDICAL_HISTORY_LIST, MESSAGE_HEADER_READ,
	NARRATIVE_READ, PARENT_INFORMATION_LIST, PARENT_MEDICAL_HISTORY_LIST,
	PARENT_PAST_DRUG_LIST, PAST_DRUG_CREATE, PAST_DRUG_DELETE, PAST_DRUG_LIST,
	PAST_DRUG_READ, PAST_DRUG_UPDATE, PATIENT_DEATH_LIST, PATIENT_IDENTIFIER_LIST,
	PATIENT_READ, PRIMARY_SOURCE_LIST, REACTION_CREATE, REACTION_DELETE,
	REACTION_LIST, REACTION_READ, REACTION_UPDATE, RECEIVER_READ,
	SAFETY_REPORT_READ, SAFETY_REPORT_UPDATE, SENDER_DIAGNOSIS_LIST,
	SENDER_INFORMATION_LIST, STUDY_INFORMATION_LIST, STUDY_REGISTRATION_LIST,
	TEST_RESULT_CREATE, TEST_RESULT_DELETE, TEST_RESULT_LIST, TEST_RESULT_READ,
	TEST_RESULT_UPDATE,
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
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::DrugReactionAssessmentBmc;
use lib_core::model::drug_recurrence::DrugRecurrenceInformationBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	NarrativeInformationBmc, NarrativeInformationForCreate,
	NarrativeInformationForUpdate, SenderDiagnosisBmc, SenderDiagnosisFilter,
};
use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryFilter, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryFilter,
};
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeFilter, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistoryBmc, PastDrugHistoryFilter, PastDrugHistoryForCreate,
	PastDrugHistoryForUpdate, PatientDeathInformationBmc,
	PatientDeathInformationFilter, PatientIdentifierBmc, PatientIdentifierFilter,
	PatientInformationBmc, PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathFilter,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use lib_core::model::receiver::ReceiverInformationBmc;
use lib_core::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter, LiteratureReferenceBmc,
	LiteratureReferenceFilter, LiteratureReferenceForCreate,
	LiteratureReferenceForUpdate, PatchValue, PrimarySourceBmc, PrimarySourceFilter,
	PrimarySourceForCreate, PrimarySourceForUpdate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForUpdate, SenderInformationBmc,
	SenderInformationFilter, SenderInformationForCreate, SenderInformationForUpdate,
	StudyInformationBmc, StudyInformationFilter, StudyInformationForCreate,
	StudyInformationForUpdate, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter,
};
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use lib_core::model::ModelManager;
use lib_core::validation::{
	validate_case_for_profiles, ValidationIssue, ValidationProfile,
};
use lib_rest_core::prelude::*;
use lib_rest_core::Error;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde::Deserialize;
use serde_json::{json, Map, Value};
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
	profiles: Option<String>,
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

fn parse_editor_profiles(value: Option<&str>) -> Result<Vec<ValidationProfile>> {
	let Some(value) = value else {
		return Ok(vec![ValidationProfile::Ich]);
	};
	let mut profiles = Vec::new();
	for raw in value
		.split(',')
		.map(str::trim)
		.filter(|raw| !raw.is_empty())
	{
		let profile =
			ValidationProfile::parse(raw).ok_or_else(|| Error::BadRequest {
				message: format!(
				"invalid validation profile '{raw}' (expected: ich, fda or mfds)"
			),
			})?;
		if !profiles.contains(&profile) {
			profiles.push(profile);
		}
	}
	if profiles.is_empty() {
		Ok(vec![ValidationProfile::Ich])
	} else {
		Ok(profiles)
	}
}

fn profile_strings(profiles: &[ValidationProfile]) -> Vec<String> {
	profiles
		.iter()
		.map(|profile| profile.as_str().to_string())
		.collect()
}

fn request_profiles_csv(profiles: Option<&[String]>) -> Option<String> {
	profiles.map(|profiles| profiles.join(","))
}

fn editor_projection_context(
	focused_appendix: Option<String>,
	requested_profiles: Option<String>,
) -> Result<(Vec<ValidationProfile>, FocusedAppendixResponse)> {
	let explicit_profiles = requested_profiles.is_some();
	let profiles = if explicit_profiles {
		parse_editor_profiles(requested_profiles.as_deref())?
	} else {
		parse_editor_profiles(focused_appendix.as_deref())?
	};
	let focused_appendix = if explicit_profiles {
		FocusedAppendixResponse::omitted()
	} else {
		FocusedAppendixResponse::legacy(normalize_appendix(focused_appendix)?)
	};
	Ok((profiles, focused_appendix))
}

fn insert_editor_json_context(
	map: &mut Map<String, Value>,
	focused_appendix: Option<String>,
	requested_profiles: Option<String>,
) -> Result<()> {
	let (profiles, focused_appendix) =
		editor_projection_context(focused_appendix, requested_profiles)?;
	map.insert("profiles".to_string(), json!(profile_strings(&profiles)));
	if let FocusedAppendixResponse::Legacy(value) = focused_appendix {
		map.insert("focusedAppendix".to_string(), json!(value));
	}
	Ok(())
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
	requested_profiles: Option<String>,
) -> Result<CaseEditorPageProjectionResponse> {
	let (profiles, focused_appendix) =
		editor_projection_context(focused_appendix, requested_profiles)?;
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
		profiles: profile_strings(&profiles),
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
		build_ci_page_projection(&ctx, &mm, case_id, query.appendix, query.profiles)
			.await?;
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

	if !request.changes.is_empty() {
		SafetyReportIdentificationBmc::update_by_case(&ctx, &mm, case_id, update)
			.await?;
		CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	}
	let requested_profiles = request_profiles_csv(request.profiles.as_deref());
	let projection = build_ci_page_projection(
		&ctx,
		&mm,
		case_id,
		request.appendix,
		requested_profiles,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub async fn patch_editor_rp_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "RP", request).await
}

pub async fn patch_editor_sd_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SD", request).await
}

pub async fn patch_editor_lr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "LR", request).await
}

pub async fn patch_editor_si_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "SI", request).await
}

pub async fn patch_editor_dm_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "DM", request).await
}

pub async fn patch_editor_nr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, case_id, "NR", request).await
}

async fn patch_direct_page_projection(
	mm: ModelManager,
	ctx_w: CtxW,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	if !request.changes.is_empty() {
		return Err(Error::BadRequest {
			message: format!("{page_id} page patch fields are not implemented yet"),
		});
	}

	if !request.rows.is_empty() {
		apply_direct_page_rows_patch(&ctx, &mm, case_id, page_id, &request.rows)
			.await?;
		CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	}

	let data = match page_id {
		"RP" => load_editor_rp_data(&ctx, &mm, case_id).await?,
		"SD" => load_editor_sd_data(&ctx, &mm, case_id).await?,
		"LR" => load_editor_lr_data(&ctx, &mm, case_id).await?,
		"SI" => load_editor_si_data(&ctx, &mm, case_id).await?,
		"DM" => load_editor_dm_data(&ctx, &mm, case_id).await?,
		"NR" => load_editor_nr_data(&ctx, &mm, case_id).await?,
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported direct page '{page_id}'"),
			})
		}
	};
	let projection = direct_page_projection_response(
		&ctx,
		&mm,
		case_id,
		page_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
		data,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

fn reject_unknown_row_keys(
	page_id: &str,
	rows: &BTreeMap<String, Value>,
	allowed: &[&str],
) -> Result<()> {
	for key in rows.keys() {
		if !allowed.contains(&key.as_str()) {
			return Err(Error::BadRequest {
				message: format!("unknown {page_id} row '{key}'"),
			});
		}
	}
	Ok(())
}

fn as_object<'a>(
	page_id: &str,
	key: &str,
	value: &'a Value,
) -> Result<&'a serde_json::Map<String, Value>> {
	value.as_object().ok_or_else(|| Error::BadRequest {
		message: format!("{page_id}.{key} must be an object"),
	})
}

fn first_array_object<'a>(
	page_id: &str,
	key: &str,
	value: &'a Value,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
	let Some(items) = value.as_array() else {
		return Err(Error::BadRequest {
			message: format!("{page_id}.{key} must be an array"),
		});
	};
	items
		.first()
		.map(|item| as_object(page_id, key, item))
		.transpose()
}

fn optional_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
	rows.get(key)
		.map(|value| as_object(page_id, key, value))
		.transpose()
}

fn required_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<&'a serde_json::Map<String, Value>> {
	optional_row_object(page_id, rows, key)?.ok_or_else(|| Error::BadRequest {
		message: format!("{page_id}.{key} row payload is required"),
	})
}

fn optional_first_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
	rows.get(key)
		.map(|value| first_array_object(page_id, key, value))
		.transpose()
		.map(Option::flatten)
}

fn string_field(
	map: &serde_json::Map<String, Value>,
	aliases: &[&str],
) -> Option<String> {
	for alias in aliases {
		if let Some(value) = map.get(*alias) {
			if value.is_null() {
				return None;
			}
			if let Some(value) = value.as_str() {
				return Some(value.to_string());
			}
			return Some(value.to_string());
		}
	}
	None
}

fn i32_field(map: &serde_json::Map<String, Value>, aliases: &[&str]) -> Option<i32> {
	for alias in aliases {
		if let Some(value) = map.get(*alias) {
			if let Some(value) = value.as_i64() {
				return i32::try_from(value).ok();
			}
		}
	}
	None
}

fn bool_field(
	map: &serde_json::Map<String, Value>,
	aliases: &[&str],
) -> Option<bool> {
	for alias in aliases {
		if let Some(value) = map.get(*alias) {
			return value.as_bool();
		}
	}
	None
}

fn insert_alias(
	map: &mut serde_json::Map<String, Value>,
	target: &str,
	aliases: &[&str],
) {
	if map.contains_key(target) {
		return;
	}
	for alias in aliases {
		if let Some(value) = map.get(*alias) {
			map.insert(target.to_string(), value.clone());
			return;
		}
	}
}

fn row_model_value(
	row: &serde_json::Map<String, Value>,
	aliases: &[(&str, &[&str])],
	extra: &[(&str, Value)],
) -> Value {
	let mut map = row.clone();
	for (target, aliases) in aliases {
		insert_alias(&mut map, target, aliases);
	}
	for (key, value) in extra {
		map.insert((*key).to_string(), value.clone());
	}
	Value::Object(map)
}

fn parse_row_model<T: serde::de::DeserializeOwned>(
	page_id: &str,
	key: &str,
	value: Value,
) -> Result<T> {
	serde_json::from_value(value).map_err(|err| Error::BadRequest {
		message: format!("invalid {page_id}.{key} row payload: {err}"),
	})
}

fn uuid_field(
	map: &serde_json::Map<String, Value>,
	aliases: &[&str],
) -> Option<Uuid> {
	string_field(map, aliases).and_then(|value| Uuid::parse_str(&value).ok())
}

async fn apply_direct_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	match page_id {
		"RP" => apply_rp_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SD" => apply_sd_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"LR" => apply_lr_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SI" => apply_si_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"DM" => apply_dm_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"NR" => apply_nr_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		_ => Err(Error::BadRequest {
			message: format!("unsupported direct page '{page_id}'"),
		}),
	}
}

async fn apply_rp_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(page_id, rows, &["primarySources"])?;
	let Some(source) = optional_first_row_object(page_id, rows, "primarySources")?
	else {
		return Ok(());
	};
	let update = PrimarySourceForUpdate {
		reporter_title: string_field(source, &["reporterTitle", "reporter_title"]),
		reporter_given_name: string_field(
			source,
			&["reporterGivenName", "reporter_given_name"],
		),
		reporter_middle_name: string_field(
			source,
			&["reporterMiddleName", "reporter_middle_name"],
		),
		reporter_family_name: string_field(
			source,
			&["reporterFamilyName", "reporter_family_name"],
		),
		organization: string_field(
			source,
			&["reporterOrganization", "organization"],
		),
		department: string_field(source, &["reporterDepartment", "department"]),
		street: string_field(source, &["reporterStreet", "street"]),
		city: string_field(source, &["reporterCity", "city"]),
		state: string_field(source, &["reporterState", "state"]),
		postcode: string_field(source, &["reporterPostcode", "postcode"]),
		telephone: string_field(source, &["reporterTelephone", "telephone"]),
		country_code: string_field(source, &["reporterCountry", "country_code"]),
		email: string_field(source, &["reporterEmail", "email"]),
		qualification: string_field(source, &["qualification"]),
		qualification_kr1: string_field(
			source,
			&["qualificationKr1", "qualification_kr1"],
		),
		primary_source_regulatory: string_field(
			source,
			&[
				"primarySourceForRegulatoryPurposes",
				"primary_source_regulatory",
			],
		),
	};
	if let Some(id) = uuid_field(source, &["id"]) {
		PrimarySourceBmc::update(ctx, mm, id, update).await?;
	} else {
		PrimarySourceBmc::create(
			ctx,
			mm,
			PrimarySourceForCreate {
				case_id,
				sequence_number: i32_field(
					source,
					&["sequenceNumber", "sequence_number"],
				)
				.unwrap_or(1),
				reporter_title: update.reporter_title,
				reporter_given_name: update.reporter_given_name,
				reporter_middle_name: update.reporter_middle_name,
				reporter_family_name: update.reporter_family_name,
				organization: update.organization,
				department: update.department,
				street: update.street,
				city: update.city,
				state: update.state,
				postcode: update.postcode,
				telephone: update.telephone,
				country_code: update.country_code,
				email: update.email,
				qualification: update.qualification,
				qualification_kr1: update.qualification_kr1,
				primary_source_regulatory: update.primary_source_regulatory,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_sd_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&[
			"safetyReportIdentification",
			"messageHeader",
			"senderInformation",
			"receiverInformation",
		],
	)?;
	let Some(sender) = optional_row_object(page_id, rows, "senderInformation")?
	else {
		return Ok(());
	};
	let update = SenderInformationForUpdate {
		sender_type: string_field(sender, &["senderType", "sender_type"]),
		organization_name: string_field(
			sender,
			&["organizationName", "organization_name"],
		),
		department: string_field(sender, &["department"]),
		street_address: string_field(sender, &["streetAddress", "street_address"]),
		city: string_field(sender, &["city"]),
		state: string_field(sender, &["state"]),
		postcode: string_field(sender, &["postcode"]),
		country_code: string_field(sender, &["countryCode", "country_code"]),
		person_title: string_field(sender, &["personTitle", "person_title"]),
		person_given_name: string_field(
			sender,
			&["personGivenName", "person_given_name"],
		),
		person_middle_name: string_field(
			sender,
			&["personMiddleName", "person_middle_name"],
		),
		person_family_name: string_field(
			sender,
			&["personFamilyName", "person_family_name"],
		),
		telephone: string_field(sender, &["telephone"]),
		fax: string_field(sender, &["fax"]),
		email: string_field(sender, &["email"]),
	};
	if let Some(id) = uuid_field(sender, &["id"]) {
		SenderInformationBmc::update(ctx, mm, id, update).await?;
	} else {
		SenderInformationBmc::create(
			ctx,
			mm,
			SenderInformationForCreate {
				case_id,
				sender_type: update.sender_type,
				organization_name: update.organization_name,
				department: update.department,
				street_address: update.street_address,
				city: update.city,
				state: update.state,
				postcode: update.postcode,
				country_code: update.country_code,
				person_title: update.person_title,
				person_given_name: update.person_given_name,
				person_middle_name: update.person_middle_name,
				person_family_name: update.person_family_name,
				telephone: update.telephone,
				fax: update.fax,
				email: update.email,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_lr_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(page_id, rows, &["literatureReferences"])?;
	let Some(reference) =
		optional_first_row_object(page_id, rows, "literatureReferences")?
	else {
		return Ok(());
	};
	let update = LiteratureReferenceForUpdate {
		reference_text: string_field(
			reference,
			&["referenceText", "reference_text"],
		),
		sequence_number: i32_field(
			reference,
			&["sequenceNumber", "sequence_number"],
		),
		document_base64: string_field(
			reference,
			&["documentBase64", "document_base64"],
		),
		media_type: string_field(reference, &["mediaType", "media_type"]),
		representation: string_field(reference, &["representation"]),
		compression: string_field(reference, &["compression"]),
	};
	if let Some(id) = uuid_field(reference, &["id"]) {
		LiteratureReferenceBmc::update(ctx, mm, id, update).await?;
	} else if let Some(reference_text) = update.reference_text {
		LiteratureReferenceBmc::create(
			ctx,
			mm,
			LiteratureReferenceForCreate {
				case_id,
				reference_text,
				sequence_number: update.sequence_number.unwrap_or(1),
				document_base64: update.document_base64,
				media_type: update.media_type,
				representation: update.representation,
				compression: update.compression,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_si_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["studyInformation", "studyRegistrationNumbers"],
	)?;
	let Some(study) = optional_row_object(page_id, rows, "studyInformation")? else {
		return Ok(());
	};
	let update = StudyInformationForUpdate {
		study_name: string_field(study, &["studyName", "study_name"]),
		sponsor_study_number: string_field(
			study,
			&["sponsorStudyNumber", "sponsor_study_number"],
		),
		study_type_reaction: string_field(
			study,
			&["studyTypeReaction", "study_type_reaction"],
		),
		study_type_reaction_kr1: string_field(
			study,
			&["studyTypeReactionKr1", "study_type_reaction_kr1"],
		),
	};
	if let Some(id) = uuid_field(study, &["id"]) {
		StudyInformationBmc::update(ctx, mm, id, update).await?;
	} else {
		StudyInformationBmc::create(
			ctx,
			mm,
			StudyInformationForCreate {
				case_id,
				study_name: update.study_name,
				sponsor_study_number: update.sponsor_study_number,
				study_type_reaction: update.study_type_reaction,
				study_type_reaction_kr1: update.study_type_reaction_kr1,
			},
		)
		.await?;
	}
	Ok(())
}

async fn apply_dm_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&[
			"patientInformation",
			"patientIdentifiers",
			"medicalHistoryEpisodes",
			"deathInfo",
			"reportedCauses",
			"autopsyCauses",
			"parentInfo",
			"parentMedicalHistory",
			"parentPastDrugs",
		],
	)?;
	let Some(patient) = optional_row_object(page_id, rows, "patientInformation")?
	else {
		return Ok(());
	};
	let update = PatientInformationForUpdate {
		patient_initials: string_field(
			patient,
			&["patientInitials", "patient_initials"],
		),
		patient_given_name: string_field(
			patient,
			&["patientGivenName", "patient_given_name"],
		),
		patient_family_name: string_field(
			patient,
			&["patientFamilyName", "patient_family_name"],
		),
		patient_initials_null_flavor: string_field(
			patient,
			&["patientInitialsNullFlavor", "patient_initials_null_flavor"],
		),
		birth_date: None,
		birth_date_null_flavor: string_field(
			patient,
			&["birthDateNullFlavor", "birth_date_null_flavor"],
		),
		age_at_time_of_onset: None,
		age_at_time_of_onset_null_flavor: string_field(
			patient,
			&[
				"ageAtTimeOfOnsetNullFlavor",
				"age_at_time_of_onset_null_flavor",
			],
		),
		age_unit: string_field(patient, &["ageUnit", "age_unit"]),
		gestation_period: None,
		gestation_period_unit: string_field(
			patient,
			&["gestationPeriodUnit", "gestation_period_unit"],
		),
		age_group: string_field(patient, &["ageGroup", "age_group"]),
		weight_kg: None,
		height_cm: None,
		sex: string_field(patient, &["sex"]),
		sex_null_flavor: string_field(
			patient,
			&["sexNullFlavor", "sex_null_flavor"],
		),
		race_code: string_field(patient, &["raceCode", "race_code"]),
		ethnicity_code: string_field(patient, &["ethnicityCode", "ethnicity_code"]),
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: string_field(
			patient,
			&[
				"lastMenstrualPeriodDateNullFlavor",
				"last_menstrual_period_date_null_flavor",
			],
		),
		medical_history_text: string_field(
			patient,
			&["medicalHistoryText", "medical_history_text"],
		),
		concomitant_therapy: None,
	};
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(_) => {
			PatientInformationBmc::update_by_case(ctx, mm, case_id, update).await?
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {
			PatientInformationBmc::create(
				ctx,
				mm,
				PatientInformationForCreate {
					case_id,
					patient_initials: update.patient_initials,
					patient_given_name: update.patient_given_name,
					patient_family_name: update.patient_family_name,
					patient_initials_null_flavor: update
						.patient_initials_null_flavor,
					birth_date: None,
					birth_date_null_flavor: update.birth_date_null_flavor,
					age_at_time_of_onset: None,
					age_at_time_of_onset_null_flavor: update
						.age_at_time_of_onset_null_flavor,
					age_unit: update.age_unit,
					gestation_period: None,
					gestation_period_unit: update.gestation_period_unit,
					age_group: update.age_group,
					weight_kg: None,
					height_cm: None,
					sex: update.sex,
					sex_null_flavor: update.sex_null_flavor,
					race_code: update.race_code,
					ethnicity_code: update.ethnicity_code,
					last_menstrual_period_date: None,
					last_menstrual_period_date_null_flavor: update
						.last_menstrual_period_date_null_flavor,
					medical_history_text: update.medical_history_text,
					concomitant_therapy: None,
				},
			)
			.await?;
		}
		Err(err) => return Err(err.into()),
	}
	Ok(())
}

async fn apply_nr_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["narrative", "senderDiagnoses", "caseSummaryInformation"],
	)?;
	let Some(narrative) = optional_row_object(page_id, rows, "narrative")? else {
		return Ok(());
	};
	let case_narrative =
		string_field(narrative, &["caseNarrative", "case_narrative"]);
	let update = NarrativeInformationForUpdate {
		case_narrative: case_narrative.clone(),
		reporter_comments: string_field(
			narrative,
			&["reporterComments", "reporter_comments"],
		),
		sender_comments: string_field(
			narrative,
			&["senderComments", "sender_comments"],
		),
	};
	match NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await? {
		Some(_) => {
			NarrativeInformationBmc::update_by_case(ctx, mm, case_id, update).await?
		}
		None => {
			let Some(case_narrative) = case_narrative else {
				return Ok(());
			};
			NarrativeInformationBmc::create(
				ctx,
				mm,
				NarrativeInformationForCreate {
					case_id,
					case_narrative,
					reporter_comments: update.reporter_comments,
					sender_comments: update.sender_comments,
				},
			)
			.await?;
		}
	}
	Ok(())
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
	requested_profiles: Option<String>,
	data: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	let (profiles, focused_appendix) =
		editor_projection_context(focused_appendix, requested_profiles)?;
	let saved = direct_page_saved(page_id, &data);
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		profiles: profile_strings(&profiles),
		focused_appendix,
		saved,
		required_count: 0,
		fields: BTreeMap::new(),
		rows: rows_from_direct_section(data),
		section_summaries: Vec::new(),
	})
}

fn repeatable_page_projection_response(
	case_id: Uuid,
	page_id: &'static str,
	focused_appendix: Option<String>,
	requested_profiles: Option<String>,
	rows: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	let (profiles, focused_appendix) =
		editor_projection_context(focused_appendix, requested_profiles)?;
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		profiles: profile_strings(&profiles),
		focused_appendix,
		saved: rows
			.get("rows")
			.and_then(Value::as_array)
			.map(|items| !items.is_empty())
			.unwrap_or(false),
		required_count: 0,
		fields: BTreeMap::new(),
		rows: rows_from_direct_section(rows),
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
		query.profiles,
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
		query.profiles,
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
		query.profiles,
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
		query.profiles,
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
		query.profiles,
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
		query.profiles,
		load_editor_nr_data(&ctx, &mm, case_id).await?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

async fn load_editor_ae_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseEditorAeListRowDto>> {
	Ok(ReactionBmc::list_by_case(ctx, mm, case_id)
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
		.collect())
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

	let rows = load_editor_ae_list_rows(&ctx, &mm, case_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_ae_page_projection(
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
	require_permission(&ctx, REACTION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_ae_list_rows(&ctx, &mm, case_id).await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"AE",
		query.appendix,
		query.profiles,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
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

pub async fn get_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query.appendix,
		query.profiles,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_ae_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	appendix: Option<String>,
	profiles: Option<String>,
) -> Result<Value> {
	let reaction = ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("AE"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, appendix, profiles)?;
	response.insert("data".to_string(), json!({ "reaction": reaction }));
	Ok(Value::Object(response))
}

pub async fn create_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	let row = required_row_object("AE", &request.rows, "reaction")?;
	let value = row_model_value(
		row,
		&[
			(
				"primary_source_reaction",
				&["reactionPrimarySourceNative"][..],
			),
			(
				"primary_source_reaction_translation",
				&["reactionPrimarySourceTranslation"][..],
			),
			("reaction_meddra_version", &["meddraVersion"][..]),
			("reaction_meddra_code", &["meddraCode"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
		],
	);
	let create = parse_row_model::<ReactionForCreate>("AE", "reaction", value)?;
	let row_id = ReactionBmc::create(&ctx, &mm, create).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let row = required_row_object("AE", &request.rows, "reaction")?;
	let value = row_model_value(
		row,
		&[
			(
				"primary_source_reaction",
				&["reactionPrimarySourceNative"][..],
			),
			(
				"primary_source_reaction_translation",
				&["reactionPrimarySourceTranslation"][..],
			),
			("reaction_meddra_version", &["meddraVersion"][..]),
			("reaction_meddra_code", &["meddraCode"][..]),
		],
		&[],
	);
	let update = parse_row_model::<ReactionForUpdate>("AE", "reaction", value)?;
	ReactionBmc::update(&ctx, &mm, row_id, update).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	ReactionBmc::delete(&ctx, &mm, row_id).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn load_editor_lb_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseEditorLbListRowDto>> {
	Ok(TestResultBmc::list_by_case(ctx, mm, case_id)
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
		.collect())
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

	let rows = load_editor_lb_list_rows(&ctx, &mm, case_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_lb_page_projection(
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
	require_permission(&ctx, TEST_RESULT_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_lb_list_rows(&ctx, &mm, case_id).await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"LB",
		query.appendix,
		query.profiles,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
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

pub async fn get_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query.appendix,
		query.profiles,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_lb_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	appendix: Option<String>,
	profiles: Option<String>,
) -> Result<Value> {
	let test_result = TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("LB"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, appendix, profiles)?;
	response.insert("data".to_string(), json!({ "testResult": test_result }));
	Ok(Value::Object(response))
}

pub async fn create_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	let row = required_row_object("LB", &request.rows, "testResult")?;
	let value = row_model_value(
		row,
		&[
			("test_name", &["testName"][..]),
			("test_result_value", &["resultValue"][..]),
			("test_result_unit", &["resultUnit"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
		],
	);
	let create = parse_row_model::<TestResultForCreate>("LB", "testResult", value)?;
	let row_id = TestResultBmc::create(&ctx, &mm, create).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let row = required_row_object("LB", &request.rows, "testResult")?;
	let value = row_model_value(
		row,
		&[
			("test_name", &["testName"][..]),
			("test_result_value", &["resultValue"][..]),
			("test_result_unit", &["resultUnit"][..]),
		],
		&[],
	);
	let update = parse_row_model::<TestResultForUpdate>("LB", "testResult", value)?;
	TestResultBmc::update(&ctx, &mm, row_id, update).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	TestResultBmc::delete(&ctx, &mm, row_id).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn load_editor_dg_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseEditorDgListRowDto>> {
	Ok(DrugInformationBmc::list_by_case(ctx, mm, case_id)
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
		.collect())
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

	let rows = load_editor_dg_list_rows(&ctx, &mm, case_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_dg_page_projection(
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
	require_permission(&ctx, DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_dg_list_rows(&ctx, &mm, case_id).await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"DG",
		query.appendix,
		query.profiles,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
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

async fn load_editor_dg_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Value> {
	let drug = DrugInformationBmc::get_in_case(ctx, mm, case_id, drug_id).await?;
	let active_substances = DrugActiveSubstanceBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugActiveSubstanceFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let dosage_information = DosageInformationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DosageInformationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let indications = DrugIndicationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugIndicationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let drug_reaction_assessments =
		DrugReactionAssessmentBmc::list_by_drug(ctx, mm, drug_id).await?;
	let drug_recurrences =
		DrugRecurrenceInformationBmc::list_by_drug(ctx, mm, drug_id).await?;

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
	Ok(drug)
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

	let drug = load_editor_dg_row_detail(&ctx, &mm, case_id, drug_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: drug_id,
			data: json!({ "drugs": [drug] }),
		}),
	))
}

pub async fn get_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_READ)?;
	require_permission(&ctx, DRUG_SUBSTANCE_LIST)?;
	require_permission(&ctx, DRUG_DOSAGE_LIST)?;
	require_permission(&ctx, DRUG_INDICATION_LIST)?;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_LIST)?;
	require_permission(&ctx, DRUG_RECURRENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query.appendix,
		query.profiles,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_dg_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	appendix: Option<String>,
	profiles: Option<String>,
) -> Result<Value> {
	let drug = load_editor_dg_row_detail(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("DG"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, appendix, profiles)?;
	response.insert("data".to_string(), json!({ "drug": drug }));
	Ok(Value::Object(response))
}

pub async fn create_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	let row = required_row_object("DG", &request.rows, "drug")?;
	let value = row_model_value(
		row,
		&[
			("medicinal_product", &["medicinalProduct"][..]),
			("drug_characterization", &["drugRole"][..]),
			("action_taken", &["actionTaken"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
			(
				"drug_characterization",
				json!(string_field(row, &["drugRole", "drug_characterization"])
					.unwrap_or_else(|| "1".to_string())),
			),
		],
	);
	let create = parse_row_model::<DrugInformationForCreate>("DG", "drug", value)?;
	let row_id = DrugInformationBmc::create(&ctx, &mm, create).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	DrugInformationBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let row = required_row_object("DG", &request.rows, "drug")?;
	let value = row_model_value(
		row,
		&[
			("medicinal_product", &["medicinalProduct"][..]),
			("drug_characterization", &["drugRole"][..]),
			("action_taken", &["actionTaken"][..]),
		],
		&[],
	);
	let update = parse_row_model::<DrugInformationForUpdate>("DG", "drug", value)?;
	DrugInformationBmc::update(&ctx, &mm, row_id, update).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	DrugInformationBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	DrugInformationBmc::delete(&ctx, &mm, row_id).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn load_editor_dh_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseEditorDhListRowDto>> {
	let patient = match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(patient) => patient,
		Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "patient_information",
			..
		}) => return Ok(Vec::new()),
		Err(err) => return Err(err.into()),
	};
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(patient
			.id
			.to_string()))])),
		..Default::default()
	};
	Ok(PastDrugHistoryBmc::list(
		ctx,
		mm,
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
	.collect())
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

	let rows = load_editor_dh_list_rows(&ctx, &mm, case_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_dh_page_projection(
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
	require_permission(&ctx, PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_dh_list_rows(&ctx, &mm, case_id).await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"DH",
		query.appendix,
		query.profiles,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
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

pub async fn get_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query.appendix,
		query.profiles,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn load_editor_dh_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<Value> {
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let history = PastDrugHistoryBmc::get(&ctx, &mm, row_id).await?;
	if history.patient_id != patient.id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "past_drug_history",
			id: row_id,
		}
		.into());
	}
	Ok(json!(history))
}

async fn build_editor_dh_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	appendix: Option<String>,
	profiles: Option<String>,
) -> Result<Value> {
	let history = load_editor_dh_row_detail(ctx, mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("DH"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, appendix, profiles)?;
	response.insert("data".to_string(), json!({ "pastDrugHistory": history }));
	Ok(Value::Object(response))
}

pub async fn create_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let row = required_row_object("DH", &request.rows, "pastDrugHistory")?;
	let value = row_model_value(
		row,
		&[
			("drug_name", &["drugName"][..]),
			("indication_meddra_code", &["indication"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("patient_id", json!(patient.id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
		],
	);
	let create =
		parse_row_model::<PastDrugHistoryForCreate>("DH", "pastDrugHistory", value)?;
	let row_id = PastDrugHistoryBmc::create(&ctx, &mm, create).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	load_editor_dh_row_detail(&ctx, &mm, case_id, row_id).await?;
	let row = required_row_object("DH", &request.rows, "pastDrugHistory")?;
	let value = row_model_value(
		row,
		&[
			("drug_name", &["drugName"][..]),
			("indication_meddra_code", &["indication"][..]),
		],
		&[],
	);
	let update =
		parse_row_model::<PastDrugHistoryForUpdate>("DH", "pastDrugHistory", value)?;
	PastDrugHistoryBmc::update(&ctx, &mm, row_id, update).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		request.appendix,
		request_profiles_csv(request.profiles.as_deref()),
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	load_editor_dh_row_detail(&ctx, &mm, case_id, row_id).await?;
	PastDrugHistoryBmc::delete(&ctx, &mm, row_id).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, case_id).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}
