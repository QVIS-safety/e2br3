use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorDgListRowDto, CaseEditorDhListRowDto,
	CaseEditorDirectSectionResponse, CaseEditorLbListRowDto, CaseEditorListResponse,
	CaseEditorRowDetailResponse, CaseEditorShellDto,
};
use crate::web::rest::case_rest::case_to_read_result;
use axum::extract::{Path, State};
use axum::Json;
use lib_core::model::acs::{
	CASE_IDENTIFIER_LIST, CASE_READ, CASE_SUMMARY_LIST, DEATH_CAUSE_LIST,
	DRUG_DOSAGE_LIST, DRUG_INDICATION_LIST, DRUG_LIST,
	DRUG_REACTION_ASSESSMENT_LIST, DRUG_READ, DRUG_RECURRENCE_LIST,
	DRUG_SUBSTANCE_LIST, LITERATURE_REFERENCE_LIST, MEDICAL_HISTORY_LIST,
	MESSAGE_HEADER_READ, NARRATIVE_READ, PARENT_INFORMATION_LIST,
	PARENT_MEDICAL_HISTORY_LIST, PARENT_PAST_DRUG_LIST, PAST_DRUG_LIST,
	PAST_DRUG_READ, PATIENT_DEATH_LIST, PATIENT_IDENTIFIER_LIST, PATIENT_READ,
	PRIMARY_SOURCE_LIST, REACTION_LIST, REACTION_READ, RECEIVER_READ,
	SAFETY_REPORT_READ, SENDER_DIAGNOSIS_LIST, SENDER_INFORMATION_LIST,
	STUDY_INFORMATION_LIST, STUDY_REGISTRATION_LIST, TEST_RESULT_LIST,
	TEST_RESULT_READ,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberFilter, OtherCaseIdentifierBmc,
	OtherCaseIdentifierFilter,
};
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
	LiteratureReferenceFilter, PrimarySourceBmc, PrimarySourceFilter,
	SafetyReportIdentificationBmc, SenderInformationBmc, SenderInformationFilter,
	StudyInformationBmc, StudyInformationFilter, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter,
};
use lib_core::model::test_result::TestResultBmc;
use lib_core::model::ModelManager;
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::{json, Value};
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

	let case = CaseBmc::get(&ctx, &mm, case_id).await?;
	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let message_header =
		match MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let receiver_information =
		ReceiverInformationBmc::get_by_case_optional(&ctx, &mm, case_id).await?;
	let other_case_identifiers = OtherCaseIdentifierBmc::list(
		&ctx,
		&mm,
		Some(vec![OtherCaseIdentifierFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let linked_reports = LinkedReportNumberBmc::list(
		&ctx,
		&mm,
		Some(vec![LinkedReportNumberFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let documents_held_by_sender = DocumentsHeldBySenderBmc::list(
		&ctx,
		&mm,
		Some(vec![DocumentsHeldBySenderFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut safety_report_identification = safety_report_identification
		.map(|entity| json!(entity))
		.unwrap_or_else(|| json!({}));
	if let Value::Object(ref mut map) = safety_report_identification {
		map.insert(
			"otherCaseIdentifiers".to_string(),
			json!(other_case_identifiers),
		);
		map.insert("linkedReports".to_string(), json!(linked_reports));
		map.insert(
			"documentsHeldBySender".to_string(),
			json!(documents_held_by_sender),
		);
	}

	Ok(direct_section_response(
		case_id,
		json!({
			"case": case,
			"safetyReportIdentification": safety_report_identification,
			"messageHeader": message_header,
			"receiverInformation": receiver_information,
			"receiver": receiver_information,
		}),
	))
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

	let primary_sources = PrimarySourceBmc::list(
		&ctx,
		&mm,
		Some(vec![PrimarySourceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(direct_section_response(
		case_id,
		json!({ "primarySources": primary_sources }),
	))
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

	let safety_report_identification =
		match SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		};
	let sender_information = SenderInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(uuid_eq(case_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let sender = sender_information.first().cloned();

	Ok(direct_section_response(
		case_id,
		json!({
			"safetyReportIdentification": safety_report_identification,
			"senderInformation": sender_information,
			"sender": sender,
		}),
	))
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

	let literature_references = LiteratureReferenceBmc::list(
		&ctx,
		&mm,
		Some(vec![LiteratureReferenceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok(direct_section_response(
		case_id,
		json!({ "literatureReferences": literature_references }),
	))
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

	let mut studies = StudyInformationBmc::list(
		&ctx,
		&mm,
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
			&ctx,
			&mm,
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
	let mut study_information = study_information
		.map(|entity| json!(entity))
		.unwrap_or_else(|| json!({}));
	if let Value::Object(ref mut map) = study_information {
		map.insert(
			"studyRegistrationNumbers".to_string(),
			json!(study_registration_numbers),
		);
	}

	Ok(direct_section_response(
		case_id,
		json!({ "studyInformation": study_information }),
	))
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

	let Some(patient) =
		(match PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		})
	else {
		return Ok(direct_section_response(
			case_id,
			json!({
				"patientInformation": null,
				"patientIdentifiers": [],
				"medicalHistoryEpisodes": [],
				"patientDeath": {
					"reportedCausesOfDeath": [],
					"autopsyCausesOfDeath": [],
				},
				"parents": [],
				"parentInformation": {
					"medicalHistory": [],
					"pastDrugHistory": [],
					"pastDrugs": [],
				},
			}),
		));
	};

	let patient_id = patient.id;
	let patient_identifiers = PatientIdentifierBmc::list(
		&ctx,
		&mm,
		Some(vec![PatientIdentifierFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let medical_history_episodes = MedicalHistoryEpisodeBmc::list(
		&ctx,
		&mm,
		Some(vec![MedicalHistoryEpisodeFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let parent_information_rows = ParentInformationBmc::list(
		&ctx,
		&mm,
		Some(vec![ParentInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::default()),
	)
	.await?;
	let mut parents = Vec::new();
	for parent in parent_information_rows {
		let medical_history = ParentMedicalHistoryBmc::list(
			&ctx,
			&mm,
			Some(vec![ParentMedicalHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let past_drug_history = ParentPastDrugHistoryBmc::list(
			&ctx,
			&mm,
			Some(vec![ParentPastDrugHistoryFilter {
				parent_id: Some(uuid_eq(parent.id)),
				..Default::default()
			}]),
			Some(ListOptions::default()),
		)
		.await?;
		let mut parent = json!(parent);
		if let Value::Object(ref mut map) = parent {
			map.insert("medicalHistory".to_string(), json!(medical_history));
			map.insert("pastDrugHistory".to_string(), json!(past_drug_history));
			map.insert("pastDrugs".to_string(), json!(past_drug_history));
		}
		parents.push(parent);
	}
	let death_information = PatientDeathInformationBmc::list(
		&ctx,
		&mm,
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
				&ctx,
				&mm,
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
				&ctx,
				&mm,
				Some(vec![AutopsyCauseOfDeathFilter {
					death_info_id: Some(uuid_eq(death_info.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?,
		);
	}
	let mut patient_death = death_information
		.into_iter()
		.next()
		.map(|entity| json!(entity))
		.unwrap_or_else(|| json!({}));
	if let Value::Object(ref mut map) = patient_death {
		map.insert("reportedCausesOfDeath".to_string(), json!(reported_causes));
		map.insert("autopsyCausesOfDeath".to_string(), json!(autopsy_causes));
	}
	let parent_information = parents.first().cloned().unwrap_or_else(|| {
		json!({
			"medicalHistory": [],
			"pastDrugHistory": [],
			"pastDrugs": [],
		})
	});

	let mut patient_information = json!(patient);
	if let Value::Object(ref mut map) = patient_information {
		map.insert("patientIdentifiers".to_string(), json!(patient_identifiers));
		map.insert(
			"medicalHistoryEpisodes".to_string(),
			json!(medical_history_episodes),
		);
		map.insert("parentInformation".to_string(), json!(parent_information));
		map.insert("parents".to_string(), json!(parents));
		map.insert("patientDeath".to_string(), patient_death);
	}

	Ok(direct_section_response(
		case_id,
		json!({ "patientInformation": patient_information }),
	))
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

	let narrative =
		NarrativeInformationBmc::get_by_case_optional(&ctx, &mm, case_id).await?;
	let (sender_diagnoses, case_summary_information) =
		if let Some(ref narrative) = narrative {
			let sender_diagnoses = SenderDiagnosisBmc::list(
				&ctx,
				&mm,
				Some(vec![SenderDiagnosisFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			let case_summary_information = CaseSummaryInformationBmc::list(
				&ctx,
				&mm,
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
	let mut narrative = narrative
		.map(|entity| json!(entity))
		.unwrap_or_else(|| json!({}));
	if let Value::Object(ref mut map) = narrative {
		map.insert("senderDiagnoses".to_string(), json!(sender_diagnoses));
	}

	Ok(direct_section_response(
		case_id,
		json!({
			"narrative": narrative,
			"caseSummaryInformation": case_summary_information,
		}),
	))
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
