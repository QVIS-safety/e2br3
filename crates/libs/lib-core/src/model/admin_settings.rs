// Admin Settings BMC — manages the `app_settings` table.
//
// Business logic (payload normalization, defaults, workflow validation)
// stays in the REST layer. This BMC owns only the raw database operations.

use crate::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_USER, SYSTEM_ORG_ID,
};
use crate::model::store::dbx::Dbx;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

pub struct AdminSettingsBmc;

fn changed_settings_fields(
	old_value: Option<&Value>,
	new_value: &Value,
) -> Option<Value> {
	let old_object = old_value.and_then(Value::as_object);
	let new_object = new_value.as_object();
	let mut keys = BTreeSet::new();

	if let Some(object) = old_object {
		keys.extend(object.keys().cloned());
	}
	if let Some(object) = new_object {
		keys.extend(object.keys().cloned());
	}

	let mut changed = Map::new();
	for key in keys {
		let old_field = old_object
			.and_then(|object| object.get(&key))
			.cloned()
			.unwrap_or(Value::Null);
		let new_field = new_object
			.and_then(|object| object.get(&key))
			.cloned()
			.unwrap_or(Value::Null);
		if old_field != new_field {
			changed.insert(
				key,
				serde_json::json!({
					"old": old_field,
					"new": new_field,
				}),
			);
		}
	}

	if changed.is_empty() {
		None
	} else {
		Some(Value::Object(changed))
	}
}

fn notice_text(notice: &Value, key: &str) -> Option<String> {
	notice
		.get(key)
		.and_then(Value::as_str)
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(str::to_string)
}

fn notice_json(
	notice_key: &str,
	title: String,
	body: Option<String>,
	effective_date: Option<String>,
	expire_date: Option<String>,
	writer: Option<String>,
) -> Value {
	serde_json::json!({
		"id": notice_key,
		"title": title,
		"body": body,
		"effective_date": effective_date,
		"expire_date": expire_date,
		"writer": writer,
	})
}

