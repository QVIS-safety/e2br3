//! Shared imports and helpers for case editor REST modules.

pub(super) use super::input_contract_save::{
	validate_direct_rows, validate_row_payload,
};
pub(super) use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorCiCaseDto, CaseEditorCiDocumentDto,
	CaseEditorCiLinkedReportDto, CaseEditorCiOtherIdentifierDto,
	CaseEditorCiRowsDto, CaseEditorCiSafetyReportDto, CaseEditorCiSourceDocumentDto,
	CaseEditorDgListRowDto, CaseEditorDhListRowDto, CaseEditorDirectSectionResponse,
	CaseEditorLbListRowDto, CaseEditorPagePatchRequest,
	CaseEditorPageProjectionResponse, CaseEditorRowDetailResponse,
	CaseEditorShellDto,
};
pub(super) use crate::web::rest::case_rest::case_to_read_result;
pub(super) use axum::extract::{Path, Query, State};
pub(super) use axum::Json;
pub(super) use lib_core::model::case::{
	CaseBmc, CaseForUpdate, SourceDocumentBmc, SourceDocumentFilter,
	SourceDocumentForCreate, SourceDocumentForUpdate,
};
pub(super) use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberFilter, LinkedReportNumberForCreate,
	LinkedReportNumberForUpdate, OtherCaseIdentifierBmc, OtherCaseIdentifierFilter,
	OtherCaseIdentifierForCreate, OtherCaseIdentifierForUpdate,
};
pub(super) use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
pub(super) use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationFilter, DrugActiveSubstanceBmc,
	DrugActiveSubstanceFilter, DrugIndicationBmc, DrugIndicationFilter,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
pub(super) use lib_core::model::drug_reaction_assessment::DrugReactionAssessmentBmc;
pub(super) use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationFilter,
	CaseSummaryInformationForCreate, CaseSummaryInformationForUpdate,
	NarrativeInformationBmc, NarrativeInformationForCreate,
	NarrativeInformationForUpdate, SenderDiagnosisBmc, SenderDiagnosisFilter,
	SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
pub(super) use lib_core::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryFilter,
	ParentMedicalHistoryForCreate, ParentMedicalHistoryForUpdate,
	ParentPastDrugHistoryBmc, ParentPastDrugHistoryFilter,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
pub(super) use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeFilter, MedicalHistoryEpisodeForCreate,
	MedicalHistoryEpisodeForUpdate, ParentInformationBmc, ParentInformationFilter,
	ParentInformationForCreate, ParentInformationForUpdate, PastDrugHistoryBmc,
	PastDrugHistoryFilter, PastDrugHistoryForCreate, PastDrugHistoryForUpdate,
	PatientDeathInformationBmc, PatientDeathInformationFilter,
	PatientDeathInformationForCreate, PatientDeathInformationForUpdate,
	PatientIdentifierBmc, PatientIdentifierFilter, PatientIdentifierForCreate,
	PatientIdentifierForUpdate, PatientInformationBmc, PatientInformationForCreate,
	PatientInformationForUpdate, ReportedCauseOfDeathBmc,
	ReportedCauseOfDeathFilter, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
pub(super) use lib_core::model::reaction::{
	ReactionBmc, ReactionForCreate, ReactionForUpdate,
};
pub(super) use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
pub(super) use lib_core::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter,
	DocumentsHeldBySenderForCreate, DocumentsHeldBySenderForUpdate,
	LiteratureReferenceBmc, LiteratureReferenceFilter, LiteratureReferenceForCreate,
	LiteratureReferenceForUpdate, PatchValue, PrimarySourceBmc, PrimarySourceFilter,
	PrimarySourceForCreate, PrimarySourceForUpdate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForUpdate, SenderInformationBmc,
	SenderInformationFilter, SenderInformationForCreate, SenderInformationForUpdate,
	StudyFdaCrossReportedIndBmc, StudyFdaCrossReportedIndFilter,
	StudyFdaCrossReportedIndForCreate, StudyFdaCrossReportedIndForUpdate,
	StudyInformationBmc, StudyInformationFilter, StudyInformationForCreate,
	StudyInformationForUpdate, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
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

