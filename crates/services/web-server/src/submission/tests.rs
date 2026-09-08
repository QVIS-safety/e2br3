use super::ack::{merge_submission_status, status_from_ack};
use super::SubmissionStatus;

#[test]
fn submission_rejects_every_validation_issue() {
	let mut report = validator::CaseValidationReport {
		authority: "fda".into(),
		case_id: uuid::Uuid::nil(),
		ok: false,
		issue_count: 1,
		section_summaries: vec![],
		subsection_summaries: vec![],
		issues: vec![validator::ValidationIssue {
			code: "FDA.W0001".into(),
			message:
				"A linked report number should be provided for an aggregate report."
					.into(),
			path: "linkedReports.0.linkedReportNumber".into(),
			field_path: None,
			section: "C".into(),
			subsection: "C.1".into(),
		}],
	};
	for code in [
		"FDA.W0001",
		"FDA.W0002",
		"FDA.W0003",
		"FDA.W0004",
		"FDA.W0006",
		"FDA.W0007",
	] {
		report.issues[0].code = code.into();
		assert!(
			super::check_submission_validation(&report)
				.unwrap_err()
				.to_string()
				.contains("1 validation issue(s)"),
			"{code}"
		);
	}
	report.issues.push(validator::ValidationIssue {
		code: "ICH.MEDDRA.VERSION.UNAVAILABLE".into(),
		message: "MedDRA 12.0 is not loaded".into(),
		path: "reactions.0.reactionMeddraVersion".into(),
		field_path: None,
		section: "E".into(),
		subsection: "E.i".into(),
	});
	report.issue_count = report.issues.len();
	let error = super::check_submission_validation(&report)
		.unwrap_err()
		.to_string();
	assert!(error.contains("2 validation issue(s)"));
	assert!(error.contains("MedDRA 12.0 is not loaded"));
	assert!(error.contains("linked report number"));
}

#[test]
fn ack_status_mapping_success() {
	assert_eq!(
		status_from_ack(1, true).unwrap(),
		SubmissionStatus::Ack1Received
	);
	assert_eq!(
		status_from_ack(2, true).unwrap(),
		SubmissionStatus::Ack2Received
	);
	assert_eq!(
		status_from_ack(3, true).unwrap(),
		SubmissionStatus::Ack3Received
	);
	assert_eq!(
		status_from_ack(4, true).unwrap(),
		SubmissionStatus::Ack4Received
	);
}

#[test]
fn ack_status_mapping_rejected() {
	assert_eq!(
		status_from_ack(2, false).unwrap(),
		SubmissionStatus::Rejected
	);
}

#[test]
fn ack_status_merge_never_regresses() {
	assert_eq!(
		merge_submission_status(
			&SubmissionStatus::Ack3Received,
			&SubmissionStatus::Ack2Received
		),
		SubmissionStatus::Ack3Received
	);
}

#[test]
fn ack_status_merge_respects_terminal() {
	assert_eq!(
		merge_submission_status(
			&SubmissionStatus::Ack4Received,
			&SubmissionStatus::Ack2Received
		),
		SubmissionStatus::Ack4Received
	);
	assert_eq!(
		merge_submission_status(
			&SubmissionStatus::Rejected,
			&SubmissionStatus::Ack4Received
		),
		SubmissionStatus::Rejected
	);
}
