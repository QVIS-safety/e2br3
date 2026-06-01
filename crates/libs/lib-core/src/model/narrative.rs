// Section H - Narrative and Other Information

use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- NarrativeInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct NarrativeInformation {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_narrative_presave_id: Option<Uuid>,

	// H.1 - Case Narrative
	pub case_narrative: String,

	// H.2 - Reporter's Comments
	pub reporter_comments: Option<String>,

	// H.4 - Sender's Comments
	pub sender_comments: Option<String>,

	// NR_SPONSOR - Additional Information
	pub additional_information: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NarrativeInformationForCreate {
	pub case_id: Uuid,
	pub source_narrative_presave_id: Option<Uuid>,
	pub case_narrative: String,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
	pub additional_information: Option<String>,
}

#[derive(Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NarrativeInformationForUpdate {
	pub source_narrative_presave_id: Option<Uuid>,
	pub case_narrative: Option<String>,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
	pub additional_information: Option<String>,
}

// -- SenderDiagnosis

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderDiagnosis {
	pub id: Uuid,
	pub narrative_id: Uuid,
	pub sequence_number: i32,

	// H.3.r.1 - Diagnosis/Syndrome (MedDRA coded)
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct SenderDiagnosisForCreate {
	pub narrative_id: Uuid,
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct SenderDiagnosisForUpdate {
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct SenderDiagnosisFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub narrative_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- CaseSummaryInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct CaseSummaryInformation {
	pub id: Uuid,
	pub narrative_id: Uuid,
	pub sequence_number: i32,

	// H.5.r.1 - Case Summary Type
	pub summary_type: Option<String>,

	// H.5.r.2 - Case Summary Language
	pub language_code: Option<String>,

	// H.5.r.3 - Text
	pub summary_text: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct CaseSummaryInformationForCreate {
	pub narrative_id: Uuid,
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct CaseSummaryInformationForUpdate {
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct CaseSummaryInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub narrative_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- BMCs

pub struct NarrativeInformationBmc;
impl DbBmc for NarrativeInformationBmc {
	const TABLE: &'static str = "narrative_information";
}

impl NarrativeInformationBmc {
	pub async fn create(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		data: NarrativeInformationForCreate,
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
					source_narrative_presave_id,
					case_narrative,
					reporter_comments,
					sender_comments,
				additional_information,
				created_at,
				updated_at,
					created_by
				)
				 VALUES ($1, $2, $3, $4, $5, $6, now(), now(), $7)
				 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.source_narrative_presave_id)
					.bind(data.case_narrative)
					.bind(data.reporter_comments)
					.bind(data.sender_comments)
					.bind(data.additional_information)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get_by_case(
		_ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<NarrativeInformation> {
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let narrative = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, NarrativeInformation>(&sql).bind(case_id),
			)
			.await?;
		narrative.ok_or(crate::model::Error::EntityUuidNotFound {
			entity: Self::TABLE,
			id: case_id,
		})
	}

	pub async fn get_by_case_optional(
		_ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Option<NarrativeInformation>> {
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let narrative = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, NarrativeInformation>(&sql).bind(case_id),
			)
			.await?;
		Ok(narrative)
	}

	pub async fn update_by_case(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: NarrativeInformationForUpdate,
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
				 SET source_narrative_presave_id = COALESCE($2, source_narrative_presave_id),
				     case_narrative = COALESCE($3, case_narrative),
				     reporter_comments = COALESCE($4, reporter_comments),
				     sender_comments = COALESCE($5, sender_comments),
				     additional_information = COALESCE($6, additional_information),
				     updated_at = now(),
				     updated_by = $7
				 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.source_narrative_presave_id)
					.bind(data.case_narrative)
					.bind(data.reporter_comments)
					.bind(data.sender_comments)
					.bind(data.additional_information)
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

pub struct SenderDiagnosisBmc;
impl DbBmc for SenderDiagnosisBmc {
	const TABLE: &'static str = "sender_diagnoses";
}

impl SenderDiagnosisBmc {
	pub async fn create(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		data: SenderDiagnosisForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<SenderDiagnosis> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		filters: Option<Vec<SenderDiagnosisFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<SenderDiagnosis>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: SenderDiagnosisForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct CaseSummaryInformationBmc;
impl DbBmc for CaseSummaryInformationBmc {
	const TABLE: &'static str = "case_summary_information";
}

impl CaseSummaryInformationBmc {
	pub async fn create(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		data: CaseSummaryInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<CaseSummaryInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		filters: Option<Vec<CaseSummaryInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<CaseSummaryInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: CaseSummaryInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(
		ctx: &crate::ctx::Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
