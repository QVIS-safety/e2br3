use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::validation::{CaseValidationReport, ValidationProfile};
use sqlx::types::Uuid;

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	crate::validation::case::validate_case_for_profile(
		ctx,
		mm,
		case_id,
		ValidationProfile::Ich,
	)
	.await
}
