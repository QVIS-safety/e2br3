use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::xml::validate::{
	build_report, load_base_validation_context, load_fda_validation_context,
	CaseValidationReport, ValidationProfile,
};
use sqlx::types::Uuid;

use super::rules::apply_fda_rules;

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;
	let fda_ctx = load_fda_validation_context(mm, case_id).await?;
	let mut issues = crate::xml::ich::validation::apply_ich_rules(&validation_ctx);
	apply_fda_rules(mm, &validation_ctx, &fda_ctx, &mut issues).await?;

	Ok(build_report(ValidationProfile::Fda, case_id, issues))
}
