//! Permission definitions for the Access Control System
//!
//! Defines resources, actions, and the permission matrix for RBAC.

use super::*;
use crate::ctx::{
	canonical_role, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_SYSTEM_ADMIN, ROLE_USER,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

// region:    --- Role Permission Mappings

/// Returns all permissions for the admin role
fn admin_permissions() -> &'static [Permission] {
	&[
		// Case - full access
		CASE_CREATE,
		CASE_READ,
		CASE_UPDATE,
		CASE_DELETE,
		CASE_LIST,
		CASE_APPROVE,
		// Patient
		PATIENT_CREATE,
		PATIENT_READ,
		PATIENT_UPDATE,
		PATIENT_DELETE,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_CREATE,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_UPDATE,
		PATIENT_IDENTIFIER_DELETE,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_CREATE,
		DRUG_READ,
		DRUG_UPDATE,
		DRUG_DELETE,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_CREATE,
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_UPDATE,
		DRUG_SUBSTANCE_DELETE,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_CREATE,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_UPDATE,
		DRUG_DOSAGE_DELETE,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_CREATE,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_UPDATE,
		DRUG_INDICATION_DELETE,
		DRUG_INDICATION_LIST,
		DRUG_DEVICE_CHARACTERISTIC_CREATE,
		DRUG_DEVICE_CHARACTERISTIC_READ,
		DRUG_DEVICE_CHARACTERISTIC_UPDATE,
		DRUG_DEVICE_CHARACTERISTIC_DELETE,
		DRUG_DEVICE_CHARACTERISTIC_LIST,
		DRUG_REACTION_ASSESSMENT_CREATE,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_UPDATE,
		DRUG_REACTION_ASSESSMENT_DELETE,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_CREATE,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_UPDATE,
		RELATEDNESS_ASSESSMENT_DELETE,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_CREATE,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_UPDATE,
		DRUG_RECURRENCE_DELETE,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_CREATE,
		REACTION_READ,
		REACTION_UPDATE,
		REACTION_DELETE,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_CREATE,
		TEST_RESULT_READ,
		TEST_RESULT_UPDATE,
		TEST_RESULT_DELETE,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_CREATE,
		NARRATIVE_READ,
		NARRATIVE_UPDATE,
		NARRATIVE_DELETE,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_CREATE,
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_UPDATE,
		SENDER_DIAGNOSIS_DELETE,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_CREATE,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_UPDATE,
		CASE_SUMMARY_DELETE,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_CREATE,
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_UPDATE,
		MESSAGE_HEADER_DELETE,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_CREATE,
		SAFETY_REPORT_READ,
		SAFETY_REPORT_UPDATE,
		SAFETY_REPORT_DELETE,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_CREATE,
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_UPDATE,
		SENDER_INFORMATION_DELETE,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_CREATE,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_UPDATE,
		PRIMARY_SOURCE_DELETE,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_CREATE,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_UPDATE,
		LITERATURE_REFERENCE_DELETE,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_CREATE,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_UPDATE,
		STUDY_INFORMATION_DELETE,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_CREATE,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_UPDATE,
		STUDY_REGISTRATION_DELETE,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_CREATE,
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_UPDATE,
		MEDICAL_HISTORY_DELETE,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_CREATE,
		PAST_DRUG_READ,
		PAST_DRUG_UPDATE,
		PAST_DRUG_DELETE,
		PAST_DRUG_LIST,
		PATIENT_DEATH_CREATE,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_UPDATE,
		PATIENT_DEATH_DELETE,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_CREATE,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_UPDATE,
		DEATH_CAUSE_DELETE,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_CREATE,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_UPDATE,
		PARENT_INFORMATION_DELETE,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_CREATE,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_UPDATE,
		PARENT_MEDICAL_HISTORY_DELETE,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_CREATE,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_UPDATE,
		PARENT_PAST_DRUG_DELETE,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_CREATE,
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_UPDATE,
		CASE_IDENTIFIER_DELETE,
		CASE_IDENTIFIER_LIST,
		RECEIVER_CREATE,
		RECEIVER_READ,
		RECEIVER_UPDATE,
		RECEIVER_DELETE,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_CREATE,
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_UPDATE,
		PRESAVE_TEMPLATE_DELETE,
		PRESAVE_TEMPLATE_LIST,
		// User - full access
		USER_CREATE,
		USER_READ,
		USER_UPDATE,
		USER_DELETE,
		USER_LIST,
		// Organization - full access
		ORG_CREATE,
		ORG_READ,
		ORG_UPDATE,
		ORG_DELETE,
		ORG_LIST,
		// AuditLog
		AUDIT_READ,
		AUDIT_LIST,
		// Settings
		SETTINGS_READ,
		SETTINGS_UPDATE,
		DASHBOARD_NOTICE_READ,
		DASHBOARD_NOTICE_UPDATE,
		// Terminology
		TERMINOLOGY_READ,
		TERMINOLOGY_IMPORT,
		TERMINOLOGY_APPROVE,
		// XML
		XML_EXPORT_READ,
		XML_EXPORT,
		XML_IMPORT_READ,
		XML_IMPORT,
	]
}

