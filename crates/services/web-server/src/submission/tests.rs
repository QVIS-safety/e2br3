use super::ack::{merge_submission_status, status_from_ack};
use super::SubmissionStatus;

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
