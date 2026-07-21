use super::{enum_name, AuthorizationCatalogRepository};
use crate::authorization::{Availability, BuiltInIdentityKind, PolicyRegistry};
use crate::model::acs::{
	permissions_for_menu_privileges, role_permissions, AdminMenuPrivilege,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres, Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

pub type MigrationResult<T> = Result<T, AuthorizationMigrationError>;

#[derive(Debug)]
pub enum AuthorizationMigrationError {
	Sqlx(sqlx::Error),
	Configuration(String),
	Registry(String),
	CatalogHashMismatch { stored: String, deployed: String },
	Rejected(Vec<MigrationRejection>),
}

impl Display for AuthorizationMigrationError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for AuthorizationMigrationError {}

impl From<sqlx::Error> for AuthorizationMigrationError {
	fn from(error: sqlx::Error) -> Self {
		Self::Sqlx(error)
	}
}

#[derive(Debug, Clone)]
pub struct MigrationRejection {
	pub user_id: Option<Uuid>,
	pub organization_id: Option<Uuid>,
	pub legacy_role: Option<String>,
	pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
	pub assignments: u64,
	pub custom_roles: u64,
}

pub struct AuthorizationMigrationService;

impl AuthorizationMigrationService {
	pub async fn reconcile_database(
		pool: &Pool<Postgres>,
		registry: &PolicyRegistry,
	) -> MigrationResult<MigrationReport> {
		let mut transaction = pool.begin().await?;
		sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
			.execute(&mut *transaction)
			.await?;
		sqlx::query("SELECT pg_advisory_xact_lock(20260720, 1)")
			.execute(&mut *transaction)
			.await?;
		match Self::reconcile_and_backfill(&mut transaction, registry).await {
			Ok(report) => {
				transaction.commit().await?;
				Ok(report)
			}
			Err(AuthorizationMigrationError::Rejected(rejections)) => {
				transaction.rollback().await?;
				Self::persist_rejections(pool, &rejections).await?;
				Err(AuthorizationMigrationError::Rejected(rejections))
			}
			Err(error) => {
				transaction.rollback().await?;
				Err(error)
			}
		}
	}

	pub async fn reconcile_and_backfill(
		transaction: &mut Transaction<'_, Postgres>,
		registry: &PolicyRegistry,
	) -> MigrationResult<MigrationReport> {
		let catalog_hash =
			AuthorizationCatalogRepository::reconcile(transaction, registry).await?;
		Self::reconcile_builtin_roles(transaction, registry).await?;
		let custom_roles =
			Self::reconcile_custom_roles(transaction, registry).await?;
		let assignments =
			Self::reconcile_assignments(transaction, registry, &catalog_hash)
				.await?;
		Self::remove_stale_custom_roles(transaction).await?;
		Self::reconcile_revision_rows(transaction).await?;
		Ok(MigrationReport {
			assignments,
			custom_roles,
		})
	}

	async fn reconcile_builtin_roles(
		transaction: &mut Transaction<'_, Postgres>,
		registry: &PolicyRegistry,
	) -> MigrationResult<()> {
		for identity in registry.built_in_identities() {
			sqlx::query("INSERT INTO authorization_roles (id, organization_id, stable_key, identity_kind, role_class, name, built_in, active) VALUES ($1, NULL, $2, $3, $4, $2, true, true) ON CONFLICT (id) DO UPDATE SET stable_key = EXCLUDED.stable_key, identity_kind = EXCLUDED.identity_kind, role_class = EXCLUDED.role_class, name = EXCLUDED.name, built_in = true, active = true, updated_at = now()")
				.bind(identity.id)
				.bind(&identity.stable_key)
				.bind(enum_name(&identity.kind)?)
				.bind(enum_name(&identity.role_class)?)
				.execute(&mut **transaction)
				.await?;
			sqlx::query("DELETE FROM role_grants WHERE role_id = $1")
				.bind(identity.id)
				.execute(&mut **transaction)
				.await?;
			for grant in &identity.grants {
				sqlx::query(
					"INSERT INTO role_grants (role_id, grant_id) VALUES ($1, $2)",
				)
				.bind(identity.id)
				.bind(grant.as_str())
				.execute(&mut **transaction)
				.await?;
			}
		}
		Ok(())
	}

	async fn reconcile_custom_roles(
		transaction: &mut Transaction<'_, Postgres>,
		registry: &PolicyRegistry,
	) -> MigrationResult<u64> {
		let rows = sqlx::query("SELECT id, organization_id, name, active, privileges_json FROM permission_profiles WHERE NOT built_in")
			.fetch_all(&mut **transaction)
			.await?;
		let mut rejections = Vec::new();
		for row in &rows {
			let id: Uuid = row.try_get("id")?;
			let organization_id: Uuid = row.try_get("organization_id")?;
			let name: String = row.try_get("name")?;
			let active: bool = row.try_get("active")?;
			let raw: Value = row.try_get("privileges_json")?;
			let privileges: Vec<AdminMenuPrivilege> =
				match serde_json::from_value(raw) {
					Ok(privileges) => privileges,
					Err(error) => {
						rejections.push(MigrationRejection {
							user_id: None,
							organization_id: Some(organization_id),
							legacy_role: Some(id.to_string()),
							reason: format!("invalid privileges_json: {error}"),
						});
						continue;
					}
				};
			let grants = match grants_for_legacy_privileges(registry, &privileges) {
				Ok(grants) => grants,
				Err(reason) => {
					rejections.push(MigrationRejection {
						user_id: None,
						organization_id: Some(organization_id),
						legacy_role: Some(id.to_string()),
						reason,
					});
					continue;
				}
			};
			sqlx::query("INSERT INTO authorization_roles (id, organization_id, stable_key, identity_kind, role_class, name, built_in, active) VALUES ($1, $2, NULL, NULL, 'custom', $3, false, $4) ON CONFLICT (id) DO UPDATE SET organization_id = EXCLUDED.organization_id, name = EXCLUDED.name, active = EXCLUDED.active, updated_at = now()")
				.bind(id).bind(organization_id).bind(name).bind(active)
				.execute(&mut **transaction).await?;
			sqlx::query("DELETE FROM role_grants WHERE role_id = $1")
				.bind(id)
				.execute(&mut **transaction)
				.await?;
			if active {
				for grant in grants {
					sqlx::query("INSERT INTO role_grants (role_id, grant_id) VALUES ($1, $2)").bind(id).bind(grant).execute(&mut **transaction).await?;
				}
			}
		}
		if !rejections.is_empty() {
			return Err(AuthorizationMigrationError::Rejected(rejections));
		}
		Ok(rows.len() as u64)
	}

	async fn reconcile_assignments(
		transaction: &mut Transaction<'_, Postgres>,
		registry: &PolicyRegistry,
		catalog_hash: &str,
	) -> MigrationResult<u64> {
		let builtin_roles = [
			("system_admin", BuiltInIdentityKind::PlatformAdministrator),
			(
				"sponsor_admin_cro",
				BuiltInIdentityKind::SponsorCroAdministrator,
			),
			(
				"sponsor_admin_company",
				BuiltInIdentityKind::SponsorCompanyAdministrator,
			),
			("user", BuiltInIdentityKind::OperationalUser),
		]
		.into_iter()
		.map(|(legacy_role, kind)| {
			registry
				.built_in_identity(kind)
				.map(|identity| (legacy_role, identity.id))
				.ok_or_else(|| {
					AuthorizationMigrationError::Registry(format!(
						"missing registry identity for {kind:?}"
					))
				})
		})
		.collect::<MigrationResult<BTreeMap<_, _>>>()?;
		let rows = sqlx::query("SELECT m.user_id, m.organization_id, u.role FROM user_organization_memberships m JOIN users u ON u.id = m.user_id WHERE m.active")
			.fetch_all(&mut **transaction).await?;
		let mut assignments = Vec::new();
		let mut rejections = Vec::new();
		for row in rows {
			let user_id: Uuid = row.try_get("user_id")?;
			let organization_id: Uuid = row.try_get("organization_id")?;
			let legacy_role: String = row.try_get("role")?;
			let role_id = if let Some(id) = builtin_roles.get(legacy_role.as_str()) {
				Some(*id)
			} else if let Ok(id) = Uuid::parse_str(&legacy_role) {
				sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM authorization_roles r JOIN permission_profiles p ON p.id = r.id AND p.organization_id = r.organization_id WHERE r.id = $1 AND r.organization_id = $2 AND r.active AND NOT r.built_in AND p.active AND NOT p.built_in)")
					.bind(id).bind(organization_id).fetch_one(&mut **transaction).await?.then_some(id)
			} else {
				None
			};
			if let Some(role_id) = role_id {
				assignments.push((user_id, organization_id, legacy_role, role_id));
			} else {
				rejections.push(MigrationRejection {
					user_id: Some(user_id),
					organization_id: Some(organization_id),
					legacy_role: Some(legacy_role),
					reason: "unknown or inactive legacy role".to_string(),
				});
			}
		}
		if !rejections.is_empty() {
			return Err(AuthorizationMigrationError::Rejected(rejections));
		}
		let desired_keys = assignments
			.iter()
			.map(|(user_id, organization_id, _, _)| (*user_id, *organization_id))
			.collect::<BTreeSet<_>>();
		let existing_keys = sqlx::query_as::<_, (Uuid, Uuid)>(
			"SELECT user_id, organization_id FROM user_role_assignments",
		)
		.fetch_all(&mut **transaction)
		.await?;
		for (user_id, organization_id) in existing_keys {
			if !desired_keys.contains(&(user_id, organization_id)) {
				sqlx::query("DELETE FROM user_role_assignments WHERE user_id = $1 AND organization_id = $2")
					.bind(user_id).bind(organization_id).execute(&mut **transaction).await?;
				sqlx::query("DELETE FROM authorization_migration_reconciliations WHERE user_id = $1 AND organization_id = $2")
					.bind(user_id).bind(organization_id).execute(&mut **transaction).await?;
			}
		}
		for (user_id, organization_id, legacy_role, role_id) in &assignments {
			let mut legacy_effective_access =
				if builtin_roles.contains_key(legacy_role.as_str()) {
					role_permissions(legacy_role)
						.iter()
						.map(ToString::to_string)
						.collect::<Vec<_>>()
				} else {
					let raw = sqlx::query_scalar::<_, Value>(
					"SELECT privileges_json FROM permission_profiles WHERE id = $1",
				)
				.bind(role_id)
				.fetch_one(&mut **transaction)
				.await?;
					let privileges: Vec<AdminMenuPrivilege> =
						serde_json::from_value(raw).map_err(|error| {
							AuthorizationMigrationError::Registry(error.to_string())
						})?;
					permissions_for_menu_privileges(&privileges)
						.iter()
						.map(ToString::to_string)
						.collect()
				};
			legacy_effective_access.sort_unstable();
			legacy_effective_access.dedup();
			let grant_ids = sqlx::query_scalar::<_, String>(
				"SELECT grant_id FROM role_grants WHERE role_id = $1 ORDER BY grant_id",
			)
			.bind(role_id)
			.fetch_all(&mut **transaction)
			.await?;
			let (identity_kind, role_class) =
				sqlx::query_as::<_, (Option<String>, String)>(
					"SELECT identity_kind, role_class FROM authorization_roles WHERE id = $1",
				)
				.bind(role_id)
				.fetch_one(&mut **transaction)
				.await?;
			let normalized_effective_access = registry
				.effective_entitlements(grant_ids.iter().map(String::as_str))
				.map_err(|error| {
					AuthorizationMigrationError::Registry(error.to_string())
				})?
				.into_iter()
				.map(|entitlement| entitlement.to_string())
				.collect::<Vec<_>>();
			let evidence_hash = migration_evidence_hash(
				catalog_hash,
				legacy_role,
				*role_id,
				identity_kind.as_deref(),
				&role_class,
				&legacy_effective_access,
				&normalized_effective_access,
			)?;
			sqlx::query("INSERT INTO user_role_assignments (user_id, organization_id, role_id, assigned_at) VALUES ($1, $2, $3, now()) ON CONFLICT (user_id, organization_id) DO UPDATE SET role_id = EXCLUDED.role_id, assigned_at = CASE WHEN user_role_assignments.role_id IS DISTINCT FROM EXCLUDED.role_id THEN now() ELSE user_role_assignments.assigned_at END")
				.bind(user_id).bind(organization_id).bind(role_id).execute(&mut **transaction).await?;
			sqlx::query(r#"
				INSERT INTO authorization_migration_reconciliations AS current (
					user_id, organization_id, legacy_role, normalized_role_id,
					legacy_effective_access, normalized_effective_access,
					evidence_hash, proof_hash, equivalent, comparison_status, reconciled_at
				) VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, NULL, 'pending_action_binding', now())
				ON CONFLICT (user_id, organization_id) DO UPDATE SET
					legacy_role = EXCLUDED.legacy_role,
					normalized_role_id = EXCLUDED.normalized_role_id,
					legacy_effective_access = EXCLUDED.legacy_effective_access,
					normalized_effective_access = EXCLUDED.normalized_effective_access,
					proof_hash = CASE WHEN current.evidence_hash = EXCLUDED.evidence_hash THEN current.proof_hash ELSE NULL END,
					equivalent = CASE WHEN current.evidence_hash = EXCLUDED.evidence_hash THEN current.equivalent ELSE NULL END,
					comparison_status = CASE WHEN current.evidence_hash = EXCLUDED.evidence_hash THEN current.comparison_status ELSE 'pending_action_binding' END,
					evidence_hash = EXCLUDED.evidence_hash,
					reconciled_at = now()
			"#)
				.bind(user_id).bind(organization_id).bind(legacy_role).bind(role_id)
				.bind(sqlx::types::Json(legacy_effective_access))
				.bind(sqlx::types::Json(normalized_effective_access))
				.bind(evidence_hash)
				.execute(&mut **transaction).await?;
		}
		sqlx::query(
			"DELETE FROM authorization_migration_reconciliations r WHERE NOT EXISTS (SELECT 1 FROM user_role_assignments a WHERE a.user_id = r.user_id AND a.organization_id = r.organization_id)",
		)
		.execute(&mut **transaction)
		.await?;
		Ok(assignments.len() as u64)
	}

	async fn remove_stale_custom_roles(
		transaction: &mut Transaction<'_, Postgres>,
	) -> MigrationResult<()> {
		sqlx::query(
			"DELETE FROM authorization_roles r WHERE NOT r.built_in AND NOT EXISTS (SELECT 1 FROM permission_profiles p WHERE p.id = r.id AND NOT p.built_in)",
		)
		.execute(&mut **transaction)
		.await?;
		Ok(())
	}

	async fn reconcile_revision_rows(
		transaction: &mut Transaction<'_, Postgres>,
	) -> MigrationResult<()> {
		sqlx::query("INSERT INTO organization_policy_state (organization_id) SELECT id FROM organizations ON CONFLICT (organization_id) DO NOTHING").execute(&mut **transaction).await?;
		sqlx::query("INSERT INTO principal_authorization_state (user_id, organization_id) SELECT user_id, organization_id FROM user_organization_memberships ON CONFLICT (user_id, organization_id) DO NOTHING").execute(&mut **transaction).await?;
		Ok(())
	}

	async fn persist_rejections(
		pool: &Pool<Postgres>,
		rejections: &[MigrationRejection],
	) -> MigrationResult<()> {
		let mut transaction = pool.begin().await?;
		for rejection in rejections {
			sqlx::query("INSERT INTO authorization_migration_rejections (user_id, organization_id, legacy_role, reason) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING")
				.bind(rejection.user_id).bind(rejection.organization_id).bind(&rejection.legacy_role).bind(&rejection.reason).execute(&mut *transaction).await?;
		}
		transaction.commit().await?;
		Ok(())
	}
}

