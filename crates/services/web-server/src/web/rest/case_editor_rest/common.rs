//! Shared imports and helpers for case editor REST modules.

pub(super) use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorDgListRowDto, CaseEditorDhListRowDto,
	CaseEditorDirectSectionResponse, CaseEditorFieldPatch, CaseEditorLbListRowDto,
	CaseEditorListResponse, CaseEditorPagePatchRequest,
	CaseEditorPageProjectionResponse, CaseEditorRowDetailResponse,
	CaseEditorShellDto,
};
pub(super) use crate::web::rest::case_rest::{case_to_read_result, PublicCaseView};
pub(super) use axum::extract::{Path, Query, State};
pub(super) use axum::Json;
pub(super) use lib_core::model::acs::{
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
pub(super) use lib_core::model::case::CaseBmc;
pub(super) use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberFilter, OtherCaseIdentifierBmc,
	OtherCaseIdentifierFilter,
};
pub(super) use lib_core::model::case_validation_report_cache::CaseValidationReportCacheBmc;
pub(super) use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
pub(super) use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationFilter, DrugActiveSubstanceBmc,
	DrugActiveSubstanceFilter, DrugIndicationBmc, DrugIndicationFilter,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
pub(super) use lib_core::model::drug_reaction_assessment::DrugReactionAssessmentBmc;
pub(super) use lib_core::model::drug_recurrence::DrugRecurrenceInformationBmc;
pub(super) use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForUpdate,
};
pub(super) use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	NarrativeInformationBmc, NarrativeInformationForCreate,
	NarrativeInformationForUpdate, SenderDiagnosisBmc, SenderDiagnosisFilter,
};
pub(super) use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryFilter, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryFilter,
};
pub(super) use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeFilter, ParentInformationBmc, ParentInformationFilter,
	PastDrugHistoryBmc, PastDrugHistoryFilter, PastDrugHistoryForCreate,
	PastDrugHistoryForUpdate, PatientDeathInformationBmc,
	PatientDeathInformationFilter, PatientIdentifierBmc, PatientIdentifierFilter,
	PatientInformationBmc, PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathFilter,
};
pub(super) use lib_core::model::reaction::{
	ReactionBmc, ReactionForCreate, ReactionForUpdate,
};
pub(super) use lib_core::model::receiver::ReceiverInformationBmc;
pub(super) use lib_core::model::safety_report::{
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
pub(super) use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
pub(super) use lib_core::model::ModelManager;
pub(super) use lib_core::regulatory::RegulatoryAuthority;
pub(super) use lib_rest_core::prelude::*;
pub(super) use lib_rest_core::Error;
pub(super) use lib_web::middleware::mw_auth::CtxW;
pub(super) use modql::filter::{ListOptions, OpValValue, OpValsValue};
pub(super) use serde::Deserialize;
pub(super) use serde_json::{json, Map, Value};
pub(super) use std::collections::BTreeMap;
pub(super) use uuid::Uuid;

pub(super) fn uuid_eq(id: Uuid) -> OpValsValue {
	OpValsValue::from(vec![OpValValue::Eq(json!(id.to_string()))])
}

pub(super) fn direct_section_response(
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
	pub(super) authorities: Option<String>,
	pub(super) include_deleted: Option<bool>,
}

pub(super) fn query_authorities_csv(
	query: &CaseEditorPageProjectionQuery,
) -> Result<Option<String>> {
	Ok(query.authorities.clone())
}

pub(super) fn parse_editor_authorities(
	value: Option<&str>,
) -> Result<Vec<RegulatoryAuthority>> {
	let Some(value) = value else {
		return Ok(vec![RegulatoryAuthority::Ich]);
	};
	let mut authorities = Vec::new();
	for raw in value
		.split(',')
		.map(str::trim)
		.filter(|raw| !raw.is_empty())
	{
		let authority =
			RegulatoryAuthority::parse(raw).ok_or_else(|| Error::BadRequest {
				message: format!(
				"invalid validation authority '{raw}' (expected: ich, fda or mfds)"
			),
			})?;
		if !authorities.contains(&authority) {
			authorities.push(authority);
		}
	}
	if authorities.is_empty() {
		Ok(vec![RegulatoryAuthority::Ich])
	} else {
		Ok(authorities)
	}
}

pub(super) fn authority_strings(authorities: &[RegulatoryAuthority]) -> Vec<String> {
	authorities
		.iter()
		.map(|authority| authority.as_str().to_string())
		.collect()
}

pub(super) fn validate_request_projection_context(
	authorities: Option<&[String]>,
) -> Result<Option<String>> {
	let requested_authorities = authorities.map(|authorities| authorities.join(","));
	editor_projection_context(requested_authorities.clone())?;
	Ok(requested_authorities)
}

pub(super) fn editor_projection_context(
	requested_authorities: Option<String>,
) -> Result<Vec<RegulatoryAuthority>> {
	parse_editor_authorities(requested_authorities.as_deref())
}

pub(super) fn insert_editor_json_context(
	map: &mut Map<String, Value>,
	requested_authorities: Option<String>,
) -> Result<()> {
	let authorities = editor_projection_context(requested_authorities)?;
	let authority_values = authority_strings(&authorities);
	map.insert("authorities".to_string(), json!(authority_values));
	Ok(())
}

/// After an editor save, invalidate the case's cached validation for every
/// authority and immediately recompute the ones the editor is working with, so
/// the read-only `/validation/cache` endpoint reflects the edit right away.
pub(super) async fn refresh_editor_validation_cache(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	requested_authorities: Option<String>,
) -> Result<()> {
	let authorities = editor_projection_context(requested_authorities)?;
	CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, case_id).await?;
	CaseValidationReportCacheBmc::mark_stale_for_case(ctx, mm, case_id).await?;
	crate::web::rest::case_validation_rest::refresh_case_validation_cache(
		ctx,
		mm,
		case_id,
		&authorities,
	)
	.await?;
	Ok(())
}

