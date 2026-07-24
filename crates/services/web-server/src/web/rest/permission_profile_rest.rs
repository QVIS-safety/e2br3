use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	built_in_role_metadata, canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY,
	ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::{
	built_in_menu_privileges, normalize_menu_privileges, AdminMenuPrivilege,
	PrivilegeAdapterError,
};
use lib_core::model::organization::{
	OrganizationBmc, ORG_TYPE_CRO, ORG_TYPE_PHARMACEUTICAL_COMPANY,
};
use lib_core::model::permission_profile::{
	DbPermissionProfileRow, PermissionProfileBmc, PermissionProfileCreateData,
	PermissionProfileUpdateData,
};
use lib_core::model::ModelManager;
use lib_rest_core::{require_role_admin, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

const ROLE_NAME_MAX_LEN: usize = 128;
const ROLE_DESCRIPTION_MAX_LEN: usize = 512;

#[derive(Debug, Default, Deserialize)]
pub struct PermissionProfileScope {
	#[serde(default, alias = "organizationId")]
	pub organization_id: Option<Uuid>,
}

async fn permission_profile_ctx(
	ctx: &Ctx,
	mm: &ModelManager,
	scope: &PermissionProfileScope,
) -> Result<Ctx> {
	require_role_admin(ctx)?;
	if !ctx.is_system_admin() {
		return Ok(ctx.clone());
	}
	let organization_id =
		scope.organization_id.ok_or_else(|| Error::BadRequest {
			message: "organization_id is required".to_string(),
		})?;
	if organization_id.is_nil() {
		return Err(Error::BadRequest {
			message: "system organization cannot own custom roles".to_string(),
		});
	}
	let organization = OrganizationBmc::get(ctx, mm, organization_id)
		.await
		.map_err(Error::Model)?;
	let valid_type = organization
		.org_type
		.as_deref()
		.and_then(OrganizationBmc::normalize_org_type)
		.is_some_and(|org_type| {
			org_type == ORG_TYPE_CRO || org_type == ORG_TYPE_PHARMACEUTICAL_COMPANY
		});
	if !organization.active || !valid_type {
		return Err(Error::BadRequest {
			message: "target organization must be an active CRO or company"
				.to_string(),
		});
	}
	Ctx::new(
		ctx.user_id(),
		organization_id,
		ROLE_SYSTEM_ADMIN.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "invalid target organization context".to_string(),
	})
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionProfileRow {
	pub id: String,
	pub name: String,
	pub description: Option<String>,
	pub privileges: Vec<AdminMenuPrivilege>,
	pub active: bool,
	pub built_in: bool,
	pub editable: bool,
}

#[derive(Debug, Deserialize)]
pub struct PermissionProfileCreateBody {
	pub name: Option<String>,
	pub description: Option<String>,
	pub privileges: Option<Vec<AdminMenuPrivilege>>,
	pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionProfileUpdateBody {
	pub name: Option<String>,
	pub description: Option<String>,
	pub privileges: Option<Vec<AdminMenuPrivilege>>,
	pub active: Option<bool>,
}

fn validate_role_name(name: &str) -> Result<()> {
	if name.is_empty() {
		return Err(Error::BadRequest {
			message: "name is required".to_string(),
		});
	}
	if name.chars().count() > ROLE_NAME_MAX_LEN {
		return Err(Error::BadRequest {
			message: "role name must be 128 characters or fewer".to_string(),
		});
	}
	Ok(())
}

fn normalize_role_description(value: Option<String>) -> Result<Option<String>> {
	let description = value
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty());
	if description
		.as_ref()
		.is_some_and(|value| value.chars().count() > ROLE_DESCRIPTION_MAX_LEN)
	{
		return Err(Error::BadRequest {
			message: "role description must be 512 characters or fewer".to_string(),
		});
	}
	Ok(description)
}

fn build_role_row(
	id: String,
	name: String,
	description: Option<String>,
	privileges: Vec<AdminMenuPrivilege>,
	active: bool,
	built_in: bool,
	editable: bool,
) -> PermissionProfileRow {
	PermissionProfileRow {
		id,
		name,
		description,
		privileges,
		active,
		built_in,
		editable,
	}
}

fn normalize_admin_privileges(
	privileges: Option<Vec<AdminMenuPrivilege>>,
) -> Result<Vec<AdminMenuPrivilege>> {
	let raw = privileges
		.unwrap_or_default()
		.into_iter()
		.filter(|privilege| !privilege.menu_key.trim().is_empty())
		.collect::<Vec<_>>();
	normalize_menu_privileges(&raw).map_err(|error| match error {
		PrivilegeAdapterError::UnknownMenu { menu_key } => Error::BadRequest {
			message: format!("unknown role privilege menu '{menu_key}'"),
		},
	})
}

fn built_in_role_row(role_id: &str) -> PermissionProfileRow {
	let metadata = built_in_role_metadata(role_id)
		.expect("built-in role row requires canonical metadata");
	build_role_row(
		metadata.role_id.to_string(),
		metadata.display_name.to_string(),
		Some(metadata.description.to_string()),
		built_in_menu_privileges(metadata.role_id),
		true,
		true,
		false,
	)
}

async fn visible_built_in_roles(
	ctx: &Ctx,
	_mm: &ModelManager,
) -> Result<Vec<PermissionProfileRow>> {
	let roles = match canonical_role(ctx.role()).as_str() {
		ROLE_SYSTEM_ADMIN => vec![
			built_in_role_row(ROLE_SYSTEM_ADMIN),
			built_in_role_row(ROLE_SPONSOR_ADMIN_CRO),
			built_in_role_row(ROLE_SPONSOR_ADMIN_COMPANY),
		],
		ROLE_SPONSOR_ADMIN_CRO => vec![built_in_role_row(ROLE_SPONSOR_ADMIN_CRO)],
		ROLE_SPONSOR_ADMIN_COMPANY => {
			vec![built_in_role_row(ROLE_SPONSOR_ADMIN_COMPANY)]
		}
		_ => Vec::new(),
	};
	Ok(roles)
}

fn row_to_api(row: DbPermissionProfileRow) -> PermissionProfileRow {
	build_role_row(
		row.id.to_string(),
		row.name,
		row.description,
		row.privileges_json.0,
		row.active,
		row.built_in,
		row.editable,
	)
}

fn is_built_in_role_id(id: &str) -> bool {
	built_in_role_metadata(id).is_some()
}

fn parse_custom_role_id(id: &str) -> Result<Uuid> {
	Uuid::parse_str(id.trim()).map_err(|_| Error::BadRequest {
		message: "custom role id must be a UUID".to_string(),
	})
}

pub async fn refresh_dynamic_roles(mm: &ModelManager) -> Result<()> {
	PermissionProfileBmc::refresh_dynamic_roles(mm)
		.await
		.map_err(Error::Model)
}

/// GET /api/admin/permission-profiles
pub async fn list_permission_profiles(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(scope): Query<PermissionProfileScope>,
) -> Result<(StatusCode, Json<Vec<PermissionProfileRow>>)> {
	let request_ctx = ctx_w.0;
	let ctx = permission_profile_ctx(&request_ctx, &mm, &scope).await?;
	let mut rows = visible_built_in_roles(&ctx, &mm).await?;
	let custom_rows = PermissionProfileBmc::list(&ctx, &mm)
		.await
		.map_err(Error::Model)?;
	rows.extend(custom_rows.into_iter().map(row_to_api));
	Ok((StatusCode::OK, Json(rows)))
}

/// GET /api/admin/permission-profiles/{id}
pub async fn get_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<String>,
	Query(scope): Query<PermissionProfileScope>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	let request_ctx = ctx_w.0;
	let ctx = permission_profile_ctx(&request_ctx, &mm, &scope).await?;
	if let Some(row) = visible_built_in_roles(&ctx, &mm)
		.await?
		.into_iter()
		.find(|row| row.id == id)
	{
		return Ok((StatusCode::OK, Json(row)));
	}
	let id = parse_custom_role_id(&id)?;
	let row = PermissionProfileBmc::get(&ctx, &mm, id)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// POST /api/admin/permission-profiles
pub async fn create_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(scope): Query<PermissionProfileScope>,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForCreate<PermissionProfileCreateBody>,
	>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	let request_ctx = ctx_w.0;
	let ctx = permission_profile_ctx(&request_ctx, &mm, &scope).await?;
	let data = params.data;
	let name = data
		.name
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.unwrap_or("Custom Role")
		.to_string();
	validate_role_name(&name)?;
	if PermissionProfileBmc::name_exists_in_org(&ctx, &mm, &name, None)
		.await
		.map_err(Error::Model)?
	{
		return Err(Error::BadRequest {
			message: "role name already exists in this organization".to_string(),
		});
	}
	let description = normalize_role_description(data.description)?;
	let active = data.active.unwrap_or(true);
	let privileges = normalize_admin_privileges(data.privileges)?;

	let id = PermissionProfileBmc::create(
		&ctx,
		&mm,
		PermissionProfileCreateData {
			name,
			description,
			privileges: SqlxJson(privileges),
			active,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = PermissionProfileBmc::get(&ctx, &mm, id)
		.await
		.map_err(Error::Model)?;

	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::CREATED, Json(row_to_api(row))))
}