fn system_admin_permissions() -> &'static [Permission] {
	&[
		SETTINGS_READ,
		SETTINGS_UPDATE,
		DASHBOARD_NOTICE_READ,
		DASHBOARD_NOTICE_UPDATE,
	]
}

/// Returns operational edit permissions used by permission profiles.
fn profile_edit_permissions() -> &'static [Permission] {
	&[
		CASE_CREATE,
		CASE_READ,
		CASE_UPDATE,
		CASE_LIST,
		PATIENT_CREATE,
		PATIENT_READ,
		PATIENT_UPDATE,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_CREATE,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_UPDATE,
		PATIENT_IDENTIFIER_LIST,
		DRUG_CREATE,
		DRUG_READ,
		DRUG_UPDATE,
		DRUG_LIST,
		DRUG_SUBSTANCE_CREATE,
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_UPDATE,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_CREATE,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_UPDATE,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_CREATE,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_UPDATE,
		DRUG_INDICATION_LIST,
		DRUG_DEVICE_CHARACTERISTIC_CREATE,
		DRUG_DEVICE_CHARACTERISTIC_READ,
		DRUG_DEVICE_CHARACTERISTIC_UPDATE,
		DRUG_DEVICE_CHARACTERISTIC_LIST,
		DRUG_REACTION_ASSESSMENT_CREATE,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_UPDATE,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_CREATE,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_UPDATE,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_CREATE,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_UPDATE,
		DRUG_RECURRENCE_LIST,
		REACTION_CREATE,
		REACTION_READ,
		REACTION_UPDATE,
		REACTION_LIST,
		TEST_RESULT_CREATE,
		TEST_RESULT_READ,
		TEST_RESULT_UPDATE,
		TEST_RESULT_LIST,
		NARRATIVE_CREATE,
		NARRATIVE_READ,
		NARRATIVE_UPDATE,
		NARRATIVE_LIST,
		SENDER_DIAGNOSIS_CREATE,
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_UPDATE,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_CREATE,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_UPDATE,
		CASE_SUMMARY_LIST,
		MESSAGE_HEADER_CREATE,
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_UPDATE,
		MESSAGE_HEADER_LIST,
		SAFETY_REPORT_CREATE,
		SAFETY_REPORT_READ,
		SAFETY_REPORT_UPDATE,
		SAFETY_REPORT_LIST,
		SENDER_INFORMATION_CREATE,
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_UPDATE,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_CREATE,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_UPDATE,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_CREATE,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_UPDATE,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_CREATE,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_UPDATE,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_CREATE,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_UPDATE,
		STUDY_REGISTRATION_LIST,
		MEDICAL_HISTORY_CREATE,
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_UPDATE,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_CREATE,
		PAST_DRUG_READ,
		PAST_DRUG_UPDATE,
		PAST_DRUG_LIST,
		PATIENT_DEATH_CREATE,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_UPDATE,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_CREATE,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_UPDATE,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_CREATE,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_UPDATE,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_CREATE,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_UPDATE,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_CREATE,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_UPDATE,
		PARENT_PAST_DRUG_LIST,
		CASE_IDENTIFIER_CREATE,
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_UPDATE,
		CASE_IDENTIFIER_LIST,
		RECEIVER_CREATE,
		RECEIVER_READ,
		RECEIVER_UPDATE,
		RECEIVER_LIST,
		PRESAVE_TEMPLATE_CREATE,
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_UPDATE,
		PRESAVE_TEMPLATE_DELETE,
		PRESAVE_TEMPLATE_LIST,
		USER_READ,
		ORG_READ,
		TERMINOLOGY_READ,
		XML_EXPORT,
	]
}

