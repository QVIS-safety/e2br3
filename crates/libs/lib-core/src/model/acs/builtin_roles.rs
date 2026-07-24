use super::*;
use std::sync::OnceLock;

macro_rules! append_permission_selection {
	($target:ident, $group:ident, all) => {
		$target.extend_from_slice($group);
	};
	($target:ident, $group:ident, [$($action:ident),+ $(,)?]) => {
		$target.extend(
			$group
				.iter()
				.copied()
				.filter(|permission| matches!(permission.action(), $(Action::$action)|+)),
		);
	};
}

macro_rules! permission_set {
	(
		$cell:ident,
		$function:ident,
		$( $group:ident => $selection:tt ),+ $(,)?
	) => {
		static $cell: OnceLock<Vec<Permission>> = OnceLock::new();

		pub(crate) fn $function() -> &'static [Permission] {
			$cell.get_or_init(|| {
				let mut permissions = Vec::new();
				$(append_permission_selection!(permissions, $group, $selection);)+
				permissions
			})
		}
	};
}

permission_set! {
	ADMIN_PERMISSIONS,
	admin_permissions,
	CASE_PERMISSIONS => all,
	PATIENT_PERMISSIONS => all,
	PATIENT_IDENTIFIER_PERMISSIONS => all,
	DRUG_PERMISSIONS => all,
	DRUG_SUBSTANCE_PERMISSIONS => all,
	DRUG_DOSAGE_PERMISSIONS => all,
	DRUG_INDICATION_PERMISSIONS => all,
	DRUG_DEVICE_CHARACTERISTIC_PERMISSIONS => all,
	DRUG_REACTION_ASSESSMENT_PERMISSIONS => all,
	RELATEDNESS_ASSESSMENT_PERMISSIONS => all,
	DRUG_RECURRENCE_PERMISSIONS => all,
	REACTION_PERMISSIONS => all,
	TEST_RESULT_PERMISSIONS => all,
	NARRATIVE_PERMISSIONS => all,
	SENDER_DIAGNOSIS_PERMISSIONS => all,
	CASE_SUMMARY_PERMISSIONS => all,
	MESSAGE_HEADER_PERMISSIONS => all,
	SAFETY_REPORT_PERMISSIONS => all,
	SENDER_INFORMATION_PERMISSIONS => all,
	PRIMARY_SOURCE_PERMISSIONS => all,
	LITERATURE_REFERENCE_PERMISSIONS => all,
	STUDY_INFORMATION_PERMISSIONS => all,
	STUDY_REGISTRATION_PERMISSIONS => all,
	MEDICAL_HISTORY_PERMISSIONS => all,
	PAST_DRUG_PERMISSIONS => all,
	PATIENT_DEATH_PERMISSIONS => all,
	DEATH_CAUSE_PERMISSIONS => all,
	PARENT_INFORMATION_PERMISSIONS => all,
	PARENT_MEDICAL_HISTORY_PERMISSIONS => all,
	PARENT_PAST_DRUG_PERMISSIONS => all,
	CASE_IDENTIFIER_PERMISSIONS => all,
	RECEIVER_PERMISSIONS => all,
	PRESAVE_TEMPLATE_PERMISSIONS => all,
	USER_PERMISSIONS => all,
	ORGANIZATION_PERMISSIONS => all,
	AUDIT_LOG_PERMISSIONS => all,
	SETTINGS_PERMISSIONS => all,
	DASHBOARD_NOTICE_PERMISSIONS => all,
	TERMINOLOGY_PERMISSIONS => all,
	XML_EXPORT_PERMISSIONS => all,
	XML_IMPORT_PERMISSIONS => all,
	EMAIL_NOTIFICATION_PERMISSIONS => all,
}

permission_set! {
	SYSTEM_ADMIN_PERMISSIONS,
	system_admin_permissions,
	USER_PERMISSIONS => all,
	ORGANIZATION_PERMISSIONS => all,
	AUDIT_LOG_PERMISSIONS => all,
	SETTINGS_PERMISSIONS => all,
	DASHBOARD_NOTICE_PERMISSIONS => all,
}

