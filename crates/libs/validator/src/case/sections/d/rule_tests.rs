use super::golden_companion_tests::{
	autopsy_cause, death_info, empty_ctx, medhist, parent, parent_medhist,
	parent_past_drug, past_drug, patient, patient_identifier, reported_cause,
};
use super::*;
use lib_core::model::case_identifiers::LinkedReportNumber;
use lib_core::model::message_header::MessageHeader;
use lib_core::model::reaction::Reaction;
use lib_core::model::safety_report::{SafetyReportIdentification, StudyInformation};
use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;
use time::Month;

include!("rule_fixtures/common.rs");

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	let legacy_version_code = matches!(
		code,
		"ICH.D.7.1.r.1a.VOCABULARY"
			| "ICH.D.8.r.6a.VOCABULARY"
			| "ICH.D.8.r.7a.VOCABULARY"
			| "ICH.D.9.2.r.1a.VOCABULARY"
			| "ICH.D.9.4.r.1a.VOCABULARY"
			| "ICH.D.10.7.1.r.1a.VOCABULARY"
			| "ICH.D.10.8.r.6a.VOCABULARY"
			| "ICH.D.10.8.r.7a.VOCABULARY"
	);
	let emitted_code = if legacy_version_code {
		"ICH.MEDDRA.VERSION.UNAVAILABLE"
	} else {
		code
	};
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == emitted_code && issue.path == path),
		"{emitted_code} missing at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == emitted_code),
		"{emitted_code} remained after repair: {repaired:?}"
	);
	if legacy_version_code {
		assert!(!invalid.iter().any(|issue| issue.code == code));
		assert!(!repaired.iter().any(|issue| issue.code == code));
	}
}

#[path = "rule_tests/fda.rs"]
mod fda;
#[path = "rule_tests/ich.rs"]
mod ich;
#[path = "rule_tests/mfds.rs"]
mod mfds;
