pub(crate) mod sections;

use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::validation::{
	build_report, load_base_validation_context, load_fda_validation_context,
	load_mfds_validation_context, CaseValidationReport, FdaValidationContext,
	MfdsValidationContext, RegulatoryAuthority, ValidationProfile,
};
use sqlx::types::Uuid;

pub async fn validate_case_for_profile(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profile: ValidationProfile,
) -> Result<CaseValidationReport> {
	let reports =
		validate_case_for_profiles(ctx, mm, case_id, &[profile]).await?;
	Ok(reports.into_iter().next().expect("single profile returns one report"))
}

/// Validate a case against multiple profiles simultaneously, sharing the
/// base context load.  Returns one `CaseValidationReport` per profile in the
/// same order as `profiles`.  Duplicate profiles are deduplicated internally
/// (each context is loaded at most once).
pub async fn validate_case_for_profiles(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profiles: &[ValidationProfile],
) -> Result<Vec<CaseValidationReport>> {
	if profiles.is_empty() {
		return Ok(Vec::new());
	}

	let validation_ctx = load_base_validation_context(ctx, mm, case_id).await?;

	let needs_fda = profiles
		.iter()
		.any(|p| RegulatoryAuthority::from_validation_profile(*p).requires_fda_context());
	let needs_mfds = profiles
		.iter()
		.any(|p| RegulatoryAuthority::from_validation_profile(*p).requires_mfds_context());

	let fda_ctx: Option<FdaValidationContext> = if needs_fda {
		Some(load_fda_validation_context(mm, case_id).await?)
	} else {
		None
	};
	let mfds_ctx: Option<MfdsValidationContext> = if needs_mfds {
		Some(load_mfds_validation_context(mm, case_id).await?)
	} else {
		None
	};

	let mut reports = Vec::with_capacity(profiles.len());
	for &profile in profiles {
		let issues = sections::collect_section_issues(
			profile,
			mm,
			&validation_ctx,
			fda_ctx.as_ref(),
			mfds_ctx.as_ref(),
		)
		.await?;
		reports.push(build_report(profile, case_id, issues));
	}
	Ok(reports)
}
