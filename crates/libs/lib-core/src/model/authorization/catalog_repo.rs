use crate::authorization::{export_contract, PolicyRegistry};
use sqlx::{Postgres, Transaction};

use super::{AuthorizationMigrationError, MigrationResult};

pub struct AuthorizationCatalogRepository;

impl AuthorizationCatalogRepository {
	pub async fn reconcile(
		transaction: &mut Transaction<'_, Postgres>,
		registry: &PolicyRegistry,
	) -> MigrationResult<String> {
		let contract = export_contract(registry).map_err(|error| {
			AuthorizationMigrationError::Registry(error.to_string())
		})?;
		let stored_hash: Option<String> = sqlx::query_scalar(
			"SELECT catalog_hash FROM authorization_catalog_state WHERE singleton FOR UPDATE",
		)
		.fetch_optional(&mut **transaction)
		.await?;
		if let Some(stored_hash) = stored_hash {
			if stored_hash != contract.catalog_hash {
				return Err(AuthorizationMigrationError::CatalogHashMismatch {
					stored: stored_hash,
					deployed: contract.catalog_hash,
				});
			}
		}

		for grant in registry.grants() {
			sqlx::query(
				"INSERT INTO authorization_grant_catalog (grant_id, pdf_order, pdf_menu, pdf_type, pdf_privilege, availability) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (grant_id) DO UPDATE SET pdf_order = EXCLUDED.pdf_order, pdf_menu = EXCLUDED.pdf_menu, pdf_type = EXCLUDED.pdf_type, pdf_privilege = EXCLUDED.pdf_privilege, availability = EXCLUDED.availability",
			)
			.bind(grant.id.as_str())
			.bind(i16::try_from(grant.pdf_order).expect("validated PDF order fits i16"))
			.bind(&grant.pdf_menu)
			.bind(&grant.pdf_type)
			.bind(&grant.pdf_privilege)
			.bind(enum_name(&grant.availability)?)
			.execute(&mut **transaction)
			.await?;
		}
		let stored_grants = sqlx::query_scalar::<_, String>(
			"SELECT grant_id FROM authorization_grant_catalog ORDER BY grant_id",
		)
		.fetch_all(&mut **transaction)
		.await?;
		if let Some(unknown) = stored_grants
			.iter()
			.find(|grant_id| registry.grant(grant_id).is_none())
		{
			return Err(AuthorizationMigrationError::Registry(format!(
				"stored grant {unknown:?} is absent from the deployed registry"
			)));
		}

		sqlx::query("DELETE FROM authorization_grant_role_classes")
			.execute(&mut **transaction)
			.await?;
		for grant in registry.grants() {
			for role_class in &grant.assignable_role_classes {
				sqlx::query("INSERT INTO authorization_grant_role_classes (grant_id, role_class) VALUES ($1, $2)")
					.bind(grant.id.as_str())
					.bind(enum_name(role_class)?)
					.execute(&mut **transaction)
					.await?;
			}
		}

		sqlx::query("INSERT INTO authorization_catalog_state (singleton, schema_version, catalog_hash, reconciled_at) VALUES (true, 1, $1, now()) ON CONFLICT (singleton) DO UPDATE SET schema_version = EXCLUDED.schema_version, catalog_hash = EXCLUDED.catalog_hash, reconciled_at = now()")
			.bind(&contract.catalog_hash)
			.execute(&mut **transaction)
			.await?;
		Ok(contract.catalog_hash)
	}
}

pub(crate) fn enum_name<T: serde::Serialize>(value: &T) -> MigrationResult<String> {
	serde_json::to_value(value)
		.map_err(|error| AuthorizationMigrationError::Registry(error.to_string()))?
		.as_str()
		.map(ToOwned::to_owned)
		.ok_or_else(|| {
			AuthorizationMigrationError::Registry(
				"registry enum did not serialize as a string".to_string(),
			)
		})
}
