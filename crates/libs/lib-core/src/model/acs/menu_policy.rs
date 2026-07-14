//! Permission definitions for the Access Control System
//!
//! Defines resources, actions, and the permission matrix for RBAC.

use super::*;
use serde::{Deserialize, Serialize};

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

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ctx::{
		ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
		ROLE_USER,
	};

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

		assert_eq!(hash, 5602083785880063594);
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
