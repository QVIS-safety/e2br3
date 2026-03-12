use crate::ctx::Ctx;
use crate::model::drug::{
	derive_fda_device_characteristics, DrugDeviceCharacteristic, DrugInformationBmc,
};
use crate::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use crate::model::{ModelManager, Result};
use sqlx::types::Uuid;

#[derive(Debug, Clone)]
pub struct FdaValidationContext {
	pub studies: Vec<StudyInformation>,
}

pub async fn load_fda_validation_context(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<FdaValidationContext> {
	let studies = list_studies(mm, case_id).await?;
	Ok(FdaValidationContext { studies })
}

pub async fn list_study_registrations(
	mm: &ModelManager,
	study_id: Uuid,
) -> Result<Vec<StudyRegistrationNumber>> {
	let sql = "SELECT * FROM study_registration_numbers WHERE study_information_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, StudyRegistrationNumber>(sql).bind(study_id))
		.await
		.map_err(Into::into)
}

pub async fn list_drug_characteristics(
	mm: &ModelManager,
	drug_id: Uuid,
) -> Result<Vec<DrugDeviceCharacteristic>> {
	let sys_ctx = Ctx::root_ctx();
	let drug = DrugInformationBmc::get(&sys_ctx, mm, drug_id).await?;
	let derived = derive_fda_device_characteristics(&drug);
	if !derived.is_empty() {
		return Ok(derived);
	}
	let sql = "SELECT * FROM drug_device_characteristics WHERE drug_id = $1 ORDER BY sequence_number";
	mm.dbx()
		.fetch_all(sqlx::query_as::<_, DrugDeviceCharacteristic>(sql).bind(drug_id))
		.await
		.map_err(Into::into)
}

async fn list_studies(
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
