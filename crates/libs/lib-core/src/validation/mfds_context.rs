use crate::model::drug::DrugActiveSubstance;
use crate::model::safety_report::{SenderInformation, StudyInformation};
use crate::model::{ModelManager, Result};
use sqlx::types::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RelatednessWithDrug {
	pub drug_id: Uuid,
	pub relatedness_sequence_number: i32,
	pub source_of_assessment: Option<String>,
	pub method_of_assessment: Option<String>,
	pub result_of_assessment: Option<String>,
	pub result_of_assessment_kr2: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PastDrugByCase {
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ParentPastDrugByCase {
	pub parent_id: Uuid,
	pub sequence_number: i32,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MfdsValidationContext {
	pub senders: Vec<SenderInformation>,
	pub studies: Vec<StudyInformation>,
	pub active_substances: Vec<DrugActiveSubstance>,
	pub relatedness: Vec<RelatednessWithDrug>,
	pub past_drugs: Vec<PastDrugByCase>,
	pub parent_past_drugs: Vec<ParentPastDrugByCase>,
}

pub async fn load_mfds_validation_context(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<MfdsValidationContext> {
	Ok(MfdsValidationContext {
		senders: list_senders_by_case(mm, case_id).await?,
		studies: list_studies_by_case(mm, case_id).await?,
		active_substances: list_active_substances_by_case(mm, case_id).await?,
		relatedness: list_relatedness_by_case(mm, case_id).await?,
		past_drugs: list_past_drugs_by_case(mm, case_id).await?,
		parent_past_drugs: list_parent_past_drugs_by_case(mm, case_id).await?,
	})
}

async fn list_active_substances_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<DrugActiveSubstance>> {
	let sql = r#"
SELECT das.*
FROM drug_active_substances das
JOIN drug_information di ON di.id = das.drug_id
WHERE di.case_id = $1
ORDER BY di.sequence_number, das.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugActiveSubstance>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_relatedness_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<RelatednessWithDrug>> {
	let sql = r#"
SELECT di.id as drug_id
     , ra.sequence_number as relatedness_sequence_number
     , ra.source_of_assessment
     , ra.method_of_assessment
     , ra.result_of_assessment
     , ra.result_of_assessment_kr2
FROM relatedness_assessments ra
JOIN drug_reaction_assessments dra ON dra.id = ra.drug_reaction_assessment_id
JOIN drug_information di ON di.id = dra.drug_id
WHERE di.case_id = $1
ORDER BY di.sequence_number, ra.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, RelatednessWithDrug>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_past_drugs_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<PastDrugByCase>> {
	let sql = r#"
SELECT pdh.mpid
     , pdh.mpid_version
FROM past_drug_history pdh
JOIN patient_information pi ON pi.id = pdh.patient_id
WHERE pi.case_id = $1
ORDER BY pdh.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, PastDrugByCase>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_parent_past_drugs_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<ParentPastDrugByCase>> {
	let sql = r#"
SELECT pph.parent_id
     , pph.sequence_number
     , pph.mpid
     , pph.mpid_version
FROM parent_past_drug_history pph
JOIN parent_information parent ON parent.id = pph.parent_id
JOIN patient_information pi ON pi.id = parent.patient_id
WHERE pi.case_id = $1
ORDER BY parent.created_at, pph.sequence_number
"#;
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, ParentPastDrugByCase>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_senders_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<SenderInformation>> {
	let sql =
		"SELECT * FROM sender_information WHERE case_id = $1 ORDER BY created_at";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, SenderInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}

async fn list_studies_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<StudyInformation>> {
	let sql =
		"SELECT * FROM study_information WHERE case_id = $1 ORDER BY created_at, id";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyInformation>(sql).bind(case_id))
		.await
		.map_err(Into::into)
}
