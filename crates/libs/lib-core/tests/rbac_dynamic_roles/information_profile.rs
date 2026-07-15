use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, NARRATIVE_CREATE, NARRATIVE_READ, PRESAVE_TEMPLATE_CREATE,
	PRESAVE_TEMPLATE_READ, RECEIVER_CREATE, RECEIVER_READ,
	SENDER_INFORMATION_CREATE, SENDER_INFORMATION_READ, STUDY_INFORMATION_CREATE,
	STUDY_INFORMATION_READ,
};
use serial_test::serial;

#[test]
#[serial]
fn information_read_profile_grants_all_information_views_only() {
	let _registry = RegistryGuard::new();
	install_profile("info_reader", profile("info", true, false, false, false));

	for permission in [
		PRESAVE_TEMPLATE_READ,
		SENDER_INFORMATION_READ,
		RECEIVER_READ,
		STUDY_INFORMATION_READ,
		NARRATIVE_READ,
	] {
		assert!(has_permission("info_reader", permission));
	}
	assert!(!has_permission("info_reader", NARRATIVE_CREATE));
}

#[test]
#[serial]
fn information_edit_profile_grants_all_information_writes_only() {
	let _registry = RegistryGuard::new();
	install_profile("info_editor", profile("info", false, true, false, false));

	for permission in [
		PRESAVE_TEMPLATE_CREATE,
		SENDER_INFORMATION_CREATE,
		RECEIVER_CREATE,
		STUDY_INFORMATION_CREATE,
		NARRATIVE_CREATE,
	] {
		assert!(has_permission("info_editor", permission));
	}
	assert!(!has_permission("info_editor", NARRATIVE_READ));
}
