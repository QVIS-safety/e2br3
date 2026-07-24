use crate::authorization::{
	policy_registry, Availability, BuiltInIdentityKind, GrantUiField,
};
use crate::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use crate::model::acs::{normalize_menu_privileges, AdminMenuPrivilege};
use crate::model::store::dbx::Dbx;
use crate::model::{Error, Result};
use std::collections::BTreeSet;
use uuid::Uuid;

pub struct RoleAssignmentRepository;
pub struct NormalizedRoleRepository;

impl NormalizedRoleRepository {
	pub async fn upsert_custom_role(
		dbx: &Dbx,
		role_id: Uuid,
		organization_id: Uuid,
		name: &str,
		active: bool,
		privileges: &[AdminMenuPrivilege],
	) -> Result<()> {
		let desired = normalized_grant_ids(privileges)?;
		dbx.execute(
			sqlx::query("SELECT authz_upsert_custom_role($1, $2, $3, $4, $5)")
				.bind(role_id)
				.bind(organization_id)
				.bind(name)
				.bind(active)
				.bind(desired.into_iter().collect::<Vec<_>>()),
		)
		.await?;
		Ok(())
	}
}

impl RoleAssignmentRepository {
	pub async fn assign_baseline_user_role(
		dbx: &Dbx,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<()> {
		dbx.execute(
			sqlx::query("SELECT authz_assign_baseline_user_role($1, $2)")
				.bind(user_id)
				.bind(organization_id),
		)
		.await?;
		Ok(())
	}

	pub async fn assign_legacy_role(
		dbx: &Dbx,
		user_id: Uuid,
		organization_id: Uuid,
		legacy_role: &str,
	) -> Result<()> {
		let role_id = normalized_role_id(legacy_role)?;
		dbx.execute(
			sqlx::query("SELECT authz_assign_user_role($1, $2, $3)")
				.bind(user_id)
				.bind(organization_id)
				.bind(role_id),
		)
		.await?;
		Ok(())
	}
}

fn normalized_grant_ids(
	privileges: &[AdminMenuPrivilege],
) -> Result<BTreeSet<String>> {
	let normalized = normalize_menu_privileges(privileges).map_err(|error| {
		Error::Store(format!("invalid role privileges: {error:?}"))
	})?;
	let registry = policy_registry();
	Ok(registry
		.grants()
		.filter(|grant| {
			grant.availability == Availability::Implemented
				&& normalized.iter().any(|privilege| {
					privilege.menu_key == grant.ui_binding.menu_key
						&& match grant.ui_binding.field {
							GrantUiField::CanRead => privilege.can_read,
							GrantUiField::CanEdit => privilege.can_edit,
							GrantUiField::CanReview => privilege.can_review,
							GrantUiField::CanLock => privilege.can_lock,
						}
				})
		})
		.map(|grant| grant.id.to_string())
		.collect())
}

fn normalized_role_id(legacy_role: &str) -> Result<Uuid> {
	let kind = match legacy_role.trim() {
		ROLE_SYSTEM_ADMIN => Some(BuiltInIdentityKind::PlatformAdministrator),
		ROLE_SPONSOR_ADMIN_CRO => Some(BuiltInIdentityKind::SponsorCroAdministrator),
		ROLE_SPONSOR_ADMIN_COMPANY => {
			Some(BuiltInIdentityKind::SponsorCompanyAdministrator)
		}
		ROLE_USER => Some(BuiltInIdentityKind::OperationalUser),
		_ => None,
	};
	if let Some(kind) = kind {
		return policy_registry()
			.built_in_identities()
			.iter()
			.find(|identity| identity.kind == kind)
			.map(|identity| identity.id)
			.ok_or_else(|| Error::Store(format!("missing built-in role {kind:?}")));
	}
	Uuid::parse_str(legacy_role.trim())
		.map_err(|_| Error::Store(format!("unknown role {legacy_role:?}")))
}