/// PUT /api/admin/permission-profiles/{id}
pub async fn update_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<String>,
	Query(scope): Query<PermissionProfileScope>,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForUpdate<PermissionProfileUpdateBody>,
	>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	let request_ctx = ctx_w.0;
	let ctx = permission_profile_ctx(&request_ctx, &mm, &scope).await?;
	if is_built_in_role_id(&id) {
		return Err(Error::AccessDenied {
			required_role: "editable_custom_role".to_string(),
		});
	}
	let id = parse_custom_role_id(&id)?;
	let current = PermissionProfileBmc::get(&ctx, &mm, id)
		.await
		.map_err(Error::Model)?;

	let data = params.data;
	let next_name = data
		.name
		.unwrap_or_else(|| current.name.clone())
		.trim()
		.to_string();
	validate_role_name(&next_name)?;
	if PermissionProfileBmc::name_exists_in_org(&ctx, &mm, &next_name, Some(id))
		.await
		.map_err(Error::Model)?
	{
		return Err(Error::BadRequest {
			message: "role name already exists in this organization".to_string(),
		});
	}
	let next_description = normalize_role_description(data.description)?
		.or_else(|| current.description.clone());
	let next_privileges = if data.privileges.is_some() {
		normalize_admin_privileges(data.privileges)?
	} else {
		row_to_api(current.clone()).privileges
	};
	let next_active = data.active.unwrap_or(current.active);

	PermissionProfileBmc::update(
		&ctx,
		&mm,
		id,
		PermissionProfileUpdateData {
			name: next_name,
			description: next_description,
			privileges: SqlxJson(next_privileges),
			active: next_active,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = PermissionProfileBmc::get(&ctx, &mm, id)
		.await
		.map_err(Error::Model)?;

	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// DELETE /api/admin/permission-profiles/{id}
pub async fn delete_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<String>,
	Query(scope): Query<PermissionProfileScope>,
) -> Result<StatusCode> {
	let request_ctx = ctx_w.0;
	let ctx = permission_profile_ctx(&request_ctx, &mm, &scope).await?;
	if is_built_in_role_id(&id) {
		return Err(Error::BadRequest {
			message: "built-in permission profiles cannot be deleted".to_string(),
		});
	}
	let id = parse_custom_role_id(&id)?;
	let current = PermissionProfileBmc::get(&ctx, &mm, id)
		.await
		.map_err(Error::Model)?;
	PermissionProfileBmc::update(
		&ctx,
		&mm,
		id,
		PermissionProfileUpdateData {
			name: current.name,
			description: current.description,
			privileges: current.privileges_json,
			active: false,
		},
	)
	.await
	.map_err(Error::Model)?;
	PermissionProfileBmc::evict_dynamic_role(id);
	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok(StatusCode::NO_CONTENT)
}
