use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::xml::import_runtime::shared;
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_d(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let patient_id =
		shared::import_patient_information(ctx, mm, xml, case_id).await?;
	if let Some(patient_id) = patient_id {
		shared::import_patient_identifiers(ctx, mm, xml, patient_id).await?;
		shared::import_medical_history(ctx, mm, xml, patient_id).await?;
		shared::import_past_drug_history(ctx, mm, xml, patient_id).await?;
		shared::import_patient_death(ctx, mm, xml, patient_id).await?;
		shared::import_parent_information(ctx, mm, xml, patient_id).await?;
	}
	Ok(())
}
