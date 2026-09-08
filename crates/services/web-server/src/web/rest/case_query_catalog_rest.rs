//! REST endpoints for the Export/Submission dynamic query (Phase 2, 2.1/2.2).
//!
//! - `GET  /api/case-query/catalog` returns the queryable pages/items.
//! - `POST /api/cases/query` runs a catalog-validated condition query and
//!   returns the matching case ids (scoped to the caller).
//!
//! Server-only routing detail (`FieldSource`) is never serialized to the client.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::case::{case_scope_where, CaseBmc, CaseListViewRow};
use lib_core::model::case_query::{
	build_where, combine_where, validate_conditions, RawCondition, ReportFilters,
};
use lib_core::model::case_query_catalog::{
	catalog, find_page, join_has_deleted_filter, join_sequence_column, CatalogItem,
	CatalogPage, DataType, JoinKind,
};
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{with_rls_read, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use std::collections::BTreeMap;
use uuid::Uuid;

const MAX_CASE_QUERY_ROWS: usize = 5_000;

/// GET /api/case-query/catalog
pub async fn get_case_query_catalog(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CatalogPage>>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_collection(
		&ctx,
		&snapshot,
		&_mm,
		move |_ctx, _mm, _scope| {
			Box::pin(async move {
				let pages = catalog().to_vec();
				Ok((StatusCode::OK, Json(DataRestResult { data: pages })))
			})
		},
	)
	.await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryRequest {
	#[serde(flatten)]
	pub validation: super::case_validation_rest::ValidationAuthoritiesQuery,
	#[serde(default)]
	pub conditions: Vec<RawCondition>,
	#[serde(default)]
	pub result_pages: Vec<String>,
	#[serde(default)]
	pub report_type_last: bool,
	#[serde(default)]
	pub no_ack_accept_history: bool,
	pub limit: Option<u32>,
	pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryElement {
	pub page: String,
	pub item: String,
	pub label: String,
	pub data_type: DataType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryElementValues {
	pub case_id: Uuid,
	pub values: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryResult {
	pub case_ids: Vec<Uuid>,
	pub items: Vec<CaseListViewRow>,
	pub elements: Vec<CaseQueryElement>,
	pub element_values: Vec<CaseQueryElementValues>,
	pub total: usize,
}

#[derive(sqlx::FromRow)]
struct CaseQueryPageRow {
	case_ids: Vec<Uuid>,
	total: i64,
}

#[derive(sqlx::FromRow)]
struct CaseQueryElementValuesRow {
	case_id: Uuid,
	values: SqlxJson<BTreeMap<String, Vec<String>>>,
}

fn element_key(page: &CatalogPage, item: &CatalogItem) -> String {
	format!("{}.{}", page.id, item.id)
}

fn element_value_expression(item: &CatalogItem) -> String {
	match item.source.join {
		JoinKind::CaseColumn => format!(
			"CASE WHEN c.{column} IS NULL THEN '[]'::jsonb ELSE jsonb_build_array(c.{column}::text) END",
			column = item.source.column,
		),
		JoinKind::OneToOne(table) | JoinKind::OneToMany(table) => {
			let active = if join_has_deleted_filter(table) {
				" AND t.deleted = false"
			} else {
				""
			};
			let order = join_sequence_column(table)
				.map(|column| format!(" ORDER BY t.{column}"))
				.unwrap_or_default();
			format!(
				"COALESCE((SELECT jsonb_agg(t.{column}::text{order}) FROM {table} t WHERE t.case_id = c.id{active} AND t.{column} IS NOT NULL), '[]'::jsonb)",
				column = item.source.column,
				table = table,
				order = order,
				active = active,
			)
		}
	}
}

fn result_elements(pages: &[&'static CatalogPage]) -> Vec<CaseQueryElement> {
	pages
		.iter()
		.flat_map(|page| {
			page.items.iter().map(move |item| CaseQueryElement {
				page: page.id.to_string(),
				item: item.id.to_string(),
				label: item.label.to_string(),
				data_type: item.data_type,
			})
		})
		.collect()
}

fn result_values_sql(pages: &[&'static CatalogPage]) -> String {
	let fields = pages
		.iter()
		.flat_map(|page| {
			page.items.iter().map(move |item| {
				format!(
					"'{}', {}",
					element_key(page, item),
					element_value_expression(item)
				)
			})
		})
		.collect::<Vec<_>>()
		.join(", ");
	format!(
		"SELECT c.id AS case_id, jsonb_build_object({fields}) AS values \
			 FROM cases c WHERE c.id = ANY($1) ORDER BY array_position($1, c.id)"
	)
}

fn resolve_result_pages(page_ids: &[String]) -> Result<Vec<&'static CatalogPage>> {
	if page_ids.is_empty() {
		return Err(Error::BadRequest {
			message: "at least one result page is required".to_string(),
		});
	}

	let mut pages: Vec<&'static CatalogPage> = Vec::new();
	for page_id in page_ids {
		let page = find_page(page_id).ok_or_else(|| Error::BadRequest {
			message: format!("unknown result page {page_id}"),
		})?;
		if !pages.iter().any(|candidate| candidate.id == page.id) {
			pages.push(page);
		}
	}
	Ok(pages)
}

/// POST /api/cases/query
pub async fn search_cases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Json(request): Json<CaseQueryRequest>,
) -> Result<(StatusCode, Json<DataRestResult<CaseQueryResult>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm, scope| {
			Box::pin(async move {
				let explicit_page = request.limit.is_some() || request.offset.is_some();
				let limit = request.limit.unwrap_or(MAX_CASE_QUERY_ROWS as u32);
				if limit == 0 || limit as usize > MAX_CASE_QUERY_ROWS {
					return Err(Error::BadRequest {
						message: format!(
							"limit must be between 1 and {MAX_CASE_QUERY_ROWS}"
						),
					});
				}
				let offset = request.offset.unwrap_or(0);
				let pages = resolve_result_pages(&request.result_pages)?;
				let elements = result_elements(&pages);
				let conditions =
					validate_conditions(&request.conditions).map_err(|err| {
						Error::BadRequest {
							message: err.to_string(),
						}
					})?;
				let (where_sql, binds) = build_where(&conditions);
				let filters = ReportFilters {
					last_fu: request.report_type_last,
					no_ack_accept: request.no_ack_accept_history,
				};
				let where_sql = combine_where(&where_sql, &filters);

				let scope_bind = binds.len() + 1;
				let limit_bind = scope_bind + 3;
				let offset_bind = limit_bind + 1;
				let scope_where = case_scope_where(scope_bind);
				let matching_limit = if explicit_page {
					String::new()
				} else {
					format!(" LIMIT {}", MAX_CASE_QUERY_ROWS + 1)
				};
				let sql = format!(
					"WITH matching AS MATERIALIZED (\
					 SELECT c.id, c.created_at FROM cases c \
					 WHERE {where_sql} AND {scope_where}{matching_limit}\
					) SELECT ARRAY(SELECT id FROM matching \
					 ORDER BY created_at DESC, id DESC LIMIT ${limit_bind} OFFSET ${offset_bind}) AS case_ids, \
					(SELECT COUNT(*) FROM matching) AS total"
				);
				let (sender_ids, product_ids, study_ids) =
					if ctx.is_system_admin() || ctx.is_sponsor_admin() {
						(Vec::new(), Vec::new(), Vec::new())
					} else {
						(
							scope.sender_ids().to_vec(),
							scope.product_ids().to_vec(),
							scope.study_ids().to_vec(),
						)
					};

				let page = with_rls_read(mm, ctx, |dbx| {
					let sql = sql.clone();
					let binds = binds.clone();
					let sender_ids = sender_ids.clone();
					let product_ids = product_ids.clone();
					let study_ids = study_ids.clone();
					Box::pin(async move {
						let mut query =
							sqlx::query_as::<_, CaseQueryPageRow>(&sql);
						for value in binds {
							query = query.bind(value);
						}
						dbx.fetch_one(
							query
								.bind(sender_ids)
								.bind(product_ids)
								.bind(study_ids)
								.bind(i64::from(limit))
								.bind(i64::from(offset)),
						)
							.await
							.map_err(|err| Error::Model(err.into()))
					})
				})
				.await?;
				let total = page.total as usize;
				if !explicit_page && total > MAX_CASE_QUERY_ROWS {
					return Err(Error::BadRequest {
						message: format!(
							"case query matches more than {MAX_CASE_QUERY_ROWS} cases"
						),
					});
				}

				let case_ids = page.case_ids;
				let mut items = with_rls_read(mm, ctx, |dbx| {
					let case_ids = case_ids.clone();
					Box::pin(async move {
						CaseBmc::list_view_rows_by_ids(dbx, &case_ids)
							.await
							.map_err(Error::from)
					})
				})
				.await?;
				let cached_totals = CaseValidationSummaryBmc::cached_totals_by_case(
					ctx, mm, &case_ids, &request.validation.resolve()?,
				)
				.await?;
				for item in &mut items {
					item.warn = cached_totals
						.get(&item.case_id)
						.map(|count| count.to_string())
						.unwrap_or_else(|| "Not checked".to_string());
				}
				let element_values = if case_ids.is_empty() {
					Vec::new()
				} else {
					let sql = result_values_sql(&pages);
					with_rls_read(mm, ctx, |dbx| {
						let case_ids = case_ids.clone();
						let sql = sql.clone();
						Box::pin(async move {
							dbx.fetch_all(
								sqlx::query_as::<_, CaseQueryElementValuesRow>(&sql)
									.bind(&case_ids),
							)
							.await
							.map(|rows| {
								rows.into_iter()
									.map(|row| CaseQueryElementValues {
										case_id: row.case_id,
										values: row.values.0,
									})
									.collect()
							})
							.map_err(|err| Error::Model(err.into()))
						})
					})
					.await?
				};
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: CaseQueryResult {
							case_ids,
							items,
							elements,
							element_values,
							total,
						},
					}),
				))
			})
		},
	)
	.await
}
