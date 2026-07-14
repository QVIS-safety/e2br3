// Controlled Terminologies - MedDRA, WHODrug, ISO Countries, E2B Code Lists

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, OpValsBool, OpValsString};
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::{FromRow, Postgres, QueryBuilder};
use std::collections::HashSet;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, FromRow)]
pub struct MeddraTermKey {
	pub version: String,
	pub code: String,
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

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct MfdsProduct {
	pub id: i64,
	pub item_seq: String,
	pub product_name_kr: String,
	pub product_name_en: Option<String>,
	pub manufacturer_name_kr: Option<String>,
	pub manufacturer_name_en: Option<String>,
	pub permit_date: Option<sqlx::types::time::Date>,
	pub cancellation_date: Option<sqlx::types::time::Date>,
	pub cancellation_status: Option<String>,
	pub version: String,
	pub active: bool,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct FdaHierarchicalCodeList {
	pub id: i64,
	pub list_name: String,
	pub ncit_concept_code: String,
	pub level1_term: String,
	pub level2_term: Option<String>,
	pub level3_term: Option<String>,
	pub fda_code: String,
	pub imdrf_code: String,
}

// -- BMCs

pub struct MeddraTermBmc;
impl DbBmc for MeddraTermBmc {
	const TABLE: &'static str = "meddra_terms";
}

impl MeddraTermBmc {
	pub async fn active_versions(mm: &ModelManager) -> Result<Vec<String>> {
		let sql = format!(
			"SELECT DISTINCT version FROM {} WHERE active = true ORDER BY version",
			Self::TABLE
		);
		let rows = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, (String,)>(&sql))
			.await?;
		Ok(rows.into_iter().map(|(version,)| version).collect())
	}

	pub async fn existing_active_keys(
		mm: &ModelManager,
		keys: &[MeddraTermKey],
	) -> Result<Vec<MeddraTermKey>> {
		if keys.is_empty() {
			return Ok(Vec::new());
		}

		let mut qb: QueryBuilder<Postgres> =
			QueryBuilder::new("WITH requested(version, code) AS (");
		qb.push_values(keys, |mut row, key| {
			row.push_bind(&key.version).push_bind(&key.code);
		});
		qb.push(
			") SELECT DISTINCT terms.version, terms.code \
			 FROM meddra_terms terms \
			 JOIN requested ON requested.version = terms.version \
			 AND requested.code = terms.code \
			 WHERE terms.active = true",
		);

		Ok(mm
			.dbx()
			.fetch_all(qb.build_query_as::<MeddraTermKey>())
			.await?)
	}

	pub async fn search(
		_ctx: &Ctx,
		mm: &ModelManager,
		query: &str,
		version: Option<&str>,
		language: Option<&str>,
		limit: i64,
	) -> Result<Vec<MeddraTerm>> {
		let search_pattern = format!("%{query}%");
		let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(format!(
			"SELECT * FROM {} WHERE term ILIKE ",
			Self::TABLE
		));
		qb.push_bind(search_pattern);
		qb.push(" AND active = true");
		if let Some(ver) = version {
			qb.push(" AND version = ").push_bind(ver);
		}
		if let Some(lang) = language {
			qb.push(" AND language = ").push_bind(lang);
		}
		qb.push(" ORDER BY term LIMIT ").push_bind(limit);

		let terms = mm
			.dbx()
			.fetch_all(qb.build_query_as::<MeddraTerm>())
			.await?;

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

pub struct ControlledTermBmc;

impl ControlledTermBmc {
	pub async fn existing_active_codes(
		mm: &ModelManager,
		dictionary: &str,
		scope: &str,
		codes: &[String],
	) -> Result<HashSet<String>> {
		if codes.is_empty() {
			return Ok(HashSet::new());
		}

		let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
			"SELECT DISTINCT code FROM controlled_terminology_terms \
			 WHERE active = true AND dictionary = ",
		);
		qb.push_bind(dictionary)
			.push(" AND scope = ")
			.push_bind(scope)
			.push(" AND code IN (");
		let mut separated = qb.separated(", ");
		for code in codes {
			separated.push_bind(code);
		}
		separated.push_unseparated(")");

		let rows = mm.dbx().fetch_all(qb.build_query_as::<(String,)>()).await?;
		Ok(rows.into_iter().map(|(code,)| code).collect())
	}
}

pub struct MfdsProductBmc;
impl DbBmc for MfdsProductBmc {
	const TABLE: &'static str = "mfds_products";
}

impl MfdsProductBmc {
	pub async fn existing_active_item_seqs(
		mm: &ModelManager,
		codes: &[String],
	) -> Result<HashSet<String>> {
		if codes.is_empty() {
			return Ok(HashSet::new());
		}

		let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
			"SELECT DISTINCT item_seq FROM mfds_products \
			 WHERE active = true AND item_seq IN (",
		);
		let mut separated = qb.separated(", ");
		for code in codes {
			separated.push_bind(code);
		}
		separated.push_unseparated(")");

		let rows = mm.dbx().fetch_all(qb.build_query_as::<(String,)>()).await?;
		Ok(rows.into_iter().map(|(code,)| code).collect())
	}
}

pub struct FdaHierarchicalCodeListBmc;
impl DbBmc for FdaHierarchicalCodeListBmc {
	const TABLE: &'static str = "fda_hierarchical_code_lists";
}

impl FdaHierarchicalCodeListBmc {
	pub async fn search(
		_ctx: &Ctx,
		mm: &ModelManager,
		list_name: &str,
		query: &str,
		limit: i64,
	) -> Result<Vec<FdaHierarchicalCodeList>> {
		let search_pattern = format!("%{query}%");
		let sql = format!(
			"SELECT * FROM {} WHERE list_name = $1 AND (level1_term ILIKE $2 OR level2_term ILIKE $2 OR level3_term ILIKE $2) ORDER BY level1_term, level2_term, level3_term LIMIT $3",
			Self::TABLE
		);
		let rows = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, FdaHierarchicalCodeList>(&sql)
					.bind(list_name)
					.bind(&search_pattern)
					.bind(limit),
			)
			.await?;
		Ok(rows)
	}
}
