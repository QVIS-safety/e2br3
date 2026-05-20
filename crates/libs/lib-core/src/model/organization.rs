use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{
	FilterNodes, ListOptions, OpValsBool, OpValsString, OpValsValue,
};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

pub const ORG_TYPE_CRO: &str = "cro";
pub const ORG_TYPE_PHARMACEUTICAL_COMPANY: &str = "pharmaceutical_company";

// -- Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Organization {
	pub id: Uuid,
	pub name: String,
	#[serde(rename = "type")]
	pub org_type: Option<String>,
	pub address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub contact_email: Option<String>,
	pub contact_phone: Option<String>,
	pub active: bool,

	// Audit fields (standardized UUID-based)
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct OrganizationForCreate {
	pub name: String,
	#[serde(rename = "type", alias = "org_type")]
	pub org_type: Option<String>,
	pub address: Option<String>,
	pub contact_email: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct OrganizationForUpdate {
	pub name: Option<String>,
	#[serde(rename = "type", alias = "org_type")]
	pub org_type: Option<String>,
	pub address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub contact_email: Option<String>,
	pub contact_phone: Option<String>,
	pub active: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct OrganizationFilter {
	pub id: Option<OpValsValue>,
	pub name: Option<OpValsString>,
	pub active: Option<OpValsBool>,
}

// -- OrganizationBmc

pub struct OrganizationBmc;

impl DbBmc for OrganizationBmc {
	const TABLE: &'static str = "organizations";
}

impl OrganizationBmc {
	pub fn normalize_org_type(org_type: &str) -> Option<&'static str> {
		let normalized = org_type
			.trim()
			.to_ascii_lowercase()
			.replace(['-', ' '], "_");
		match normalized.as_str() {
			"cro" => Some(ORG_TYPE_CRO),
			"pharmaceutical_company" | "pharmaceutical" | "company" => {
				Some(ORG_TYPE_PHARMACEUTICAL_COMPANY)
			}
			_ => None,
		}
	}

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		org_c: OrganizationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, org_c).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<Organization> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<OrganizationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<Organization>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		org_u: OrganizationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, org_u).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