/// Invalidate the case's cached validation for every authority without
/// recomputing. Used by structural row create/delete/restore, where the
/// caller is not fetching a fresh report immediately.
pub(super) async fn mark_editor_validation_cache_stale(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	requested_authorities: Option<String>,
) -> Result<()> {
	editor_projection_context(requested_authorities)?;
	CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, case_id).await?;
	CaseValidationReportCacheBmc::mark_stale_for_case(ctx, mm, case_id).await?;
	Ok(())
}

pub(super) fn patch_string_value(
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

pub(super) fn patch_bool_value(
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

pub(super) fn patch_optional_string_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<Option<String>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(None);
	};
	if value.is_null() {
		return Ok(None);
	}
	let Some(value) = value.as_str() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a string or null"),
		});
	};
	Ok(Some(value.trim().to_string()))
}

pub(super) fn patch_optional_bool_value(
	field_name: &str,
	patch: &CaseEditorFieldPatch,
) -> Result<Option<bool>> {
	let Some(value) = patch.value.as_ref() else {
		return Ok(None);
	};
	if value.is_null() {
		return Ok(None);
	}
	let Some(value) = value.as_bool() else {
		return Err(Error::BadRequest {
			message: format!("{field_name} must be a boolean or null"),
		});
	};
	Ok(Some(value))
}

/// PATCH /api/cases/{case_id}/editor/pages/CI

pub(super) fn reject_unknown_row_keys(
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

pub(super) fn as_object<'a>(
	page_id: &str,
	key: &str,
	value: &'a Value,
) -> Result<&'a serde_json::Map<String, Value>> {
	value.as_object().ok_or_else(|| Error::BadRequest {
		message: format!("{page_id}.{key} must be an object"),
	})
}

pub(super) fn first_array_object<'a>(
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

pub(super) fn optional_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
	rows.get(key)
		.map(|value| as_object(page_id, key, value))
		.transpose()
}

pub(super) fn required_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<&'a serde_json::Map<String, Value>> {
	optional_row_object(page_id, rows, key)?.ok_or_else(|| Error::BadRequest {
		message: format!("{page_id}.{key} row payload is required"),
	})
}

pub(super) fn patch_json_value(patch: &CaseEditorFieldPatch) -> Value {
	patch.value.clone().unwrap_or(Value::Null)
}

