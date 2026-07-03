use super::*;

pub(super) fn has_any_permission(
	subject: &str,
	permissions: impl IntoIterator<Item = lib_core::model::acs::Permission>,
) -> bool {
	permissions
		.into_iter()
		.any(|permission| has_permission(subject, permission))
}

pub(super) fn info_capabilities_for_subject(
	subject: &str,
) -> ModuleCrudCapabilities {
	ModuleCrudCapabilities {
		read: has_any_permission(
			subject,
			[
				PRESAVE_TEMPLATE_READ,
				PRESAVE_TEMPLATE_LIST,
				SENDER_INFORMATION_READ,
				SENDER_INFORMATION_LIST,
				RECEIVER_READ,
				RECEIVER_LIST,
				STUDY_INFORMATION_READ,
				STUDY_INFORMATION_LIST,
				STUDY_REGISTRATION_READ,
				STUDY_REGISTRATION_LIST,
				NARRATIVE_READ,
				NARRATIVE_LIST,
			],
		),
		create: has_any_permission(
			subject,
			[
				PRESAVE_TEMPLATE_CREATE,
				SENDER_INFORMATION_CREATE,
				RECEIVER_CREATE,
				STUDY_INFORMATION_CREATE,
				STUDY_REGISTRATION_CREATE,
				NARRATIVE_CREATE,
			],
		),
		update: has_any_permission(
			subject,
			[
				PRESAVE_TEMPLATE_UPDATE,
				SENDER_INFORMATION_UPDATE,
				RECEIVER_UPDATE,
				STUDY_INFORMATION_UPDATE,
				STUDY_REGISTRATION_UPDATE,
				NARRATIVE_UPDATE,
			],
		),
		delete: has_any_permission(
			subject,
			[
				PRESAVE_TEMPLATE_DELETE,
				SENDER_INFORMATION_DELETE,
				RECEIVER_DELETE,
				STUDY_INFORMATION_DELETE,
				STUDY_REGISTRATION_DELETE,
				NARRATIVE_DELETE,
			],
		),
	}
}

pub(super) fn capabilities_for_subject(
	subject: &str,
	is_admin_capable: bool,
	is_system_admin: bool,
) -> UserCapabilities {
	UserCapabilities {
		case: CaseCapabilities {
			read: has_any_permission(subject, [CASE_READ, CASE_LIST]),
			create: has_permission(subject, CASE_CREATE),
			update: has_permission(subject, CASE_UPDATE),
			delete: has_permission(subject, CASE_DELETE),
			review: has_permission(subject, CASE_APPROVE),
			lock: has_permission(subject, CASE_APPROVE),
		},
		info: info_capabilities_for_subject(subject),
		import: ExecuteCapabilities {
			read: has_permission(subject, XML_IMPORT_READ),
			execute: has_permission(subject, XML_IMPORT),
		},
		export_submission: ExecuteCapabilities {
			read: has_permission(subject, XML_EXPORT_READ),
			execute: has_permission(subject, XML_EXPORT),
		},
		data: DataCapabilities {
			read: has_permission(subject, TERMINOLOGY_READ),
			import: has_permission(subject, TERMINOLOGY_IMPORT),
			approve: has_permission(subject, TERMINOLOGY_APPROVE),
		},
		admin: AdminCapabilities {
			read: is_admin_capable,
			update: is_admin_capable,
		},
		users: ModuleCrudCapabilities {
			read: is_system_admin
				|| has_any_permission(subject, [USER_READ, USER_LIST]),
			create: is_system_admin || has_permission(subject, USER_CREATE),
			update: is_system_admin || has_permission(subject, USER_UPDATE),
			delete: is_system_admin || has_permission(subject, USER_DELETE),
		},
		roles: ModuleCrudCapabilities {
			read: is_admin_capable,
			create: is_admin_capable,
			update: is_admin_capable,
			delete: is_admin_capable,
		},
		settings: AdminCapabilities {
			read: is_admin_capable || has_permission(subject, SETTINGS_READ),
			update: is_admin_capable || has_permission(subject, SETTINGS_UPDATE),
		},
		home_notice: HomeNoticeCapabilities {
			read: is_admin_capable || has_permission(subject, DASHBOARD_NOTICE_READ),
			update: is_admin_capable
				|| has_permission(subject, DASHBOARD_NOTICE_UPDATE),
		},
	}
}
