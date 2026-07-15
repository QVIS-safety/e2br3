use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::acs::{
	has_all_permissions, has_any_permission, has_permission, remove_dynamic_role,
	replace_dynamic_roles, upsert_dynamic_role_permissions, Permission, CASE_LIST,
	CASE_READ, CASE_UPDATE, USER_CREATE, USER_READ,
};
use std::collections::HashMap;

struct RegistryCleanup;

impl Drop for RegistryCleanup {
	fn drop(&mut self) {
		replace_dynamic_roles(HashMap::new());
	}
}

#[test]
fn dynamic_role_lifecycle_and_builtin_precedence() {
	let _cleanup = RegistryCleanup;
	replace_dynamic_roles(HashMap::new());

	upsert_dynamic_role_permissions(
		"  CUSTOM_REVIEWER  ",
		vec![CASE_READ, CASE_LIST],
	);
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

	upsert_dynamic_role_permissions("custom_reviewer", vec![CASE_UPDATE]);
	assert!(has_permission("custom_reviewer", CASE_UPDATE));
	assert!(!has_permission("custom_reviewer", CASE_READ));

	remove_dynamic_role(" CUSTOM_REVIEWER ");
	assert!(!has_permission("custom_reviewer", CASE_UPDATE));

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

	upsert_dynamic_role_permissions(ROLE_SPONSOR_ADMIN_CRO, vec![CASE_READ]);
	assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, CASE_READ));
	assert!(!has_permission(ROLE_SPONSOR_ADMIN_CRO, USER_CREATE));

	remove_dynamic_role(ROLE_SPONSOR_ADMIN_CRO);
	assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, USER_CREATE));
}
