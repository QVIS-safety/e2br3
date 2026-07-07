//! Shared, declarative rule-table helpers for case-section validators.
//!
//! These collapse the repetitive hand-coded `if ... { push_issue_by_code(...) }`
//! blocks into data. Each section owns its rule *tables* (the data); the item
//! types and evaluators here are generic and shared across sections. Behavior is
//! intentionally identical to the hand-coded form — these are structural
//! refactors, not rule changes.
//!
//! Two rule shapes are provided:
//! - [`ValueRule`] / [`IndexedRule`]: "this field must be non-empty (or carry an
//!   allowed nullFlavor)", evaluated through the catalog engine
//!   (`push_issue_if_rule_invalid` + `ValuePolicy`).
//! - [`CompanionRule`]: "if X is present, its companion Y is required".

use crate::{
	push_issue_by_code, push_issue_if_rule_invalid, RuleFacts, ValidationIssue,
};
use std::borrow::Cow;

/// A value pulled from a model plus its optional nullFlavor. `Cow` lets string
/// fields borrow directly while computed values (e.g. a date `to_string()`)
/// carry an owned string — sidestepping the temporary-`&str` lifetime problem.
pub(crate) enum RuleValue<'a> {
	Text {
		value: Option<Cow<'a, str>>,
		null_flavor: Option<&'a str>,
	},
}

impl<'a> RuleValue<'a> {
	pub(crate) fn borrowed(
		value: Option<&'a str>,
		null_flavor: Option<&'a str>,
	) -> Self {
		RuleValue::Text {
			value: value.map(Cow::Borrowed),
			null_flavor,
		}
	}

	pub(crate) fn owned(
		value: Option<String>,
		null_flavor: Option<&'a str>,
	) -> Self {
		RuleValue::Text {
			value: value.map(Cow::Owned),
			null_flavor,
		}
	}
}

/// Facts supplier that always yields the default (unconditional) [`RuleFacts`].
pub(crate) fn no_facts<T>(_: &T) -> RuleFacts {
	RuleFacts::default()
}

/// Declarative presence/value rule for a single object: a catalog code, its
/// issue path, and how to extract the value. Evaluation reuses the catalog
/// engine, so tables change *structure*, not behavior.
pub(crate) struct ValueRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
}

/// Evaluates every value rule against a single item.
pub(crate) fn eval_value<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[ValueRule<T>],
) {
	for rule in rules {
		let RuleValue::Text { value, null_flavor } = (rule.value)(item);
		let _ = push_issue_if_rule_invalid(
			issues,
			rule.code,
			rule.path,
			value.as_deref(),
			null_flavor,
			RuleFacts::default(),
		);
	}
}

/// Same idea as [`ValueRule`] for a *repeated* field. `path` receives the item
/// index to build the `collection.{idx}.field` path; `facts` supplies the
/// per-item [`RuleFacts`] that gate conditional rules (e.g. study-only rules).
pub(crate) struct IndexedRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
	pub facts: fn(&T) -> RuleFacts,
}

/// Evaluates every rule against every item, tagging issues with the item index.
/// `push_issue_if_rule_invalid` itself skips rules whose condition (per the
/// supplied facts) is not satisfied, so conditional rules are handled uniformly.
pub(crate) fn eval_indexed<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			let RuleValue::Text { value, null_flavor } = (rule.value)(item);
			let _ = push_issue_if_rule_invalid(
				issues,
				rule.code,
				(rule.path)(idx),
				value.as_deref(),
				null_flavor,
				(rule.facts)(item),
			);
		}
	}
}

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