/// Returns all permissions for the viewer role
fn viewer_permissions() -> &'static [Permission] {
	&[
		// Case - read only
		CASE_READ,
		CASE_LIST,
		// Patient
		PATIENT_READ,
		PATIENT_LIST,
		PATIENT_IDENTIFIER_READ,
		PATIENT_IDENTIFIER_LIST,
		// Drug
		DRUG_READ,
		DRUG_LIST,
		// Drug sub-resources
		DRUG_SUBSTANCE_READ,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_READ,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_READ,
		DRUG_INDICATION_LIST,
		DRUG_REACTION_ASSESSMENT_READ,
		DRUG_REACTION_ASSESSMENT_LIST,
		RELATEDNESS_ASSESSMENT_READ,
		RELATEDNESS_ASSESSMENT_LIST,
		DRUG_RECURRENCE_READ,
		DRUG_RECURRENCE_LIST,
		// Reaction
		REACTION_READ,
		REACTION_LIST,
		// Test Result
		TEST_RESULT_READ,
		TEST_RESULT_LIST,
		// Narrative
		NARRATIVE_READ,
		NARRATIVE_LIST,
		// Narrative sub-resources
		SENDER_DIAGNOSIS_READ,
		SENDER_DIAGNOSIS_LIST,
		CASE_SUMMARY_READ,
		CASE_SUMMARY_LIST,
		// MessageHeader
		MESSAGE_HEADER_READ,
		MESSAGE_HEADER_LIST,
		// SafetyReport
		SAFETY_REPORT_READ,
		SAFETY_REPORT_LIST,
		// SafetyReport sub-resources
		SENDER_INFORMATION_READ,
		SENDER_INFORMATION_LIST,
		PRIMARY_SOURCE_READ,
		PRIMARY_SOURCE_LIST,
		LITERATURE_REFERENCE_READ,
		LITERATURE_REFERENCE_LIST,
		STUDY_INFORMATION_READ,
		STUDY_INFORMATION_LIST,
		STUDY_REGISTRATION_READ,
		STUDY_REGISTRATION_LIST,
		// Patient sub-resources
		MEDICAL_HISTORY_READ,
		MEDICAL_HISTORY_LIST,
		PAST_DRUG_READ,
		PAST_DRUG_LIST,
		PATIENT_DEATH_READ,
		PATIENT_DEATH_LIST,
		DEATH_CAUSE_READ,
		DEATH_CAUSE_LIST,
		PARENT_INFORMATION_READ,
		PARENT_INFORMATION_LIST,
		PARENT_MEDICAL_HISTORY_READ,
		PARENT_MEDICAL_HISTORY_LIST,
		PARENT_PAST_DRUG_READ,
		PARENT_PAST_DRUG_LIST,
		// Case identifiers and receiver
		CASE_IDENTIFIER_READ,
		CASE_IDENTIFIER_LIST,
		RECEIVER_READ,
		RECEIVER_LIST,
		// Presave templates
		PRESAVE_TEMPLATE_READ,
		PRESAVE_TEMPLATE_LIST,
		// User - read only
		USER_READ,
		USER_LIST,
		// Organization - read own
		ORG_READ,
		// XML - export only (viewing)
		XML_EXPORT,
	]
}

// endregion: --- Role Permission Mappings

fn dynamic_roles() -> &'static RwLock<HashMap<String, Vec<Permission>>> {
	static DYNAMIC_ROLES: OnceLock<RwLock<HashMap<String, Vec<Permission>>>> =
		OnceLock::new();
	DYNAMIC_ROLES.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminMenuPrivilege {
	pub menu_key: String,
	pub can_read: bool,
	pub can_edit: bool,
	pub can_review: bool,
	pub can_lock: bool,
}

