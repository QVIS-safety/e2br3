use super::support::{install, RegistryGuard};
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::acs::{
	has_permission, remove_dynamic_role, CASE_READ, USER_CREATE,
};
use serial_test::serial;

#[test]
#[serial]
fn dynamic_permissions_replace_builtin_permissions() {
	let _registry = RegistryGuard::new();
	install(ROLE_SPONSOR_ADMIN_CRO, vec![CASE_READ]);

	assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, CASE_READ));
	assert!(!has_permission(ROLE_SPONSOR_ADMIN_CRO, USER_CREATE));
}

#[test]
#[serial]
fn removing_override_restores_builtin_permissions() {
	let _registry = RegistryGuard::new();
	install(ROLE_SPONSOR_ADMIN_CRO, vec![CASE_READ]);
	remove_dynamic_role(ROLE_SPONSOR_ADMIN_CRO);

	assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, USER_CREATE));
}
