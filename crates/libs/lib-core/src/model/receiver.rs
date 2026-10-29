// Section A - Receiver Information

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::store::{
	set_full_context_dbx_or_rollback, set_full_context_from_ctx_dbx,
};
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- ReceiverInformation
// A.1.4 through A.1.5.10 - Receiver details for routing messages to regulatory authorities

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReceiverInformation {
	pub id: Uuid,
	pub case_id: Uuid,

	// A.1.4 - Receiver Type
	pub receiver_type: Option<String>, // 1-6 (same codes as sender_type)

	// A.1.5.1 - Receiver Organization
	pub organization_name: Option<String>,

	// A.1.5.2 - Receiver Department
	pub department: Option<String>,

	// A.1.5.3 - Receiver Street Address
	pub street_address: Option<String>,

	// A.1.5.4 - Receiver City
	pub city: Option<String>,

	// A.1.5.5 - Receiver State/Province
	pub state_province: Option<String>,

	// A.1.5.6 - Receiver Postcode
	pub postcode: Option<String>,

	// A.1.5.7 - Receiver Country Code
	pub country_code: Option<String>, // ISO 3166-1 alpha-2

	// A.1.5.8 - Receiver Telephone
	pub telephone: Option<String>,

	// A.1.5.9 - Receiver Fax
	pub fax: Option<String>,

	// A.1.5.10 - Receiver Email
	pub email: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ReceiverInformationForCreate {
	pub case_id: Uuid,
	pub receiver_type: Option<String>,
	pub organization_name: Option<String>,
	pub department: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state_province: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ReceiverInformationForUpdate {
	pub receiver_type: Option<String>,
	pub organization_name: Option<String>,
	pub department: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state_province: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

// -- BMC

pub struct ReceiverInformationBmc;
impl DbBmc for ReceiverInformationBmc {
	const TABLE: &'static str = "receiver_information";
}

impl ReceiverInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ReceiverInformationForCreate,
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
			"INSERT INTO {} (
				case_id,
				receiver_type,
				organization_name,
				department,
				street_address,
				city,
				state_province,
				postcode,
				country_code,
				telephone,
				fax,
				email,
				created_at,
				updated_at,
				created_by
			)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now(), $13)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.receiver_type)
					.bind(data.organization_name)
					.bind(data.department)
					.bind(data.street_address)
					.bind(data.city)
					.bind(data.state_province)
					.bind(data.postcode)
					.bind(data.country_code)
					.bind(data.telephone)
					.bind(data.fax)
					.bind(data.email)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<ReceiverInformation> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let entity = match mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, ReceiverInformation>(&sql).bind(case_id),
			)
			.await
		{
			Ok(Some(entity)) => entity,
			Ok(None) => {
				mm.dbx().commit_txn().await?;
				return Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id: case_id,
				});
			}
			Err(err) => {
				mm.dbx().rollback_txn().await?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await?;
		Ok(entity)
	}

	pub async fn get_by_case_optional(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Option<ReceiverInformation>> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let entity = match mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, ReceiverInformation>(&sql).bind(case_id),
			)
			.await
		{
			Ok(entity) => entity,
			Err(err) => {
				mm.dbx().rollback_txn().await?;
				return Err(err.into());
			}
		};
		mm.dbx().commit_txn().await?;
		Ok(entity)
	}

	pub async fn update_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: ReceiverInformationForUpdate,
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
			 SET receiver_type = COALESCE($2, receiver_type),
			     organization_name = COALESCE($3, organization_name),
			     department = COALESCE($4, department),
			     street_address = COALESCE($5, street_address),
			     city = COALESCE($6, city),
			     state_province = COALESCE($7, state_province),
			     postcode = COALESCE($8, postcode),
			     country_code = COALESCE($9, country_code),
			     telephone = COALESCE($10, telephone),
			     fax = COALESCE($11, fax),
			     email = COALESCE($12, email),
			     updated_at = now(),
			     updated_by = $13
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.receiver_type)
					.bind(data.organization_name)
					.bind(data.department)
					.bind(data.street_address)
					.bind(data.city)
					.bind(data.state_province)
					.bind(data.postcode)
					.bind(data.country_code)
					.bind(data.telephone)
					.bind(data.fax)
					.bind(data.email)
					.bind(ctx.user_id()),
			)
			.await?;

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

	pub async fn update_by_case_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: ReceiverInformationForUpdate,
		clear_fields: &[&str],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let clears: Vec<String> =
			clear_fields.iter().map(|field| (*field).into()).collect();
		let sql = format!("UPDATE {} SET
			receiver_type = CASE WHEN 'receiver_type' = ANY($14) THEN NULL ELSE COALESCE($2, receiver_type) END,
			organization_name = CASE WHEN 'organization_name' = ANY($14) THEN NULL ELSE COALESCE($3, organization_name) END,
			department = CASE WHEN 'department' = ANY($14) THEN NULL ELSE COALESCE($4, department) END,
			street_address = CASE WHEN 'street_address' = ANY($14) THEN NULL ELSE COALESCE($5, street_address) END,
			city = CASE WHEN 'city' = ANY($14) THEN NULL ELSE COALESCE($6, city) END,
			state_province = CASE WHEN 'state_province' = ANY($14) THEN NULL ELSE COALESCE($7, state_province) END,
			postcode = CASE WHEN 'postcode' = ANY($14) THEN NULL ELSE COALESCE($8, postcode) END,
			country_code = CASE WHEN 'country_code' = ANY($14) THEN NULL ELSE COALESCE($9, country_code) END,
			telephone = CASE WHEN 'telephone' = ANY($14) THEN NULL ELSE COALESCE($10, telephone) END,
			fax = CASE WHEN 'fax' = ANY($14) THEN NULL ELSE COALESCE($11, fax) END,
			email = CASE WHEN 'email' = ANY($14) THEN NULL ELSE COALESCE($12, email) END,
			updated_at = now(), updated_by = $13 WHERE case_id = $1", Self::TABLE);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.receiver_type)
					.bind(data.organization_name)
					.bind(data.department)
					.bind(data.street_address)
					.bind(data.city)
					.bind(data.state_province)
					.bind(data.postcode)
					.bind(data.country_code)
					.bind(data.telephone)
					.bind(data.fax)
					.bind(data.email)
					.bind(ctx.user_id())
					.bind(clears),
			)
			.await?;
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
		ctx: &Ctx,
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
		let result = mm.dbx().execute(sqlx::query(&sql).bind(case_id)).await?;

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