fn push_unique(target: &mut Vec<Permission>, source: &[Permission]) {
	for permission in source {
		if !target.contains(permission) {
			target.push(*permission);
		}
	}
}

fn permissions_for_menu_key(
	menu_key: &str,
	can_read: bool,
	can_edit: bool,
	can_review: bool,
	can_lock: bool,
) -> Vec<Permission> {
	let mut permissions = Vec::new();
	match menu_key {
		"home_workflow" => {
			if can_read {
				push_unique(&mut permissions, &[CASE_READ, CASE_LIST]);
			}
		}
		"home_notice" => {
			if can_read {
				push_unique(&mut permissions, &[DASHBOARD_NOTICE_READ]);
			}
			if can_edit || can_review || can_lock {
				push_unique(
					&mut permissions,
					&[DASHBOARD_NOTICE_READ, DASHBOARD_NOTICE_UPDATE],
				);
			}
		}
		// Home e-mail notifications: the UI exposes a single "Send" checkbox
		// bound to can_edit. Feature is pending; the permission is reserved so
		// the checkbox persists and grants correctly once e-mail ships.
		"home_email" => {
			if can_edit || can_review || can_lock {
				push_unique(&mut permissions, &[EMAIL_NOTIFICATION_SEND]);
			}
		}
		"case" => {
			if can_read {
				push_unique(&mut permissions, viewer_permissions());
			}
			if can_edit {
				push_unique(&mut permissions, profile_edit_permissions());
			}
			if can_review || can_lock {
				push_unique(&mut permissions, &[CASE_APPROVE, CASE_UPDATE]);
			}
		}
		"info" => {
			if can_read {
				push_unique(
					&mut permissions,
					&[
						PRESAVE_TEMPLATE_READ,
						PRESAVE_TEMPLATE_LIST,
						SENDER_INFORMATION_READ,
						SENDER_INFORMATION_LIST,
						RECEIVER_READ,
						RECEIVER_LIST,
						STUDY_INFORMATION_READ,
						STUDY_INFORMATION_LIST,
						NARRATIVE_READ,
						NARRATIVE_LIST,
					],
				);
			}
			if can_edit {
				push_unique(
					&mut permissions,
					&[
						PRESAVE_TEMPLATE_CREATE,
						PRESAVE_TEMPLATE_UPDATE,
						PRESAVE_TEMPLATE_DELETE,
						SENDER_INFORMATION_CREATE,
						SENDER_INFORMATION_UPDATE,
						SENDER_INFORMATION_DELETE,
						RECEIVER_CREATE,
						RECEIVER_UPDATE,
						RECEIVER_DELETE,
						STUDY_INFORMATION_CREATE,
						STUDY_INFORMATION_UPDATE,
						STUDY_INFORMATION_DELETE,
						NARRATIVE_CREATE,
						NARRATIVE_UPDATE,
						NARRATIVE_DELETE,
					],
				);
			}
		}
		"import" => {
			if can_read {
				push_unique(&mut permissions, &[XML_IMPORT_READ]);
			}
			if can_edit {
				push_unique(&mut permissions, &[XML_IMPORT]);
			}
		}
		"export_submission" | "submission" | "export" => {
			if can_read {
				push_unique(&mut permissions, &[XML_EXPORT_READ]);
			}
			if can_edit {
				push_unique(&mut permissions, &[XML_EXPORT]);
			}
		}
		"user" | "users" => {
			if can_read {
				push_unique(&mut permissions, &[USER_READ, USER_LIST]);
			}
			if can_edit || can_review || can_lock {
				push_unique(
					&mut permissions,
					&[USER_CREATE, USER_UPDATE, USER_DELETE],
				);
			}
		}
		// Organization management is system-admin only (org endpoints use
		// require_system_admin), so it is intentionally NOT a profile-matrix
		// privilege. No arm here means the org menu key grants nothing.
		"audit" => {
			if can_read || can_review {
				push_unique(&mut permissions, &[AUDIT_READ, AUDIT_LIST]);
			}
		}
		"data" | "terminology" => {
			if can_read {
				push_unique(&mut permissions, &[TERMINOLOGY_READ]);
			}
			if can_edit || can_review {
				push_unique(
					&mut permissions,
					&[TERMINOLOGY_IMPORT, TERMINOLOGY_APPROVE],
				);
			}
		}
		"admin" => {
			if can_read || can_edit || can_review || can_lock {
				push_unique(&mut permissions, admin_permissions());
			}
		}
		"settings" | "roles" => {
			if menu_key == "settings" {
				if can_read {
					push_unique(&mut permissions, &[SETTINGS_READ]);
				}
				if can_edit || can_review || can_lock {
					push_unique(&mut permissions, &[SETTINGS_READ, SETTINGS_UPDATE]);
				}
			} else if can_edit || can_review || can_lock {
				push_unique(&mut permissions, admin_permissions());
			}
		}
		_ => {}
	}
	permissions
}

