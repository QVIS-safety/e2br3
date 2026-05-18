// Permission Profile BMC — manages the `permission_profiles` table.
//
// Business logic (privilege normalization, built-in role definitions, response
// shape) stays in the REST layer. This BMC owns only the raw database operations
// and the dynamic-role permission cache refresh.

use crate::ctx::Ctx;
use crate::model::acs::{
	permissions_for_menu_privileges, remove_dynamic_role, replace_dynamic_roles,
	AdminMenuPrivilege,
};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Types

#[derive(Debug, Clone, FromRow)]
pub struct DbPermissionProfileRow {
	pub organization_id: Uuid,
	pub profile_id: String,
	pub name: String,
	pub description: Option<String>,
	pub privileges_json: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub built_in: bool,
	pub editable: bool,
	pub sponsor_admin_capable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileCreateData {
	pub profile_id: String,
	pub name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileUpdateData {
	pub name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub sponsor_admin_capable: bool,
}

const PROFILE_SELECT: &str = r#"
	SELECT organization_id, profile_id, name, description, privileges_json,
	       active, built_in, editable, sponsor_admin_capable
	FROM permission_profiles
"#;

// -- PermissionProfileBmc

pub struct PermissionProfileBmc;

impl PermissionProfileBmc {
	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<Vec<DbPermissionProfileRow>> {
		let sql = format!("{PROFILE_SELECT} ORDER BY built_in DESC, name ASC");
		Self::fetch_all_with_ctx(
			ctx,
			mm,
			sqlx::query_as::<_, DbPermissionProfileRow>(&sql),
		)
		.await
	}

	pub async fn list_active_custom(
		mm: &ModelManager,
	) -> Result<Vec<DbPermissionProfileRow>> {
		let ctx = Ctx::root_ctx();
		let sql = format!(
			"{PROFILE_SELECT} WHERE active = true AND built_in = false ORDER BY name ASC"
		);
		Self::fetch_all_with_ctx(
			&ctx,
			mm,
			sqlx::query_as::<_, DbPermissionProfileRow>(&sql),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		profile_id: &str,
	) -> Result<DbPermissionProfileRow> {
		let sql = format!("{PROFILE_SELECT} WHERE profile_id = $1");
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let row = match dbx
			.fetch_one(
				sqlx::query_as::<_, DbPermissionProfileRow>(&sql).bind(profile_id),
			)
			.await
		{
			Ok(row) => row,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(row)
	}

	pub async fn name_exists_in_org(
		ctx: &Ctx,
		mm: &ModelManager,
		name: &str,
		excluding_profile_id: Option<&str>,
	) -> Result<bool> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let exists = match dbx
			.fetch_one(
				sqlx::query_as::<_, (bool,)>(
					r#"
					SELECT EXISTS (
						SELECT 1
						FROM permission_profiles
						WHERE organization_id = $1
						  AND lower(btrim(name)) = lower(btrim($2))
						  AND ($3::text IS NULL OR profile_id <> $3)
					)
					"#,
				)
				.bind(ctx.organization_id())
				.bind(name)
				.bind(excluding_profile_id),
			)
			.await
		{
			Ok((exists,)) => exists,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(exists)
	}

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PermissionProfileCreateData,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		match dbx
			.execute(
				sqlx::query(
					r#"
					INSERT INTO permission_profiles
						(organization_id, profile_id, name, description, privileges_json, active,
						 built_in, editable, sponsor_admin_capable)
						VALUES ($1, $2, $3, $4, $5, $6, false, true, $7)
					"#,
				)
				.bind(ctx.organization_id())
				.bind(&data.profile_id)
				.bind(&data.name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active)
				.bind(data.sponsor_admin_capable),
			)
			.await
		{
			Ok(_) => {
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::Store(err.to_string()))
			}
		}
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		profile_id: &str,
		data: PermissionProfileUpdateData,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		match dbx
			.execute(
				sqlx::query(
					r#"
					UPDATE permission_profiles
					SET name = $2,
					    description = $3,
						    privileges_json = $4,
						    active = $5,
						    sponsor_admin_capable = $6,
						    updated_at = now()
					WHERE profile_id = $1
					"#,
				)
				.bind(profile_id)
				.bind(&data.name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active)
				.bind(data.sponsor_admin_capable),
			)
			.await
		{
			Ok(_) => {
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::Store(err.to_string()))
			}
		}
	}

	pub async fn delete(
		ctx: &Ctx,
		mm: &ModelManager,
		profile_id: &str,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		match dbx
			.execute(
				sqlx::query("DELETE FROM permission_profiles WHERE profile_id = $1")
					.bind(profile_id),
			)
			.await
		{
			Ok(_) => {
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::Store(err.to_string()))
			}
		}
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
				(row.profile_id, permissions)
			})
			.collect();
		replace_dynamic_roles(mapped);
		Ok(())
	}

	/// Remove a single role from the in-memory cache without a full reload.
	/// Call before `refresh_dynamic_roles` on delete, or standalone for cache eviction.
	pub fn evict_dynamic_role(profile_id: &str) {
		remove_dynamic_role(profile_id);
	}

	async fn fetch_all_with_ctx<'q>(
		ctx: &Ctx,
		mm: &ModelManager,
		query: sqlx::query::QueryAs<
			'q,
			sqlx::Postgres,
			DbPermissionProfileRow,
			sqlx::postgres::PgArguments,
		>,
	) -> Result<Vec<DbPermissionProfileRow>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let rows = match dbx.fetch_all(query).await {
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(rows)
	}
}
