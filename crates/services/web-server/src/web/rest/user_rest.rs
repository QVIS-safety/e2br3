// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::{
	USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
};
use lib_core::model::user::{
	User, UserBmc, UserFilter, UserForCreate, UserForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	require_permission, require_safety_db_admin_role, routing_profile_for_user,
	safety_db_admin_db_ctx, sponsor_admin_provisioning_db_ctx,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ScopeListInput {
	List(Vec<String>),
	Encoded(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRoleMetadata {
	pub canonical_role_id: String,
	pub display_name: String,
	pub is_builtin: bool,
	pub is_editable: bool,
	pub is_sponsor_admin: bool,
	pub is_operational: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserScopeView {
	pub assigned_sender_ids: Vec<String>,
	pub assigned_product_ids: Vec<String>,
	pub assigned_study_ids: Vec<String>,
	pub access_blind_allowed: bool,
	pub active_sender_identifier: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
	pub id: Uuid,
	pub organization_id: Uuid,
	#[serde(rename = "organization_id")]
	pub organization_id_legacy: Uuid,
	pub email: String,
	pub username: String,
	pub role: String,
	pub role_meta: UserRoleMetadata,
	pub first_name: Option<String>,
	pub last_name: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub scope: UserScopeView,
	#[serde(rename = "access_sender_ids")]
	pub access_sender_ids_legacy: Option<String>,
	#[serde(rename = "access_product_ids")]
	pub access_product_ids_legacy: Option<String>,
	#[serde(rename = "access_study_ids")]
	pub access_study_ids_legacy: Option<String>,
	#[serde(rename = "access_blind_allowed")]
	pub access_blind_allowed_legacy: Option<bool>,
	#[serde(rename = "active_sender_identifier")]
	pub active_sender_identifier_legacy: Option<String>,
	pub active: bool,
	pub must_change_password: bool,
	#[serde(rename = "must_change_password")]
	pub must_change_password_legacy: bool,
	pub last_login_at: Option<OffsetDateTime>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Option<Uuid>,
	pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserProfileView {
	pub user: UserView,
	pub routing: lib_rest_core::RoutingProfile,
}

#[derive(Debug, Deserialize)]
pub struct UserForCreateAdminPayload {
	pub organization_id: Uuid,
	pub email: String,
	pub username: Option<String>,
	pub role: Option<String>,
	pub first_name: Option<String>,
	pub last_name: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub active_sender_identifier: Option<String>,
	pub access_sender_ids: Option<ScopeListInput>,
	pub access_product_ids: Option<ScopeListInput>,
	pub access_study_ids: Option<ScopeListInput>,
	pub access_blind_allowed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserForUpdateAdminPayload {
	pub email: Option<String>,
	pub username: Option<String>,
	pub role: Option<String>,
	pub first_name: Option<String>,
	pub last_name: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub active_sender_identifier: Option<String>,
	pub access_sender_ids: Option<ScopeListInput>,
	pub access_product_ids: Option<ScopeListInput>,
	pub access_study_ids: Option<ScopeListInput>,
	pub access_blind_allowed: Option<bool>,
	pub active: Option<bool>,
	pub last_login_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingSelectionBody {
	pub active_sender_identifier: Option<String>,
}

fn parse_scope_input(value: Option<ScopeListInput>) -> Option<Vec<String>> {
	match value {
		None => None,
		Some(ScopeListInput::List(values)) => Some(values),
		Some(ScopeListInput::Encoded(raw)) => {
			serde_json::from_str::<Vec<String>>(&raw).ok().or_else(|| {
				Some(
					raw.split(',')
						.map(|value| value.trim().to_string())
						.filter(|value| !value.is_empty())
						.collect::<Vec<_>>(),
				)
			})
		}
	}
}

fn serialize_scope_input(value: Option<ScopeListInput>) -> Option<String> {
	parse_scope_input(value).and_then(|values| {
		let values = values
			.into_iter()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.collect::<Vec<_>>();
		if values.is_empty() {
			None
		} else {
			Some(serde_json::json!(values).to_string())
		}
	})
}

fn role_display_name(role: &str) -> String {
	match canonical_role(role).as_str() {
		ROLE_SYSTEM_ADMIN => "System Administrator".to_string(),
		ROLE_SPONSOR_ADMIN_CRO => "Sponsor Administrator (CRO)".to_string(),
		ROLE_SPONSOR_ADMIN_COMPANY => {
			"Sponsor Administrator (Pharmaceutical Company)".to_string()
		}
		other => other.replace('_', " "),
	}
}

fn role_metadata(role: &str) -> UserRoleMetadata {
	let canonical_role_id = canonical_role(role);
	let is_builtin = matches!(
		canonical_role_id.as_str(),
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	);
	let is_sponsor_admin = matches!(
		canonical_role_id.as_str(),
		ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	);
	UserRoleMetadata {
		display_name: role_display_name(&canonical_role_id),
		canonical_role_id: canonical_role_id.clone(),
		is_builtin,
		is_editable: !is_builtin,
		is_sponsor_admin,
		is_operational: canonical_role_id != ROLE_SYSTEM_ADMIN,
	}
}

fn is_sponsor_admin_role(role: &str) -> bool {
	matches!(
		canonical_role(role).as_str(),
		ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	)
}

fn system_admin_forbidden() -> Error {
	Error::AccessDenied {
		required_role: "sponsor_admin_provisioning".to_string(),
	}
}

fn create_has_scope_assignment(data: &UserForCreateAdminPayload) -> bool {
	data.active_sender_identifier.is_some()
		|| data.access_sender_ids.is_some()
		|| data.access_product_ids.is_some()
		|| data.access_study_ids.is_some()
		|| data.access_blind_allowed.is_some()
}

fn update_has_scope_assignment(data: &UserForUpdateAdminPayload) -> bool {
	data.active_sender_identifier.is_some()
		|| data.access_sender_ids.is_some()
		|| data.access_product_ids.is_some()
		|| data.access_study_ids.is_some()
		|| data.access_blind_allowed.is_some()
}

fn user_view(user: User) -> UserView {
	let access_sender_ids = user.access_sender_ids.clone();
	let access_product_ids = user.access_product_ids.clone();
	let access_study_ids = user.access_study_ids.clone();
	let access_blind_allowed = user.access_blind_allowed;
	let active_sender_identifier = user.active_sender_identifier.clone();
	UserView {
		id: user.id,
		organization_id: user.organization_id,
		organization_id_legacy: user.organization_id,
		email: user.email,
		username: user.username,
		role: user.role.clone(),
		role_meta: role_metadata(&user.role),
		first_name: user.first_name,
		last_name: user.last_name,
		comments: user.comments,
		other_information: user.other_information,
		scope: UserScopeView {
			assigned_sender_ids: lib_rest_core::scope_values_from_raw(
				access_sender_ids.as_deref(),
			),
			assigned_product_ids: lib_rest_core::scope_values_from_raw(
				access_product_ids.as_deref(),
			),
			assigned_study_ids: lib_rest_core::scope_values_from_raw(
				access_study_ids.as_deref(),
			),
			access_blind_allowed: access_blind_allowed == Some(true),
			active_sender_identifier: active_sender_identifier.clone(),
			access_start_at: user.access_start_at,
			access_end_at: user.access_end_at,
		},
		access_sender_ids_legacy: access_sender_ids,
		access_product_ids_legacy: access_product_ids,
		access_study_ids_legacy: access_study_ids,
		access_blind_allowed_legacy: access_blind_allowed,
		active_sender_identifier_legacy: active_sender_identifier,
		active: user.active,
		must_change_password: user.must_change_password,
		must_change_password_legacy: user.must_change_password,
		last_login_at: user.last_login_at,
		created_at: user.created_at,
		updated_at: user.updated_at,
		created_by: user.created_by,
		updated_by: user.updated_by,
	}
}

/// POST /api/users
/// Create a new user
/// **Requires User.Create permission (admin only)**
pub async fn create_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<UserForCreateAdminPayload>>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let ParamsForCreate { data } = params;
	if ctx.is_system_admin() {
		if data.organization_id.is_nil()
			|| !data
				.role
				.as_deref()
				.map(is_sponsor_admin_role)
				.unwrap_or(false)
			|| create_has_scope_assignment(&data)
		{
			return Err(system_admin_forbidden());
		}
		let db_ctx = sponsor_admin_provisioning_db_ctx(&ctx)?;
		let create = UserForCreate {
			organization_id: data.organization_id,
			email: data.email,
			username: data.username,
			pwd_clear: "welcome".to_string(),
			role: data.role,
			first_name: data.first_name,
			last_name: data.last_name,
			comments: data.comments,
			other_information: data.other_information,
			access_start_at: data.access_start_at,
			access_end_at: data.access_end_at,
			active_sender_identifier: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
		};
		let id = UserBmc::create(&db_ctx, &mm, create).await?;
		UserBmc::set_must_change_password(&db_ctx, &mm, id, true).await?;
		let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
		return Ok((
			StatusCode::CREATED,
			Json(DataRestResult {
				data: user_view(entity),
			}),
		));
	}
	require_safety_db_admin_role(&ctx, &mm).await?;
	require_permission(&ctx, USER_CREATE)?;
	let db_ctx = safety_db_admin_db_ctx(&ctx, &mm).await?;
	let mut organization_id = data.organization_id;
	if organization_id.is_nil() {
		if ctx.organization_id().is_nil() {
			return Err(Error::BadRequest {
				message: "organization_id is required".to_string(),
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
		comments: data.comments,
		other_information: data.other_information,
		access_start_at: data.access_start_at,
		access_end_at: data.access_end_at,
		active_sender_identifier: data.active_sender_identifier,
		access_sender_ids: parse_scope_input(data.access_sender_ids),
		access_product_ids: parse_scope_input(data.access_product_ids),
		access_study_ids: parse_scope_input(data.access_study_ids),
		access_blind_allowed: data.access_blind_allowed,
	};
	let id = UserBmc::create(&db_ctx, &mm, create).await?;
	UserBmc::set_must_change_password(&db_ctx, &mm, id, true).await?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::CREATED,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

/// GET /api/users/:id
/// Get a user by ID
/// **Requires User.Read permission (all authenticated users)**
pub async fn get_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	if ctx.is_system_admin() {
		let db_ctx = sponsor_admin_provisioning_db_ctx(&ctx)?;
		let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
		if !is_sponsor_admin_role(&entity.role) {
			return Err(system_admin_forbidden());
		}
		return Ok((
			StatusCode::OK,
			Json(DataRestResult {
				data: user_view(entity),
			}),
		));
	}
	require_safety_db_admin_role(&ctx, &mm).await?;
	require_permission(&ctx, USER_READ)?;
	let db_ctx = safety_db_admin_db_ctx(&ctx, &mm).await?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
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
	let ParamsForCreate { data } = params;
	let new_password = data.new_password.trim();
	if new_password.is_empty() {
		return Err(Error::BadRequest {
			message: "new_password is required".to_string(),
		});
	}
	let privileged_ctx = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "valid user context required".to_string(),
	})?;
	UserBmc::update_pwd_and_clear_must_change(
		&privileged_ctx,
		&mm,
		ctx.user_id(),
		new_password,
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users
/// List all users with optional filtering
/// **Requires User.List permission (all authenticated users can list users in their org)**
pub async fn list_users(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<UserView>>>)> {
	let ctx = ctx_w.0;
	let params = ParamsList::<UserFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	if ctx.is_system_admin() {
		let db_ctx = sponsor_admin_provisioning_db_ctx(&ctx)?;
		let entities =
			UserBmc::list(&db_ctx, &mm, params.filters, params.list_options).await?;
		let entities = entities
			.into_iter()
			.filter(|user| is_sponsor_admin_role(&user.role))
			.map(user_view)
			.collect::<Vec<_>>();
		return Ok((StatusCode::OK, Json(DataRestResult { data: entities })));
	}
	require_safety_db_admin_role(&ctx, &mm).await?;
	require_permission(&ctx, USER_LIST)?;
	let db_ctx = safety_db_admin_db_ctx(&ctx, &mm).await?;
	let entities =
		UserBmc::list(&db_ctx, &mm, params.filters, params.list_options).await?;
	let entities = entities.into_iter().map(user_view).collect::<Vec<_>>();
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// PUT /api/users/:id
/// Update a user
/// **Requires User.Update permission (admin only)**
pub async fn update_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<UserForUpdateAdminPayload>>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let ParamsForUpdate { data } = params;
	if ctx.is_system_admin() {
		let db_ctx = sponsor_admin_provisioning_db_ctx(&ctx)?;
		let current: User = UserBmc::get(&db_ctx, &mm, id).await?;
		if !is_sponsor_admin_role(&current.role)
			|| data
				.role
				.as_deref()
				.is_some_and(|role| !is_sponsor_admin_role(role))
			|| update_has_scope_assignment(&data)
		{
			return Err(system_admin_forbidden());
		}
		let update = UserForUpdate {
			email: data.email,
			username: data.username,
			role: data.role,
			first_name: data.first_name,
			last_name: data.last_name,
			comments: data.comments,
			other_information: data.other_information,
			access_start_at: data.access_start_at,
			access_end_at: data.access_end_at,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
			active: data.active,
			last_login_at: data.last_login_at,
		};
		UserBmc::update(&db_ctx, &mm, id, update).await?;
		let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
		return Ok((
			StatusCode::OK,
			Json(DataRestResult {
				data: user_view(entity),
			}),
		));
	}
	require_safety_db_admin_role(&ctx, &mm).await?;
	require_permission(&ctx, USER_UPDATE)?;
	let db_ctx = safety_db_admin_db_ctx(&ctx, &mm).await?;
	let update = UserForUpdate {
		email: data.email,
		username: data.username,
		role: data.role,
		first_name: data.first_name,
		last_name: data.last_name,
		comments: data.comments,
		other_information: data.other_information,
		access_start_at: data.access_start_at,
		access_end_at: data.access_end_at,
		access_sender_ids: serialize_scope_input(data.access_sender_ids),
		access_product_ids: serialize_scope_input(data.access_product_ids),
		access_study_ids: serialize_scope_input(data.access_study_ids),
		access_blind_allowed: data.access_blind_allowed,
		active_sender_identifier: data.active_sender_identifier,
		active: data.active,
		last_login_at: data.last_login_at,
	};
	UserBmc::update(&db_ctx, &mm, id, update).await?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
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
	if ctx.is_system_admin() {
		return Err(system_admin_forbidden());
	}
	require_safety_db_admin_role(&ctx, &mm).await?;
	require_permission(&ctx, USER_DELETE)?;
	if id == ctx.user_id() {
		return Err(Error::BadRequest {
			message: "cannot delete yourself".to_string(),
		});
	}
	let db_ctx = safety_db_admin_db_ctx(&ctx, &mm).await?;
	UserBmc::delete(&db_ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/me
/// Get current user's profile
/// **Any authenticated user**
pub async fn get_current_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let entity: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

pub async fn get_current_user_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<CurrentUserProfileView>>)> {
	let ctx = ctx_w.0;
	let entity: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CurrentUserProfileView {
				user: user_view(entity),
				routing,
			},
		}),
	))
}

pub async fn get_current_user_routing(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_rest_core::RoutingProfile>>,
)> {
	let ctx = ctx_w.0;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: routing })))
}

pub async fn update_current_user_routing(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForUpdate<RoutingSelectionBody>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_rest_core::RoutingProfile>>,
)> {
	let ctx = ctx_w.0;
	let next_sender = validate_active_sender_selection(
		&ctx,
		&mm,
		params.data.active_sender_identifier.as_deref(),
	)
	.await?;
	let routing_update_ctx = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "valid routing update context required".to_string(),
	})?;
	UserBmc::update(
		&routing_update_ctx,
		&mm,
		ctx.user_id(),
		UserForUpdate {
			email: None,
			username: None,
			role: None,
			first_name: None,
			last_name: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: next_sender,
			active: None,
			last_login_at: None,
		},
	)
	.await?;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: routing })))
}
