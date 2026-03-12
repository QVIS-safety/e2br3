#![allow(dead_code)]

pub mod config;
pub mod openapi;
pub mod submission;
pub mod web;

use axum::middleware;
use axum::Router;
use lib_core::model::ModelManager;
use lib_web::middleware::mw_auth::mw_ctx_resolver;
use lib_web::middleware::mw_db_ctx::mw_ctx_require_and_set_dbx;
use lib_web::middleware::mw_req_stamp::mw_req_stamp_resolver;
use lib_web::middleware::mw_res_map::mw_response_map;
use lib_web::routes::routes_static;
use tower_cookies::CookieManagerLayer;

pub fn app(mm: ModelManager) -> Router {
	let routes_rest = web::routes_rest::routes(mm.clone()).route_layer(
		middleware::from_fn_with_state(mm.clone(), mw_ctx_require_and_set_dbx),
	);
	let routes_internal = web::routes_internal::routes(mm.clone());
	let routes_login = web::routes_login::routes(mm.clone());

	Router::new()
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
