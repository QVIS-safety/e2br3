use axum::routing::{get, post};
use axum::Router;
use lib_core::model::ModelManager;

use crate::web::rest::{
	audit_rest, case_query_catalog_rest, import_rest, terminology_rest,
	validation_rules_rest,
};

/// Routes for /api/terminology
pub fn routes_terminology(mm: ModelManager) -> Router {
	Router::new()
		.route("/terminology/meddra", get(terminology_rest::search_meddra))
		.route(
			"/terminology/whodrug",
			get(terminology_rest::search_whodrug),
		)
		.route(
			"/terminology/mfds-products",
			get(terminology_rest::search_mfds_products),
		)
		.route(
			"/terminology/import/meddra",
			axum::routing::post(terminology_rest::import_meddra),
		)
		.route(
			"/terminology/import/whodrug",
			axum::routing::post(terminology_rest::import_whodrug),
		)
		.route(
			"/terminology/releases",
			get(terminology_rest::list_releases),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/approve",
			axum::routing::post(terminology_rest::approve_release),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/activate",
			axum::routing::post(terminology_rest::activate_release),
		)
		.route(
			"/terminology/releases/{dictionary}/{version}/rollback",
			axum::routing::post(terminology_rest::rollback_release),
		)
		.route(
			"/terminology/countries",
			get(terminology_rest::list_countries),
		)
		.route(
			"/terminology/code-lists",
			get(terminology_rest::get_code_list),
		)
		.route(
			"/terminology/fda-code-search",
			get(terminology_rest::search_fda_hierarchical_code),
		)
		.route(
			"/terminology/ucum-units",
			get(terminology_rest::list_ucum_units),
		)
		.with_state(mm)
}

/// Routes for /api/case-query
/// Routes for /api/case-query
pub fn routes_case_query(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/case-query/catalog",
			get(case_query_catalog_rest::get_case_query_catalog),
		)
		.route("/cases/query", post(case_query_catalog_rest::search_cases))
		.route(
			"/case-query/search",
			post(case_query_catalog_rest::search_cases),
		)
		.with_state(mm)
}

/// Routes for /api/import
/// Routes for /api/import
pub fn routes_import(mm: ModelManager) -> Router {
	Router::new()
		.route("/import/xml/history", get(import_rest::list_import_history))
		.route(
			"/import/xml/history/{id}/error.txt",
			get(import_rest::download_import_history_error),
		)
		.route(
			"/import/xml/validate",
			axum::routing::post(import_rest::validate_xml),
		)
		.route("/import/xml", axum::routing::post(import_rest::import_xml))
		.with_state(mm)
}

/// Routes for /api/audit-logs
/// Routes for /api/audit-logs
pub fn routes_audit(mm: ModelManager) -> Router {
	Router::new()
		.route("/audit-logs", get(audit_rest::list_audit_logs))
		.route(
			"/audit-logs/verify-integrity",
			get(audit_rest::verify_audit_log_integrity),
		)
		.route(
			"/audit-logs/by-record/{table_name}/{record_id}",
			get(audit_rest::list_audit_logs_by_record),
		)
		.with_state(mm)
}

/// Routes for /api/validation
/// Routes for /api/validation
pub fn routes_validation(mm: ModelManager) -> Router {
	Router::new()
		.route(
			"/validation/rules",
			get(validation_rules_rest::list_validation_rules),
		)
		.with_state(mm)
}
