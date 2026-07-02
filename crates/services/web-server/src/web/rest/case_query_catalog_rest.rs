//! REST endpoint exposing the case-query field catalog (Phase 2, 2.1).
//!
//! `GET /api/case-query/catalog` returns the pages and queryable items the
//! Export/Submission query builder offers. Server-only routing detail
//! (`FieldSource`) is not serialized, so the client sees only ids, labels,
//! data types, and allowed operators.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::CASE_LIST;
use lib_core::model::case_query_catalog::{catalog, CatalogPage};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;

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
