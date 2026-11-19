use super::{
	policy_registry, Availability, BuiltInIdentityKind, GrantId, GrantUiField,
	RegistryError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminMenuPrivilege {
	pub menu_key: String,
	pub can_read: bool,
	pub can_edit: bool,
	pub can_review: bool,
	pub can_lock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivilegeAdapterError {
	UnknownMenu { menu_key: String },
	InvalidPolicyRegistry(RegistryError),
}

impl std::fmt::Display for PrivilegeAdapterError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for PrivilegeAdapterError {}

impl From<RegistryError> for PrivilegeAdapterError {
	fn from(error: RegistryError) -> Self {
		Self::InvalidPolicyRegistry(error)
	}
}

fn empty_privilege(menu_key: String) -> AdminMenuPrivilege {
	AdminMenuPrivilege {
		menu_key,
		can_read: false,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}
}

fn set_enabled(privilege: &mut AdminMenuPrivilege, field: GrantUiField) {
	match field {
		GrantUiField::CanRead => privilege.can_read = true,
		GrantUiField::CanEdit => privilege.can_edit = true,
		GrantUiField::CanReview => privilege.can_review = true,
		GrantUiField::CanLock => privilege.can_lock = true,
	}
}

pub fn built_in_menu_privileges(
	kind: BuiltInIdentityKind,
) -> Result<Vec<AdminMenuPrivilege>, PrivilegeAdapterError> {
	let registry = policy_registry()?;
	let Some(identity) = registry.built_in_identity(kind) else {
		return Ok(Vec::new());
	};
	let effective_grants = registry
		.effective_grants(identity.grants.iter().map(|grant| grant.as_str()))?;
	let mut privileges = BTreeMap::new();
	for grant_id in effective_grants {
		let Some(grant) = registry.grant(grant_id.as_str()) else {
			return Err(PrivilegeAdapterError::InvalidPolicyRegistry(
				RegistryError::UnknownGrant {
					owner: "effective grants".to_string(),
					grant: grant_id.to_string(),
				},
			));
		};
		if grant.availability != Availability::Implemented {
			continue;
		}
		let privilege = privileges
			.entry(grant.ui_binding.menu_key.clone())
			.or_insert_with(|| empty_privilege(grant.ui_binding.menu_key.clone()));
		set_enabled(privilege, grant.ui_binding.field);
	}
	Ok(privileges.into_values().collect())
}

pub fn normalize_menu_privileges(
	privileges: &[AdminMenuPrivilege],
) -> Result<Vec<AdminMenuPrivilege>, PrivilegeAdapterError> {
	normalize_menu_privileges_with_aliases(privileges, true)
}

pub fn normalize_current_menu_privileges(
	privileges: &[AdminMenuPrivilege],
) -> Result<Vec<AdminMenuPrivilege>, PrivilegeAdapterError> {
	normalize_menu_privileges_with_aliases(privileges, false)
}

pub fn grant_ids_for_menu_privileges(
	privileges: &[AdminMenuPrivilege],
	allow_legacy_aliases: bool,
) -> Result<BTreeSet<GrantId>, PrivilegeAdapterError> {
	let normalized =
		normalize_menu_privileges_with_aliases(privileges, allow_legacy_aliases)?;
	let registry = policy_registry()?;
	Ok(registry
		.grants()
		.filter(|grant| {
			grant.availability == Availability::Implemented
				&& normalized.iter().any(|privilege| {
					privilege.menu_key == grant.ui_binding.menu_key
						&& enabled(privilege, grant.ui_binding.field)
				})
		})
		.map(|grant| grant.id.clone())
		.collect())
}

fn normalize_menu_privileges_with_aliases(
	privileges: &[AdminMenuPrivilege],
	allow_legacy_aliases: bool,
) -> Result<Vec<AdminMenuPrivilege>, PrivilegeAdapterError> {
	let registry = policy_registry()?;
	let mut normalized = BTreeMap::new();
	for privilege in privileges {
		let menu_key = privilege.menu_key.trim().to_ascii_lowercase();
		let direct = registry
			.grants()
			.filter(|grant| grant.ui_binding.menu_key == menu_key)
			.collect::<Vec<_>>();
		let alias_prefix = format!("{menu_key}.");
		let has_alias = registry
			.legacy_aliases()
			.any(|alias| alias.legacy_id.starts_with(&alias_prefix));
		if direct.is_empty() && (!allow_legacy_aliases || !has_alias) {
			return Err(PrivilegeAdapterError::UnknownMenu { menu_key });
		}

		for field in [
			GrantUiField::CanRead,
			GrantUiField::CanEdit,
			GrantUiField::CanReview,
			GrantUiField::CanLock,
		] {
			if !enabled(privilege, field) {
				continue;
			}
			let grant = direct
				.iter()
				.copied()
				.find(|grant| grant.ui_binding.field == field)
				.or_else(|| {
					if !allow_legacy_aliases {
						return None;
					}
					let legacy_id =
						format!("{menu_key}.{}", legacy_flag_name(field));
					registry
						.legacy_alias(&legacy_id)
						.and_then(|alias| registry.grant(alias.grant_id.as_str()))
				});
			let Some(grant) = grant else { continue };
			if grant.availability == Availability::Reserved {
				continue;
			}
			let entry = normalized
				.entry(grant.ui_binding.menu_key.clone())
				.or_insert_with(|| {
					empty_privilege(grant.ui_binding.menu_key.clone())
				});
			set_enabled(entry, grant.ui_binding.field);
		}
	}
	Ok(normalized.into_values().collect())
}

fn legacy_flag_name(field: GrantUiField) -> &'static str {
	match field {
		GrantUiField::CanRead => "read",
		GrantUiField::CanEdit => "edit",
		GrantUiField::CanReview => "review",
		GrantUiField::CanLock => "lock",
	}
}

fn enabled(privilege: &AdminMenuPrivilege, field: GrantUiField) -> bool {
	match field {
		GrantUiField::CanRead => privilege.can_read,
		GrantUiField::CanEdit => privilege.can_edit,
		GrantUiField::CanReview => privilege.can_review,
		GrantUiField::CanLock => privilege.can_lock,
	}
}
