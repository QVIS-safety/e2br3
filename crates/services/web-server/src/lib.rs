pub mod config;
pub mod runtime_settings;
pub mod submission;
pub mod web;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{middleware, routing::get, Json, Router};
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
		.after_connect(|conn, _meta| {
			Box::pin(async move {
				sqlx::query(
					"SELECT set_config('app.current_user_id', $1, false),
					        set_config('app.current_organization_id', $2, false),
					        set_config('app.platform_isolation_bypass', 'true', false)",
				)
				.bind("00000000-0000-0000-0000-000000000001")
				.bind("00000000-0000-0000-0000-000000000000")
				.execute(&mut *conn)
				.await?;
				Ok(())
			})
		})
		.connect(&database_url)
		.await?;
	let registry = lib_core::authorization::policy_registry()
		.map_err(|error| AuthorizationMigrationError::Registry(error.to_string()))?;
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
}

pub async fn initialize_authorization_storage(
) -> Result<AuthorizationStartupStatus, AuthorizationMigrationError> {
	classify_authorization_startup(reconcile_authorization_storage().await)
}

fn classify_authorization_startup(
	result: Result<MigrationReport, AuthorizationMigrationError>,
) -> Result<AuthorizationStartupStatus, AuthorizationMigrationError> {
	result.map(AuthorizationStartupStatus::Reconciled)
}

pub fn app(mm: ModelManager) -> Result<Router, lib_utils::envs::Error> {
	let routes_rest = web::routes_rest::routes(mm.clone())
		.fallback(route_not_found)
		.route_layer(middleware::from_fn_with_state(
			mm.clone(),
			mw_ctx_require_and_set_dbx,
		))
		.route_layer(middleware::from_fn(mw_csrf_origin));
	let routes_internal =
		web::routes_internal::routes(mm.clone()).fallback(route_not_found);
	let routes_login =
		web::routes_login::routes(mm.clone()).fallback(route_not_found);

	Ok(Router::new()
		.route("/health", get(health))
		.nest("/auth/v1", routes_login)
		.nest("/api", routes_rest)
		.nest("/internal", routes_internal)
		.layer(middleware::map_response(mw_response_map))
		.layer(middleware::from_fn_with_state(mm, mw_ctx_resolver))
		.layer(CookieManagerLayer::new())
		.layer(middleware::from_fn(mw_req_stamp_resolver))
		.fallback_service(routes_static::serve_dir(
			&config::web_config()?.WEB_FOLDER,
		)))
}

async fn mw_csrf_origin(req: Request<Body>, next: middleware::Next) -> Response {
	let state_changing = !matches!(
		req.method(),
		&Method::GET | &Method::HEAD | &Method::OPTIONS | &Method::TRACE
	);
	let cookie = match req
		.headers()
		.get(header::COOKIE)
		.map(|value| value.to_str())
		.transpose()
	{
		Ok(cookie) => cookie,
		Err(_) => {
			return (StatusCode::BAD_REQUEST, "invalid cookie header")
				.into_response()
		}
	};
	let has_auth_cookie = cookie.is_some_and(|value| {
		value
			.split(';')
			.any(|part| part.trim_start().starts_with("auth-token="))
	});
	if state_changing && has_auth_cookie && !csrf_origin_allowed(&req) {
		return (
			StatusCode::FORBIDDEN,
			"cross-origin state-changing request rejected",
		)
			.into_response();
	}
	next.run(req).await
}

fn csrf_origin_allowed(req: &Request<Body>) -> bool {
	let Some(origin) = req.headers().get(header::ORIGIN) else {
		return true;
	};
	let Ok(origin) = origin.to_str() else {
		return false;
	};
	if origin.eq_ignore_ascii_case("null") {
		return false;
	}
	let Ok(expected) = std::env::var("E2BR3_PUBLIC_ORIGIN") else {
		return false;
	};
	let expected = expected.trim().trim_end_matches('/');
	!expected.is_empty()
		&& origin.trim_end_matches('/').eq_ignore_ascii_case(expected)
}

async fn health() -> StatusCode {
	StatusCode::NO_CONTENT
}

async fn route_not_found() -> impl IntoResponse {
	(
		StatusCode::NOT_FOUND,
		Json(serde_json::json!({
			"error": "route_not_found",
			"message": "API route not found"
		})),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::model::authorization::MigrationRejection;

	#[test]
	fn legacy_role_rejections_stop_startup() {
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
			Err(AuthorizationMigrationError::Rejected(rejections))
				if rejections.len() == 1
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

	#[test]
	fn csrf_rejects_null_origin() {
		let request = Request::builder()
			.header(header::ORIGIN, "null")
			.body(Body::empty())
			.expect("request");
		assert!(!csrf_origin_allowed(&request));
	}

	#[test]
	fn csrf_does_not_trust_forwarded_host_as_an_origin_allowlist() {
		let request = Request::builder()
			.header(header::HOST, "127.0.0.1:8216")
			.header("x-forwarded-host", "localhost:4033")
			.header(header::ORIGIN, "http://localhost:4033")
			.body(Body::empty())
			.expect("request");
		assert!(!csrf_origin_allowed(&request));
	}

	#[tokio::test]
	async fn api_route_not_found_is_json() {
		let response = route_not_found().await.into_response();
		assert_eq!(response.status(), StatusCode::NOT_FOUND);
		assert_eq!(
			response.headers().get(header::CONTENT_TYPE).unwrap(),
			"application/json"
		);
	}
}