permission_set! {
	PROFILE_EDIT_PERMISSIONS,
	profile_edit_permissions,
	CASE_PERMISSIONS => [Create, Read, Update, List],
	PATIENT_PERMISSIONS => [Create, Read, Update, List],
	PATIENT_IDENTIFIER_PERMISSIONS => [Create, Read, Update, List],
	DRUG_PERMISSIONS => [Create, Read, Update, List],
	DRUG_SUBSTANCE_PERMISSIONS => [Create, Read, Update, List],
	DRUG_DOSAGE_PERMISSIONS => [Create, Read, Update, List],
	DRUG_INDICATION_PERMISSIONS => [Create, Read, Update, List],
	DRUG_DEVICE_CHARACTERISTIC_PERMISSIONS => [Create, Read, Update, List],
	DRUG_REACTION_ASSESSMENT_PERMISSIONS => [Create, Read, Update, List],
	RELATEDNESS_ASSESSMENT_PERMISSIONS => [Create, Read, Update, List],
	DRUG_RECURRENCE_PERMISSIONS => [Create, Read, Update, List],
	REACTION_PERMISSIONS => [Create, Read, Update, List],
	TEST_RESULT_PERMISSIONS => [Create, Read, Update, List],
	NARRATIVE_PERMISSIONS => [Create, Read, Update, List],
	SENDER_DIAGNOSIS_PERMISSIONS => [Create, Read, Update, List],
	CASE_SUMMARY_PERMISSIONS => [Create, Read, Update, List],
	MESSAGE_HEADER_PERMISSIONS => [Create, Read, Update, List],
	SAFETY_REPORT_PERMISSIONS => [Create, Read, Update, List],
	SENDER_INFORMATION_PERMISSIONS => [Create, Read, Update, List],
	PRIMARY_SOURCE_PERMISSIONS => [Create, Read, Update, List],
	LITERATURE_REFERENCE_PERMISSIONS => [Create, Read, Update, List],
	STUDY_INFORMATION_PERMISSIONS => [Create, Read, Update, List],
	STUDY_REGISTRATION_PERMISSIONS => [Create, Read, Update, List],
	MEDICAL_HISTORY_PERMISSIONS => [Create, Read, Update, List],
	PAST_DRUG_PERMISSIONS => [Create, Read, Update, List],
	PATIENT_DEATH_PERMISSIONS => [Create, Read, Update, List],
	DEATH_CAUSE_PERMISSIONS => [Create, Read, Update, List],
	PARENT_INFORMATION_PERMISSIONS => [Create, Read, Update, List],
	PARENT_MEDICAL_HISTORY_PERMISSIONS => [Create, Read, Update, List],
	PARENT_PAST_DRUG_PERMISSIONS => [Create, Read, Update, List],
	CASE_IDENTIFIER_PERMISSIONS => [Create, Read, Update, List],
	RECEIVER_PERMISSIONS => [Create, Read, Update, List],
	PRESAVE_TEMPLATE_PERMISSIONS => all,
	USER_PERMISSIONS => [Read],
	ORGANIZATION_PERMISSIONS => [Read],
	TERMINOLOGY_PERMISSIONS => [Read],
}

permission_set! {
	CASE_VIEW_PERMISSIONS,
	case_view_permissions,
	CASE_PERMISSIONS => [Read, List],
	PATIENT_PERMISSIONS => [Read, List],
	PATIENT_IDENTIFIER_PERMISSIONS => [Read, List],
	DRUG_PERMISSIONS => [Read, List],
	DRUG_SUBSTANCE_PERMISSIONS => [Read, List],
	DRUG_DOSAGE_PERMISSIONS => [Read, List],
	DRUG_INDICATION_PERMISSIONS => [Read, List],
	DRUG_DEVICE_CHARACTERISTIC_PERMISSIONS => [Read, List],
	DRUG_REACTION_ASSESSMENT_PERMISSIONS => [Read, List],
	RELATEDNESS_ASSESSMENT_PERMISSIONS => [Read, List],
	DRUG_RECURRENCE_PERMISSIONS => [Read, List],
	REACTION_PERMISSIONS => [Read, List],
	TEST_RESULT_PERMISSIONS => [Read, List],
	NARRATIVE_PERMISSIONS => [Read, List],
	SENDER_DIAGNOSIS_PERMISSIONS => [Read, List],
	CASE_SUMMARY_PERMISSIONS => [Read, List],
	MESSAGE_HEADER_PERMISSIONS => [Read, List],
	SAFETY_REPORT_PERMISSIONS => [Read, List],
	SENDER_INFORMATION_PERMISSIONS => [Read, List],
	PRIMARY_SOURCE_PERMISSIONS => [Read, List],
	LITERATURE_REFERENCE_PERMISSIONS => [Read, List],
	STUDY_INFORMATION_PERMISSIONS => [Read, List],
	STUDY_REGISTRATION_PERMISSIONS => [Read, List],
	MEDICAL_HISTORY_PERMISSIONS => [Read, List],
	PAST_DRUG_PERMISSIONS => [Read, List],
	PATIENT_DEATH_PERMISSIONS => [Read, List],
	DEATH_CAUSE_PERMISSIONS => [Read, List],
	PARENT_INFORMATION_PERMISSIONS => [Read, List],
	PARENT_MEDICAL_HISTORY_PERMISSIONS => [Read, List],
	PARENT_PAST_DRUG_PERMISSIONS => [Read, List],
	CASE_IDENTIFIER_PERMISSIONS => [Read, List],
	RECEIVER_PERMISSIONS => [Read, List],
	PRESAVE_TEMPLATE_PERMISSIONS => [Read, List],
	ORGANIZATION_PERMISSIONS => [Read],
}

pub fn role_permissions(role: &str) -> &'static [Permission] {
	let normalized = crate::ctx::canonical_role(role);
	match normalized.as_str() {
		crate::ctx::ROLE_SYSTEM_ADMIN => system_admin_permissions(),
		crate::ctx::ROLE_SPONSOR_ADMIN_CRO
		| crate::ctx::ROLE_SPONSOR_ADMIN_COMPANY => admin_permissions(),
		crate::ctx::ROLE_USER => &[],
		_ => &[],
	}
}
