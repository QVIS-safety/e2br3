use super::support::{install_profile, profile, RegistryGuard};
use lib_core::model::acs::{
	has_permission, XML_EXPORT, XML_EXPORT_READ, XML_IMPORT, XML_IMPORT_READ,
};
use serial_test::serial;

#[test]
#[serial]
fn import_read_and_execute_profiles_are_separate() {
	let _registry = RegistryGuard::new();
	install_profile(
		"import_reader",
		profile("import", true, false, false, false),
	);
	install_profile("importer", profile("import", false, true, false, false));

	assert!(has_permission("import_reader", XML_IMPORT_READ));
	assert!(!has_permission("import_reader", XML_IMPORT));
	assert!(has_permission("importer", XML_IMPORT));
	assert!(!has_permission("importer", XML_IMPORT_READ));
}

#[test]
#[serial]
fn export_read_and_execute_profiles_are_separate() {
	let _registry = RegistryGuard::new();
	install_profile(
		"export_reader",
		profile("export", true, false, false, false),
	);
	install_profile("exporter", profile("export", false, true, false, false));

	assert!(has_permission("export_reader", XML_EXPORT_READ));
	assert!(!has_permission("export_reader", XML_EXPORT));
	assert!(has_permission("exporter", XML_EXPORT));
	assert!(!has_permission("exporter", XML_EXPORT_READ));
}

#[test]
#[serial]
fn every_export_alias_expands_identically() {
	let _registry = RegistryGuard::new();
	let expected = profile("export", true, true, true, true);
	assert_eq!(profile("submission", true, true, true, true), expected);
	assert_eq!(
		profile("export_submission", true, true, true, true),
		expected
	);
}