pub(super) fn changes_to_object(
	page_id: &str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
	aliases: &[(&str, &str)],
) -> Result<serde_json::Map<String, Value>> {
	let mut row = serde_json::Map::new();
	for (field, patch) in changes {
		let Some((_, target)) = aliases.iter().find(|(source, _)| source == field)
		else {
			return Err(Error::BadRequest {
				message: format!("unknown {page_id} field '{field}'"),
			});
		};
		row.insert((*target).to_string(), patch_json_value(patch));
	}
	Ok(row)
}

pub(super) fn row_payload_from_changes(
	page_id: &str,
	row_key: &str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
	aliases: &[(&str, &str)],
) -> Result<BTreeMap<String, Value>> {
	Ok(BTreeMap::from([(
		row_key.to_string(),
		Value::Object(changes_to_object(page_id, changes, aliases)?),
	)]))
}

pub(super) fn row_array_payload_from_changes(
	page_id: &str,
	row_key: &str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
	aliases: &[(&str, &str)],
) -> Result<BTreeMap<String, Value>> {
	Ok(BTreeMap::from([(
		row_key.to_string(),
		Value::Array(vec![Value::Object(changes_to_object(
			page_id, changes, aliases,
		)?)]),
	)]))
}

pub(super) fn optional_first_row_object<'a>(
	page_id: &str,
	rows: &'a BTreeMap<String, Value>,
	key: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>> {
	rows.get(key)
		.map(|value| first_array_object(page_id, key, value))
		.transpose()
		.map(Option::flatten)
}

pub(super) fn string_field(
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

pub(super) fn i32_field(
	map: &serde_json::Map<String, Value>,
	aliases: &[&str],
) -> Option<i32> {
	for alias in aliases {
		if let Some(value) = map.get(*alias) {
			if let Some(value) = value.as_i64() {
				return i32::try_from(value).ok();
			}
		}
	}
	None
}

pub(super) fn bool_field(
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

pub(super) fn insert_alias(
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

pub(super) fn row_model_value(
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

pub(super) fn parse_row_model<T: serde::de::DeserializeOwned>(
	page_id: &str,
	key: &str,
	value: Value,
) -> Result<T> {
	serde_json::from_value(value).map_err(|err| Error::BadRequest {
		message: format!("invalid {page_id}.{key} row payload: {err}"),
	})
}

pub(super) fn uuid_field(
	map: &serde_json::Map<String, Value>,
	aliases: &[&str],
) -> Option<Uuid> {
	string_field(map, aliases).and_then(|value| Uuid::parse_str(&value).ok())
}

pub(super) fn rows_from_direct_section(data: Value) -> BTreeMap<String, Value> {
	match data {
		Value::Object(map) => map.into_iter().collect(),
		value => BTreeMap::from([("data".to_string(), value)]),
	}
}

pub(super) fn direct_page_saved(page_id: &str, data: &Value) -> bool {
	let Some(map) = data.as_object() else {
		return false;
	};
	match page_id {
		"RP" => map
			.get("primarySources")
			.and_then(Value::as_array)
			.map(|rows| !rows.is_empty())
			.unwrap_or(false),
		"CI" => map
			.get("safetyReportIdentification")
			.map(|value| !value.is_null())
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

pub(super) async fn direct_page_projection_response(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	requested_authorities: Option<String>,
	data: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	let authorities = editor_projection_context(requested_authorities)?;
	let authority_values = authority_strings(&authorities);
	let saved = direct_page_saved(page_id, &data);
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		authorities: authority_values,
		saved,
		required_count: 0,
		fields: BTreeMap::new(),
		rows: rows_from_direct_section(data),
		section_summaries: Vec::new(),
	})
}

pub(super) fn repeatable_page_projection_response(
	case_id: Uuid,
	page_id: &'static str,
	requested_authorities: Option<String>,
	rows: Value,
) -> Result<CaseEditorPageProjectionResponse> {
	let authorities = editor_projection_context(requested_authorities)?;
	let authority_values = authority_strings(&authorities);
	Ok(CaseEditorPageProjectionResponse {
		case_id,
		page_id,
		authorities: authority_values,
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

pub(super) fn editor_page_row_response(
	case_id: Uuid,
	section: &'static str,
	row_id: Uuid,
	requested_authorities: Option<String>,
	data: Value,
) -> Result<Value> {
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!(section));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, requested_authorities)?;
	response.insert("data".to_string(), data);
	Ok(Value::Object(response))
}

macro_rules! repeatable_page_row_read_handler {
	($fn_name:ident, [$($permission:expr),+ $(,)?], $build_response:ident $(,)?) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
			Query(query): Query<CaseEditorPageProjectionQuery>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			$(require_permission(&ctx, $permission)?;)+
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

			let response = $build_response(
				&ctx,
				&mm,
				case_id,
				row_id,
				query_authorities_csv(&query)?,
			)
			.await?;
			Ok((axum::http::StatusCode::OK, Json(response)))
		}
	};
}

macro_rules! repeatable_page_row_create_handler {
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		permission: $permission:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		aliases: $aliases:expr,
		extras_fn: $extras_fn:ident,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(case_id): Path<Uuid>,
			Json(request): Json<CaseEditorPagePatchRequest>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let requested_authorities =
				validate_request_projection_context(request.authorities.as_deref())?;

			let row = required_row_object($section, &request.rows, $row_key)?;
			let extras = $extras_fn(&ctx, &mm, case_id, row).await?;
			let value = row_model_value(row, $aliases, &extras);
			let create = parse_row_model::<$model>($section, $row_key, value)?;
			let row_id = $bmc::create(&ctx, &mm, create).await?;
			mark_editor_validation_cache_stale(
				&ctx,
				&mm,
				case_id,
				requested_authorities.clone(),
			)
			.await?;
			let response =
				$build_response(&ctx, &mm, case_id, row_id, requested_authorities)
					.await?;
			Ok((axum::http::StatusCode::CREATED, Json(response)))
		}
	};
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		permission: $permission:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		aliases: $aliases:expr,
		extras: |$case_id:ident, $row:ident| $extras:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(case_id): Path<Uuid>,
			Json(request): Json<CaseEditorPagePatchRequest>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let requested_authorities =
				validate_request_projection_context(request.authorities.as_deref())?;

			let row = required_row_object($section, &request.rows, $row_key)?;
			let extras = {
				let $case_id = case_id;
				let $row = row;
				$extras
			};
			let value = row_model_value(row, $aliases, &extras);
			let create = parse_row_model::<$model>($section, $row_key, value)?;
			let row_id = $bmc::create(&ctx, &mm, create).await?;
			mark_editor_validation_cache_stale(
				&ctx,
				&mm,
				case_id,
				requested_authorities.clone(),
			)
			.await?;
			let response =
				$build_response(&ctx, &mm, case_id, row_id, requested_authorities)
					.await?;
			Ok((axum::http::StatusCode::CREATED, Json(response)))
		}
	};
}

