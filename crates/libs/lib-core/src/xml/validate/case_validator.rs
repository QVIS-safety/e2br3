use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::xml::validate::{CaseValidationReport, ValidationProfile};
use sqlx::types::Uuid;

pub async fn validate_case_for_profile(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profile: ValidationProfile,
) -> Result<CaseValidationReport> {
	match profile {
		ValidationProfile::Ich => {
			crate::xml::ich::validation::validate_case(ctx, mm, case_id).await
		}
		ValidationProfile::Fda => {
			crate::xml::fda::validation::validate_case(ctx, mm, case_id).await
		}
		ValidationProfile::Mfds => {
			crate::xml::mfds::validation::validate_case(ctx, mm, case_id).await
		}
	}
}
