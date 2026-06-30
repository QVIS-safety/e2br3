// G.k.8.r - Drug Recurrence Information

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValBool, OpValsBool, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- DrugRecurrenceInformation
// G.k.8.r - Structured recurrence data for rechallenge scenarios

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugRecurrenceInformation {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.8.r.1 - Rechallenge Action
	pub rechallenge_action: Option<String>, // 1-4

	// G.k.8.r.2a - MedDRA Version
	pub reaction_meddra_version: Option<String>,

	// G.k.8.r.2b - Reaction Recurred (MedDRA code)
	pub reaction_meddra_code: Option<String>,

	// G.k.8.r.3 - Did Reaction Recur on Readministration
	pub reaction_recurred: Option<String>, // 1-3

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugRecurrenceInformationForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub rechallenge_action: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_recurred: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugRecurrenceInformationForUpdate {
	pub rechallenge_action: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_recurred: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugRecurrenceInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMC

pub struct DrugRecurrenceInformationBmc;
impl DbBmc for DrugRecurrenceInformationBmc {
	const TABLE: &'static str = "drug_recurrence_information";
}

impl DrugRecurrenceInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugRecurrenceInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugRecurrenceInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugRecurrenceInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugRecurrenceInformation>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(DrugRecurrenceInformationFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugRecurrenceInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn list_by_drug(
		_ctx: &Ctx,
		mm: &ModelManager,
		drug_id: Uuid,
	) -> Result<Vec<DrugRecurrenceInformation>> {
		let sql = format!(
			"SELECT * FROM {} WHERE drug_id = $1 AND deleted = false ORDER BY sequence_number ASC",
			Self::TABLE
		);
		let entities = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugRecurrenceInformation>(&sql).bind(drug_id),
			)
			.await?;
		Ok(entities)
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}
