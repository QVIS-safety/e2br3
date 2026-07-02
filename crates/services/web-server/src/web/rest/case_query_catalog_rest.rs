//! REST endpoints for the Export/Submission dynamic query (Phase 2, 2.1/2.2).
//!
//! - `GET  /api/case-query/catalog` returns the queryable pages/items.
//! - `POST /api/case-query/search` runs a catalog-validated condition query and
//!   returns the matching case ids (scoped to the caller).
//!
//! Server-only routing detail (`FieldSource`) is never serialized to the client.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::CASE_LIST;
use lib_core::model::case_query::{
	build_where, validate_conditions, RawCondition,
};
use lib_core::model::case_query_catalog::{catalog, CatalogPage};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	case_matches_user_scope, require_permission, with_rls_read, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GET /api/case-query/catalog
pub async fn get_case_query_catalog(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CatalogPage>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_LIST)?;
	let pages = catalog().to_vec();
	Ok((StatusCode::OK, Json(DataRestResult { data: pages })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryRequest {
	#[serde(default)]
	pub conditions: Vec<RawCondition>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseQueryResult {
	pub case_ids: Vec<Uuid>,
	pub total: usize,
}

#[derive(sqlx::FromRow)]
struct CaseIdRow {
	id: Uuid,
}

/// POST /api/case-query/search
pub async fn search_cases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(request): Json<CaseQueryRequest>,
) -> Result<(StatusCode, Json<DataRestResult<CaseQueryResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_LIST)?;

	let conditions = validate_conditions(&request.conditions)
		.map_err(|err| Error::BadRequest { message: err.to_string() })?;
	let (where_sql, binds) = build_where(&conditions);

	let sql = format!(
		"SELECT c.id FROM cases c WHERE {where_sql} \
		 ORDER BY c.created_at DESC, c.id DESC"
	);

	let rows = with_rls_read(&mm, &ctx, |dbx| {
		let sql = sql.clone();
		let binds = binds.clone();
		Box::pin(async move {
			let mut query = sqlx::query_as::<_, CaseIdRow>(&sql);
			for value in binds {
				query = query.bind(value);
			}
			dbx.fetch_all(query)
				.await
				.map_err(|err| Error::Model(err.into()))
		})
	})
	.await?;

	// Enforce per-user case scope on top of RLS.
	let mut case_ids = Vec::new();
	for row in rows {
		if case_matches_user_scope(&ctx, &mm, row.id).await? {
			case_ids.push(row.id);
		}
	}

	let total = case_ids.len();
	Ok((
		StatusCode::OK,
		Json(DataRestResult { data: CaseQueryResult { case_ids, total } }),
	))
}
