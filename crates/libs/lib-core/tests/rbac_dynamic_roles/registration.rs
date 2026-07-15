use super::support::{install, RegistryGuard};
use lib_core::model::acs::{
	has_all_permissions, has_any_permission, has_permission, CASE_LIST, CASE_READ,
	CASE_UPDATE,
};
use serial_test::serial;

#[test]
#[serial]
fn registration_normalizes_role_name_and_exposes_permissions() {
	let _registry = RegistryGuard::new();
	install("  CUSTOM_REVIEWER  ", vec![CASE_READ, CASE_LIST]);

	assert!(has_permission("custom_reviewer", CASE_READ));
	assert!(has_permission(" CUSTOM_REVIEWER ", CASE_LIST));
	assert!(has_any_permission(
		"custom_reviewer",
		&[CASE_UPDATE, CASE_READ]
	));
	assert!(has_all_permissions(
		"custom_reviewer",
		&[CASE_READ, CASE_LIST]
	));
}