macro_rules! repeatable_page_row_patch_handler {
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		permission: $permission:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		verify: $verify_fn:ident,
		changes: $changes:expr,
		aliases: $aliases:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
			Json(request): Json<CaseEditorPagePatchRequest>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let requested_authorities =
				validate_request_projection_context(request.authorities.as_deref())?;

			$verify_fn(&ctx, &mm, case_id, row_id).await?;
			let synthesized_rows;
			let rows = if !request.changes.is_empty() {
				synthesized_rows = row_payload_from_changes(
					$section,
					$row_key,
					&request.changes,
					$changes,
				)?;
				&synthesized_rows
			} else {
				&request.rows
			};
			let row = required_row_object($section, rows, $row_key)?;
			let value = row_model_value(row, $aliases, &[]);
			let update = parse_row_model::<$model>($section, $row_key, value)?;
			$bmc::update(&ctx, &mm, row_id, update).await?;
			refresh_editor_validation_cache(
				&ctx,
				&mm,
				case_id,
				requested_authorities.clone(),
			)
			.await?;
			let response =
				$build_response(&ctx, &mm, case_id, row_id, requested_authorities)
					.await?;
			Ok((axum::http::StatusCode::OK, Json(response)))
		}
	};
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		permission: $permission:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		changes: $changes:expr,
		aliases: $aliases:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
			Json(request): Json<CaseEditorPagePatchRequest>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let requested_authorities =
				validate_request_projection_context(request.authorities.as_deref())?;

			$bmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
			let synthesized_rows;
			let rows = if !request.changes.is_empty() {
				synthesized_rows = row_payload_from_changes(
					$section,
					$row_key,
					&request.changes,
					$changes,
				)?;
				&synthesized_rows
			} else {
				&request.rows
			};
			let row = required_row_object($section, rows, $row_key)?;
			let value = row_model_value(row, $aliases, &[]);
			let update = parse_row_model::<$model>($section, $row_key, value)?;
			$bmc::update(&ctx, &mm, row_id, update).await?;
			refresh_editor_validation_cache(
				&ctx,
				&mm,
				case_id,
				requested_authorities.clone(),
			)
			.await?;
			let response =
				$build_response(&ctx, &mm, case_id, row_id, requested_authorities)
					.await?;
			Ok((axum::http::StatusCode::OK, Json(response)))
		}
	};
}

