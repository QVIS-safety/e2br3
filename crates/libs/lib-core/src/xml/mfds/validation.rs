use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, load_base_validation_context, load_mfds_validation_context,
	CaseValidationReport, ValidationProfile,
};
use sqlx::types::Uuid;

use super::rules::apply_mfds_rules;

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;
	let mfds_ctx = load_mfds_validation_context(mm, case_id).await?;
	let mut issues = crate::xml::ich::validation::apply_ich_rules(&validation_ctx);
	apply_mfds_rules(&validation_ctx, &mfds_ctx, &mut issues);

	Ok(build_report(ValidationProfile::Mfds, case_id, issues))
}
