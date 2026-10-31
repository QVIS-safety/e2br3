use super::tests::{empty_ctx, reaction};
use super::*;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	let emitted_code = if code == "ICH.E.i.2.1a.VOCABULARY" {
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

fn invoke(
	reaction: &Reaction,
	function: fn(usize, &Reaction, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	function(0, reaction, &mut issues);
	issues
}

fn context_invoke(
	reaction: &Reaction,
	vocabulary: crate::context::VocabularyContext,
	function: fn(usize, &Reaction, &ValidationContext, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut ctx = empty_ctx();
	ctx.vocabulary = vocabulary;
	ctx.reactions.push(reaction.clone());
	let mut issues = Vec::new();
	function(0, reaction, &ctx, &mut issues);
	super::super::collect_meddra_version_issues(&ctx, &mut issues);
	issues
}

#[path = "rule_tests/fda.rs"]
mod fda;
#[path = "rule_tests/ich.rs"]
mod ich;
#[path = "rule_tests/mfds.rs"]
mod mfds;
