pub(crate) mod sections;

use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::validation::{
	build_report, load_base_validation_context, load_fda_validation_context,
	load_mfds_validation_context, CaseValidationReport, RegulatoryAuthority,
	ValidationProfile,
};
use sqlx::types::Uuid;

pub async fn validate_case_for_profile(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profile: ValidationProfile,
) -> Result<CaseValidationReport> {
	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;
	let authority = RegulatoryAuthority::from_validation_profile(profile);
	let fda_ctx = if authority.requires_fda_context() {
		Some(load_fda_validation_context(mm, case_id).await?)
	} else {
		None
	};
	let mfds_ctx = if authority.requires_mfds_context() {
		Some(load_mfds_validation_context(mm, case_id).await?)
	} else {
		None
	};

	let issues = sections::collect_section_issues(
		profile,
		mm,
		&validation_ctx,
		fda_ctx.as_ref(),
		mfds_ctx.as_ref(),
	)
	.await?;

	Ok(build_report(profile, case_id, issues))
}
