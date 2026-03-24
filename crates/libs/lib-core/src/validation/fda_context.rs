use crate::ctx::Ctx;
use crate::model::drug::{
	derive_fda_device_characteristics, DrugDeviceCharacteristic, DrugInformationBmc,
};
use crate::model::safety_report::{StudyInformation, StudyRegistrationNumber};
use crate::model::{ModelManager, Result};
use sqlx::types::Uuid;

fn merge_fda_characteristics(
	mut derived: Vec<DrugDeviceCharacteristic>,
	raw: Vec<DrugDeviceCharacteristic>,
) -> Vec<DrugDeviceCharacteristic> {
	for row in raw {
		let code = row.code.as_deref().map(str::trim).unwrap_or("");
		if code.is_empty() {
			continue;
		}
		let duplicate = derived.iter().any(|existing| {
			existing.code.as_deref().map(str::trim) == Some(code)
				&& existing.value_code.as_deref().map(str::trim)
					== row.value_code.as_deref().map(str::trim)
				&& existing.value_value.as_deref().map(str::trim)
					== row.value_value.as_deref().map(str::trim)
		});
		if !duplicate {
			derived.push(row);
		}
	}
	derived.sort_by_key(|row| row.sequence_number);
	derived
}

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
	let sql = "SELECT * FROM drug_device_characteristics WHERE drug_id = $1 ORDER BY sequence_number";
	let raw = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, DrugDeviceCharacteristic>(sql).bind(drug_id))
		.await
		.map_err(crate::model::Error::from)?;
	Ok(merge_fda_characteristics(derived, raw))
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
