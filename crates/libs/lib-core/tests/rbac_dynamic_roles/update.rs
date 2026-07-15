use super::support::{install, RegistryGuard};
use lib_core::model::acs::{has_permission, CASE_READ, CASE_UPDATE};
use serial_test::serial;

#[test]
#[serial]
fn update_replaces_the_entire_permission_vector() {
	let _registry = RegistryGuard::new();
	install("editor", vec![CASE_READ]);
	install("editor", vec![CASE_UPDATE]);

	assert!(has_permission("editor", CASE_UPDATE));
	assert!(!has_permission("editor", CASE_READ));
}
