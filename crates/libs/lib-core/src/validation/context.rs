use crate::ctx::Ctx;
use crate::model::case::{Case, CaseBmc};
use crate::model::case_identifiers::OtherCaseIdentifier;
use crate::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugIndication, DrugInformation,
};
use crate::model::drug_reaction_assessment::DrugReactionAssessment;
use crate::model::message_header::{MessageHeader, MessageHeaderBmc};
use crate::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, NarrativeInformationBmc,
	SenderDiagnosis,
};
use crate::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use crate::model::patient::{
	AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
	PatientInformationBmc, ReportedCauseOfDeath,
};
use crate::model::reaction::Reaction;
use crate::model::safety_report::{
	DocumentsHeldBySender, PrimarySource, PrimarySourceBmc, PrimarySourceFilter,
	SafetyReportIdentification, SafetyReportIdentificationBmc, SenderInformation,
	SenderInformationBmc, SenderInformationFilter, StudyInformation,
};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::test_result::TestResult;
use crate::model::{ModelManager, Result};
use modql::filter::{OpValValue, OpValsValue};
use serde_json::json;
use sqlx::types::Uuid;

#[derive(Debug, Clone)]
pub struct ValidationContext {
	pub case: Case,
	pub safety_report: Option<SafetyReportIdentification>,
	pub message_header: Option<MessageHeader>,
	pub sender: Option<SenderInformation>,
	pub patient: Option<PatientInformation>,
	pub narrative: Option<NarrativeInformation>,
	pub sender_diagnoses: Vec<SenderDiagnosis>,
	pub case_summaries: Vec<CaseSummaryInformation>,
	pub medical_history: Vec<MedicalHistoryEpisode>,
	pub past_drugs: Vec<PastDrugHistory>,
	pub death_info: Option<PatientDeathInformation>,
	pub reported_causes_of_death: Vec<ReportedCauseOfDeath>,
	pub autopsy_causes_of_death: Vec<AutopsyCauseOfDeath>,
	pub parents: Vec<ParentInformation>,
	pub parent_medical_history: Vec<ParentMedicalHistory>,
	pub parent_past_drugs: Vec<ParentPastDrugHistory>,
	pub primary_sources: Vec<PrimarySource>,
	pub documents_held_by_sender: Vec<DocumentsHeldBySender>,
	pub other_case_identifiers: Vec<OtherCaseIdentifier>,
	pub studies: Vec<StudyInformation>,
	pub reactions: Vec<Reaction>,
	pub tests: Vec<TestResult>,
	pub drugs: Vec<DrugInformation>,
	pub active_substances: Vec<DrugActiveSubstance>,
	pub indications: Vec<DrugIndication>,
	pub dosages: Vec<DosageInformation>,
	pub drug_reaction_assessments: Vec<DrugReactionAssessment>,
	pub patient_identifiers: Vec<PatientIdentifier>,
}