async fn insert_notice_audit(
	dbx: &Dbx,
	record_id: Uuid,
	organization_id: Uuid,
	action: &str,
	user_id: Uuid,
	changed_fields: Option<Value>,
	old_values: Option<Value>,
	new_values: Option<Value>,
) -> Result<()> {
	dbx.execute(
		sqlx::query(
			r#"
			INSERT INTO audit_logs (
				table_name,
				record_id,
				organization_id,
				action,
				user_id,
				changed_fields,
				old_values,
				new_values
			)
			VALUES ('dashboard_notices', $1, $2, $3, $4, $5, $6, $7)
			"#,
		)
		.bind(record_id)
		.bind(organization_id)
		.bind(action)
		.bind(user_id)
		.bind(changed_fields)
		.bind(old_values)
		.bind(new_values),
	)
	.await
	.map(|_| ())
	.map_err(|err| crate::model::Error::Store(err.to_string()))
}

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
		let old_value = match dbx
			.fetch_optional(
				sqlx::query_as::<_, (Value,)>(
					"SELECT value FROM app_settings WHERE organization_id = $1 AND key = $2",
				)
				.bind(organization_id)
				.bind(key),
			)
			.await
		{
			Ok(row) => row.map(|(value,)| value),
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		let changed_fields = changed_settings_fields(old_value.as_ref(), value);
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
		if let Some(changed_fields) = changed_fields {
			let action = if old_value.is_some() {
				"UPDATE"
			} else {
				"CREATE"
			};
			if let Err(err) = dbx
				.execute(
					sqlx::query(
						r#"
						INSERT INTO audit_logs (
							table_name,
							record_id,
							organization_id,
							action,
							user_id,
							changed_fields,
							old_values,
							new_values
						)
						VALUES ('app_settings', $1, $1, $2, $3, $4, $5, $6)
						"#,
					)
					.bind(organization_id)
					.bind(action)
					.bind(updated_by.unwrap_or_else(|| ctx.user_id()))
					.bind(changed_fields)
					.bind(old_value)
					.bind(value),
				)
				.await
			{
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		}
		dbx.commit_txn().await?;
		Ok(())
	}

	pub async fn list_dashboard_notices(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<Vec<Value>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, (Value,)>(
					r#"
					SELECT jsonb_build_object(
						'id', notice_key,
						'title', title,
						'body', body,
						'effective_date', effective_date,
						'expire_date', expire_date,
						'writer', writer
					)
					FROM dashboard_notices
					WHERE organization_id = $1
					ORDER BY sort_order ASC, created_at ASC, notice_key ASC
					"#,
				)
				.bind(ctx.organization_id()),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		dbx.commit_txn().await?;
		Ok(rows.into_iter().map(|(value,)| value).collect())
	}

	pub async fn replace_dashboard_notices(
		ctx: &Ctx,
		mm: &ModelManager,
		notices: &[Value],
		updated_by: Uuid,
	) -> Result<()> {
		let organization_id = ctx.organization_id();
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}

		let old_rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, (Uuid, String, Value)>(
					r#"
					SELECT id,
					       notice_key,
					       jsonb_build_object(
					           'id', notice_key,
					           'title', title,
					           'body', body,
					           'effective_date', effective_date,
					           'expire_date', expire_date,
					           'writer', writer
					       )
					FROM dashboard_notices
					WHERE organization_id = $1
					"#,
				)
				.bind(organization_id),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
		};
		let mut old_by_key = old_rows
			.into_iter()
			.map(|(id, key, value)| (key, (id, value)))
			.collect::<HashMap<_, _>>();
		let mut retained_keys = HashSet::new();

		for (index, notice) in notices.iter().enumerate() {
			let notice_key = notice_text(notice, "id")
				.unwrap_or_else(|| format!("notice-{}", index + 1));
			let new_value = notice_json(
				&notice_key,
				notice_text(notice, "title").unwrap_or_default(),
				notice_text(notice, "body"),
				notice_text(notice, "effective_date"),
				notice_text(notice, "expire_date"),
				notice_text(notice, "writer"),
			);
			let old = old_by_key.get(&notice_key).cloned();
			let (record_id,) = match dbx
				.fetch_one(
					sqlx::query_as::<_, (Uuid,)>(
						r#"
						INSERT INTO dashboard_notices (
							organization_id,
							notice_key,
							title,
							body,
							effective_date,
							expire_date,
							writer,
							sort_order,
							updated_by
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
						ON CONFLICT (organization_id, notice_key)
						DO UPDATE SET
							title = EXCLUDED.title,
							body = EXCLUDED.body,
							effective_date = EXCLUDED.effective_date,
							expire_date = EXCLUDED.expire_date,
							writer = EXCLUDED.writer,
							sort_order = EXCLUDED.sort_order,
							updated_at = now(),
							updated_by = EXCLUDED.updated_by
						RETURNING id
						"#,
					)
					.bind(organization_id)
					.bind(&notice_key)
					.bind(notice_text(notice, "title").unwrap_or_default())
					.bind(notice_text(notice, "body"))
					.bind(notice_text(notice, "effective_date"))
					.bind(notice_text(notice, "expire_date"))
					.bind(notice_text(notice, "writer"))
					.bind(index as i32)
					.bind(updated_by),
				)
				.await
			{
				Ok(row) => row,
				Err(err) => {
					dbx.rollback_txn().await?;
					return Err(crate::model::Error::Store(err.to_string()));
				}
			};
			retained_keys.insert(notice_key);
			let old_value = old.map(|(_, value)| value);
			if let Some(changed_fields) =
				changed_settings_fields(old_value.as_ref(), &new_value)
			{
				let action = if old_value.is_some() {
					"UPDATE"
				} else {
					"CREATE"
				};
				if let Err(err) = insert_notice_audit(
					dbx,
					record_id,
					organization_id,
					action,
					updated_by,
					Some(changed_fields),
					old_value,
					Some(new_value),
				)
				.await
				{
					dbx.rollback_txn().await?;
					return Err(err);
				}
			}
		}

		for (notice_key, (record_id, old_value)) in old_by_key.drain() {
			if retained_keys.contains(&notice_key) {
				continue;
			}
			if let Err(err) = dbx
				.execute(
					sqlx::query(
						"DELETE FROM dashboard_notices WHERE organization_id = $1 AND notice_key = $2",
					)
					.bind(organization_id)
					.bind(&notice_key),
				)
				.await
			{
				dbx.rollback_txn().await?;
				return Err(crate::model::Error::Store(err.to_string()));
			}
			if let Err(err) = insert_notice_audit(
				dbx,
				record_id,
				organization_id,
				"DELETE",
				updated_by,
				None,
				Some(old_value),
				None,
			)
			.await
			{
				dbx.rollback_txn().await?;
				return Err(err);
			}
		}

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
