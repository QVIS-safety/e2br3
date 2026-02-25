use lib_core::ctx::Ctx;
use lib_core::model::e_signature::{ESignatureBmc, ESignatureForCreate};
use lib_core::model::user::{User, UserBmc};
use lib_core::model::ModelManager;
use lib_rest_core::{Error, Result};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct ESignatureInput {
	pub meaning: String,
	pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComplianceActionInput {
	pub reason_for_change: String,
	pub e_signature: ESignatureInput,
}

impl ComplianceActionInput {
	pub fn validate(&self) -> Result<()> {
		if self.reason_for_change.trim().is_empty() {
			return Err(Error::BadRequest {
				message: "reason_for_change is required".to_string(),
			});
		}
		if self.e_signature.meaning.trim().is_empty() {
			return Err(Error::BadRequest {
				message: "e_signature.meaning is required".to_string(),
			});
		}
		if self.e_signature.password.trim().is_empty() {
			return Err(Error::BadRequest {
				message: "e_signature.password is required".to_string(),
			});
		}
		Ok(())
	}
}

pub async fn capture_e_signature(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Option<Uuid>,
	action: &str,
	input: &ComplianceActionInput,
) -> Result<Uuid> {
	input.validate()?;

	let verified = UserBmc::verify_password(
		ctx,
		mm,
		ctx.user_id(),
		&input.e_signature.password,
	)
	.await?;
	if !verified {
		return Err(Error::BadRequest {
			message: "invalid e-signature credentials".to_string(),
		});
	}

	let user: User = UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let signature_id = ESignatureBmc::create(
		ctx,
		mm,
		ESignatureForCreate {
			case_id,
			signer_user_id: ctx.user_id(),
			signer_username: user.username,
			action: action.to_string(),
			meaning: input.e_signature.meaning.trim().to_string(),
			reason: input.reason_for_change.trim().to_string(),
			signature_method: Some("password_reentry".to_string()),
			signed_at: Some(time::OffsetDateTime::now_utc()),
		},
	)
	.await?;

	Ok(signature_id)
}
