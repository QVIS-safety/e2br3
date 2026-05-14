use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use lib_core::model::acs::AdminMenuPrivilege;
use lib_core::model::permission_profile::{
	DbPermissionProfileRow, PermissionProfileBmc, PermissionProfileCreateData,
	PermissionProfileUpdateData,
};
use lib_core::model::ModelManager;
use lib_rest_core::{require_admin, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionProfileRow {
	pub profile_id: String,
	pub name: String,
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
pub struct PermissionProfileCreateBody {
	pub profile_id: String,
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

fn normalize_profile_id(value: &str) -> String {
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
	profile_id: String,
	name: String,
	description: Option<String>,
	privileges: Vec<AdminMenuPrivilege>,
	active: bool,
	built_in: bool,
	editable: bool,
	sponsor_admin_capable: bool,
) -> PermissionProfileRow {
	let is_system = profile_id == ROLE_SYSTEM_ADMIN;
	let (can_view, can_review, can_lock, can_admin) =
		role_summary_booleans(&privileges);
	let sponsor_admin_capable = sponsor_admin_capable || can_admin;
	PermissionProfileRow {
		profile_id,
		name,
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
) -> Result<Vec<AdminMenuPrivilege>> {
	let raw = privileges.unwrap_or_default();
	let mut out = BTreeMap::<String, AdminMenuPrivilege>::new();
	for privilege in raw {
		let menu_key = privilege.menu_key.trim().to_ascii_lowercase();
		if menu_key.is_empty() {
			continue;
		}
		if !matches!(
			menu_key.as_str(),
			"home_notice"
				| "home_workflow"
				| "case" | "info"
				| "import" | "export_submission"
				| "submission"
				| "export" | "user"
				| "users" | "organization"
				| "organizations"
				| "audit" | "data"
				| "terminology"
				| "monitoring"
				| "sync" | "sync_mapping"
				| "admin" | "settings"
				| "roles" | "report_due_mail"
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
	Ok(out.into_values().collect::<Vec<_>>())
}

const ADMIN_ROLE_MENU_KEYS: &[&str] = &[
	"case",
	"info",
	"import",
	"export_submission",
	"users",
	"roles",
	"settings",
	"audit",
	"data",
];

fn full_menu_privileges() -> Vec<AdminMenuPrivilege> {
	ADMIN_ROLE_MENU_KEYS
		.iter()
		.map(|menu_key| AdminMenuPrivilege {
			menu_key: (*menu_key).to_string(),
			can_read: true,
			can_edit: true,
			can_review: *menu_key == "case",
			can_lock: *menu_key == "case",
		})
		.collect()
}

fn built_in_roles() -> Vec<PermissionProfileRow> {
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
				"Fixed in-database sponsor permission profile for CRO operations.".to_string(),
			),
			full_menu_privileges(),
			true,
			true,
			false,
			true,
		),
		build_role_row(
			ROLE_SPONSOR_ADMIN_COMPANY.to_string(),
			"Sponsor Administrator (Pharmaceutical Company)".to_string(),
			Some(
				"Fixed in-database sponsor permission profile for sponsor-company operations."
					.to_string(),
			),
			full_menu_privileges(),
			true,
			true,
			false,
			true,
		),
	]
}

fn row_to_api(row: DbPermissionProfileRow) -> PermissionProfileRow {
	build_role_row(
		row.profile_id,
		row.name,
		row.description,
		row.privileges_json.0,
		row.active,
		row.built_in,
		row.editable,
		row.sponsor_admin_capable,
	)
}

fn is_built_in_profile_id(profile_id: &str) -> bool {
	matches!(
		profile_id,
		ROLE_SYSTEM_ADMIN | ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY
	)
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
) -> Result<(StatusCode, Json<Vec<PermissionProfileRow>>)> {
	require_admin(&ctx_w.0, &mm).await?;
	let mut rows = built_in_roles();
	let custom_rows = PermissionProfileBmc::list(&mm)
		.await
		.map_err(Error::Model)?;
	rows.extend(custom_rows.into_iter().map(row_to_api));
	Ok((StatusCode::OK, Json(rows)))
}

/// GET /api/admin/permission-profiles/{profile_id}
pub async fn get_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(profile_id): Path<String>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	require_admin(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_profile_id(&profile_id);
	if let Some(row) = built_in_roles()
		.into_iter()
		.find(|row| row.profile_id == normalized_role)
	{
		return Ok((StatusCode::OK, Json(row)));
	}
	let row = PermissionProfileBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// POST /api/admin/permission-profiles
pub async fn create_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForCreate<PermissionProfileCreateBody>,
	>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	require_admin(&ctx_w.0, &mm).await?;
	let data = params.data;
	let profile_id = normalize_profile_id(&data.profile_id);
	if is_built_in_profile_id(&profile_id) {
		return Err(Error::BadRequest {
			message: "cannot use a built-in profile id".to_string(),
		});
	}
	let name = data
		.name
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| profile_id.clone());
	if profile_id.is_empty() {
		return Err(Error::BadRequest {
			message: "profile_id is required".to_string(),
		});
	}
	let description = data.description.map(|value| value.trim().to_string());
	let active = data.active.unwrap_or(true);
	let privileges = normalize_admin_privileges(data.privileges)?;
	let (_, _, _, sponsor_admin_capable) = role_summary_booleans(&privileges);

	PermissionProfileBmc::create(
		&mm,
		PermissionProfileCreateData {
			profile_id: profile_id.clone(),
			name,
			description,
			privileges: SqlxJson(privileges),
			active,
			sponsor_admin_capable,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = PermissionProfileBmc::get(&mm, &profile_id)
		.await
		.map_err(Error::Model)?;

	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::CREATED, Json(row_to_api(row))))
}

/// PUT /api/admin/permission-profiles/{profile_id}
pub async fn update_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(profile_id): Path<String>,
	Json(params): Json<
		lib_rest_core::rest_params::ParamsForUpdate<PermissionProfileUpdateBody>,
	>,
) -> Result<(StatusCode, Json<PermissionProfileRow>)> {
	require_admin(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_profile_id(&profile_id);
	if is_built_in_profile_id(&normalized_role) {
		return Err(Error::AccessDenied {
			required_role: "editable_custom_role".to_string(),
		});
	}
	let current = PermissionProfileBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;

	let data = params.data;
	let next_name = data
		.name
		.unwrap_or_else(|| current.name.clone())
		.trim()
		.to_string();
	let next_description = data.description.or_else(|| current.description.clone());
	let next_privileges = if data.privileges.is_some() {
		normalize_admin_privileges(data.privileges)?
	} else {
		row_to_api(current.clone()).privileges
	};
	let next_active = data.active.unwrap_or(current.active);
	let (_, _, _, sponsor_admin_capable) = role_summary_booleans(&next_privileges);

	PermissionProfileBmc::update(
		&mm,
		&normalized_role,
		PermissionProfileUpdateData {
			name: next_name,
			description: next_description,
			privileges: SqlxJson(next_privileges),
			active: next_active,
			sponsor_admin_capable,
		},
	)
	.await
	.map_err(Error::Model)?;

	let row = PermissionProfileBmc::get(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;

	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok((StatusCode::OK, Json(row_to_api(row))))
}

/// DELETE /api/admin/permission-profiles/{profile_id}
pub async fn delete_permission_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(profile_id): Path<String>,
) -> Result<StatusCode> {
	require_admin(&ctx_w.0, &mm).await?;
	let normalized_role = normalize_profile_id(&profile_id);
	if is_built_in_profile_id(&normalized_role) {
		return Err(Error::BadRequest {
			message: "built-in permission profiles cannot be deleted".to_string(),
		});
	}
	PermissionProfileBmc::evict_dynamic_role(&normalized_role);
	PermissionProfileBmc::delete(&mm, &normalized_role)
		.await
		.map_err(Error::Model)?;
	PermissionProfileBmc::refresh_dynamic_roles(&mm)
		.await
		.map_err(Error::Model)?;
	Ok(StatusCode::NO_CONTENT)
}
