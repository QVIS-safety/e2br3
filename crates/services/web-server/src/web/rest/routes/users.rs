use axum::routing::get;
use axum::Router;
use lib_core::model::ModelManager;
use lib_web::handlers::handlers_rest::rest_collection_item_routes;

use crate::web::rest::{
	admin_settings_rest, organization_rest, permission_profile_rest, user_rest,
};

/// Routes for /api/organizations
pub fn routes_organizations(mm: ModelManager) -> Router {
	rest_collection_item_routes(
		"/organizations",
		"/organizations/{id}",
		get(organization_rest::list_organizations)
			.post(organization_rest::create_organization),
		get(organization_rest::get_organization)
			.put(organization_rest::update_organization)
			.delete(organization_rest::delete_organization),
	)
	.with_state(mm)
}

/// Routes for /api/users
/// Routes for /api/users
pub fn routes_users(mm: ModelManager) -> Router {
	Router::new()
		// GET /api/users/me - must be before /users/{id} to avoid matching
		.route("/users/me", get(user_rest::get_current_user))
		.route(
			"/users/me/profile",
			get(user_rest::get_current_user_profile),
		)
		.route(
			"/users/me/routing",
			get(user_rest::get_current_user_routing)
				.put(user_rest::update_current_user_routing),
		)
		.route(
			"/users/me/organization",
			axum::routing::put(user_rest::update_current_user_organization),
		)
		.route(
			"/users/me/password",
			axum::routing::post(user_rest::set_my_password),
		)
		// Standard collection routes
		.route(
			"/users",
			get(user_rest::list_users).post(user_rest::create_user),
		)
		.route(
			"/users/{id}",
			get(user_rest::get_user)
				.put(user_rest::update_user)
				.delete(user_rest::delete_user),
		)
		.route(
			"/settings/runtime",
			get(admin_settings_rest::get_runtime_settings),
		)
		.route(
			"/admin/settings",
			get(admin_settings_rest::get_admin_settings)
				.put(admin_settings_rest::update_admin_settings),
		)
		.route(
			"/admin/notices",
			axum::routing::put(admin_settings_rest::update_admin_notices),
		)
		.route(
			"/admin/permission-profiles",
			get(permission_profile_rest::list_permission_profiles)
				.post(permission_profile_rest::create_permission_profile),
		)
		.route(
			"/admin/permission-profiles/{id}",
			get(permission_profile_rest::get_permission_profile)
				.delete(permission_profile_rest::delete_permission_profile)
				.put(permission_profile_rest::update_permission_profile),
		)
		.with_state(mm)
}
