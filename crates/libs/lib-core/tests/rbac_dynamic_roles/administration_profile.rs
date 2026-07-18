use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, AUDIT_LIST, AUDIT_READ, CASE_CREATE, EMAIL_NOTIFICATION_SEND,
	SETTINGS_READ, SETTINGS_UPDATE, TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT,
	TERMINOLOGY_READ, USER_CREATE, USER_DELETE, USER_LIST, USER_READ, USER_UPDATE,
};
use serial_test::serial;

#[test]
#[serial]
fn user_read_and_write_profiles_are_separate_and_aliases_match() {
	let _registry = RegistryGuard::new();
	assert_eq!(
		profile("user", true, true, true, true),
		profile("users", true, true, true, true)
	);
	install_profile("user_reader", profile("users", true, false, false, false));
	install_profile("user_writer", profile("users", false, true, false, false));
	assert!(has_permission("user_reader", USER_READ));
	assert!(has_permission("user_reader", USER_LIST));
	assert!(!has_permission("user_reader", USER_CREATE));
	for permission in [USER_CREATE, USER_UPDATE, USER_DELETE] {
		assert!(has_permission("user_writer", permission));
	}
}

#[test]
#[serial]
fn audit_read_grants_views_while_review_is_removed() {
	let _registry = RegistryGuard::new();
	assert!(profile("audit", false, false, true, false).is_empty());
	install_profile("auditor", profile("audit", true, false, false, false));
	assert!(has_permission("auditor", AUDIT_READ));
	assert!(has_permission("auditor", AUDIT_LIST));
	assert!(!has_permission("auditor", USER_READ));
}

#[test]
#[serial]
fn terminology_profiles_separate_read_from_import_and_approval() {
	let _registry = RegistryGuard::new();
	assert_eq!(
		profile("data", true, true, true, true),
		profile("terminology", true, true, true, true)
	);
	install_profile(
		"term_reader",
		profile("terminology", true, false, false, false),
	);
	install_profile(
		"term_editor",
		profile("terminology", false, true, false, false),
	);
	assert!(has_permission("term_reader", TERMINOLOGY_READ));
	assert!(!has_permission("term_reader", TERMINOLOGY_IMPORT));
	assert!(has_permission("term_editor", TERMINOLOGY_IMPORT));
	assert!(has_permission("term_editor", TERMINOLOGY_APPROVE));
}

#[test]
#[serial]
fn settings_read_and_update_profiles_are_separate() {
	let _registry = RegistryGuard::new();
	install_profile(
		"settings_reader",
		profile("settings", true, false, false, false),
	);
	install_profile(
		"settings_editor",
		profile("settings", false, true, false, false),
	);
	assert!(has_permission("settings_reader", SETTINGS_READ));
	assert!(!has_permission("settings_reader", SETTINGS_UPDATE));
	assert!(has_permission("settings_editor", SETTINGS_READ));
	assert!(has_permission("settings_editor", SETTINGS_UPDATE));
	assert!(!has_permission("settings_editor", USER_CREATE));
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
fn admin_edit_grants_every_admin_permission_while_read_grants_nothing() {
	let _registry = RegistryGuard::new();
	assert!(profile("admin", true, false, false, false).is_empty());
	let granted =
		install_profile("full_admin", profile("admin", false, true, false, false));
	assert!(granted.len() > 100);
	assert!(has_permission("full_admin", CASE_CREATE));
	assert!(has_permission("full_admin", USER_DELETE));
	assert!(has_permission("full_admin", EMAIL_NOTIFICATION_SEND));
}
