use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, AUDIT_LIST, AUDIT_READ, CASE_CREATE, EMAIL_NOTIFICATION_SEND,
	SETTINGS_READ, SETTINGS_UPDATE, TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT,
	TERMINOLOGY_READ, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
};
use serial_test::serial;

#[test]
#[serial]
fn removed_user_rows_do_not_create_hidden_admin_grants() {
	let _registry = RegistryGuard::new();
	for key in ["user", "users"] {
		assert!(profile(key, true, true, true, true).is_empty());
	}
}

#[test]
#[serial]
fn removed_audit_row_grants_nothing() {
	let _registry = RegistryGuard::new();
	assert!(profile("audit", true, true, true, true).is_empty());
}

#[test]
#[serial]
fn removed_terminology_rows_grant_nothing() {
	let _registry = RegistryGuard::new();
	for key in ["data", "terminology"] {
		assert!(profile(key, true, true, true, true).is_empty());
	}
}

#[test]
#[serial]
fn removed_settings_row_grants_nothing() {
	let _registry = RegistryGuard::new();
	assert!(profile("settings", true, true, true, true).is_empty());
}

#[test]
#[serial]
fn roles_alias_grants_nothing() {
	let _registry = RegistryGuard::new();
	assert!(profile("roles", true, false, false, false).is_empty());
	assert!(profile("roles", false, true, false, false).is_empty());
}

#[test]
#[serial]
fn admin_read_and_edit_are_the_only_administration_grants() {
	let _registry = RegistryGuard::new();
	install_profile("admin_reader", profile("admin", true, false, false, false));
	assert!(has_permission("admin_reader", USER_READ));
	assert!(has_permission("admin_reader", USER_LIST));
	assert!(has_permission("admin_reader", SETTINGS_READ));
	assert!(has_permission("admin_reader", AUDIT_READ));
	assert!(has_permission("admin_reader", AUDIT_LIST));
	assert!(has_permission("admin_reader", TERMINOLOGY_READ));
	assert!(!has_permission("admin_reader", USER_CREATE));

	install_profile("admin_editor", profile("admin", false, true, false, false));
	for permission in [USER_CREATE, USER_UPDATE, USER_DELETE, SETTINGS_UPDATE] {
		assert!(has_permission("admin_editor", permission));
	}
	assert!(has_permission("admin_editor", TERMINOLOGY_IMPORT));
	assert!(has_permission("admin_editor", TERMINOLOGY_APPROVE));
	assert!(!has_permission("admin_editor", CASE_CREATE));
	assert!(!has_permission("admin_editor", EMAIL_NOTIFICATION_SEND));
}