// Row lists are patches: [] changes nothing; [{}] adds an unfinished row.
pub(super) fn without_empty_row_lists(
	rows: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
	rows.iter()
		.filter_map(|(key, value)| {
			if value.as_array().is_some_and(Vec::is_empty) {
				return None;
			}
			let mut value = value.clone();
			let nested: &[&str] = match key.as_str() {
				"studyInformation" => &["fdaCrossReportedIndNumbers"],
				"parentInfo" => &["medicalHistoryEpisodes", "pastDrugHistory"],
				"deathInfo" => &["reportedCausesOfDeath", "autopsyCausesOfDeath"],
				_ => &[],
			};
			if let Some(object) = value.as_object_mut() {
				let before = object.len();
				object.retain(|key, value| {
					!(nested.contains(&key.as_str())
						&& value.as_array().is_some_and(Vec::is_empty))
				});
				if object.len() != before && object.keys().all(|key| key == "id") {
					return None;
				}
			}
			Some((key.clone(), value))
		})
		.collect()
}

pub(super) async fn next_child_sequence(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	table: &'static str,
	parent_column: &'static str,
	parent_id: Uuid,
	active_only: bool,
) -> Result<i32> {
	let deleted_filter = if active_only {
		" AND deleted = false"
	} else {
		""
	};
	let sql = format!(
		"SELECT COALESCE(MAX(sequence_number), 0) + 1 FROM {table} WHERE {parent_column} = $1{deleted_filter}"
	);
	lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_one(sqlx::query_as::<_, (i32,)>(&sql).bind(parent_id))
				.await
				.map_err(lib_core::model::Error::from)
				.map_err(Error::from)
				.map(|(sequence,)| sequence)
		})
	})
	.await
}

pub(super) fn child_row_has_content(row: &Map<String, Value>) -> bool {
	row.iter().any(|(key, value)| {
		if matches!(
			key.as_str(),
			"id" | "deleted"
				| "_delete" | "sequenceNumber"
				| "drugReactionAssessmentId"
				| "reactionId"
				| "_deleteAssessment"
		) {
			return false;
		}
		match value {
			Value::Null => false,
			Value::String(value) => !value.trim().is_empty(),
			Value::Array(values) => !values.is_empty(),
			Value::Object(values) => !values.is_empty(),
			Value::Bool(_) | Value::Number(_) => true,
		}
	})
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

/// Invalidate list-view validation summaries after any persisted editor change.
/// Full field-level validation is requested by the editor after the save.
pub(super) async fn mark_editor_validation_summary_stale(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	requested_authorities: Option<String>,
) -> Result<()> {
	editor_projection_context(requested_authorities)?;
	CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, case_id).await?;
	Ok(())
}

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
				return (!value.trim().is_empty()).then(|| value.to_string());
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
		let mut segments = alias.split('.');
		let Some(first) = segments.next() else {
			continue;
		};
		let mut value = map.get(first);
		for segment in segments {
			value = value
				.and_then(Value::as_object)
				.and_then(|object| object.get(segment));
		}
		if let Some(value) = value {
			map.insert(target.to_string(), value.clone());
			return;
		}
	}
}

pub(super) fn explicit_null_model_fields(
	row: &serde_json::Map<String, Value>,
	aliases: &'static [(&'static str, &'static [&'static str])],
) -> Vec<&'static str> {
	fn value_at_path<'a>(
		row: &'a serde_json::Map<String, Value>,
		path: &str,
	) -> Option<&'a Value> {
		let mut segments = path.split('.');
		let mut value = row.get(segments.next()?)?;
		for segment in segments {
			value = value.as_object()?.get(segment)?;
		}
		Some(value)
	}

	aliases
		.iter()
		.filter_map(|(field, candidates)| {
			std::iter::once(*field)
				.chain(candidates.iter().copied())
				.find_map(|path| value_at_path(row, path))
				.is_some_and(Value::is_null)
				.then_some(*field)
		})
		.collect()
}

