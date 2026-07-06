//! Shared, declarative rule-table helpers for case-section validators.
//!
//! These collapse the repetitive hand-coded `if ... { push_issue_by_code(...) }`
//! blocks into data. Each section owns its rule tables; the evaluators here are
//! generic over the item type. Behavior is intentionally identical to the
//! hand-coded form — these are structural refactors, not rule changes.

use crate::validation::{push_issue_by_code, ValidationIssue};

/// The "if X is present, its companion Y is required" pattern, evaluated per
/// item of a repeated field. Emits `code` at `path(idx)` when `trigger` holds
/// but `required` does not (e.g. MedDRA code present but version missing).
///
/// Bidirectional pairs (X⇒Y and Y⇒X) are simply two `CompanionRule`s.
pub(crate) struct CompanionRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub trigger: fn(&T) -> bool,
	pub required: fn(&T) -> bool,
}

/// Evaluates every companion rule against every item, tagging issues with the
/// item index.
pub(crate) fn eval_companions<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[CompanionRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if (rule.trigger)(item) && !(rule.required)(item) {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}
