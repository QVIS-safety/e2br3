use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum XmlImportDecisionAction {
	New,
	FollowUp,
	Skip,
	Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlImportIncomingKey {
	pub safety_report_id: Option<String>,
	/// E2B C.1.2 (Date of Creation), preserving the sender's exact timestamp.
	pub transmission_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlImportExistingCase {
	pub case_id: Uuid,
	pub safety_report_id: String,
	pub version: i32,
	/// E2B C.1.2 (Date of Creation), as stored in the case.
	pub transmission_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlImportDuplicateMatch {
	pub case_id: Uuid,
	pub safety_report_id: Option<String>,
	pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlImportDecision {
	pub action: XmlImportDecisionAction,
	pub matched_case_id: Option<Uuid>,
	pub matched_case_number: Option<String>,
	pub matched_case_version: Option<i32>,
	pub message: Option<String>,
}

pub fn decide_xml_import(
	incoming: &XmlImportIncomingKey,
	existing_cases: &[XmlImportExistingCase],
	duplicate_matches: &[XmlImportDuplicateMatch],
) -> XmlImportDecision {
	if let Some(existing) = existing_cases
		.iter()
		.filter(|case| {
			!case.safety_report_id.trim().is_empty()
				&& Some(case.safety_report_id.as_str())
					== incoming.safety_report_id.as_deref()
				&& incoming
					.transmission_date
					.as_deref()
					.is_some_and(|value| !value.trim().is_empty())
				&& case.transmission_date.as_deref()
					== incoming.transmission_date.as_deref()
		})
		.max_by_key(|case| case.version)
	{
		return decision_from_existing(
			incoming,
			existing.case_id,
			&existing.safety_report_id,
			existing.version,
			existing.transmission_date.as_deref(),
		);
	}

	if let Some(existing) = existing_cases
		.iter()
		.filter(|case| {
			!case.safety_report_id.trim().is_empty()
				&& Some(case.safety_report_id.as_str())
					== incoming.safety_report_id.as_deref()
		})
		.max_by_key(|case| case.version)
	{
		return decision_from_existing(
			incoming,
			existing.case_id,
			&existing.safety_report_id,
			existing.version,
			existing.transmission_date.as_deref(),
		);
	}

	if let Some(duplicate) = duplicate_matches.iter().max_by_key(|case| case.version)
	{
		return decision_from_duplicate(
			duplicate.case_id,
			duplicate.safety_report_id.as_deref(),
			duplicate.version,
		);
	}

	XmlImportDecision {
		action: XmlImportDecisionAction::New,
		matched_case_id: None,
		matched_case_number: None,
		matched_case_version: None,
		message: Some(
			"No matching prior case found; import as new case.".to_string(),
		),
	}
}

fn decision_from_duplicate(
	case_id: Uuid,
	case_number: Option<&str>,
	version: i32,
) -> XmlImportDecision {
	XmlImportDecision {
		action: XmlImportDecisionAction::New,
		matched_case_id: Some(case_id),
		matched_case_number: case_number.map(str::to_owned),
		matched_case_version: Some(version),
		message: Some(
			"Potential duplicate matched on patient/event/product fields; imported with warning because the C.1.1/C.1.2 pair is not an exact match."
				.to_string(),
		),
	}
}

fn decision_from_existing(
	incoming: &XmlImportIncomingKey,
	case_id: Uuid,
	case_number: &str,
	version: i32,
	existing_transmission_date: Option<&str>,
) -> XmlImportDecision {
	let action = if incoming
		.transmission_date
		.as_deref()
		.is_some_and(|value| !value.trim().is_empty())
		&& existing_transmission_date == incoming.transmission_date.as_deref()
	{
		XmlImportDecisionAction::Skip
	} else {
		XmlImportDecisionAction::FollowUp
	};
	let message = match action {
		XmlImportDecisionAction::Skip => {
			"Existing case has the same C.1.1 and C.1.2; import skipped."
		}
		XmlImportDecisionAction::FollowUp => {
			"Existing case matched with a different C.1.2; import as follow-up."
		}
		XmlImportDecisionAction::New | XmlImportDecisionAction::Error => {
			"Import decision resolved."
		}
	};
	XmlImportDecision {
		action,
		matched_case_id: Some(case_id),
		matched_case_number: Some(case_number.to_string()),
		matched_case_version: Some(version),
		message: Some(message.to_string()),
	}
}

#[cfg(test)]
mod tests {
	use super::{
		decide_xml_import, XmlImportDecisionAction, XmlImportDuplicateMatch,
		XmlImportExistingCase, XmlImportIncomingKey,
	};
	use uuid::Uuid;

	fn key(report_id: &str, transmission_date: &str) -> XmlImportIncomingKey {
		XmlImportIncomingKey {
			safety_report_id: Some(report_id.to_string()),
			transmission_date: Some(transmission_date.to_string()),
		}
	}

	fn existing(
		case_id: Uuid,
		report_id: &str,
		version: i32,
	) -> XmlImportExistingCase {
		XmlImportExistingCase {
			case_id,
			safety_report_id: report_id.to_string(),
			version,
			transmission_date: Some("20260701000000".to_string()),
		}
	}

	fn duplicate(
		case_id: Uuid,
		report_id: &str,
		version: i32,
	) -> XmlImportDuplicateMatch {
		XmlImportDuplicateMatch {
			case_id,
			safety_report_id: Some(report_id.to_string()),
			version,
		}
	}

	#[test]
	fn same_report_id_and_same_creation_date_skips() {
		let incoming = key("CASE-1", "20260701000000");
		let case_id = Uuid::from_u128(1);
		let existing = existing(case_id, "CASE-1", 2);

		let decision = decide_xml_import(&incoming, &[existing], &[]);

		assert_eq!(decision.action, XmlImportDecisionAction::Skip);
		assert_eq!(decision.matched_case_id, Some(case_id));
		assert_eq!(decision.matched_case_number.as_deref(), Some("CASE-1"));
		assert_eq!(decision.matched_case_version, Some(2));
	}

	#[test]
	fn same_report_id_and_different_creation_date_is_follow_up() {
		let incoming = key("CASE-1", "20260702000000");
		let case_id = Uuid::from_u128(2);
		let existing = existing(case_id, "CASE-1", 2);

		let decision = decide_xml_import(&incoming, &[existing], &[]);

		assert_eq!(decision.action, XmlImportDecisionAction::FollowUp);
		assert_eq!(decision.matched_case_id, Some(case_id));
		assert_eq!(decision.matched_case_version, Some(2));
	}

	#[test]
	fn duplicate_match_with_different_date_imports_with_warning() {
		let incoming = key("CASE-2", "20260702000000");
		let case_id = Uuid::from_u128(3);
		let duplicate = duplicate(case_id, "CASE-1", 1);

		let decision = decide_xml_import(&incoming, &[], &[duplicate]);

		assert_eq!(decision.action, XmlImportDecisionAction::New);
		assert_eq!(decision.matched_case_id, Some(case_id));
		assert_eq!(decision.matched_case_number.as_deref(), Some("CASE-1"));
		assert!(decision.message.as_deref().unwrap().contains("warning"));
	}

	#[test]
	fn duplicate_match_with_same_date_imports_with_warning() {
		let incoming = key("CASE-2", "20260702000000");
		let case_id = Uuid::from_u128(4);
		let duplicate = duplicate(case_id, "CASE-1", 1);

		let decision = decide_xml_import(&incoming, &[], &[duplicate]);

		assert_eq!(decision.action, XmlImportDecisionAction::New);
		assert_eq!(decision.matched_case_id, Some(case_id));
		assert!(decision
			.message
			.as_deref()
			.unwrap()
			.contains("not an exact match"));
	}

	#[test]
	fn no_existing_or_duplicate_match_is_new() {
		let incoming = key("CASE-3", "20260702000000");

		let decision = decide_xml_import(&incoming, &[], &[]);

		assert_eq!(decision.action, XmlImportDecisionAction::New);
		assert_eq!(decision.matched_case_id, None);
	}
}

#[cfg(test)]
mod draft_identity_tests {
	use super::*;

	#[test]
	fn missing_identity_is_not_an_exact_duplicate() {
		let existing = XmlImportExistingCase {
			case_id: Uuid::nil(),
			safety_report_id: "REPORT".into(),
			version: 1,
			transmission_date: None,
		};
		let incoming = XmlImportIncomingKey {
			safety_report_id: None,
			transmission_date: None,
		};
		assert_eq!(
			decide_xml_import(&incoming, std::slice::from_ref(&existing), &[])
				.action,
			XmlImportDecisionAction::New
		);
		let incoming = XmlImportIncomingKey {
			safety_report_id: Some("REPORT".into()),
			transmission_date: None,
		};
		assert_eq!(
			decide_xml_import(&incoming, &[existing], &[]).action,
			XmlImportDecisionAction::FollowUp
		);
	}
	#[test]
	fn empty_identity_and_timestamp_cannot_prove_a_duplicate() {
		for value in ["", "  "] {
			let mut incoming = XmlImportIncomingKey {
				safety_report_id: Some(value.into()),
				transmission_date: Some(value.into()),
			};
			let mut existing = XmlImportExistingCase {
				case_id: Uuid::nil(),
				safety_report_id: value.into(),
				version: 1,
				transmission_date: Some(value.into()),
			};
			assert_eq!(
				decide_xml_import(&incoming, &[existing.clone()], &[]).action,
				XmlImportDecisionAction::New
			);
			incoming.safety_report_id = Some("REPORT".into());
			existing.safety_report_id = "REPORT".into();
			assert_eq!(
				decide_xml_import(&incoming, &[existing], &[]).action,
				XmlImportDecisionAction::FollowUp
			);
		}
	}
}