macro_rules! repeatable_page_row_delete_handler {
	(
		$fn_name:ident,
		permission: $permission:expr,
		bmc: $bmc:ident,
		verify: $verify_fn:ident $(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
		) -> Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

			$verify_fn(&ctx, &mm, case_id, row_id).await?;
			$bmc::delete(&ctx, &mm, row_id).await?;
			mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
			Ok(axum::http::StatusCode::NO_CONTENT)
		}
	};
}

macro_rules! repeatable_list_handler {
	(
		$fn_name:ident,
		$row_dto:ty,
		$list_permission:expr,
		$load_rows:ident,
		include_deleted
		$(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(case_id): Path<Uuid>,
		) -> Result<(
			axum::http::StatusCode,
			Json<CaseEditorListResponse<$row_dto>>,
		)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, CASE_READ)?;
			require_permission(&ctx, $list_permission)?;
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

			let rows = $load_rows(&ctx, &mm, case_id, false).await?;

			Ok((
				axum::http::StatusCode::OK,
				Json(CaseEditorListResponse { case_id, rows }),
			))
		}
	};
	(
		$fn_name:ident,
		$row_dto:ty,
		$list_permission:expr,
		$load_rows:ident
		$(,)?
	) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(case_id): Path<Uuid>,
		) -> Result<(
			axum::http::StatusCode,
			Json<CaseEditorListResponse<$row_dto>>,
		)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, CASE_READ)?;
			require_permission(&ctx, $list_permission)?;
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

			let rows = $load_rows(&ctx, &mm, case_id).await?;

			Ok((
				axum::http::StatusCode::OK,
				Json(CaseEditorListResponse { case_id, rows }),
			))
		}
	};
}

macro_rules! repeatable_page_row_delete_restore_handlers {
	(
		delete: $delete_fn:ident,
		restore: $restore_fn:ident,
		bmc: $bmc:ident,
		delete_permission: $delete_permission:expr,
		update_permission: $update_permission:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
		) -> Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $delete_permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

			$bmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
			$bmc::delete(&ctx, &mm, row_id).await?;
			mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
			Ok(axum::http::StatusCode::NO_CONTENT)
		}

		pub async fn $restore_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((case_id, row_id)): Path<(Uuid, Uuid)>,
		) -> Result<(axum::http::StatusCode, Json<Value>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, $update_permission)?;
			lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

			$bmc::get_in_case_with_deleted(&ctx, &mm, case_id, row_id, true).await?;
			$bmc::restore_in_case(&ctx, &mm, case_id, row_id).await?;
			mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
			let response = $build_response(&ctx, &mm, case_id, row_id, None).await?;
			Ok((axum::http::StatusCode::OK, Json(response)))
		}
	};
}

macro_rules! direct_page_projection_handler {
	(
		$fn_name:ident,
		$section:literal,
		$loader:ident,
		[$($perm:path),* $(,)?]
		$(,)?
	) => {
		pub async fn $fn_name(
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
			$( require_permission(&ctx, $perm)?; )*
			lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

			let projection = direct_page_projection_response(
				&ctx,
				&mm,
				case_id,
				$section,
				query_authorities_csv(&query)?,
				$loader(&ctx, &mm, case_id).await?,
			)
			.await?;
			Ok((axum::http::StatusCode::OK, Json(projection)))
		}
	};
}

pub(super) use direct_page_projection_handler;
pub(super) use repeatable_list_handler;
pub(super) use repeatable_page_row_create_handler;
pub(super) use repeatable_page_row_delete_handler;
pub(super) use repeatable_page_row_delete_restore_handlers;
pub(super) use repeatable_page_row_patch_handler;
pub(super) use repeatable_page_row_read_handler;
