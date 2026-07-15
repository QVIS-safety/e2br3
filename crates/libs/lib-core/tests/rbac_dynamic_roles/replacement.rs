use super::support::RegistryGuard;
use lib_core::model::acs::{
	has_permission, replace_dynamic_roles, Permission, CASE_LIST, CASE_READ,
	USER_READ,
};
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial]
fn full_replacement_adds_included_and_removes_omitted_roles() {
	let _registry = RegistryGuard::new();
	let mut first = HashMap::<String, Vec<Permission>>::new();
	first.insert("alpha".to_string(), vec![CASE_READ]);
	first.insert("beta".to_string(), vec![USER_READ]);
	replace_dynamic_roles(first);

	assert!(has_permission("alpha", CASE_READ));
	assert!(has_permission("beta", USER_READ));

	let mut second = HashMap::new();
	second.insert("gamma".to_string(), vec![CASE_LIST]);
	replace_dynamic_roles(second);

	assert!(!has_permission("alpha", CASE_READ));
	assert!(!has_permission("beta", USER_READ));
	assert!(has_permission("gamma", CASE_LIST));
}
