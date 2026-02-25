use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ESignature {
	pub id: Uuid,
	pub case_id: Option<Uuid>,
	pub signer_user_id: Uuid,
	pub signer_username: String,
	pub action: String,
	pub meaning: String,
	pub reason: String,
	pub signature_method: String,
	pub signed_at: OffsetDateTime,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Fields, Deserialize)]
pub struct ESignatureForCreate {
	pub case_id: Option<Uuid>,
	pub signer_user_id: Uuid,
	pub signer_username: String,
	pub action: String,
	pub meaning: String,
	pub reason: String,
	pub signature_method: Option<String>,
	pub signed_at: Option<OffsetDateTime>,
}

pub struct ESignatureBmc;

impl DbBmc for ESignatureBmc {
	const TABLE: &'static str = "e_signatures";
}

impl ESignatureBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ESignatureForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}
}
