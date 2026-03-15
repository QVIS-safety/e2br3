use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::xml::import_runtime::shared::{self, ImportIdMap};
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_g(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	reaction_map: &ImportIdMap,
) -> Result<ImportIdMap> {
	let drug_map = shared::import_drugs(ctx, mm, xml, case_id).await?;
	shared::import_drug_recurrences(ctx, mm, xml, &drug_map).await?;
	shared::import_drug_reaction_assessments(ctx, mm, xml, &drug_map, reaction_map)
		.await?;
	Ok(drug_map)
}
