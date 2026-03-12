use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsString, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Case {
	pub id: Uuid,
	pub organization_id: Uuid,

	// E2B fields
	pub safety_report_id: String,
	pub version: i32,
	pub dg_prd_key: Option<String>,
	pub status: String,
	pub validation_profile: Option<String>,
	pub appendices_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,

	// Workflow
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
	pub submitted_by: Option<Uuid>,
	pub submitted_at: Option<OffsetDateTime>,

	// Raw imported XML (optional)
	pub raw_xml: Option<Vec<u8>>,
	pub dirty_c: bool,
	pub dirty_d: bool,
	pub dirty_e: bool,
	pub dirty_f: bool,
	pub dirty_g: bool,
	pub dirty_h: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Fields, Deserialize)]
pub struct CaseForCreate {
	pub organization_id: Uuid,
	pub safety_report_id: String,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub validation_profile: Option<String>,
	pub appendices_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub version: Option<i32>,
}

#[derive(Fields, Deserialize)]
pub struct CaseForUpdate {
	pub safety_report_id: Option<String>,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub validation_profile: Option<String>,
	pub appendices_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub submitted_by: Option<Uuid>,
	pub submitted_at: Option<OffsetDateTime>,
	pub raw_xml: Option<Vec<u8>>,
	pub dirty_c: Option<bool>,
	pub dirty_d: Option<bool>,
	pub dirty_e: Option<bool>,
	pub dirty_f: Option<bool>,
	pub dirty_g: Option<bool>,
	pub dirty_h: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct CaseFilter {
	pub organization_id: Option<OpValsValue>,
	pub safety_report_id: Option<OpValsString>,
	pub status: Option<OpValsString>,
}

// -- CaseBmc (Business Model Controller)

pub struct CaseBmc;

impl DbBmc for CaseBmc {
	const TABLE: &'static str = "cases";
}

impl CaseBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		case_c: CaseForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, case_c).await
	}

	pub async fn get(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<Case> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<CaseFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<Case>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		case_u: CaseForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, case_u).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
