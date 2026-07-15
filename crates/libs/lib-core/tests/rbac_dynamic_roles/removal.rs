use super::support::{install, RegistryGuard};
use lib_core::model::acs::{has_permission, remove_dynamic_role, CASE_READ};
use serial_test::serial;

#[test]
#[serial]
fn removal_denies_a_custom_role_afterward() {
	let _registry = RegistryGuard::new();
	install("reviewer", vec![CASE_READ]);
	remove_dynamic_role(" REVIEWER ");

	assert!(!has_permission("reviewer", CASE_READ));
}
