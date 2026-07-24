//! Compatibility adapter from the legacy menu-flag storage shape to the
//! registry-owned grant model.

use super::*;
use crate::authorization::{policy_registry, Availability, GrantUiField};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

pub fn built_in_menu_privileges(role: &str) -> Vec<AdminMenuPrivilege> {
	let normalized_role = crate::ctx::canonical_role(role);
	let allowed_menus: Option<&[&str]> = match normalized_role.as_str() {
		crate::ctx::ROLE_SYSTEM_ADMIN => Some(&["home_notice", "admin"]),
		crate::ctx::ROLE_SPONSOR_ADMIN_CRO
		| crate::ctx::ROLE_SPONSOR_ADMIN_COMPANY => None,
		_ => return Vec::new(),
	};
	let mut privileges = BTreeMap::new();
	for grant in policy_registry().grants().filter(|grant| {
		grant.availability == Availability::Implemented
			&& allowed_menus.is_none_or(|menus| {
				menus.contains(&grant.ui_binding.menu_key.as_str())
			})
	}) {
		let privilege = privileges
			.entry(grant.ui_binding.menu_key.clone())
			.or_insert_with(|| empty_privilege(grant.ui_binding.menu_key.clone()));
		set_enabled(privilege, grant.ui_binding.field);
	}
	privileges.into_values().collect()
}

fn legacy_flag_name(field: GrantUiField) -> &'static str {
	match field {
		GrantUiField::CanRead => "read",
		GrantUiField::CanEdit => "edit",
		GrantUiField::CanReview => "review",
		GrantUiField::CanLock => "lock",
	}
}

pub fn normalize_menu_privileges(
	privileges: &[AdminMenuPrivilege],
) -> Result<Vec<AdminMenuPrivilege>, PrivilegeAdapterError> {
	let registry = policy_registry();
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
		if direct.is_empty() && !has_alias {
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

fn enabled(privilege: &AdminMenuPrivilege, field: GrantUiField) -> bool {
	match field {
		GrantUiField::CanRead => privilege.can_read,
		GrantUiField::CanEdit => privilege.can_edit,
		GrantUiField::CanReview => privilege.can_review,
		GrantUiField::CanLock => privilege.can_lock,
	}
}

fn push_unique(target: &mut Vec<Permission>, source: &[Permission]) {
	for permission in source {
		if !target.contains(permission) {
			target.push(*permission);
		}
	}
}

fn append_permissions_for_grant(target: &mut Vec<Permission>, id: &str) {
	match id {
		"home.notice.read" => push_unique(target, &[DASHBOARD_NOTICE_READ]),
		"home.notice.edit" => push_unique(target, &[DASHBOARD_NOTICE_UPDATE]),
		"home.workflow.read" | "case.workflow.read" => {
			push_unique(target, &[CASE_READ, CASE_LIST]);
		}
		"case.read" => push_unique(target, case_view_permissions()),
		"case.edit" => {
			push_unique(target, &[CASE_CREATE]);
			push_unique(target, profile_edit_permissions());
		}
		"case.review" => push_unique(target, &[CASE_APPROVE]),
		"case.lock" => push_unique(target, &[CASE_LOCK]),
		"info.read" => push_unique(
			target,
			&[
				PRESAVE_TEMPLATE_READ,
				PRESAVE_TEMPLATE_LIST,
				SENDER_INFORMATION_READ,
				SENDER_INFORMATION_LIST,
				RECEIVER_READ,
				RECEIVER_LIST,
				STUDY_INFORMATION_READ,
				STUDY_INFORMATION_LIST,
				NARRATIVE_READ,
				NARRATIVE_LIST,
			],
		),
		"info.edit" => push_unique(
			target,
			&[
				PRESAVE_TEMPLATE_CREATE,
				PRESAVE_TEMPLATE_UPDATE,
				PRESAVE_TEMPLATE_DELETE,
				SENDER_INFORMATION_CREATE,
				SENDER_INFORMATION_UPDATE,
				SENDER_INFORMATION_DELETE,
				RECEIVER_CREATE,
				RECEIVER_UPDATE,
				RECEIVER_DELETE,
				STUDY_INFORMATION_CREATE,
				STUDY_INFORMATION_UPDATE,
				STUDY_INFORMATION_DELETE,
				NARRATIVE_CREATE,
				NARRATIVE_UPDATE,
				NARRATIVE_DELETE,
			],
		),
		"import.history.read" => push_unique(target, &[XML_IMPORT_READ]),
		"import.execute" => push_unique(target, &[XML_IMPORT]),
		"submission.history.read" => push_unique(target, &[XML_EXPORT_READ]),
		"submission.execute" => push_unique(target, &[XML_EXPORT]),
		"admin.read" => push_unique(
			target,
			&[
				USER_READ,
				USER_LIST,
				ORG_READ,
				ORG_LIST,
				SETTINGS_READ,
				AUDIT_READ,
				AUDIT_LIST,
				TERMINOLOGY_READ,
			],
		),
		"admin.edit" => push_unique(
			target,
			&[
				USER_CREATE,
				USER_UPDATE,
				USER_DELETE,
				ORG_CREATE,
				ORG_UPDATE,
				ORG_DELETE,
				SETTINGS_UPDATE,
				TERMINOLOGY_IMPORT,
				TERMINOLOGY_APPROVE,
			],
		),
		_ => {}
	}
}

pub fn permissions_for_menu_privileges(
	privileges: &[AdminMenuPrivilege],
) -> Vec<Permission> {
	let registry = policy_registry();
	let mut grants = Vec::new();
	let normalized = privileges
		.iter()
		.filter_map(|privilege| {
			normalize_menu_privileges(std::slice::from_ref(privilege)).ok()
		})
		.flatten()
		.collect::<Vec<_>>();
	for privilege in &normalized {
		for grant in registry.grants().filter(|grant| {
			grant.availability == Availability::Implemented
				&& grant.ui_binding.menu_key == privilege.menu_key
				&& enabled(privilege, grant.ui_binding.field)
		}) {
			if !grants.iter().any(|id| id == &grant.id) {
				grants.push(grant.id.clone());
			}
		}
	}

	let Ok(grants) =
		registry.effective_grants(grants.iter().map(|grant| grant.as_str()))
	else {
		return Vec::new();
	};
	let mut permissions = Vec::new();
	for grant in grants {
		append_permissions_for_grant(&mut permissions, grant.as_str());
	}
	permissions
}