fn migration_evidence_hash(
	catalog_hash: &str,
	legacy_role: &str,
	normalized_role_id: Uuid,
	identity_kind: Option<&str>,
	role_class: &str,
	legacy_effective_access: &[String],
	normalized_effective_access: &[String],
) -> MigrationResult<String> {
	let canonical = serde_json::to_vec(&(
		catalog_hash,
		legacy_role,
		normalized_role_id,
		identity_kind,
		role_class,
		legacy_effective_access,
		normalized_effective_access,
	))
	.map_err(|error| AuthorizationMigrationError::Registry(error.to_string()))?;
	Ok(format!("{:x}", Sha256::digest(canonical)))
}

fn grants_for_legacy_privileges(
	registry: &PolicyRegistry,
	privileges: &[AdminMenuPrivilege],
) -> Result<BTreeSet<String>, String> {
	let mut grants = BTreeSet::new();
	for privilege in privileges {
		let key = privilege.menu_key.trim();
		let candidates = [
			(privilege.can_read, format!("{key}.read")),
			(privilege.can_edit, format!("{key}.edit")),
			(
				privilege.can_review && key == "case",
				"case.qc.edit".to_string(),
			),
			(
				privilege.can_lock && key == "case",
				"case.lock.edit".to_string(),
			),
		];
		for (enabled, legacy_id) in candidates {
			if !enabled {
				continue;
			}
			let definition = registry
				.grant(&legacy_id)
				.or_else(|| {
					registry
						.legacy_alias(&legacy_id)
						.and_then(|alias| registry.grant(alias.grant_id.as_str()))
				})
				.ok_or_else(|| {
					format!("legacy privilege {legacy_id:?} has no safe PDF grant mapping")
				})?;
			if definition.availability != Availability::Implemented {
				return Err(format!(
					"legacy privilege {legacy_id:?} maps to a reserved grant"
				));
			}
			grants.insert(definition.id.to_string());
		}
	}
	Ok(grants)
}