pub fn permissions_for_menu_privileges(
	privileges: &[AdminMenuPrivilege],
) -> Vec<Permission> {
	let mut permissions = Vec::new();
	for privilege in privileges {
		let menu_permissions = permissions_for_menu_key(
			privilege.menu_key.trim(),
			privilege.can_read,
			privilege.can_edit,
			privilege.can_review,
			privilege.can_lock,
		);
		push_unique(&mut permissions, &menu_permissions);
	}
	permissions
}

pub fn replace_dynamic_roles(map: HashMap<String, Vec<Permission>>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		*guard = map;
	}
}

pub fn upsert_dynamic_role_permissions(role: &str, permissions: Vec<Permission>) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.insert(role.trim().to_ascii_lowercase(), permissions);
	}
}

pub fn remove_dynamic_role(role: &str) {
	if let Ok(mut guard) = dynamic_roles().write() {
		guard.remove(&role.trim().to_ascii_lowercase());
	}
}

// region:    --- Permission Checking Functions

/// Returns the permissions for a given role
pub fn role_permissions(role: &str) -> &'static [Permission] {
	let normalized = canonical_role(role);
	match normalized.as_str() {
		ROLE_SYSTEM_ADMIN => system_admin_permissions(),
		ROLE_SPONSOR_ADMIN_CRO => admin_permissions(),
		ROLE_SPONSOR_ADMIN_COMPANY => admin_permissions(),
		ROLE_USER => &[],
		_ => &[], // Unknown role has no permissions
	}
}

/// Checks if a role has a specific permission
pub fn has_permission(role: &str, permission: Permission) -> bool {
	let normalized = canonical_role(role);
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(perms) = guard.get(&normalized) {
			return perms.contains(&permission);
		}
	}
	role_permissions(&normalized).contains(&permission)
}

/// Checks if a role has any of the given permissions
pub fn has_any_permission(role: &str, permissions: &[Permission]) -> bool {
	let normalized = canonical_role(role);
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(role_perms) = guard.get(&normalized) {
			return permissions.iter().any(|p| role_perms.contains(p));
		}
	}
	let role_perms = role_permissions(&normalized);
	permissions.iter().any(|p| role_perms.contains(p))
}

/// Checks if a role has all of the given permissions
pub fn has_all_permissions(role: &str, permissions: &[Permission]) -> bool {
	let normalized = canonical_role(role);
	if let Ok(guard) = dynamic_roles().read() {
		if let Some(role_perms) = guard.get(&normalized) {
			return permissions.iter().all(|p| role_perms.contains(p));
		}
	}
	let role_perms = role_permissions(&normalized);
	permissions.iter().all(|p| role_perms.contains(p))
}

