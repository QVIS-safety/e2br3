pub(crate) mod sections;

use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::validation::{
	build_report, load_base_validation_context, load_fda_validation_context,
	load_mfds_validation_context, CaseValidationReport, FdaValidationContext,
	MfdsValidationContext, RegulatoryAuthority,
};
use sqlx::types::Uuid;

pub async fn validate_case_for_authority(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
) -> Result<CaseValidationReport> {
	let reports =
		validate_case_for_authorities(ctx, mm, case_id, &[authority]).await?;
	Ok(reports
		.into_iter()
		.next()
		.expect("single authority returns one report"))
}

/// Validate a case against multiple authorities simultaneously, sharing the
/// base context load.  Returns one `CaseValidationReport` per authority in the
/// same order as `authorities`.  Duplicate authorities are deduplicated internally
/// (each context is loaded at most once).
pub async fn validate_case_for_authorities(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authorities: &[RegulatoryAuthority],
) -> Result<Vec<CaseValidationReport>> {
	if authorities.is_empty() {
		return Ok(Vec::new());
	}

	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;

	let needs_fda = authorities.iter().any(|p| p.requires_fda_context());
	let needs_mfds = authorities.iter().any(|p| p.requires_mfds_context());

	let fda_ctx: Option<FdaValidationContext> = if needs_fda {
		Some(load_fda_validation_context(ctx, mm, case_id).await?)
	} else {
		None
	};
	let mfds_ctx: Option<MfdsValidationContext> = if needs_mfds {
		Some(load_mfds_validation_context(ctx, mm, case_id).await?)
	} else {
		None
	};

	let mut reports = Vec::with_capacity(authorities.len());
	for &authority in authorities {
		let issues = sections::collect_section_issues(
			ctx,
			authority,
			mm,
			&validation_ctx,
			fda_ctx.as_ref(),
			mfds_ctx.as_ref(),
		)
		.await?;
		reports.push(build_report(authority, case_id, issues));
	}
	Ok(reports)
}
