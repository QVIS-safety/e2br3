// Admin Settings BMC — manages the `app_settings` table.
//
// Business logic (payload normalization, defaults, workflow validation)
// stays in the REST layer. This BMC owns only the raw database operations.

use crate::ctx::{
	canonical_role, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER,
};
use crate::model::ModelManager;
use crate::model::Result;
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

pub struct AdminSettingsBmc;

impl AdminSettingsBmc {
	/// Fetch the JSON value stored for `key`, or `None` if not set.
	pub async fn get(mm: &ModelManager, key: &str) -> Result<Option<Value>> {
		let row = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Value,)>(
					"SELECT value FROM app_settings WHERE key = $1",
				)
				.bind(key),
			)
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;
		Ok(row.map(|(value,)| value))
	}

	/// Upsert the JSON value for `key`.
	pub async fn upsert(
		mm: &ModelManager,
		key: &str,
		value: &Value,
		updated_by: Option<Uuid>,
	) -> Result<()> {
		mm.dbx()
			.execute(
				sqlx::query(
					r#"
					INSERT INTO app_settings (key, value, updated_by)
					VALUES ($1, $2, $3)
					ON CONFLICT (key)
					DO UPDATE SET
						value = EXCLUDED.value,
						updated_at = now(),
						updated_by = EXCLUDED.updated_by
					"#,
				)
				.bind(key)
				.bind(value)
				.bind(updated_by),
			)
			.await
			.map(|_| ())
			.map_err(|e| crate::model::Error::Store(e.to_string()))
	}

	/// Return the set of all known workflow role identifiers (built-in + active custom).
	pub async fn known_workflow_roles(mm: &ModelManager) -> Result<HashSet<String>> {
		let mut roles = [
			ROLE_SPONSOR_ADMIN_CRO,
			ROLE_SPONSOR_ADMIN_COMPANY,
			ROLE_USER,
		]
		.into_iter()
		.map(str::to_string)
		.collect::<HashSet<_>>();

		let rows = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, (String,)>(
				"SELECT profile_id FROM permission_profiles WHERE active = true",
			))
			.await
			.map_err(|e| crate::model::Error::Store(e.to_string()))?;
		for (profile_id,) in rows {
			let role = canonical_role(&profile_id);
			if !role.is_empty() {
				roles.insert(role);
			}
		}
		Ok(roles)
	}
}
