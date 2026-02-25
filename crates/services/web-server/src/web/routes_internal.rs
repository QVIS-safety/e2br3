use axum::Router;
use lib_core::model::ModelManager;

use super::rest;

/// Internal machine-to-machine routes (no user auth middleware).
pub fn routes(mm: ModelManager) -> Router {
	Router::new().merge(rest::routes_submissions_internal(mm))
}
