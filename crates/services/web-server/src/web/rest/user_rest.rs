// User REST endpoints with RBAC permission checks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use lib_core::model::acs::{
	has_permission, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
};
use lib_core::model::organization::{
	Organization, OrganizationBmc, ORG_TYPE_CRO, ORG_TYPE_PHARMACEUTICAL_COMPANY,
};
use lib_core::model::user::{
	User, UserBmc, UserFilter, UserForCreate, UserForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	admin_db_ctx, require_admin, require_permission, routing_profile_for_user,
	validate_active_sender_selection, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::types::time::OffsetDateTime;
use time::{format_description, PrimitiveDateTime};
use uuid::Uuid;

const USERNAME_MAX_LEN: usize = 128;
const EMAIL_MAX_LEN: usize = 255;

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
	pub can_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserScopeView {
	pub assigned_sender_ids: Vec<String>,
	pub assigned_product_ids: Vec<String>,
	pub assigned_study_ids: Vec<String>,
	pub access_blind_allowed: bool,
	pub active_sender_identifier: Option<String>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub access_end_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub email: String,
	pub username: String,
	pub role: String,
	pub permission_profile_id: Option<String>,
	pub role_meta: UserRoleMetadata,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	pub scope: UserScopeView,
	pub active: bool,
	pub must_change_password: bool,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub last_login_at: Option<OffsetDateTime>,
	#[serde(with = "time::serde::rfc3339")]
	pub created_at: OffsetDateTime,
	#[serde(with = "time::serde::rfc3339")]
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
	#[serde(default)]
	pub organization_id: Option<Uuid>,
	pub email: String,
	pub username: Option<String>,
	pub pwd_clear: Option<String>,
	pub role: Option<String>,
	pub permission_profile_id: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
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
	pub permission_profile_id: Option<String>,
	pub comments: Option<String>,
	pub other_information: Option<String>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_start_at: Option<OffsetDateTime>,
	#[serde(default, deserialize_with = "deserialize_access_datetime_option")]
	pub access_end_at: Option<OffsetDateTime>,
	pub active_sender_identifier: Option<String>,
	pub access_sender_ids: Option<ScopeListInput>,
	pub access_product_ids: Option<ScopeListInput>,
	pub access_study_ids: Option<ScopeListInput>,
	pub access_blind_allowed: Option<bool>,
	pub active: Option<bool>,
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub last_login_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingSelectionBody {
	#[serde(default, alias = "sender_id")]
	pub active_sender_identifier: Option<String>,
}

fn validate_username(username: &str) -> Result<()> {
	if username.chars().count() > USERNAME_MAX_LEN {
		return Err(Error::BadRequest {
			message: "username must be 128 characters or fewer".to_string(),
		});
	}
	Ok(())
}

fn validate_email(email: &str) -> Result<()> {
	if email.chars().count() > EMAIL_MAX_LEN {
		return Err(Error::BadRequest {
			message: "email must be 255 characters or fewer".to_string(),
		});
	}
	Ok(())
}

fn normalize_email_input(email: String) -> Result<String> {
	let email = email.trim().to_string();
	validate_email(&email)?;
	Ok(email)
}

fn normalize_optional_email_input(email: Option<String>) -> Result<Option<String>> {
	email.map(normalize_email_input).transpose()
}

fn normalize_optional_username_input(
	username: Option<String>,
) -> Result<Option<String>> {
	username
		.map(|value| {
			let username = value.trim().to_string();
			validate_username(&username)?;
			Ok(username)
		})
		.transpose()
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

fn role_metadata(
	role: &str,
	permission_profile_id: Option<&str>,
) -> UserRoleMetadata {
	let canonical_role_id = canonical_role(role);
	let permission_subject = if canonical_role_id == ROLE_USER {
		permission_profile_id.unwrap_or(&canonical_role_id)
	} else {
		&canonical_role_id
	};
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
		can_admin: is_builtin || has_permission(permission_subject, USER_CREATE),
	}
}

fn normalize_user_role_and_profile(
	role: Option<String>,
	permission_profile_id: Option<String>,
) -> (Option<String>, Option<String>) {
	let normalized_role = role
		.map(|role| canonical_role(&role))
		.filter(|role| !role.trim().is_empty());
	let normalized_profile = permission_profile_id
		.map(|profile| canonical_role(&profile))
		.filter(|profile| !profile.trim().is_empty());
	match normalized_role.as_deref() {
		Some(
			ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY,
		) => (normalized_role, None),
		Some(ROLE_USER) => (Some(ROLE_USER.to_string()), normalized_profile),
		Some(custom_profile) => (
			Some(ROLE_USER.to_string()),
			Some(custom_profile.to_string()),
		),
		None => (None, normalized_profile),
	}
}

fn sponsor_admin_role_error() -> Error {
	Error::BadRequest {
		message: "sponsor_admin_cro can only be assigned in CRO organizations; sponsor_admin_company can only be assigned in Pharmaceutical company organizations".to_string(),
	}
}

fn validate_create_role_selection(
	role: Option<&str>,
	permission_profile_id: Option<&str>,
) -> Result<()> {
	match (role, permission_profile_id) {
		(Some(ROLE_USER), None) | (None, _) => Err(Error::BadRequest {
			message: "role selection is required".to_string(),
		}),
		_ => Ok(()),
	}
}

async fn validate_sponsor_admin_role_for_org(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
	role: Option<&str>,
) -> Result<()> {
	let Some(role) = role else {
		return Ok(());
	};
	if !matches!(role, ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY) {
		return Ok(());
	}
	let organization: Organization =
		OrganizationBmc::get(ctx, mm, organization_id).await?;
	match (role, organization.org_type.as_deref()) {
		(ROLE_SPONSOR_ADMIN_CRO, Some(ORG_TYPE_CRO))
		| (ROLE_SPONSOR_ADMIN_COMPANY, Some(ORG_TYPE_PHARMACEUTICAL_COMPANY)) => Ok(()),
		_ => Err(sponsor_admin_role_error()),
	}
}

fn initial_password(pwd_clear: Option<String>) -> String {
	pwd_clear
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| "welcome".to_string())
}

fn deserialize_access_datetime_option<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<OffsetDateTime>, D::Error>
where
	D: Deserializer<'de>,
{
	let value = Option::<String>::deserialize(deserializer)?;
	value
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(parse_access_datetime)
		.transpose()
		.map_err(de::Error::custom)
}

fn parse_access_datetime(
	value: &str,
) -> std::result::Result<OffsetDateTime, String> {
	if let Ok(datetime) =
		OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
	{
		return Ok(datetime);
	}

	for format in [
		"[year]-[month]-[day]T[hour]:[minute]",
		"[year]-[month]-[day]T[hour]:[minute]:[second]",
	] {
		let description = format_description::parse(format)
			.map_err(|err| format!("invalid datetime parser format: {err}"))?;
		if let Ok(datetime) = PrimitiveDateTime::parse(value, &description) {
			return Ok(datetime.assume_utc());
		}
	}

	Err("expected RFC3339 or datetime-local format".to_string())
}

fn user_is_effectively_active(user: &User) -> bool {
	if !user.active {
		return false;
	}
	let now = OffsetDateTime::now_utc();
	if user.access_start_at.is_some_and(|start_at| start_at > now) {
		return false;
	}
	if user.access_end_at.is_some_and(|end_at| end_at < now) {
		return false;
	}
	true
}

fn has_sender_scope_assignment(
	active_sender_identifier: &Option<String>,
	access_sender_ids: &Option<ScopeListInput>,
) -> bool {
	active_sender_identifier.is_some() || access_sender_ids.is_some()
}

fn sender_scope_assignment_forbidden_for_ctx(ctx: &Ctx) -> bool {
	!ctx.is_cro_sponsor_admin()
}

fn sender_scope_assignment_forbidden() -> Error {
	Error::AccessDenied {
		required_role: "sender_scope_assignment_cro_admin".to_string(),
	}
}

fn user_view(user: User) -> UserView {
	let active = user_is_effectively_active(&user);
	let access_sender_ids = user.access_sender_ids.clone();
	let access_product_ids = user.access_product_ids.clone();
	let access_study_ids = user.access_study_ids.clone();
	let access_blind_allowed = user.access_blind_allowed;
	let active_sender_identifier = user.active_sender_identifier.clone();
	UserView {
		id: user.id,
		organization_id: user.organization_id,
		email: user.email,
		username: user.username,
		role: user.role.clone(),
		permission_profile_id: user.permission_profile_id.clone(),
		role_meta: role_metadata(&user.role, user.permission_profile_id.as_deref()),
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
		active,
		must_change_password: user.must_change_password,
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
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, USER_CREATE)?;
	}
	if sender_scope_assignment_forbidden_for_ctx(&ctx)
		&& has_sender_scope_assignment(
			&data.active_sender_identifier,
			&data.access_sender_ids,
		) {
		return Err(sender_scope_assignment_forbidden());
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let organization_id = if ctx.is_system_admin() {
		data.organization_id.ok_or_else(|| Error::BadRequest {
			message: "organization_id is required".to_string(),
		})?
	} else {
		ctx.organization_id()
	};
	if organization_id.is_nil() {
		return Err(Error::BadRequest {
			message: "organization context is required".to_string(),
		});
	}
	// New users are provisioned with a temporary password and must reset it on first login.
	let (role, permission_profile_id) =
		normalize_user_role_and_profile(data.role, data.permission_profile_id);
	validate_create_role_selection(
		role.as_deref(),
		permission_profile_id.as_deref(),
	)?;
	validate_sponsor_admin_role_for_org(
		&db_ctx,
		&mm,
		organization_id,
		role.as_deref(),
	)
	.await?;
	let email = normalize_email_input(data.email)?;
	let username = normalize_optional_username_input(data.username)?
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());
	validate_username(&username)?;
	let create = UserForCreate {
		organization_id,
		email,
		username: Some(username),
		pwd_clear: initial_password(data.pwd_clear),
		role,
		permission_profile_id,
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
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, USER_READ)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
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
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, USER_LIST)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
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
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, USER_UPDATE)?;
	}
	if sender_scope_assignment_forbidden_for_ctx(&ctx)
		&& has_sender_scope_assignment(
			&data.active_sender_identifier,
			&data.access_sender_ids,
		) {
		return Err(sender_scope_assignment_forbidden());
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let (role, permission_profile_id) =
		normalize_user_role_and_profile(data.role, data.permission_profile_id);
	if role.is_some() {
		let existing: User = UserBmc::get(&db_ctx, &mm, id).await?;
		validate_sponsor_admin_role_for_org(
			&db_ctx,
			&mm,
			existing.organization_id,
			role.as_deref(),
		)
		.await?;
	}
	let email = normalize_optional_email_input(data.email)?;
	let username = normalize_optional_username_input(data.username)?;
	let update = UserForUpdate {
		email,
		username,
		role,
		permission_profile_id,
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
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, USER_DELETE)?;
	}
	if id == ctx.user_id() {
		return Err(Error::BadRequest {
			message: "cannot delete yourself".to_string(),
		});
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
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
			permission_profile_id: None,
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