pub async fn load_base_validation_context(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<ValidationContext> {
	set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;
	let (case, safety_report, message_header, sender, patient, narrative) = tokio::try_join!(
		CaseBmc::get(ctx, mm, case_id),
		get_safety_report_optional(ctx, mm, case_id),
		get_message_header_optional(ctx, mm, case_id),
		get_sender_optional(ctx, mm, case_id),
		get_patient_optional(ctx, mm, case_id),
		get_narrative_optional(ctx, mm, case_id),
	)?;

	let (sender_diagnoses, case_summaries) = tokio::try_join!(
		list_sender_diagnoses(mm, narrative.as_ref()),
		list_case_summaries(mm, narrative.as_ref()),
	)?;

	let (medical_history, past_drugs, death_info, parents) = tokio::try_join!(
		list_medical_history(mm, patient.as_ref()),
		list_past_drugs(mm, patient.as_ref()),
		get_death_info_optional(mm, patient.as_ref()),
		list_parents(mm, patient.as_ref()),
	)?;

	let (reported_causes_of_death, autopsy_causes_of_death) = tokio::try_join!(
		list_reported_causes_of_death(mm, death_info.as_ref()),
		list_autopsy_causes_of_death(mm, death_info.as_ref()),
	)?;

	let (parent_medical_history, parent_past_drugs) = tokio::try_join!(
		list_parent_medical_history(mm, &parents),
		list_parent_past_drugs(mm, &parents),
	)?;

	let (
		primary_sources,
		documents_held_by_sender,
		other_case_identifiers,
		studies,
		reactions,
		tests,
		drugs,
	) = tokio::try_join!(
		list_primary_sources(ctx, mm, case_id),
		list_documents_held_by_sender(mm, case_id),
		list_other_case_identifiers(mm, case_id),
		list_studies(mm, case_id),
		crate::model::reaction::ReactionBmc::list_by_case(ctx, mm, case_id),
		crate::model::test_result::TestResultBmc::list_by_case(ctx, mm, case_id),
		crate::model::drug::DrugInformationBmc::list_by_case(ctx, mm, case_id),
	)?;

	let (active_substances, indications, dosages, drug_reaction_assessments) = tokio::try_join!(
		list_active_substances(mm, &drugs),
		list_indications(mm, &drugs),
		list_dosages(mm, &drugs),
		list_drug_reaction_assessments(mm, &drugs),
	)?;
	let patient_identifiers = list_patient_identifiers(mm, patient.as_ref()).await?;

	Ok(ValidationContext {
		case,
		safety_report,
		message_header,
		sender,
		patient,
		narrative,
		sender_diagnoses,
		case_summaries,
		medical_history,
		past_drugs,
		death_info,
		reported_causes_of_death,
		autopsy_causes_of_death,
		parents,
		parent_medical_history,
		parent_past_drugs,
		primary_sources,
		documents_held_by_sender,
		other_case_identifiers,
		studies,
		reactions,
		tests,
		drugs,
		active_substances,
		indications,
		dosages,
		drug_reaction_assessments,
		patient_identifiers,
	})
}

async fn get_safety_report_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SafetyReportIdentification>> {
	match SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(crate::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_message_header_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<MessageHeader>> {
	match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(crate::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_patient_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<PatientInformation>> {
	match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(crate::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn get_narrative_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<NarrativeInformation>> {
	match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(value) => Ok(Some(value)),
		Err(crate::model::Error::EntityUuidNotFound { .. }) => Ok(None),
		Err(err) => Err(err),
	}
}

async fn list_primary_sources(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<PrimarySource>> {
	let filter = PrimarySourceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case_id))])),
		..Default::default()
	};
	let mut rows = PrimarySourceBmc::list(ctx, mm, Some(vec![filter]), None).await?;
	rows.sort_by_key(|row| row.sequence_number);
	Ok(rows)
}

async fn list_other_case_identifiers(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<OtherCaseIdentifier>> {
	let sql = "SELECT * FROM other_case_identifiers WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, OtherCaseIdentifier>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_documents_held_by_sender(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DocumentsHeldBySender>> {
	let sql =
		"SELECT * FROM documents_held_by_sender WHERE case_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DocumentsHeldBySender>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_active_substances(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugActiveSubstance>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugActiveSubstance>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_dosages(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DosageInformation>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM dosage_information WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DosageInformation>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_indications(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugIndication>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_indications WHERE drug_id = ANY($1) ORDER BY drug_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugIndication>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_drug_reaction_assessments(
	mm: &ModelManager,
	drugs: &[DrugInformation],
) -> Result<Vec<DrugReactionAssessment>> {
	if drugs.is_empty() {
		return Ok(Vec::new());
	}
	let drug_ids: Vec<Uuid> = drugs.iter().map(|drug| drug.id).collect();
	let sql = "SELECT * FROM drug_reaction_assessments WHERE drug_id = ANY($1) ORDER BY drug_id, reaction_id";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugReactionAssessment>(sql).bind(&drug_ids))
		.await
		.map_err(Into::into)
}

async fn list_patient_identifiers(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<PatientIdentifier>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM patient_identifiers WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PatientIdentifier>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_medical_history(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<MedicalHistoryEpisode>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM medical_history_episodes WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, MedicalHistoryEpisode>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_sender_diagnoses(
	mm: &ModelManager,
	narrative: Option<&NarrativeInformation>,
) -> Result<Vec<SenderDiagnosis>> {
	let Some(narrative) = narrative else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM sender_diagnoses WHERE narrative_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, SenderDiagnosis>(sql).bind(narrative.id))
		.await
		.map_err(Into::into)
}

async fn list_case_summaries(
	mm: &ModelManager,
	narrative: Option<&NarrativeInformation>,
) -> Result<Vec<CaseSummaryInformation>> {
	let Some(narrative) = narrative else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM case_summary_information WHERE narrative_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, CaseSummaryInformation>(sql).bind(narrative.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_past_drugs(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<PastDrugHistory>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM past_drug_history WHERE patient_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PastDrugHistory>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn get_death_info_optional(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Option<PatientDeathInformation>> {
	let Some(patient) = patient else {
		return Ok(None);
	};
	let sql = "SELECT * FROM patient_death_information WHERE patient_id = $1";
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, PatientDeathInformation>(sql).bind(patient.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_reported_causes_of_death(
	mm: &ModelManager,
	death_info: Option<&PatientDeathInformation>,
) -> Result<Vec<ReportedCauseOfDeath>> {
	let Some(death_info) = death_info else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM reported_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, ReportedCauseOfDeath>(sql).bind(death_info.id),
		)
		.await
		.map_err(Into::into)
}

async fn list_autopsy_causes_of_death(
	mm: &ModelManager,
	death_info: Option<&PatientDeathInformation>,
) -> Result<Vec<AutopsyCauseOfDeath>> {
	let Some(death_info) = death_info else {
		return Ok(Vec::new());
	};
	let sql = "SELECT * FROM autopsy_causes_of_death WHERE death_info_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, AutopsyCauseOfDeath>(sql).bind(death_info.id))
		.await
		.map_err(Into::into)
}

async fn list_parents(
	mm: &ModelManager,
	patient: Option<&PatientInformation>,
) -> Result<Vec<ParentInformation>> {
	let Some(patient) = patient else {
		return Ok(Vec::new());
	};
	let sql =
		"SELECT * FROM parent_information WHERE patient_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentInformation>(sql).bind(patient.id))
		.await
		.map_err(Into::into)
}

async fn list_parent_medical_history(
	mm: &ModelManager,
	parents: &[ParentInformation],
) -> Result<Vec<ParentMedicalHistory>> {
	if parents.is_empty() {
		return Ok(Vec::new());
	}
	let parent_ids: Vec<Uuid> = parents.iter().map(|parent| parent.id).collect();
	let sql = "SELECT * FROM parent_medical_history WHERE parent_id = ANY($1) ORDER BY parent_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentMedicalHistory>(sql).bind(parent_ids))
		.await
		.map_err(Into::into)
}

async fn list_parent_past_drugs(
	mm: &ModelManager,
	parents: &[ParentInformation],
) -> Result<Vec<ParentPastDrugHistory>> {
	if parents.is_empty() {
		return Ok(Vec::new());
	}
	let parent_ids: Vec<Uuid> = parents.iter().map(|parent| parent.id).collect();
	let sql = "SELECT * FROM parent_past_drug_history WHERE parent_id = ANY($1) ORDER BY parent_id, sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentPastDrugHistory>(sql).bind(parent_ids))
		.await
		.map_err(Into::into)
}

async fn list_studies(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<StudyInformation>> {
	let sql =
		"SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn get_sender_optional(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Option<SenderInformation>> {
	let mut senders = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(case_id))])),
		}]),
		None,
	)
	.await?;
	senders.sort_by_key(|sender| sender.created_at);
	Ok(senders.into_iter().next())
}