pub(super) fn row_model_value(
	_section: &str,
	_request_prefix: &str,
	row: &serde_json::Map<String, Value>,
	aliases: &[(&str, &[&str])],
	extra: &[(&str, Value)],
) -> Value {
	fn omit_blank_strings(value: &mut Value) {
		match value {
			Value::Object(map) => {
				map.retain(|_, value| {
					!value.as_str().is_some_and(|value| value.trim().is_empty())
				});
				for value in map.values_mut() {
					omit_blank_strings(value);
				}
			}
			Value::Array(values) => {
				for value in values {
					omit_blank_strings(value);
				}
			}
			_ => {}
		}
	}

	let mut map = row.clone();
	for (target, aliases) in aliases {
		insert_alias(&mut map, target, aliases);
	}
	for (key, value) in extra {
		map.insert((*key).to_string(), value.clone());
	}
	let mut value = Value::Object(map);
	omit_blank_strings(&mut value);
	// These NOT NULL text columns use an empty string for an unfinished draft.
	for (section, source, target) in [
		("LB", "testName", "test_name"),
		("DG", "medicinalProduct", "medicinal_product"),
		("DG", "drugCharacterization", "drug_characterization"),
	] {
		if _section == section && _request_prefix.is_empty() {
			if let Some(text) = row
				.get(source)
				.or_else(|| row.get(target))
				.and_then(Value::as_str)
			{
				value[target] = json!(text);
			}
		}
	}
	value
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

pub(super) fn ci_date(value: Option<sqlx::types::time::Date>) -> Option<String> {
	value.map(|date| {
		format!(
			"{:04}{:02}{:02}",
			date.year(),
			u8::from(date.month()),
			date.day()
		)
	})
}

pub(super) fn ci_ts(value: Option<&str>) -> Option<String> {
	value.map(str::to_owned)
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
			.map(|value| !value.is_null())
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
#[cfg(test)]
mod canonical_row_persistence_tests {
	use super::*;

	#[test]
	fn lb_split_null_flavor_is_mapped_directly() {
		let row = json!({ "testDate": null, "testDateNullFlavor": "UNK" })
			.as_object()
			.expect("row object")
			.clone();
		let value = row_model_value(
			"LB",
			"",
			&row,
			&[
				("test_date", &["testDate"]),
				("test_date_null_flavor", &["testDateNullFlavor"]),
			],
			&[],
		);
		let model = value.as_object().expect("model object");

		assert_eq!(model.get("test_date"), Some(&Value::Null));
		assert_eq!(model.get("test_date_null_flavor"), Some(&json!("UNK")));
	}

	#[test]
	fn dh_split_null_flavor_is_mapped_directly() {
		let row = json!({ "drugName": null, "drugNameNullFlavor": "UNK" })
			.as_object()
			.expect("row object")
			.clone();
		let value = row_model_value(
			"DH",
			"",
			&row,
			&[
				("drug_name", &["drugName"]),
				("drug_name_null_flavor", &["drugNameNullFlavor"]),
			],
			&[],
		);
		let model = value.as_object().expect("model object");

		assert_eq!(model.get("drug_name"), Some(&Value::Null));
		assert_eq!(model.get("drug_name_null_flavor"), Some(&json!("UNK")));
	}

	#[test]
	fn blank_text_is_omitted_instead_of_reaching_char_columns() {
		let row = json!({ "drugName": " \t", "drugNameNullFlavor": "UNK" })
			.as_object()
			.expect("row object")
			.clone();
		let value = row_model_value(
			"DH",
			"",
			&row,
			&[
				("drug_name", &["drugName"]),
				("drug_name_null_flavor", &["drugNameNullFlavor"]),
			],
			&[],
		);
		let model = value.as_object().expect("model object");

		assert!(!model.contains_key("drug_name"));
		assert_eq!(model.get("drug_name_null_flavor"), Some(&json!("UNK")));
	}

	#[test]
	fn ae_required_intervention_maps_as_bool() {
		let row = json!({ "requiredIntervention": true })
			.as_object()
			.expect("row object")
			.clone();
		let value = row_model_value(
			"AE",
			"",
			&row,
			&[("required_intervention", &["requiredIntervention"])],
			&[],
		);
		let model = parse_row_model::<ReactionForUpdate>("AE", "reaction", value)
			.expect("typed reaction update");

		assert_eq!(model.required_intervention, Some(true));
	}

	#[test]
	fn parent_medical_history_keeps_continuing_null_flavor() {
		let model = parse_row_model::<ParentMedicalHistoryForUpdate>(
			"DM",
			"parentMedicalHistory",
			json!({"continuing_null_flavor": "NASK"}),
		)
		.expect("typed parent history update");

		assert_eq!(model.continuing_null_flavor.as_deref(), Some("NASK"));
	}

	#[test]
	fn sd_saved_uses_sender_object_presence() {
		assert!(direct_page_saved(
			"SD",
			&json!({"senderInformation": {"organizationName": "Big Pharma"}}),
		));
		assert!(!direct_page_saved(
			"SD",
			&json!({"senderInformation": null})
		));
	}
}
