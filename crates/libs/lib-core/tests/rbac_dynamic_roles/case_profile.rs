use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, CASE_APPROVE, CASE_CREATE, CASE_DELETE, CASE_LOCK, CASE_READ,
	CASE_UPDATE, DRUG_DEVICE_CHARACTERISTIC_LIST, DRUG_DEVICE_CHARACTERISTIC_READ,
};
use serial_test::serial;

#[test]
#[serial]
fn case_read_profile_grants_every_view_permission_only() {
	let _registry = RegistryGuard::new();
	let granted =
		install_profile("case_reader", profile("case", true, false, false, false));

	assert!(granted.contains(&CASE_READ));
	assert!(granted.contains(&DRUG_DEVICE_CHARACTERISTIC_READ));
	assert!(granted.contains(&DRUG_DEVICE_CHARACTERISTIC_LIST));
	assert!(!has_permission("case_reader", CASE_CREATE));
	assert!(!has_permission("case_reader", CASE_UPDATE));
	assert!(!has_permission("case_reader", CASE_APPROVE));
}

#[test]
#[serial]
fn case_edit_profile_grants_every_edit_permission_without_delete_or_approve() {
	let _registry = RegistryGuard::new();
	let granted =
		install_profile("case_editor", profile("case", false, true, false, false));

	assert!(granted.contains(&CASE_CREATE));
	assert!(granted.contains(&CASE_UPDATE));
	assert!(!has_permission("case_editor", CASE_DELETE));
	assert!(!has_permission("case_editor", CASE_APPROVE));
}

#[test]
#[serial]
fn case_review_profile_grants_review_permissions_only() {
	let _registry = RegistryGuard::new();
	install_profile("case_reviewer", profile("case", false, false, true, false));

	assert!(has_permission("case_reviewer", CASE_APPROVE));
	assert!(!has_permission("case_reviewer", CASE_UPDATE));
	assert!(!has_permission("case_reviewer", CASE_CREATE));
	assert!(!has_permission("case_reviewer", CASE_DELETE));
	assert!(!has_permission("case_reviewer", CASE_LOCK));
}

#[test]
#[serial]
fn case_lock_profile_grants_lock_permission_only() {
	let _registry = RegistryGuard::new();
	let lock = profile("case", false, false, false, true);
	install_profile("case_locker", lock);
	assert!(has_permission("case_locker", CASE_LOCK));
	assert!(!has_permission("case_locker", CASE_APPROVE));
	assert!(!has_permission("case_locker", CASE_UPDATE));
}
