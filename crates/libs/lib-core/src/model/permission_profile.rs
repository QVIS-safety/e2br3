// Permission Profile BMC — manages the `permission_profiles` table.
//
// Business logic (privilege normalization, built-in role definitions, response
// shape) stays in the REST layer. This BMC owns only the raw database operations
// and normalized authorization-role persistence.

use crate::authorization::AdminMenuPrivilege;
use crate::ctx::Ctx;
use crate::model::authorization::NormalizedRoleRepository;
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
	pub id: Uuid,
	pub organization_id: Uuid,
	pub name: String,
	pub description: Option<String>,
	pub privileges_json: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
	pub built_in: bool,
	pub editable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileCreateData {
	pub name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionProfileUpdateData {
	pub name: String,
	pub description: Option<String>,
	pub privileges: SqlxJson<Vec<AdminMenuPrivilege>>,
	pub active: bool,
}

const PROFILE_SELECT: &str = r#"
	SELECT id, organization_id, name, description, privileges_json,
	       active, built_in, editable
	FROM permission_profiles
"#;

// -- PermissionProfileBmc

pub struct PermissionProfileBmc;

const MAX_ACTIVE_CUSTOM_ROLES_PER_ORG: i64 = 20;

impl PermissionProfileBmc {
	async fn enforce_active_custom_role_limit(
		dbx: &crate::model::store::dbx::Dbx,
		organization_id: Uuid,
		excluding_id: Option<Uuid>,
	) -> Result<()> {
		dbx.execute(
			sqlx::query(
				"SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))",
			)
			// Keep this statement's bind type text. The XML importer uses the
			// same cached SQL with a composite text lock key.
			.bind(organization_id.to_string()),
		)
		.await?;
		let (count,) = dbx
			.fetch_one(
				sqlx::query_as::<_, (i64,)>(
					r#"
					SELECT COUNT(*)
					FROM permission_profiles
					WHERE organization_id = $1
					  AND active = true
					  AND built_in = false
					  AND ($2::uuid IS NULL OR id <> $2)
					"#,
				)
				.bind(organization_id)
				.bind(excluding_id),
			)
			.await?;
		if count >= MAX_ACTIVE_CUSTOM_ROLES_PER_ORG {
			return Err(crate::model::Error::Conflict {
				message: "active custom role limit is 20".to_string(),
			});
		}
		Ok(())
	}

	pub async fn policy_version(mm: &ModelManager) -> Result<i64> {
		let ctx = Ctx::root_ctx();
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, &ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let version = match dbx
			.fetch_one(sqlx::query_as::<_, (i64,)>(
				"SELECT version FROM rbac_policy_state WHERE singleton = true",
			))
			.await
		{
			Ok((version,)) => version,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(version)
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<Vec<DbPermissionProfileRow>> {
		let sql = format!(
			"{PROFILE_SELECT} WHERE organization_id = $1 ORDER BY built_in DESC, name ASC"
		);
		Self::fetch_all_with_ctx(
			ctx,
			mm,
			sqlx::query_as::<_, DbPermissionProfileRow>(&sql)
				.bind(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DbPermissionProfileRow> {
		let sql = format!("{PROFILE_SELECT} WHERE id = $1 AND organization_id = $2");
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let row = match dbx
			.fetch_optional(
				sqlx::query_as::<_, DbPermissionProfileRow>(&sql)
					.bind(id)
					.bind(ctx.organization_id()),
			)
			.await
		{
			Ok(Some(row)) => row,
			Ok(None) => {
				dbx.commit_txn().await?;
				return Err(crate::model::Error::EntityUuidNotFound {
					entity: "permission_profile",
					id,
				});
			}
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
		excluding_id: Option<Uuid>,
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
						  AND ($3::uuid IS NULL OR id <> $3)
					)
					"#,
				)
				.bind(ctx.organization_id())
				.bind(name)
				.bind(excluding_id),
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

	pub async fn count_custom_in_org(ctx: &Ctx, mm: &ModelManager) -> Result<i64> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let count = match dbx
			.fetch_one(
				sqlx::query_as::<_, (i64,)>(
					r#"
					SELECT COUNT(*)
					FROM permission_profiles
					WHERE organization_id = $1
					  AND built_in = false
					"#,
				)
				.bind(ctx.organization_id()),
			)
			.await
		{
			Ok((count,)) => count,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(count)
	}

	pub async fn active_custom_exists_in_org(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
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
						WHERE id = $1
						  AND organization_id = $2
						  AND active = true
						  AND built_in = false
					)
					"#,
				)
				.bind(id)
				.bind(ctx.organization_id()),
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
	) -> Result<Uuid> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		if data.active {
			if let Err(err) = Self::enforce_active_custom_role_limit(
				dbx,
				ctx.organization_id(),
				None,
			)
			.await
			{
				dbx.rollback_txn().await?;
				return Err(err);
			}
		}
		match dbx
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(
					r#"
					INSERT INTO permission_profiles
						(organization_id, name, description, privileges_json, active,
						 built_in, editable)
						VALUES ($1, $2, $3, $4, $5, false, true)
						RETURNING id
					"#,
				)
				.bind(ctx.organization_id())
				.bind(&data.name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active),
			)
			.await
		{
			Ok((id,)) => {
				if let Err(err) = NormalizedRoleRepository::upsert_custom_role(
					dbx,
					id,
					ctx.organization_id(),
					&data.name,
					data.active,
					&data.privileges.0,
				)
				.await
				{
					dbx.rollback_txn().await?;
					return Err(err);
				}
				if let Err(err) = dbx.execute(sqlx::query(
					"UPDATE rbac_policy_state SET version = version + 1, updated_at = now() WHERE singleton = true",
				))
				.await {
					dbx.rollback_txn().await?;
					return Err(crate::model::Error::Store(err.to_string()));
				}
				dbx.commit_txn().await?;
				Ok(id)
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
		id: Uuid,
		data: PermissionProfileUpdateData,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		if data.active {
			if let Err(err) = Self::enforce_active_custom_role_limit(
				dbx,
				ctx.organization_id(),
				Some(id),
			)
			.await
			{
				dbx.rollback_txn().await?;
				return Err(err);
			}
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
					    updated_at = now()
					WHERE id = $1
					  AND organization_id = $6
					"#,
				)
				.bind(id)
				.bind(&data.name)
				.bind(&data.description)
				.bind(&data.privileges)
				.bind(data.active)
				.bind(ctx.organization_id()),
			)
			.await
		{
			Ok(0) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: "permission_profile",
					id,
				})
			}
			Ok(_) => {
				if let Err(err) = NormalizedRoleRepository::upsert_custom_role(
					dbx,
					id,
					ctx.organization_id(),
					&data.name,
					data.active,
					&data.privileges.0,
				)
				.await
				{
					dbx.rollback_txn().await?;
					return Err(err);
				}
				if let Err(err) = dbx.execute(sqlx::query(
					"UPDATE rbac_policy_state SET version = version + 1, updated_at = now() WHERE singleton = true",
				))
				.await {
					dbx.rollback_txn().await?;
					return Err(crate::model::Error::Store(err.to_string()));
				}
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::Store(err.to_string()))
			}
		}
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		match dbx
			.execute(
				sqlx::query(
					"DELETE FROM permission_profiles WHERE id = $1 AND organization_id = $2",
				)
				.bind(id)
				.bind(ctx.organization_id()),
			)
			.await
		{
			Ok(0) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: "permission_profile",
					id,
				})
			}
			Ok(_) => {
				if let Err(err) = dbx.execute(sqlx::query(
					"UPDATE rbac_policy_state SET version = version + 1, updated_at = now() WHERE singleton = true",
				))
				.await {
					dbx.rollback_txn().await?;
					return Err(crate::model::Error::Store(err.to_string()));
				}
				dbx.commit_txn().await?;
				Ok(())
			}
			Err(err) => {
				dbx.rollback_txn().await?;
				Err(crate::model::Error::Store(err.to_string()))
			}
		}
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
