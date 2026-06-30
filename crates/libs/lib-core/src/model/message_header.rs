// Section N - Batch/Message Headers

use crate::model::base::DbBmc;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- MessageHeader

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct MessageHeader {
	pub id: Uuid,
	pub case_id: Uuid,

	// N.1.1 - Batch Number
	pub batch_number: Option<String>,

	// N.1.2 - Batch Sender Identifier
	pub batch_sender_identifier: Option<String>,

	// N.1.3 - Batch Receiver Identifier (Phase 1 addition)
	pub batch_receiver_identifier: Option<String>,

	// N.1.4 - Date of Batch Transmission (Phase 1 addition)
	pub batch_transmission_date: Option<OffsetDateTime>,

	// Message identification
	pub message_type: String,           // ichicsr
	pub message_format_version: String, // 2.1
	pub message_format_release: String, // 2.0
	pub message_number: String,
	pub message_sender_identifier: String,
	pub message_receiver_identifier: String,
	pub message_date_format: String, // 204 (CCYYMMDDHHMMSS)
	pub message_date: String,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct MessageHeaderForCreate {
	pub case_id: Uuid,
	pub message_number: String,
	pub message_sender_identifier: String,
	pub message_receiver_identifier: String,
	pub message_date: String,
}

#[derive(Fields, Deserialize)]
pub struct MessageHeaderForUpdate {
	pub batch_number: Option<String>,
	pub batch_sender_identifier: Option<String>,
	pub batch_receiver_identifier: Option<String>,
	pub batch_transmission_date: Option<OffsetDateTime>,
	pub message_number: Option<String>,
	pub message_sender_identifier: Option<String>,
	pub message_receiver_identifier: Option<String>,
	pub message_date: Option<String>,
}

// -- BMC

pub struct MessageHeaderBmc;
impl DbBmc for MessageHeaderBmc {
	const TABLE: &'static str = "message_headers";
}

impl MessageHeaderBmc {
	pub async fn create(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		data: MessageHeaderForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"INSERT INTO {} (case_id, message_type, message_format_version, message_format_release, message_date_format, message_number, message_sender_identifier, message_receiver_identifier, message_date, created_at, updated_at, created_by)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now(), now(), $10)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind("ichicsr")
					.bind("2.1")
					.bind("2.0")
					.bind("204")
					.bind(data.message_number)
					.bind(data.message_sender_identifier)
					.bind(data.message_receiver_identifier)
					.bind(data.message_date)
					.bind(ctx.user_id()),
			)
			.await?;
		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get_by_case(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<MessageHeader> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let header = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, MessageHeader>(&sql).bind(case_id))
			.await?;
		match header {
			Some(header) => {
				mm.dbx().commit_txn().await?;
				Ok(header)
			}
			None => {
				mm.dbx().rollback_txn().await?;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id: case_id,
				})
			}
		}
	}

	pub async fn update_by_case(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: MessageHeaderForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {}
			 SET batch_number = COALESCE($2, batch_number),
			     batch_sender_identifier = COALESCE($3, batch_sender_identifier),
			     batch_receiver_identifier = COALESCE($4, batch_receiver_identifier),
			     batch_transmission_date = COALESCE($5, batch_transmission_date),
			     message_number = COALESCE($6, message_number),
			     message_sender_identifier = COALESCE($7, message_sender_identifier),
			     message_receiver_identifier = COALESCE($8, message_receiver_identifier),
			     message_date = COALESCE($9, message_date),
			     updated_at = now(),
			     updated_by = $10
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = match mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.batch_number)
					.bind(data.batch_sender_identifier)
					.bind(data.batch_receiver_identifier)
					.bind(data.batch_transmission_date)
					.bind(data.message_number)
					.bind(data.message_sender_identifier)
					.bind(data.message_receiver_identifier)
					.bind(data.message_date)
					.bind(ctx.user_id()),
			)
			.await
		{
			Ok(res) => res,
			Err(err) => {
				mm.dbx().rollback_txn().await?;
				return Err(err.into());
			}
		};
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id: case_id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete_by_case(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("DELETE FROM {} WHERE case_id = $1", Self::TABLE);
		let result = match mm.dbx().execute(sqlx::query(&sql).bind(case_id)).await {
			Ok(res) => res,
			Err(err) => {
				mm.dbx().rollback_txn().await?;
				return Err(err.into());
			}
		};
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id: case_id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}
