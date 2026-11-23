use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model::case::Case;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;

pub(crate) use super::shared::postprocess::apply_c_d_h_sections;
use super::{base_export_skeleton, sections};

pub(crate) async fn build_fresh_export_from_db(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	case: &Case,
	authority: RegulatoryAuthority,
) -> Result<String> {
	let mut xml = base_export_skeleton().to_string();
	xml =
		sections::c::export_patch(ctx, mm, case_id, case, xml.as_bytes(), authority)
			.await?;
	xml = sections::d::export_patch(ctx, mm, case_id, xml).await?;
	xml = sections::e::export_patch(ctx, mm, case_id, xml.as_bytes(), authority)
		.await?;
	xml = sections::f::export_patch(ctx, mm, case_id, xml.as_bytes()).await?;
	xml = sections::g::export_patch(ctx, mm, case_id, xml.as_bytes(), authority)
		.await?;
	sections::h::export_patch(ctx, mm, case_id, xml.as_bytes()).await
}
