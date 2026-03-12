// Controlled Terminologies - MedDRA, WHODrug, ISO Countries, E2B Code Lists

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, OpValsBool, OpValsString};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::FromRow;

// -- MeddraTerm

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct MeddraTerm {
	pub id: i64,
	pub code: String,
	pub term: String,
	pub level: String, // LLT, PT, HLT, HLGT, SOC
	pub version: String,
	pub language: String,
	pub active: bool,
	pub created_at: OffsetDateTime,
}

#[derive(Fields, Deserialize)]
pub struct MeddraTermForCreate {
	pub code: String,
	pub term: String,
	pub level: String,
	pub version: String,
	pub language: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct MeddraTermFilter {
	pub code: Option<OpValsString>,
	pub term: Option<OpValsString>,
	pub level: Option<OpValsString>,
	pub version: Option<OpValsString>,
}

// -- WhodrugProduct

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct WhodrugProduct {
	pub id: i64,
	pub code: String,
	pub drug_name: String,
	pub atc_code: Option<String>,
	pub version: String,
	pub language: String,
	pub active: bool,
	pub created_at: OffsetDateTime,
}

#[derive(Fields, Deserialize)]
pub struct WhodrugProductForCreate {
	pub code: String,
	pub drug_name: String,
	pub atc_code: Option<String>,
	pub version: String,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct WhodrugProductFilter {
	pub code: Option<OpValsString>,
	pub drug_name: Option<OpValsString>,
	pub version: Option<OpValsString>,
}

// -- IsoCountry

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct IsoCountry {
	pub code: String, // Primary key - ISO 3166-1 alpha-2
	pub name: String,
	pub active: bool,
}

#[derive(Fields, Deserialize)]
pub struct IsoCountryForCreate {
	pub code: String,
	pub name: String,
}

// -- E2bCodeList

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct E2bCodeList {
	pub id: i32,
	pub list_name: String,
	pub code: String,
	pub display_name: String,
	pub description: Option<String>,
	pub sort_order: Option<i32>,
	pub active: bool,
}

#[derive(Fields, Deserialize)]
pub struct E2bCodeListForCreate {
	pub list_name: String,
	pub code: String,
	pub display_name: String,
	pub description: Option<String>,
	pub sort_order: Option<i32>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct E2bCodeListFilter {
	pub list_name: Option<OpValsString>,
	pub active: Option<OpValsBool>,
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct UcumUnit {
	pub id: i32,
	pub code: String,
	pub display_name: String,
	pub description: Option<String>,
	pub unit_type: Option<String>,
	pub active: bool,
}

// -- BMCs

pub struct MeddraTermBmc;
impl DbBmc for MeddraTermBmc {
	const TABLE: &'static str = "meddra_terms";
}

impl MeddraTermBmc {
	pub async fn search(
		_ctx: &Ctx,
		mm: &ModelManager,
		query: &str,
		version: Option<&str>,
		limit: i64,
	) -> Result<Vec<MeddraTerm>> {
		let sql = if let Some(_ver) = version {
			format!(
				"SELECT * FROM {} WHERE term ILIKE $1 AND version = $2 AND active = true ORDER BY term LIMIT $3",
				Self::TABLE
			)
		} else {
			format!(
				"SELECT * FROM {} WHERE term ILIKE $1 AND active = true ORDER BY term LIMIT $2",
				Self::TABLE
			)
		};

		let search_pattern = format!("%{query}%");

		let terms = if let Some(ver) = version {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, MeddraTerm>(&sql)
						.bind(&search_pattern)
						.bind(ver)
						.bind(limit),
				)
				.await?
		} else {
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, MeddraTerm>(&sql)
						.bind(&search_pattern)
						.bind(limit),
				)
				.await?
		};

		Ok(terms)
	}
}

pub struct WhodrugProductBmc;
impl DbBmc for WhodrugProductBmc {
	const TABLE: &'static str = "whodrug_products";
}

impl WhodrugProductBmc {
	pub async fn search(
		_ctx: &Ctx,
		mm: &ModelManager,
		query: &str,
		limit: i64,
	) -> Result<Vec<WhodrugProduct>> {
		let sql = format!(
			"SELECT * FROM {} WHERE drug_name ILIKE $1 AND active = true ORDER BY drug_name LIMIT $2",
			Self::TABLE
		);

		let search_pattern = format!("%{query}%");
		let products = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, WhodrugProduct>(&sql)
					.bind(&search_pattern)
					.bind(limit),
			)
			.await?;

		Ok(products)
	}
}

pub struct IsoCountryBmc;
impl DbBmc for IsoCountryBmc {
	const TABLE: &'static str = "iso_countries";
}

impl IsoCountryBmc {
	pub async fn list_all(_ctx: &Ctx, mm: &ModelManager) -> Result<Vec<IsoCountry>> {
		let sql = format!(
			"SELECT * FROM {} WHERE active = true ORDER BY name",
			Self::TABLE
		);
		let countries = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, IsoCountry>(&sql))
			.await?;
		Ok(countries)
	}
}

pub struct E2bCodeListBmc;
impl DbBmc for E2bCodeListBmc {
	const TABLE: &'static str = "e2b_code_lists";
}

impl E2bCodeListBmc {
	pub async fn get_by_list_name(
		_ctx: &Ctx,
		mm: &ModelManager,
		list_name: &str,
	) -> Result<Vec<E2bCodeList>> {
		let sql = format!(
			"SELECT * FROM {} WHERE list_name = $1 AND active = true ORDER BY sort_order, code",
			Self::TABLE
		);
		let codes = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, E2bCodeList>(&sql).bind(list_name))
			.await?;
		Ok(codes)
	}
}

pub struct UcumUnitBmc;
impl DbBmc for UcumUnitBmc {
	const TABLE: &'static str = "ucum_units";
}

impl UcumUnitBmc {
	pub async fn list_all(_ctx: &Ctx, mm: &ModelManager) -> Result<Vec<UcumUnit>> {
		let sql = format!(
			"SELECT id, code, display_name, description, unit_type, active FROM {} WHERE active = true ORDER BY unit_type NULLS LAST, code",
			Self::TABLE
		);
		let units = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, UcumUnit>(&sql))
			.await?;
		Ok(units)
	}
}
