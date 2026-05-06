use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::AdminMenuPrivilege;
use lib_core::model::admin_role::{
	AdminRoleBmc, AdminRoleCreateData, AdminRoleUpdateData, DbAdminRoleRow,
};
use lib_core::model::ModelManager;
use lib_rest_core::{require_safety_db_admin_role, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRoleRow {
	pub canonical_role_id: String,
	pub role_name: String,
	pub display_name: String,
	pub description: Option<String>,
	pub privileges: Vec<AdminMenuPrivilege>,
	pub privilege_map: BTreeMap<String, AdminMenuPrivilege>,
	pub can_view: bool,
	pub can_review: bool,
	pub can_lock: bool,
	pub can_admin: bool,
	pub active: bool,
	pub built_in: bool,
	pub editable: bool,
	pub sponsor_admin_capable: bool,
	pub is_builtin: bool,
	pub is_editable: bool,
	pub is_sponsor_admin: bool,
	pub is_operational: bool,
}

#[derive(Debug, Deserialize)]
pub struct AdminRoleCreateBody {
	pub role_name: String,
	pub display_name: Option<String>,
	pub description: Option<String>,
	pub privileges: Option<Vec<AdminMenuPrivilege>>,
	pub can_view: Option<bool>,
	pub can_review: Option<bool>,
	pub can_lock: Option<bool>,
	pub can_admin: Option<bool>,
	pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminRoleUpdateBody {
	pub display_name: Option<String>,
	pub description: Option<String>,
	pub privileges: Option<Vec<AdminMenuPrivilege>>,
	pub can_view: Option<bool>,
	pub can_review: Option<bool>,
	pub can_lock: Option<bool>,
	pub can_admin: Option<bool>,
	pub active: Option<bool>,
}

fn normalize_role_name(value: &str) -> String {
	value.trim().to_ascii_lowercase().replace(' ', "_")
}

fn privilege_map(
	privileges: &[AdminMenuPrivilege],
) -> BTreeMap<String, AdminMenuPrivilege> {
	privileges
		.iter()
		.cloned()
		.map(|privilege| (privilege.menu_key.clone(), privilege))
		.collect()
}

fn role_summary_booleans(
	privileges: &[AdminMenuPrivilege],
) -> (bool, bool, bool, bool) {
	let can_admin = privileges.iter().any(|privilege| {
		matches!(
			privilege.menu_key.as_str(),
			"admin" | "settings" | "roles" | "users" | "user"
		) && (privilege.can_edit
			|| privilege.can_review
			|| privilege.can_lock
			|| privilege.can_read && privilege.menu_key == "admin")
	});
	let can_view = can_admin
		|| privileges.iter().any(|privilege| {
			privilege.can_read
				|| privilege.can_edit
				|| privilege.can_review
				|| privilege.can_lock
		});
	let can_review =
		can_admin || privileges.iter().any(|privilege| privilege.can_review);
	let can_lock =
		can_admin || privileges.iter().any(|privilege| privilege.can_lock);
	(can_view, can_review, can_lock, can_admin)
}

fn build_role_row(
	role_name: String,
	display_name: String,
	description: Option<String>,
	privileges: Vec<AdminMenuPrivilege>,
	active: bool,
	built_in: bool,
	editable: bool,
	sponsor_admin_capable: bool,
) -> AdminRoleRow {
	let is_system = role_name == ROLE_SYSTEM_ADMIN;
	let (can_view, can_review, can_lock, can_admin) =
		role_summary_booleans(&privileges);
	let sponsor_admin_capable = sponsor_admin_capable || can_admin;
	AdminRoleRow {
		canonical_role_id: role_name.clone(),
		role_name,
		display_name,
		description,
		privilege_map: privilege_map(&privileges),
		privileges,
		can_view,
		can_review,
		can_lock,
		can_admin,
		active,
		built_in,
		editable,
		sponsor_admin_capable,
		is_builtin: built_in,
		is_editable: editable,
		is_sponsor_admin: sponsor_admin_capable,
		is_operational: !is_system,
	}
}

fn normalize_admin_privileges(
	privileges: Option<Vec<AdminMenuPrivilege>>,
	can_view: Option<bool>,
	can_review: Option<bool>,
	can_lock: Option<bool>,
	can_admin: Option<bool>,
) -> Result<Vec<AdminMenuPrivilege>> {
	let raw = privileges.unwrap_or_else(|| {
		if can_admin.unwrap_or(false) {
			vec![AdminMenuPrivilege {
				menu_key: "admin".to_string(),
				can_read: true,
				can_edit: true,
				can_review: true,
				can_lock: true,
			}]
		} else {
			vec![AdminMenuPrivilege {
				menu_key: "case".to_string(),
				can_read: can_view.unwrap_or(false),
				can_edit: can_review.unwrap_or(false) || can_lock.unwrap_or(false),
				can_review: can_review.unwrap_or(false),
				can_lock: can_lock.unwrap_or(false),
			}]
		}
	});
	let mut out = BTreeMap::<String, AdminMenuPrivilege>::new();
	for privilege in raw {
		let menu_key = privilege.menu_key.trim().to_ascii_lowercase();
		if menu_key.is_empty() {
			continue;
		}
		if !matches!(
			menu_key.as_str(),
			"case"
				| "info" | "import"
				| "export_submission"
				| "submission"
				| "export" | "user"
				| "users" | "organization"
				| "organizations"
				| "audit" | "data"
				| "terminology"
				| "admin" | "settings"
				| "roles"
		) {
			return Err(Error::BadRequest {
				message: format!("unknown role privilege menu '{menu_key}'"),
			});
		}
		let entry = out.entry(menu_key.clone()).or_insert(AdminMenuPrivilege {
			menu_key,
			can_read: false,
			can_edit: false,
			can_review: false,
			can_lock: false,
		});
		entry.can_read = entry.can_read || privilege.can_read;
		entry.can_edit = entry.can_edit || privilege.can_edit;
		entry.can_review = entry.can_review || privilege.can_review;
		entry.can_lock = entry.can_lock || privilege.can_lock;
	}
	let normalized = out.into_values().collect::<Vec<_>>();
	if normalized.is_empty() {
		return Err(Error::BadRequest {
			message: "role must define at least one privilege".to_string(),
		});
	}
	Ok(normalized)
}

fn built_in_roles() -> Vec<AdminRoleRow> {
	vec![
		build_role_row(
			ROLE_SYSTEM_ADMIN.to_string(),
			"System Administrator".to_string(),
			Some(
				"Platform-level role for provisioning and internal operations."
					.to_string(),
			),
			Vec::new(),
			true,
			true,
			false,
			false,
		),
		build_role_row(
			ROLE_SPONSOR_ADMIN_CRO.to_string(),
			"Sponsor Administrator (CRO)".to_string(),
			Some(
				"Fixed in-database sponsor admin role for CRO operations.".to_string(),
			),
			vec![AdminMenuPrivilege {
				menu_key: "admin".to_string(),
				can_read: true,
				can_edit: true,
				can_review: true,
				can_lock: true,
			}],
			true,
			true,
			false,
			true,
		),
		build_role_row(
			ROLE_SPONSOR_ADMIN_COMPANY.to_string(),
			"Sponsor Administrator (Pharmaceutical Company)".to_string(),
			Some(
				"Fixed in-database sponsor admin role for sponsor-company operations."
					.to_string(),
			),
			vec![AdminMenuPrivilege {
				menu_key: "admin".to_string(),
				can_read: true,
				can_edit: true,
				can_review: true,
				can_lock: true,
			}],
			true,
			true,
			false,
			true,
		),
	]
}

fn legacy_privileges(row: &DbAdminRoleRow) -> Vec<AdminMenuPrivilege> {
	if row.can_admin {
		return vec![AdminMenuPrivilege {
			menu_key: "admin".to_string(),
			can_read: true,
			can_edit: true,
			can_review: true,
			can_lock: true,
		}];
	}
	let mut out = Vec::new();
	if row.can_view || row.can_review || row.can_lock {
		out.push(AdminMenuPrivilege {
			menu_key: "case".to_string(),
			can_read: row.can_view,
			can_edit: row.can_review || row.can_lock,
			can_review: row.can_review,
			can_lock: row.can_lock,
		});
	}
	if row.can_view {
		out.push(AdminMenuPrivilege {
			menu_key: "info".to_string(),
			can_read: true,
			can_edit: false,
			can_review: false,
			can_lock: false,
		});
	}
	out
}

fn row_to_api(row: DbAdminRoleRow) -> AdminRoleRow {
	let privileges = if row.privileges_json.0.is_empty() {
		legacy_privileges(&row)
	} else {
		row.privileges_json.0
	};
	build_role_row(
		row.role_name,
		row.display_name,
		row.description,
		privileges,
		row.active,
		row.built_in,
		row.editable,
		row.sponsor_admin_capable,
	)
}

fn is_built_in_role_name(role_name: &str) -> bool {
	matches!(
		role_name,
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	)
}

pub async fn refresh_dynamic_roles(mm: &ModelManager) -> Result<()> {
	AdminRoleBmc::refresh_dynamic_roles(mm)
		.await
		.map_err(Error::Model)
}

/// GET /api/admin/roles
pub async fn list_admin_roles(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<Vec<AdminRoleRow>>)> {
	require_safety_db_admin_role(&ctx_w.0, &mm).await?;
	let mut rows = built_in_roles();
	let custom_rows = AdminRoleBmc::list(&mm).await.map_err(Error::Model)?;
	rows.extend(custom_rows.into_iter().map(row_to_api));
	Ok((StatusCode::OK, Json(rows)))
}

/// GET /api/admin/roles/{role_name}
pub async fn get_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(role_name): Path<String>,
) -> Result<(StatusCode, Json<AdminRoleRow>)> {
	require_safety_db_admin_role(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_role_name(&role_name);
	if let Some(row) = built_in_roles()
		.into_iter()
		.find(|row| row.role_name == normalized_role)
	{
		return Ok((StatusCode::OK, Json(row)));
	}
	let row = AdminRoleBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// POST /api/admin/roles
pub async fn create_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForCreate<AdminRoleCreateBody>,
	>,
) -> Result<(StatusCode, Json<AdminRoleRow>)> {
	require_safety_db_admin_role(&ctx_w.0, &mm).await?;
	let data = params.data;
	let role_name = normalize_role_name(&data.role_name);
	if is_built_in_role_name(&role_name) {
		return Err(Error::BadRequest {
			message: "cannot use a built-in role name".to_string(),
		});
	}
	let display_name = data
		.display_name
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| role_name.clone());
	if role_name.is_empty() {
		return Err(Error::BadRequest {
			message: "role_name is required".to_string(),
		});
	}
	let description = data.description.map(|value| value.trim().to_string());
	let active = data.active.unwrap_or(true);
	let privileges = normalize_admin_privileges(
		data.privileges,
		data.can_view,
		data.can_review,
		data.can_lock,
		data.can_admin,
	)?;
	let (_, _, _, sponsor_admin_capable) = role_summary_booleans(&privileges);

	AdminRoleBmc::create(
		&mm,
		AdminRoleCreateData {
			role_name: role_name.clone(),
			display_name,
			description,
			privileges: SqlxJson(privileges),
			active,
			sponsor_admin_capable,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = AdminRoleBmc::get(&mm, &role_name)
		.await
		.map_err(Error::Model)?;

	AdminRoleBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::CREATED, Json(row_to_api(row))))
}

/// PUT /api/admin/roles/{role_name}
pub async fn update_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(role_name): Path<String>,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForUpdate<AdminRoleUpdateBody>,
	>,
) -> Result<(StatusCode, Json<AdminRoleRow>)> {
	require_safety_db_admin_role(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_role_name(&role_name);
	if is_built_in_role_name(&normalized_role) {
		return Err(Error::AccessDenied {
			required_role: "editable_custom_role".to_string(),
		});
	}
	let current = AdminRoleBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;

	let data = params.data;
	let next_display_name = data
		.display_name
		.unwrap_or_else(|| current.display_name.clone())
		.trim()
		.to_string();
	let next_description = data.description.or_else(|| current.description.clone());
	let next_privileges = if data.privileges.is_some()
		|| data.can_view.is_some()
		|| data.can_review.is_some()
		|| data.can_lock.is_some()
		|| data.can_admin.is_some()
	{
		normalize_admin_privileges(
			data.privileges,
			data.can_view,
			data.can_review,
			data.can_lock,
			data.can_admin,
		)?
	} else {
		row_to_api(current.clone()).privileges
	};
	let next_active = data.active.unwrap_or(current.active);
	let (_, _, _, sponsor_admin_capable) = role_summary_booleans(&next_privileges);

	AdminRoleBmc::update(
		&mm,
		&normalized_role,
		AdminRoleUpdateData {
			display_name: next_display_name,
			description: next_description,
			privileges: SqlxJson(next_privileges),
			active: next_active,
			sponsor_admin_capable,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = AdminRoleBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;

	AdminRoleBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// DELETE /api/admin/roles/{role_name}
pub async fn delete_admin_role(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(role_name): Path<String>,
) -> Result<StatusCode> {
	require_safety_db_admin_role(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_role_name(&role_name);
	if is_built_in_role_name(&normalized_role) {
		return Err(Error::BadRequest {
			message: "built-in roles cannot be deleted".to_string(),
		});
	}
	AdminRoleBmc::evict_dynamic_role(&normalized_role);
	AdminRoleBmc::delete(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;
	AdminRoleBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok(StatusCode::NO_CONTENT)
}
