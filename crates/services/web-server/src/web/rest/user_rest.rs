// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{Ctx, ROLE_ADMIN};
use lib_core::model::acs::{
	has_permission, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
};
use lib_core::model::user::{
	User, UserBmc, UserFilter, UserForCreate, UserForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
use serde::Deserialize;
use uuid::Uuid;

fn require_admin_role(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_admin() {
		return Err(WebError::AccessDenied {
			required_role: "admin".to_string(),
		});
	}
	Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UserForCreateAdminPayload {
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: Option<String>,
	pub first_name: Option<String>,
	pub last_name: Option<String>,
}

/// POST /api/users
/// Create a new user
/// **Requires User.Create permission (admin only)**
pub async fn create_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<UserForCreateAdminPayload>>,
) -> Result<(StatusCode, Json<DataRestResult<User>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest create_user", "HANDLER");
	require_admin_role(&ctx)?;

	// Check permission
	if !has_permission(ctx.role(), USER_CREATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "User.Create".to_string(),
		});
	}

	let ParamsForCreate { data } = params;
	let mut organization_id = data.organization_id;
	if organization_id.is_nil() {
		if ctx.organization_id().is_nil() {
			return Err(WebError::AccessDenied {
				required_role: "organization_id is required".to_string(),
			});
		}
		organization_id = ctx.organization_id();
	}
	// New users are provisioned with a temporary password and must reset it on first login.
	let create = UserForCreate {
		organization_id,
		email: data.email,
		username: data.username,
		pwd_clear: "welcome".to_string(),
		role: data.role,
		first_name: data.first_name,
		last_name: data.last_name,
	};
	let id = UserBmc::create(&ctx, &mm, create)
		.await
		.map_err(WebError::Model)?;
	UserBmc::set_must_change_password(&ctx, &mm, id, true)
		.await
		.map_err(WebError::Model)?;
	let entity = UserBmc::get(&ctx, &mm, id).await.map_err(WebError::Model)?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/users/:id
/// Get a user by ID
/// **Requires User.Read permission (all authenticated users)**
pub async fn get_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<User>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest get_user id={}", "HANDLER", id);
	require_admin_role(&ctx)?;

	// Check permission
	if !has_permission(ctx.role(), USER_READ) {
		return Err(WebError::PermissionDenied {
			required_permission: "User.Read".to_string(),
		});
	}

	// Non-admin users can only view users in their organization
	// (RLS will enforce this at the database level)
	let entity = UserBmc::get(&ctx, &mm, id).await.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

#[derive(Debug, Deserialize)]
pub struct SetMyPasswordBody {
	pub new_password: String,
}

/// POST /api/users/me/password
/// Set current user's password and clear first-login password reset requirement.
pub async fn set_my_password(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<SetMyPasswordBody>>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest set_my_password", "HANDLER");

	let ParamsForCreate { data } = params;
	let new_password = data.new_password.trim();
	if new_password.is_empty() {
		return Err(WebError::AccessDenied {
			required_role: "new_password is required".to_string(),
		});
	}

	let privileged_ctx =
		Ctx::new(ctx.user_id(), ctx.organization_id(), ROLE_ADMIN.to_string())
			.map_err(|_| WebError::AccessDenied {
				required_role: "valid user context".to_string(),
			})?;

	UserBmc::update_pwd_and_clear_must_change(
		&privileged_ctx,
		&mm,
		ctx.user_id(),
		new_password,
	)
	.await
	.map_err(WebError::Model)?;

	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users
/// List all users with optional filtering
/// **Requires User.List permission (all authenticated users can list users in their org)**
pub async fn list_users(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<User>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest list_users", "HANDLER");
	require_admin_role(&ctx)?;

	// Check permission
	if !has_permission(ctx.role(), USER_LIST) {
		return Err(WebError::PermissionDenied {
			required_permission: "User.List".to_string(),
		});
	}

	let params = ParamsList::<UserFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| WebError::from(lib_rest_core::Error::BadRequest {
			message,
		}))?;

	// RLS will filter to users in the same organization (unless admin)
	let entities = UserBmc::list(&ctx, &mm, params.filters, params.list_options)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// PUT /api/users/:id
/// Update a user
/// **Requires User.Update permission (admin only)**
pub async fn update_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<UserForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<User>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest update_user id={}", "HANDLER", id);
	require_admin_role(&ctx)?;

	// Check permission
	if !has_permission(ctx.role(), USER_UPDATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "User.Update".to_string(),
		});
	}

	let ParamsForUpdate { data } = params;
	UserBmc::update(&ctx, &mm, id, data)
		.await
		.map_err(WebError::Model)?;
	let entity = UserBmc::get(&ctx, &mm, id).await.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/users/:id
/// Delete a user
/// **Requires User.Delete permission (admin only)**
pub async fn delete_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest delete_user id={}", "HANDLER", id);
	require_admin_role(&ctx)?;

	// Check permission
	if !has_permission(ctx.role(), USER_DELETE) {
		return Err(WebError::PermissionDenied {
			required_permission: "User.Delete".to_string(),
		});
	}

	// Prevent users from deleting themselves
	if id == ctx.user_id() {
		return Err(WebError::AccessDenied {
			required_role: "Cannot delete yourself".to_string(),
		});
	}

	UserBmc::delete(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/me
/// Get current user's profile
/// **Any authenticated user**
pub async fn get_current_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<User>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest get_current_user", "HANDLER");

	let entity = UserBmc::get(&ctx, &mm, ctx.user_id())
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}
