use lib_core::model::acs::{
	has_permission, permissions_for_menu_privileges, replace_dynamic_roles,
	upsert_dynamic_role_permissions, AdminMenuPrivilege, Permission,
};
use std::collections::HashMap;

pub struct RegistryGuard;

impl RegistryGuard {
	pub fn new() -> Self {
		replace_dynamic_roles(HashMap::new());
		Self
	}
}

impl Drop for RegistryGuard {
	fn drop(&mut self) {
		replace_dynamic_roles(HashMap::new());
	}
}

pub fn install(role: &str, permissions: Vec<Permission>) {
	upsert_dynamic_role_permissions(role, permissions);
}

pub fn profile(
	menu_key: &str,
	read: bool,
	edit: bool,
	review: bool,
	lock: bool,
) -> Vec<Permission> {
	permissions_for_menu_privileges(&[AdminMenuPrivilege {
		menu_key: menu_key.to_string(),
		can_read: read,
		can_edit: edit,
		can_review: review,
		can_lock: lock,
	}])
}

pub fn install_profile(role: &str, permissions: Vec<Permission>) -> Vec<Permission> {
	install(role, permissions.clone());
	for permission in &permissions {
		assert!(has_permission(role, *permission), "missing {permission}");
	}
	permissions
}
