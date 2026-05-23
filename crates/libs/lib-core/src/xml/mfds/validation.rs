use crate::ctx::Ctx;
use crate::model::{ModelManager, Result};
use crate::regulatory::RegulatoryAuthority;
use crate::validation::CaseValidationReport;
use sqlx::types::Uuid;

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseValidationReport> {
	crate::validation::case::validate_case_for_authority(
		ctx,
		mm,
		case_id,
		RegulatoryAuthority::Mfds,
	)
	.await
}
