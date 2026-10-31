use super::golden_c1_value_tests as fixture;
use super::*;
use crate::VocabularyScope;
use sqlx::types::Uuid;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == code && issue.path == path),
		"{code} did not emit at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == code),
		"{code} remained after repair: {repaired:?}"
	);
}

#[path = "rule_tests/fda.rs"]
mod fda;
#[path = "rule_tests/ich.rs"]
mod ich;
#[path = "rule_tests/mfds.rs"]
mod mfds;
