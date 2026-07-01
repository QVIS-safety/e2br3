// D.10.7 - Parent Medical History
// D.10.8 - Parent Past Drug History

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValBool, OpValsBool, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- ParentMedicalHistory
// D.10.7.1.r - Parent's relevant medical history episodes

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ParentMedicalHistory {
	pub id: Uuid,
	pub parent_id: Uuid,
	pub sequence_number: i32,

	// D.10.7.1.r.1a - MedDRA Version
	pub meddra_version: Option<String>,

	// D.10.7.1.r.1b - Parent's Relevant Medical History (MedDRA code)
	pub meddra_code: Option<String>,

	// D.10.7.1.r.2 - Start Date
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,

	// D.10.7.1.r.3 - Continuing
	pub continuing: Option<bool>,

	// D.10.7.1.r.4 - End Date
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,

	// D.10.7.1.r.5 - Comments
	pub comments: Option<String>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ParentMedicalHistoryForCreate {
	pub parent_id: Uuid,
	pub sequence_number: i32,
	pub meddra_code: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ParentMedicalHistoryForUpdate {
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub continuing: Option<bool>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub comments: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct ParentMedicalHistoryFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub parent_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- ParentPastDrugHistory
// D.10.8.r - Parent's past drug history

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ParentPastDrugHistory {
	pub id: Uuid,
	pub parent_id: Uuid,
	pub sequence_number: i32,

	// D.10.8.r.1 - Drug Name
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,

	// D.10.8.r.2 - MPID
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,

	// D.10.8.r.3 - PhPID
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,

	// D.10.8.r.4 - Start Date
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,

	// D.10.8.r.5 - End Date
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,

	// D.10.8.r.6a/b - Indication (MedDRA)
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,

	// D.10.8.r.7a/b - Reaction (MedDRA)
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ParentPastDrugHistoryForCreate {
	pub parent_id: Uuid,
	pub sequence_number: i32,
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ParentPastDrugHistoryForUpdate {
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct ParentPastDrugHistoryFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub parent_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMCs

pub struct ParentMedicalHistoryBmc;
impl DbBmc for ParentMedicalHistoryBmc {
	const TABLE: &'static str = "parent_medical_history";
}

impl ParentMedicalHistoryBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ParentMedicalHistoryForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ParentMedicalHistory> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<ParentMedicalHistoryFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ParentMedicalHistory>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(ParentMedicalHistoryFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ParentMedicalHistoryForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct ParentPastDrugHistoryBmc;
impl DbBmc for ParentPastDrugHistoryBmc {
	const TABLE: &'static str = "parent_past_drug_history";
}

impl ParentPastDrugHistoryBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ParentPastDrugHistoryForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ParentPastDrugHistory> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<ParentPastDrugHistoryFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ParentPastDrugHistory>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(ParentPastDrugHistoryFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ParentPastDrugHistoryForUpdate,
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
			"UPDATE {} SET
			 drug_name = CASE WHEN $1::varchar IS NOT NULL THEN NULL ELSE COALESCE($2, drug_name) END,
			 drug_name_null_flavor = CASE WHEN $2::varchar IS NOT NULL THEN NULL ELSE COALESCE($1, drug_name_null_flavor) END,
			 mpid = COALESCE($3, mpid),
			 mpid_version = COALESCE($4, mpid_version),
			 mfds_medicinal_product_version = COALESCE($5, mfds_medicinal_product_version),
			 mfds_medicinal_product_id = COALESCE($6, mfds_medicinal_product_id),
			 phpid = COALESCE($7, phpid),
			 phpid_version = COALESCE($8, phpid_version),
			 start_date = CASE WHEN $10::varchar IS NOT NULL THEN NULL ELSE COALESCE($9, start_date) END,
			 start_date_null_flavor = CASE WHEN $9::date IS NOT NULL THEN NULL ELSE COALESCE($10, start_date_null_flavor) END,
			 end_date = CASE WHEN $12::varchar IS NOT NULL THEN NULL ELSE COALESCE($11, end_date) END,
			 end_date_null_flavor = CASE WHEN $11::date IS NOT NULL THEN NULL ELSE COALESCE($12, end_date_null_flavor) END,
			 indication_meddra_version = COALESCE($13, indication_meddra_version),
			 indication_meddra_code = COALESCE($14, indication_meddra_code),
			 reaction_meddra_version = COALESCE($15, reaction_meddra_version),
			 reaction_meddra_code = COALESCE($16, reaction_meddra_code),
			 updated_at = now(),
			 updated_by = $17
			 WHERE id = $18",
			Self::TABLE
		);

		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.drug_name_null_flavor)
					.bind(data.drug_name)
					.bind(data.mpid)
					.bind(data.mpid_version)
					.bind(data.mfds_medicinal_product_version)
					.bind(data.mfds_medicinal_product_id)
					.bind(data.phpid)
					.bind(data.phpid_version)
					.bind(data.start_date)
					.bind(data.start_date_null_flavor)
					.bind(data.end_date)
					.bind(data.end_date_null_flavor)
					.bind(data.indication_meddra_version)
					.bind(data.indication_meddra_code)
					.bind(data.reaction_meddra_version)
					.bind(data.reaction_meddra_code)
					.bind(ctx.user_id())
					.bind(id),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}
