#![allow(dead_code)]

pub mod config;
pub mod openapi;
pub mod submission;
pub mod web;

use axum::Router;
use axum::{http::StatusCode, middleware, routing::get};
use lib_core::model::authorization::{
	AuthorizationMigrationError, AuthorizationMigrationService, MigrationReport,
};
use lib_core::model::ModelManager;
use lib_web::middleware::mw_auth::mw_ctx_resolver;
use lib_web::middleware::mw_db_ctx::mw_ctx_require_and_set_dbx;
use lib_web::middleware::mw_req_stamp::mw_req_stamp_resolver;
use lib_web::middleware::mw_res_map::mw_response_map;
use lib_web::routes::routes_static;
use tower_cookies::CookieManagerLayer;

pub async fn reconcile_authorization_storage(
) -> Result<MigrationReport, AuthorizationMigrationError> {
	let database_url =
		std::env::var("SERVICE_MIGRATION_DB_URL").map_err(|error| {
			AuthorizationMigrationError::Configuration(error.to_string())
		})?;
	let pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(1)
		.connect(&database_url)
		.await?;
	let result = AuthorizationMigrationService::reconcile_database(
		&pool,
		lib_core::authorization::policy_registry(),
	)
	.await;
	pool.close().await;
	result
}

pub fn app(mm: ModelManager) -> Router {
	let routes_rest = web::routes_rest::routes(mm.clone()).route_layer(
		middleware::from_fn_with_state(mm.clone(), mw_ctx_require_and_set_dbx),
	);
	let routes_internal = web::routes_internal::routes(mm.clone());
	let routes_login = web::routes_login::routes(mm.clone());

	Router::new()
		.route("/health", get(health))
		.merge(openapi::router())
		.nest("/auth/v1", routes_login)
		.nest("/api", routes_rest)
		.nest("/internal", routes_internal)
		.layer(middleware::map_response(mw_response_map))
		.layer(middleware::from_fn_with_state(mm, mw_ctx_resolver))
		.layer(CookieManagerLayer::new())
		.layer(middleware::from_fn(mw_req_stamp_resolver))
		.fallback_service(routes_static::serve_dir(&config::web_config().WEB_FOLDER))
}

async fn health() -> StatusCode {
	StatusCode::NO_CONTENT
}
