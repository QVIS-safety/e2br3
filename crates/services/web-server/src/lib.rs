#![allow(dead_code)]

pub mod config;
pub mod openapi;
pub mod submission;
pub mod web;

use axum::Router;
use axum::{http::StatusCode, middleware, routing::get};
use lib_core::model::authorization::{
	AuthorizationMigrationError, AuthorizationMigrationService, MigrationReport,
	RevisionRepository,
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
	let registry = lib_core::authorization::policy_registry();
	let result = async {
		RevisionRepository::verify_fact_triggers(&pool, registry).await?;
		let report =
			AuthorizationMigrationService::reconcile_database(&pool, registry)
				.await?;
		Ok(report)
	}
	.await;
	pool.close().await;
	result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationStartupStatus {
	Reconciled(MigrationReport),
	LegacyRuntime { rejections: usize },
}

pub async fn initialize_authorization_storage(
) -> Result<AuthorizationStartupStatus, AuthorizationMigrationError> {
	classify_authorization_startup(reconcile_authorization_storage().await)
}

fn classify_authorization_startup(
	result: Result<MigrationReport, AuthorizationMigrationError>,
) -> Result<AuthorizationStartupStatus, AuthorizationMigrationError> {
	match result {
		Ok(report) => Ok(AuthorizationStartupStatus::Reconciled(report)),
		Err(AuthorizationMigrationError::Rejected(rejections)) => {
			Ok(AuthorizationStartupStatus::LegacyRuntime {
				rejections: rejections.len(),
			})
		}
		Err(error) => Err(error),
	}
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

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::model::authorization::MigrationRejection;

	#[test]
	fn legacy_role_rejections_do_not_stop_the_legacy_runtime() {
		let result = classify_authorization_startup(Err(
			AuthorizationMigrationError::Rejected(vec![MigrationRejection {
				user_id: None,
				organization_id: None,
				legacy_role: Some("legacy-role".to_string()),
				reason: "not safely normalizable".to_string(),
			}]),
		));
		assert!(matches!(
			result,
			Ok(AuthorizationStartupStatus::LegacyRuntime { rejections: 1 })
		));
	}

	#[test]
	fn catalog_mismatch_still_stops_startup() {
		let result = classify_authorization_startup(Err(
			AuthorizationMigrationError::CatalogHashMismatch {
				stored: "old".to_string(),
				deployed: "new".to_string(),
			},
		));
		assert!(matches!(
			result,
			Err(AuthorizationMigrationError::CatalogHashMismatch { .. })
		));
	}
}