// endregion: --- Permission Checking Functions

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ctx::ROLE_SYSTEM_ADMIN;

	fn snapshot_bytes(hash: &mut u64, bytes: &[u8]) {
		for byte in bytes {
			*hash ^= u64::from(*byte);
			*hash = hash.wrapping_mul(0x100000001b3);
		}
		*hash ^= 0xff;
		*hash = hash.wrapping_mul(0x100000001b3);
	}

	#[test]
	fn rbac_builtin_and_menu_policy_snapshot() {
		let mut hash = 0xcbf29ce484222325;
		for role in [
			ROLE_SYSTEM_ADMIN,
			ROLE_SPONSOR_ADMIN_CRO,
			ROLE_SPONSOR_ADMIN_COMPANY,
			ROLE_USER,
			"viewer",
			"unknown",
		] {
			snapshot_bytes(&mut hash, role.as_bytes());
			for permission in role_permissions(role) {
				snapshot_bytes(&mut hash, permission.to_string().as_bytes());
			}
		}

		for menu_key in [
			"home_workflow",
			"home_notice",
			"home_email",
			"case",
			"info",
			"import",
			"export_submission",
			"submission",
			"export",
			"user",
			"users",
			"audit",
			"data",
			"terminology",
			"admin",
			"settings",
			"roles",
			"organization",
			"organizations",
			"unknown",
		] {
			for flags in [
				[true, false, false, false],
				[false, true, false, false],
				[false, false, true, false],
				[false, false, false, true],
				[true, true, true, true],
			] {
				snapshot_bytes(&mut hash, menu_key.as_bytes());
				for flag in flags {
					snapshot_bytes(&mut hash, &[u8::from(flag)]);
				}
				let permissions =
					permissions_for_menu_privileges(&[AdminMenuPrivilege {
						menu_key: menu_key.to_string(),
						can_read: flags[0],
						can_edit: flags[1],
						can_review: flags[2],
						can_lock: flags[3],
					}]);
				for permission in permissions {
					snapshot_bytes(&mut hash, permission.to_string().as_bytes());
				}
			}
		}

		assert_eq!(hash, 15683745391403549514);
	}

	#[test]
	fn test_system_admin_has_no_operational_permissions() {
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, CASE_CREATE));
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, CASE_DELETE));
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, USER_CREATE));
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, USER_DELETE));
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, ORG_CREATE));
		assert!(!has_permission(ROLE_SYSTEM_ADMIN, AUDIT_LIST));
	}

	#[test]
	fn test_sponsor_admin_has_admin_permissions() {
		assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, USER_CREATE));
		assert!(has_permission(ROLE_SPONSOR_ADMIN_COMPANY, USER_DELETE));
		assert!(has_permission(ROLE_SPONSOR_ADMIN_CRO, ORG_LIST));
	}

	#[test]
	fn test_legacy_workflow_role_names_have_no_builtin_permissions() {
		for role in ["manager", "pvm", "head_pv", "pvs", "viewer", "sponsor"] {
			assert!(!has_permission(role, CASE_CREATE), "{role}");
			assert!(!has_permission(role, CASE_READ), "{role}");
			assert!(!has_permission(role, USER_READ), "{role}");
			assert!(!has_permission(role, USER_CREATE), "{role}");
		}
	}

	#[test]
	fn test_user_role_has_no_builtin_operational_permissions() {
		assert!(!has_permission(ROLE_USER, CASE_CREATE));
		assert!(!has_permission(ROLE_USER, CASE_READ));
		assert!(!has_permission(ROLE_USER, CASE_UPDATE));
		assert!(!has_permission(ROLE_USER, CASE_DELETE));
		assert!(!has_permission(ROLE_USER, CASE_APPROVE));
		assert!(!has_permission(ROLE_USER, USER_READ));
		assert!(!has_permission(ROLE_USER, USER_CREATE));
		assert!(!has_permission(ROLE_USER, XML_IMPORT));
	}

	#[test]
	fn test_unknown_role_has_no_permissions() {
		assert!(!has_permission("unknown", CASE_READ));
		assert!(!has_permission("hacker", USER_DELETE));
	}

	#[test]
	fn test_has_any_permission() {
		assert!(!has_any_permission(ROLE_USER, &[CASE_CREATE, CASE_READ]));
		assert!(!has_any_permission("viewer", &[CASE_CREATE, CASE_DELETE]));
	}

	#[test]
	fn test_has_all_permissions() {
		assert!(has_all_permissions(
			ROLE_SPONSOR_ADMIN_CRO,
			&[CASE_CREATE, CASE_DELETE]
		));
		assert!(!has_all_permissions("viewer", &[CASE_READ, CASE_CREATE]));
	}

	#[test]
	fn test_menu_privileges_expand_to_expected_permissions() {
		let permissions = permissions_for_menu_privileges(&[
			AdminMenuPrivilege {
				menu_key: "case".to_string(),
				can_read: true,
				can_edit: true,
				can_review: true,
				can_lock: true,
			},
			AdminMenuPrivilege {
				menu_key: "users".to_string(),
				can_read: true,
				can_edit: true,
				can_review: false,
				can_lock: false,
			},
			AdminMenuPrivilege {
				menu_key: "import".to_string(),
				can_read: true,
				can_edit: false,
				can_review: false,
				can_lock: false,
			},
		]);

		assert!(permissions.contains(&CASE_READ));
		assert!(permissions.contains(&CASE_UPDATE));
		assert!(permissions.contains(&CASE_APPROVE));
		assert!(permissions.contains(&USER_LIST));
		assert!(permissions.contains(&USER_CREATE));
		assert!(permissions.contains(&USER_UPDATE));
		assert!(permissions.contains(&XML_IMPORT_READ));
		assert!(!permissions.contains(&XML_IMPORT));
	}

	#[test]
	fn test_home_workflow_read_expands_to_case_view_permissions() {
		let permissions = permissions_for_menu_privileges(&[AdminMenuPrivilege {
			menu_key: "home_workflow".to_string(),
			can_read: true,
			can_edit: false,
			can_review: false,
			can_lock: false,
		}]);

		assert!(permissions.contains(&CASE_READ));
		assert!(permissions.contains(&CASE_LIST));
		assert!(!permissions.contains(&CASE_UPDATE));
	}

	#[test]
	fn test_home_notice_privileges_expand_to_notice_permissions_only() {
		let read_permissions =
			permissions_for_menu_privileges(&[AdminMenuPrivilege {
				menu_key: "home_notice".to_string(),
				can_read: true,
				can_edit: false,
				can_review: false,
				can_lock: false,
			}]);
		assert!(read_permissions.contains(&DASHBOARD_NOTICE_READ));
		assert!(!read_permissions.contains(&DASHBOARD_NOTICE_UPDATE));
		assert!(!read_permissions.contains(&SETTINGS_UPDATE));

		let edit_permissions =
			permissions_for_menu_privileges(&[AdminMenuPrivilege {
				menu_key: "home_notice".to_string(),
				can_read: true,
				can_edit: true,
				can_review: false,
				can_lock: false,
			}]);
		assert!(edit_permissions.contains(&DASHBOARD_NOTICE_READ));
		assert!(edit_permissions.contains(&DASHBOARD_NOTICE_UPDATE));
		assert!(!edit_permissions.contains(&SETTINGS_UPDATE));
	}

	// Exhaustive matrix: every role-and-privilege menu key expands to exactly the
	// permissions its checkboxes imply, and a read-only check never leaks the
	// edit/write permissions. Guards the "check a permission -> it works as
	// checked" contract for the whole privilege matrix.
	#[test]
	fn test_menu_key_matrix_grants_and_isolates_all_keys() {
		fn expand(
			menu_key: &str,
			can_read: bool,
			can_edit: bool,
			can_review: bool,
			can_lock: bool,
		) -> Vec<Permission> {
			permissions_for_menu_privileges(&[AdminMenuPrivilege {
				menu_key: menu_key.to_string(),
				can_read,
				can_edit,
				can_review,
				can_lock,
			}])
		}

		// home_workflow: read grants case view only, never write.
		let p = expand("home_workflow", true, false, false, false);
		assert!(p.contains(&CASE_READ) && p.contains(&CASE_LIST));
		assert!(!p.contains(&CASE_CREATE) && !p.contains(&CASE_UPDATE));

		// home_email: single "Send" checkbox bound to can_edit (feature pending).
		assert!(expand("home_email", false, true, false, false)
			.contains(&EMAIL_NOTIFICATION_SEND));
		assert!(!expand("home_email", true, false, false, false)
			.contains(&EMAIL_NOTIFICATION_SEND));

		// case: read = viewer, edit = write, review/lock = approve.
		let read = expand("case", true, false, false, false);
		assert!(read.contains(&CASE_READ));
		assert!(!read.contains(&CASE_CREATE));
		let edit = expand("case", true, true, false, false);
		assert!(edit.contains(&CASE_CREATE) && edit.contains(&CASE_UPDATE));
		assert!(expand("case", false, false, true, false).contains(&CASE_APPROVE));

		// info: read vs edit on presave/section templates.
		let read = expand("info", true, false, false, false);
		assert!(read.contains(&PRESAVE_TEMPLATE_READ));
		assert!(!read.contains(&PRESAVE_TEMPLATE_CREATE));
		assert!(expand("info", true, true, false, false)
			.contains(&PRESAVE_TEMPLATE_CREATE));

		// import: read history vs edit files.
		assert!(
			expand("import", true, false, false, false).contains(&XML_IMPORT_READ)
		);
		assert!(!expand("import", true, false, false, false).contains(&XML_IMPORT));
		assert!(expand("import", false, true, false, false).contains(&XML_IMPORT));

		// export/submission: read vs export.
		assert!(
			expand("export", true, false, false, false).contains(&XML_EXPORT_READ)
		);
		assert!(!expand("export", true, false, false, false).contains(&XML_EXPORT));
		assert!(expand("export", false, true, false, false).contains(&XML_EXPORT));

		// users: read list vs manage.
		let read = expand("users", true, false, false, false);
		assert!(read.contains(&USER_READ) && read.contains(&USER_LIST));
		assert!(!read.contains(&USER_CREATE));
		let edit = expand("users", false, true, false, false);
		assert!(
			edit.contains(&USER_CREATE)
				&& edit.contains(&USER_UPDATE)
				&& edit.contains(&USER_DELETE)
		);

		// organizations is NOT a profile-matrix privilege (org management is
		// system-admin only), so the menu key grants nothing.
		assert!(expand("organizations", true, true, true, true).is_empty());
		assert!(expand("organization", true, true, true, true).is_empty());

		// audit: granted on read OR review; edit-only grants nothing.
		assert!(expand("audit", true, false, false, false).contains(&AUDIT_READ));
		assert!(expand("audit", false, false, true, false).contains(&AUDIT_LIST));
		assert!(!expand("audit", false, true, false, false).contains(&AUDIT_READ));

		// data/terminology: read vs import/approve.
		let read = expand("data", true, false, false, false);
		assert!(read.contains(&TERMINOLOGY_READ));
		assert!(!read.contains(&TERMINOLOGY_IMPORT));
		let edit = expand("data", false, true, false, false);
		assert!(
			edit.contains(&TERMINOLOGY_IMPORT)
				&& edit.contains(&TERMINOLOGY_APPROVE)
		);

		// admin: any check grants the full admin permission set.
		let admin = expand("admin", true, false, false, false);
		assert!(admin.contains(&SETTINGS_UPDATE) && admin.contains(&USER_CREATE));

		// roles: only edit/review/lock grant admin; read alone grants nothing.
		assert!(!expand("roles", true, false, false, false).contains(&USER_CREATE));
		assert!(expand("roles", false, true, false, false).contains(&USER_CREATE));

		// Unknown menu keys expand to nothing.
		assert!(expand("does_not_exist", true, true, true, true).is_empty());
	}

	#[test]
	fn test_settings_read_does_not_expand_to_admin_permissions() {
		let read_permissions =
			permissions_for_menu_privileges(&[AdminMenuPrivilege {
				menu_key: "settings".to_string(),
				can_read: true,
				can_edit: false,
				can_review: false,
				can_lock: false,
			}]);
		assert!(read_permissions.contains(&SETTINGS_READ));
		assert!(!read_permissions.contains(&SETTINGS_UPDATE));
		assert!(!read_permissions.contains(&CASE_CREATE));
		assert!(!read_permissions.contains(&USER_CREATE));

		let edit_permissions =
			permissions_for_menu_privileges(&[AdminMenuPrivilege {
				menu_key: "settings".to_string(),
				can_read: true,
				can_edit: true,
				can_review: false,
				can_lock: false,
			}]);
		assert!(edit_permissions.contains(&SETTINGS_READ));
		assert!(edit_permissions.contains(&SETTINGS_UPDATE));
		assert!(!edit_permissions.contains(&CASE_CREATE));
		assert!(!edit_permissions.contains(&USER_CREATE));
	}
}

// endregion: --- Tests
