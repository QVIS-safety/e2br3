// Admin Role BMC — manages the `app_roles` table.
//
// Business logic (privilege normalization, built-in role definitions, response
// shape) stays in the REST layer. This BMC owns only the raw database operations
// and the dynamic-role permission cache refresh.

use crate::model::acs::{
	permissions_for_menu_privileges, permissions_for_privileges,
	remove_dynamic_role, replace_dynamic_roles, AdminMenuPrivilege,
};
use crate::model::ModelManager;
use crate::model::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, FromRow)]
pub struct DbAdminRoleRow {
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
pub struct AdminRoleCreateData {
	pub role_name: String,
	pub display_name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminRoleUpdateData {
	pub display_name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

const ROLE_SELECT: &str = r#"
	SELECT role_name, display_name, description, can_view, can_review, can_lock, can_admin,
	       privileges_json, active, built_in, editable, sponsor_admin_capable
	FROM app_roles
"#;

// -- AdminRoleBmc

pub struct AdminRoleBmc;

impl AdminRoleBmc {
	pub async fn list(mm: &ModelManager) -> Result<Vec<DbAdminRoleRow>> {
		let sql = format!("{ROLE_SELECT} ORDER BY built_in DESC, display_name ASC");
		mm.dbx()
			.fetch_all(sqlx::query_as::<_, DbAdminRoleRow>(&sql))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn list_active_custom(
		mm: &ModelManager,
	) -> Result<Vec<DbAdminRoleRow>> {
		let sql = format!(
			"{ROLE_SELECT} WHERE active = true AND built_in = false ORDER BY display_name ASC"
		);
		mm.dbx()
			.fetch_all(sqlx::query_as::<_, DbAdminRoleRow>(&sql))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn get(mm: &ModelManager, role_name: &str) -> Result<DbAdminRoleRow> {
		let sql = format!("{ROLE_SELECT} WHERE role_name = $1");
		mm.dbx()
			.fetch_one(sqlx::query_as::<_, DbAdminRoleRow>(&sql).bind(role_name))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	pub async fn create(mm: &ModelManager, data: AdminRoleCreateData) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO app_roles
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
		data: AdminRoleUpdateData,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					UPDATE app_roles
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
				sqlx::query("DELETE FROM app_roles WHERE role_name = $1")
					.bind(role_name),
			)
			.await
			.map(|_| ())
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	/// Reload the in-memory permission cache from all active custom roles.
	/// Must be called after any create/update/delete that changes role permissions.
	pub async fn refresh_dynamic_roles(mm: &ModelManager) -> Result<()> {
		let rows = Self::list_active_custom(mm).await?;
		let mapped = rows
			.into_iter()
			.map(|row| {
				let permissions = if row.privileges_json.0.is_empty() {
					permissions_for_privileges(
						row.can_view,
						row.can_review,
						row.can_lock,
						row.can_admin,
					)
				} else {
					permissions_for_menu_privileges(&row.privileges_json.0)
				};
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
