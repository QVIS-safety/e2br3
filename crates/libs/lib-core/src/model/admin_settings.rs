// Admin Settings BMC — manages the `app_settings` table.
//
// Business logic (payload normalization, defaults, workflow validation)
// stays in the REST layer. This BMC owns only the raw database operations.

use crate::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_USER, SYSTEM_ORG_ID,
};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

pub struct AdminSettingsBmc;

impl AdminSettingsBmc {
	/// Fetch the JSON value stored for `key`, or `None` if not set.
	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		key: &str,
	) -> Result<Option<Value>> {
		Self::get_for_org(ctx, mm, ctx.organization_id(), key).await
	}

	pub async fn get_system(mm: &ModelManager, key: &str) -> Result<Option<Value>> {
		let ctx = Ctx::root_ctx();
		let org_id = Uuid::parse_str(SYSTEM_ORG_ID)
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;
		Self::get_for_org(&ctx, mm, org_id, key).await
	}

	async fn get_for_org(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
		key: &str,
	) -> Result<Option<Value>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let row = match dbx
			.fetch_optional(
				sqlx::query_as::<_, (Value,)>(
					"SELECT value FROM app_settings WHERE organization_id = $1 AND key = $2",
				)
				.bind(organization_id)
				.bind(key),
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
		Ok(row.map(|(value,)| value))
	}

	/// Upsert the JSON value for `key`.
	pub async fn upsert(
		ctx: &Ctx,
		mm: &ModelManager,
		key: &str,
		value: &Value,
		updated_by: Option<Uuid>,
	) -> Result<()> {
		Self::upsert_for_org(ctx, mm, ctx.organization_id(), key, value, updated_by)
			.await
	}

	pub async fn upsert_system(
		ctx: &Ctx,
		mm: &ModelManager,
		key: &str,
		value: &Value,
		updated_by: Option<Uuid>,
	) -> Result<()> {
		let org_id = Uuid::parse_str(SYSTEM_ORG_ID)
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;
		Self::upsert_for_org(ctx, mm, org_id, key, value, updated_by).await
	}

	async fn upsert_for_org(
		ctx: &Ctx,
		mm: &ModelManager,
		organization_id: Uuid,
		key: &str,
		value: &Value,
		updated_by: Option<Uuid>,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let count = match dbx
			.execute(
				sqlx::query(
					r#"
					INSERT INTO app_settings (organization_id, key, value, updated_by)
					VALUES ($1, $2, $3, $4)
					ON CONFLICT (organization_id, key)
					DO UPDATE SET
						value = EXCLUDED.value,
						updated_at = now(),
						updated_by = EXCLUDED.updated_by
					"#,
				)
				.bind(organization_id)
				.bind(key)
				.bind(value)
				.bind(updated_by),
			)
			.await
		{
			Ok(count) => count,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		let _ = count;
		dbx.commit_txn().await?;
		Ok(())
	}

	/// Return the set of all known workflow role identifiers (built-in + active custom).
	pub async fn known_workflow_roles(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<HashSet<String>> {
		let mut roles = [
			ROLE_SPONSOR_ADMIN_CRO,
			ROLE_SPONSOR_ADMIN_COMPANY,
			ROLE_USER,
			"manager",
			"pvm",
			"head_pv",
			"pvs",
			"viewer",
			"sponsor",
		]
		.into_iter()
		.map(str::to_string)
		.collect::<HashSet<_>>();

		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let rows = match dbx
			.fetch_all(sqlx::query_as::<_, (String,)>(
				"SELECT profile_id FROM permission_profiles WHERE active = true",
			))
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		for (profile_id,) in rows {
			let role = canonical_role(&profile_id);
			if !role.is_empty() {
				roles.insert(role);
			}
		}
		Ok(roles)
	}
}
