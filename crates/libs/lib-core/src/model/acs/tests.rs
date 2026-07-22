use super::*;
use crate::ctx::{
	ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn acs_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model/acs")
}

#[test]
fn acs_modules_separate_types_and_catalog() {
	let dir = acs_dir();
	for module in [
		"types.rs",
		"catalog.rs",
		"builtin_roles.rs",
		"registry_adapter.rs",
		"dynamic_roles.rs",
		"check.rs",
	] {
		assert!(dir.join(module).is_file(), "missing ACS module {module}");
	}
	let catalog = fs::read_to_string(dir.join("catalog.rs")).unwrap();
	assert!(catalog.contains("permission_group! {"));
	assert!(
		!dir.join("permission.rs").exists(),
		"legacy permission.rs should be removed"
	);
	assert!(
		!dir.join("menu_policy.rs").exists(),
		"handwritten menu policy should be removed"
	);
}

#[test]
fn role_privilege_policy_is_registry_owned() {
	let source = fs::read_to_string(acs_dir().join("registry_adapter.rs")).unwrap();
	assert!(source.contains("policy_registry()"));
	assert!(!source.contains("MENU_POLICIES"));
}

#[test]
fn case_read_privilege_covers_drug_device_characteristic_routes() {
	let permissions = permissions_for_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "case".to_string(),
		can_read: true,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}]);

	assert!(permissions.contains(&DRUG_DEVICE_CHARACTERISTIC_READ));
	assert!(permissions.contains(&DRUG_DEVICE_CHARACTERISTIC_LIST));
}

#[test]
fn sponsor_admin_can_send_configured_email_notifications() {
	for role in [ROLE_SPONSOR_ADMIN_CRO, ROLE_SPONSOR_ADMIN_COMPANY] {
		assert!(has_permission(role, EMAIL_NOTIFICATION_SEND), "{role}");
	}
}

#[test]
fn system_admin_profile_matches_platform_admin_endpoints() {
	for permission in [
		USER_LIST,
		USER_CREATE,
		USER_UPDATE,
		USER_DELETE,
		ORG_LIST,
		ORG_CREATE,
		ORG_UPDATE,
		ORG_DELETE,
		AUDIT_LIST,
		AUDIT_READ,
		SETTINGS_READ,
		SETTINGS_UPDATE,
	] {
		assert!(
			has_permission(ROLE_SYSTEM_ADMIN, permission),
			"{permission}"
		);
	}

	assert!(!has_permission(ROLE_SYSTEM_ADMIN, CASE_READ));
	assert!(!has_permission(ROLE_SYSTEM_ADMIN, TERMINOLOGY_READ));
}

#[test]
fn menu_aliases_expand_to_identical_permissions() {
	fn expand(menu_key: &str) -> Vec<Permission> {
		permissions_for_menu_privileges(&[AdminMenuPrivilege {
			menu_key: menu_key.to_string(),
			can_read: true,
			can_edit: true,
			can_review: true,
			can_lock: true,
		}])
	}

	for aliases in [["export_submission", "submission", "export"]] {
		assert_eq!(expand(aliases[0]), expand(aliases[1]));
		assert_eq!(expand(aliases[0]), expand(aliases[2]));
	}
	for removed in ["user", "users", "data", "terminology", "settings", "roles"] {
		assert!(expand(removed).is_empty(), "{removed}");
	}
}

#[test]
fn pdf_admin_rows_compile_through_the_registry() {
	let read = permissions_for_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "admin".to_string(),
		can_read: true,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}]);
	assert!(read.contains(&USER_READ));
	assert!(read.contains(&SETTINGS_READ));
	assert!(!read.contains(&USER_CREATE));
	assert!(!read.contains(&SETTINGS_UPDATE));

	let edit = permissions_for_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "admin".to_string(),
		can_read: false,
		can_edit: true,
		can_review: false,
		can_lock: false,
	}]);
	assert!(edit.contains(&USER_CREATE));
	assert!(edit.contains(&SETTINGS_UPDATE));
	assert!(edit.contains(&USER_READ));
}

#[test]
fn reserved_and_obsolete_email_rows_grant_nothing() {
	for menu_key in [
		"email_report_due",
		"home_email",
		"email_review",
		"email_lock",
	] {
		let permissions = permissions_for_menu_privileges(&[AdminMenuPrivilege {
			menu_key: menu_key.to_string(),
			can_read: true,
			can_edit: true,
			can_review: false,
			can_lock: false,
		}]);
		assert!(permissions.is_empty(), "{menu_key}: {permissions:?}");
	}
}

#[test]
fn registry_normalization_drops_reserved_rows_and_rejects_removed_rows() {
	let reserved = normalize_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "email_report_due".to_string(),
		can_read: true,
		can_edit: true,
		can_review: false,
		can_lock: false,
	}])
	.unwrap();
	assert!(reserved.is_empty());

	let removed = normalize_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "users".to_string(),
		can_read: true,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}]);
	assert!(matches!(
		removed,
		Err(PrivilegeAdapterError::UnknownMenu { .. })
	));

	let alias = normalize_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "export".to_string(),
		can_read: true,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}])
	.unwrap();
	assert_eq!(alias[0].menu_key, "export_submission");
	assert!(alias[0].can_read);
}

#[test]
fn permission_catalog_is_complete_unique_and_stable() {
	let values = all_permissions()
		.iter()
		.map(ToString::to_string)
		.collect::<Vec<_>>();
	let unique = values.iter().collect::<HashSet<_>>();

	assert_eq!(unique.len(), values.len());
	assert!(values.iter().all(|value| {
		let mut parts = value.split('.');
		parts.next().is_some_and(|part| !part.is_empty())
			&& parts.next().is_some_and(|part| !part.is_empty())
			&& parts.next().is_none()
	}));
	for required in [
		"Case.Read",
		"StudyRegistration.Update",
		"XmlImport.Import",
		"XmlExport.Export",
	] {
		assert!(
			values.iter().any(|value| value == required),
			"missing {required}"
		);
	}
}
