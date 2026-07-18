// G.k.9.i - Drug-Reaction Assessment (Causality)

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValBool, OpValsBool, OpValsValue};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- DrugReactionAssessment
// Links a drug (G.k) to a reaction (E.i) with causality assessment data

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugReactionAssessment {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub reaction_id: Uuid,

	// G.k.9.i.3.1a/b - Time Interval between Beginning of Drug Administration and Start of Reaction / Event
	pub administration_start_interval_value: Option<Decimal>,
	pub administration_start_interval_unit: Option<String>, // 800-805

	// G.k.9.i.3.2a/b - Time Interval between Last Dose of Drug and Start of Reaction / Event
	pub last_dose_interval_value: Option<Decimal>,
	pub last_dose_interval_unit: Option<String>, // 800-805

	// G.k.9.i.4.r.1 - Did Reaction Recur on Readministration - Action
	pub recurrence_action: Option<String>, // 1-4

	// G.k.9.i.4.r.3 - Did Reaction Recur on Readministration
	pub reaction_recurred: Option<String>, // 1-3

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugReactionAssessmentForCreate {
	pub drug_id: Uuid,
	pub reaction_id: Uuid,
	pub administration_start_interval_value: Option<Decimal>,
	pub administration_start_interval_unit: Option<String>,
	pub last_dose_interval_value: Option<Decimal>,
	pub last_dose_interval_unit: Option<String>,
	pub recurrence_action: Option<String>,
	pub reaction_recurred: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugReactionAssessmentForUpdate {
	pub administration_start_interval_value: Option<Decimal>,
	pub administration_start_interval_unit: Option<String>,
	pub last_dose_interval_value: Option<Decimal>,
	pub last_dose_interval_unit: Option<String>,
	pub recurrence_action: Option<String>,
	pub reaction_recurred: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugReactionAssessmentFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub reaction_id: Option<OpValsValue>,
}

// -- RelatednessAssessment
// G.k.9.i.2.r - Multiple assessments per drug-reaction pair

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct RelatednessAssessment {
	pub id: Uuid,
	pub drug_reaction_assessment_id: Uuid,
	pub sequence_number: i32,

	// G.k.9.i.2.r.1 - Source of Assessment
	pub source_of_assessment: Option<String>,

	// G.k.9.i.2.r.2 - Method of Assessment
	pub method_of_assessment: Option<String>,

	// G.k.9.i.2.r.3 - Result of Assessment
	pub result_of_assessment: Option<String>,
	// MFDS.G.k.9.i.2.r.3.KR.2 - Additional KR assessment result text
	pub result_of_assessment_kr2: Option<String>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct RelatednessAssessmentForCreate {
	pub drug_reaction_assessment_id: Uuid,
	pub sequence_number: i32,
	pub source_of_assessment: Option<String>,
	pub method_of_assessment: Option<String>,
	pub result_of_assessment: Option<String>,
	pub result_of_assessment_kr2: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct RelatednessAssessmentForUpdate {
	pub source_of_assessment: Option<String>,
	pub method_of_assessment: Option<String>,
	pub result_of_assessment: Option<String>,
	pub result_of_assessment_kr2: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct RelatednessAssessmentFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_reaction_assessment_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMCs

pub struct DrugReactionAssessmentBmc;
impl DbBmc for DrugReactionAssessmentBmc {
	const TABLE: &'static str = "drug_reaction_assessments";
}

impl DrugReactionAssessmentBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugReactionAssessmentForCreate,
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
			 drug_id, reaction_id, administration_start_interval_value,
			 administration_start_interval_unit, last_dose_interval_value,
			 last_dose_interval_unit, recurrence_action,
			 reaction_recurred, created_at, updated_at, created_by
			)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $10, now(), now(), $11)
			 ON CONFLICT (drug_id, reaction_id)
			 DO UPDATE SET
			  administration_start_interval_value = COALESCE(EXCLUDED.administration_start_interval_value, drug_reaction_assessments.administration_start_interval_value),
			  administration_start_interval_unit = COALESCE(EXCLUDED.administration_start_interval_unit, drug_reaction_assessments.administration_start_interval_unit),
			  last_dose_interval_value = COALESCE(EXCLUDED.last_dose_interval_value, drug_reaction_assessments.last_dose_interval_value),
			  last_dose_interval_unit = COALESCE(EXCLUDED.last_dose_interval_unit, drug_reaction_assessments.last_dose_interval_unit),
			  recurrence_action = COALESCE(EXCLUDED.recurrence_action, drug_reaction_assessments.recurrence_action),
			  reaction_recurred = COALESCE(EXCLUDED.reaction_recurred, drug_reaction_assessments.reaction_recurred),
			  updated_at = now(),
			  updated_by = EXCLUDED.created_by
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.drug_id)
					.bind(data.reaction_id)
					.bind(data.administration_start_interval_value)
					.bind(data.administration_start_interval_unit)
					.bind(data.last_dose_interval_value)
					.bind(data.last_dose_interval_unit)
					.bind(data.recurrence_action)
					.bind(Option::<String>::None)
					.bind(Option::<String>::None)
					.bind(data.reaction_recurred)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugReactionAssessment> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugReactionAssessmentFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugReactionAssessment>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn list_by_drug(
		ctx: &Ctx,
		mm: &ModelManager,
		drug_id: Uuid,
	) -> Result<Vec<DrugReactionAssessment>> {
		let filter = DrugReactionAssessmentFilter {
			drug_id: Some(OpValsValue::from(vec![modql::filter::OpValValue::Eq(
				serde_json::json!(drug_id),
			)])),
			..Default::default()
		};
		base_uuid::list::<Self, _, _>(ctx, mm, Some(vec![filter]), None).await
	}

	pub async fn list_by_reaction(
		ctx: &Ctx,
		mm: &ModelManager,
		reaction_id: Uuid,
	) -> Result<Vec<DrugReactionAssessment>> {
		let filter = DrugReactionAssessmentFilter {
			reaction_id: Some(OpValsValue::from(vec![
				modql::filter::OpValValue::Eq(serde_json::json!(reaction_id)),
			])),
			..Default::default()
		};
		base_uuid::list::<Self, _, _>(ctx, mm, Some(vec![filter]), None).await
	}

	pub async fn get_by_drug_and_reaction(
		ctx: &Ctx,
		mm: &ModelManager,
		drug_id: Uuid,
		reaction_id: Uuid,
	) -> Result<Option<DrugReactionAssessment>> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"SELECT * FROM {} WHERE drug_id = $1 AND reaction_id = $2",
			Self::TABLE
		);
		let entity = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, DrugReactionAssessment>(&sql)
					.bind(drug_id)
					.bind(reaction_id),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(entity)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugReactionAssessmentForUpdate,
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
			 SET administration_start_interval_value = COALESCE($2, administration_start_interval_value),
			     administration_start_interval_unit = COALESCE($3, administration_start_interval_unit),
			     last_dose_interval_value = COALESCE($4, last_dose_interval_value),
			     last_dose_interval_unit = COALESCE($5, last_dose_interval_unit),
			     recurrence_action = COALESCE($6, recurrence_action),
			     reaction_recurred = COALESCE($9, reaction_recurred),
			     updated_at = now(),
			     updated_by = $10
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(data.administration_start_interval_value)
					.bind(data.administration_start_interval_unit)
					.bind(data.last_dose_interval_value)
					.bind(data.last_dose_interval_unit)
					.bind(data.recurrence_action)
					.bind(Option::<String>::None)
					.bind(Option::<String>::None)
					.bind(data.reaction_recurred)
					.bind(ctx.user_id()),
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
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("DELETE FROM {} WHERE id = $1", Self::TABLE);
		let result = mm.dbx().execute(sqlx::query(&sql).bind(id)).await?;

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
}

pub struct RelatednessAssessmentBmc;
impl DbBmc for RelatednessAssessmentBmc {
	const TABLE: &'static str = "relatedness_assessments";
}

impl RelatednessAssessmentBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: RelatednessAssessmentForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<RelatednessAssessment> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<RelatednessAssessmentFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<RelatednessAssessment>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(RelatednessAssessmentFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: RelatednessAssessmentForUpdate,
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
