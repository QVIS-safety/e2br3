// Permission Profile BMC — manages the `permission_profiles` table.
//
// Business logic (privilege normalization, built-in role definitions, response
// shape) stays in the REST layer. This BMC owns only the raw database operations
// and the dynamic-role permission cache refresh.

use crate::model::acs::{
	permissions_for_menu_privileges, remove_dynamic_role, replace_dynamic_roles,
	AdminMenuPrivilege,
};
use crate::model::ModelManager;
use crate::model::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, FromRow)]
pub struct DbPermissionProfileRow {
	pub role_name: String,
	pub display_name: String,
	pub description: Option<String>,
	pub can_view: bool,
	pub can_review: bool,
	pub can_lock: bool,
	pub can_admin: bool,
	pub privileges_json: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub built_in: bool,
	pub editable: bool,
	pub sponsor_admin_capable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileCreateData {
	pub role_name: String,
	pub display_name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileUpdateData {
	pub display_name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

const PROFILE_SELECT: &str = r#"
	SELECT role_name, display_name, description, can_view, can_review, can_lock, can_admin,
	       privileges_json, active, built_in, editable, sponsor_admin_capable
	FROM permission_profiles
"#;

// -- PermissionProfileBmc

pub struct PermissionProfileBmc;

impl PermissionProfileBmc {
	pub async fn list(mm: &ModelManager) -> Result<Vec<DbPermissionProfileRow>> {
		let sql =
			format!("{PROFILE_SELECT} ORDER BY built_in DESC, display_name ASC");
		mm.dbx()
			.fetch_all(sqlx::query_as::<_, DbPermissionProfileRow>(&sql))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn list_active_custom(
		mm: &ModelManager,
	) -> Result<Vec<DbPermissionProfileRow>> {
		let sql = format!(
			"{PROFILE_SELECT} WHERE active = true AND built_in = false ORDER BY display_name ASC"
		);
		mm.dbx()
			.fetch_all(sqlx::query_as::<_, DbPermissionProfileRow>(&sql))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn get(
		mm: &ModelManager,
		role_name: &str,
	) -> Result<DbPermissionProfileRow> {
		let sql = format!("{PROFILE_SELECT} WHERE role_name = $1");
		mm.dbx()
			.fetch_one(
				sqlx::query_as::<_, DbPermissionProfileRow>(&sql).bind(role_name),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn create(
		mm: &ModelManager,
		data: PermissionProfileCreateData,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO permission_profiles
						(role_name, display_name, description, privileges_json, active,
						 built_in, editable, sponsor_admin_capable,
						 can_view, can_review, can_lock, can_admin)
						VALUES ($1, $2, $3, $4, $5, false, true, $6, false, false, false, false)
					"#,
				)
				.bind(&data.role_name)
				.bind(&data.display_name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active)
				.bind(data.sponsor_admin_capable),
			)
			.await
			.map(|_| ())
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn update(
		mm: &ModelManager,
		role_name: &str,
		data: PermissionProfileUpdateData,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					UPDATE permission_profiles
					SET display_name = $2,
					    description = $3,
						    privileges_json = $4,
						    active = $5,
						    sponsor_admin_capable = $6,
						    updated_at = now(),
					    can_view = false,
					    can_review = false,
					    can_lock = false,
					    can_admin = false
					WHERE role_name = $1
					"#,
				)
				.bind(role_name)
				.bind(&data.display_name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active)
				.bind(data.sponsor_admin_capable),
			)
			.await
			.map(|_| ())
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn delete(mm: &ModelManager, role_name: &str) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query("DELETE FROM permission_profiles WHERE role_name = $1")
					.bind(role_name),
			)
			.await
			.map(|_| ())
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	/// Reload the in-memory permission cache from all active permission profiles.
	/// Must be called after any create/update/delete that changes profile permissions.
	pub async fn refresh_dynamic_roles(mm: &ModelManager) -> Result<()> {
		let rows = Self::list_active_custom(mm).await?;
		let mapped = rows
			.into_iter()
			.map(|row| {
				let permissions =
					permissions_for_menu_privileges(&row.privileges_json.0);
				(row.role_name, permissions)
			})
			.collect();
		replace_dynamic_roles(mapped);
		Ok(())
	}

	/// Remove a single role from the in-memory cache without a full reload.
	/// Call before `refresh_dynamic_roles` on delete, or standalone for cache eviction.
	pub fn evict_dynamic_role(role_name: &str) {
		remove_dynamic_role(role_name);
	}
}
