// C.1.9.r - Other Case Identifiers
// C.1.10.r - Linked Report Numbers

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

// -- OtherCaseIdentifier
// C.1.9.r - Additional identifiers from other sources (e.g., regulatory authority numbers)

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct OtherCaseIdentifier {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// C.1.9.1.r.1 - Source of the Case Identifier
	pub source_of_identifier: String,

	// C.1.9.1.r.2 - Case Identifier
	pub case_identifier: String,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct OtherCaseIdentifierForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	pub source_of_identifier: String,
	pub case_identifier: String,
}

#[derive(Fields, Deserialize)]
pub struct OtherCaseIdentifierForUpdate {
	pub source_of_identifier: Option<String>,
	pub case_identifier: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct OtherCaseIdentifierFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- LinkedReportNumber
// C.1.10.r - Links to follow-up reports, amendments, related cases

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct LinkedReportNumber {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// C.1.10.r - Linked Report Number
	pub linked_report_number: String,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct LinkedReportNumberForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	pub linked_report_number: String,
}

#[derive(Fields, Deserialize)]
pub struct LinkedReportNumberForUpdate {
	pub linked_report_number: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct LinkedReportNumberFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMCs

pub struct OtherCaseIdentifierBmc;
impl DbBmc for OtherCaseIdentifierBmc {
	const TABLE: &'static str = "other_case_identifiers";
}

impl OtherCaseIdentifierBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: OtherCaseIdentifierForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<OtherCaseIdentifier> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<OtherCaseIdentifierFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<OtherCaseIdentifier>> {
		let mut filters = filters.unwrap_or_else(|| vec![Default::default()]);
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: OtherCaseIdentifierForUpdate,
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

pub struct LinkedReportNumberBmc;
impl DbBmc for LinkedReportNumberBmc {
	const TABLE: &'static str = "linked_report_numbers";
}

impl LinkedReportNumberBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: LinkedReportNumberForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<LinkedReportNumber> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<LinkedReportNumberFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<LinkedReportNumber>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(LinkedReportNumberFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: LinkedReportNumberForUpdate,
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
