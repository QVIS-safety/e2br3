use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, CASE_LIST, CASE_READ, CASE_UPDATE, DASHBOARD_NOTICE_READ,
	DASHBOARD_NOTICE_UPDATE, EMAIL_NOTIFICATION_SEND,
};
use serial_test::serial;

#[test]
#[serial]
fn workflow_read_profile_grants_case_queue_views_only() {
	let _registry = RegistryGuard::new();
	install_profile(
		"workflow_reader",
		profile("home_workflow", true, false, false, false),
	);
	assert!(has_permission("workflow_reader", CASE_READ));
	assert!(has_permission("workflow_reader", CASE_LIST));
	assert!(!has_permission("workflow_reader", CASE_UPDATE));
}

#[test]
#[serial]
fn notice_read_and_edit_profiles_are_separate() {
	let _registry = RegistryGuard::new();
	install_profile(
		"notice_reader",
		profile("home_notice", true, false, false, false),
	);
	install_profile(
		"notice_editor",
		profile("home_notice", false, true, false, false),
	);
	assert!(has_permission("notice_reader", DASHBOARD_NOTICE_READ));
	assert!(!has_permission("notice_reader", DASHBOARD_NOTICE_UPDATE));
	assert!(has_permission("notice_editor", DASHBOARD_NOTICE_READ));
	assert!(has_permission("notice_editor", DASHBOARD_NOTICE_UPDATE));
}

#[test]
#[serial]
fn email_edit_grants_send_but_read_review_and_lock_do_not() {
	let _registry = RegistryGuard::new();
	assert!(profile("home_email", true, false, false, false).is_empty());
	install_profile(
		"email_editor",
		profile("home_email", false, true, false, false),
	);
	assert!(has_permission("email_editor", EMAIL_NOTIFICATION_SEND));
	for flags in [[false, false, true, false], [false, false, false, true]] {
		assert!(
			profile("home_email", flags[0], flags[1], flags[2], flags[3]).is_empty()
		);
	}
}

#[test]
#[serial]
fn unsupported_menu_profile_grants_nothing() {
	let _registry = RegistryGuard::new();
	for key in ["organization", "organizations", "unknown"] {
		assert!(profile(key, true, true, true, true).is_empty(), "{key}");
	}
}
