use super::tests::{case_summary, empty_ctx, narrative, sender_diagnosis};
use super::*;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	let emitted_code = if code == "ICH.H.3.r.1a.VOCABULARY" {
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
	if emitted_code != code {
		assert!(!invalid.iter().any(|issue| issue.code == code));
		assert!(!repaired.iter().any(|issue| issue.code == code));
	}
}

#[path = "rule_tests/ich.rs"]
mod ich;
#[path = "rule_tests/mfds.rs"]
mod mfds;
