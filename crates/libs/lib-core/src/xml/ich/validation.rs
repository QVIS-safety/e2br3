use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, load_base_validation_context, CaseValidationReport,
	ValidationProfile,
};
use sqlx::types::Uuid;

pub(crate) use super::rules::apply_ich_rules;

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;
	let issues = apply_ich_rules(&validation_ctx);

	Ok(build_report(ValidationProfile::Ich, case_id, issues))
}
