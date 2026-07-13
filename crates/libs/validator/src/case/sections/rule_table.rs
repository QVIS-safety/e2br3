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

use crate::context::VocabularyContext;
use crate::{
	allowed_value_constraint_for_rule, max_length_for_rule, push_issue_by_code,
	push_issue_if_rule_invalid, vocabulary_for_rule, AllowedValueConstraintKind,
	RuleFacts, ValidationIssue,
};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Decimal;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

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

pub(crate) enum DateValues {
	One(Option<Date>),
	Two(Option<Date>, Option<Date>),
}

impl DateValues {
	fn any_future(self) -> bool {
		match self {
			DateValues::One(value) => is_future_date(value),
			DateValues::Two(left, right) => {
				is_future_date(left) || is_future_date(right)
			}
		}
	}
}

fn is_future_date(value: Option<Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	value > OffsetDateTime::now_utc().date()
}

fn invalid_code(rule_code: &str, value: Option<&str>) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};
	let constraint = allowed_value_constraint_for_rule(rule_code)
		.expect("allowed-value rule code should have a catalog constraint");
	assert_eq!(
		constraint.kind,
		AllowedValueConstraintKind::CodeSet,
		"allowed-code evaluator requires a code_set catalog constraint: {rule_code}"
	);
	!constraint.values.iter().any(|allowed| allowed == value)
}

fn invalid_true_marker(
	rule_code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> bool {
	let constraint = allowed_value_constraint_for_rule(rule_code)
		.expect("true-marker rule code should have a catalog constraint");
	assert_eq!(
		constraint.kind,
		AllowedValueConstraintKind::TrueMarker,
		"true-marker evaluator requires a true_marker catalog constraint: {rule_code}"
	);
	if null_flavor
		.map(str::trim)
		.is_some_and(|null_flavor| !null_flavor.is_empty())
	{
		return false;
	}
	matches!(value, Some(false))
}

fn invalid_numeric_text(rule_code: &str, value: Option<&str>) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};
	let constraint = allowed_value_constraint_for_rule(rule_code)
		.expect("numeric rule code should have a catalog constraint");
	assert_eq!(
		constraint.kind,
		AllowedValueConstraintKind::Numeric,
		"numeric evaluator requires a numeric catalog constraint: {rule_code}"
	);
	value.parse::<Decimal>().is_err()
}

fn invalid_datetime_text(rule_code: &str, value: Option<&str>) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};
	let constraint = allowed_value_constraint_for_rule(rule_code)
		.expect("date-time rule code should have a catalog constraint");
	assert_eq!(
		constraint.kind,
		AllowedValueConstraintKind::Format,
		"date-time evaluator requires a format catalog constraint: {rule_code}"
	);
	e2b_datetime_date(Some(value)).is_none()
}

fn invalid_vocabulary(rule_code: &str, value: Option<&str>) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};
	match vocabulary_for_rule(rule_code) {
		Some("ISO3166") => {
			value != "EU"
				&& !country_code::CountryCode::VARS
					.iter()
					.any(|country| country == value)
		}
		Some(vocabulary) => {
			panic!("unsupported vocabulary evaluator {vocabulary}: {rule_code}")
		}
		None => panic!("vocabulary rule code should exist in catalog: {rule_code}"),
	}
}

pub(crate) fn e2b_datetime_date(value: Option<&str>) -> Option<Date> {
	value.and_then(lib_core::serde::flex_date::e2b_datetime_date)
}

/// Declarative presence/value rule for a single object: a catalog code, its
/// issue path, and how to extract the value. Evaluation reuses the catalog
/// engine, so tables change *structure*, not behavior.
pub(crate) struct ValueRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
}

pub(crate) struct ConditionalValueRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub trigger: fn(&T) -> bool,
	pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
	pub facts: fn(&T) -> RuleFacts,
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

pub(crate) fn eval_conditional_value<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[ConditionalValueRule<T>],
) {
	for rule in rules {
		if !(rule.trigger)(item) {
			continue;
		}
		let RuleValue::Text { value, null_flavor } = (rule.value)(item);
		let _ = push_issue_if_rule_invalid(
			issues,
			rule.code,
			rule.path,
			value.as_deref(),
			null_flavor,
			(rule.facts)(item),
		);
	}
}

pub(crate) struct FutureDateRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub dates: fn(&T) -> DateValues,
}

pub(crate) struct DateTimeTextRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_datetime_text<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[DateTimeTextRule<T>],
) {
	for rule in rules {
		if invalid_datetime_text(rule.code, (rule.value)(item)) {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) fn eval_future_dates<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[FutureDateRule<T>],
) {
	for rule in rules {
		if (rule.dates)(item).any_future() {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) struct AllowedCodeRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) struct VocabularyRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_vocabulary<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[VocabularyRule<T>],
) {
	for rule in rules {
		if invalid_vocabulary(rule.code, (rule.value)(item)) {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) struct TrueMarkerRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> (Option<bool>, Option<&'a str>),
}

pub(crate) fn eval_true_markers<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[TrueMarkerRule<T>],
) {
	for rule in rules {
		let (value, null_flavor) = (rule.value)(item);
		if invalid_true_marker(rule.code, value, null_flavor) {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) fn eval_allowed_codes<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[AllowedCodeRule<T>],
) {
	for rule in rules {
		if invalid_code(rule.code, (rule.value)(item)) {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) struct IndexedAllowedCodeRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) struct IndexedVocabularyRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_indexed_vocabulary<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedVocabularyRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if invalid_vocabulary(rule.code, (rule.value)(item)) {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) struct IndexedMeddraRule<T> {
	pub version_code: &'static str,
	pub code_code: &'static str,
	pub version_path: fn(usize) -> String,
	pub code_path: fn(usize) -> String,
	pub values: for<'a> fn(&'a T) -> (Option<&'a str>, Option<&'a str>),
}

fn eval_meddra_values(
	issues: &mut Vec<ValidationIssue>,
	vocabulary: &VocabularyContext,
	version_code: &str,
	code_code: &str,
	version_path: String,
	code_path: String,
	version: Option<&str>,
	code: Option<&str>,
) {
	if !vocabulary.meddra_available() {
		return;
	}
	assert_eq!(vocabulary_for_rule(version_code), Some("MedDRA"));
	assert_eq!(vocabulary_for_rule(code_code), Some("MedDRA"));
	let version = version.map(str::trim).filter(|value| !value.is_empty());
	let code = code.map(str::trim).filter(|value| !value.is_empty());
	if version.is_some_and(|value| !vocabulary.contains_meddra_version(value)) {
		push_issue_by_code(issues, version_code, version_path);
	}
	if let (Some(version), Some(code)) = (version, code) {
		if !vocabulary.contains_meddra_term(version, code) {
			push_issue_by_code(issues, code_code, code_path);
		}
	}
}

pub(crate) fn eval_indexed_meddra<T>(
	issues: &mut Vec<ValidationIssue>,
	vocabulary: &VocabularyContext,
	items: &[T],
	rules: &[IndexedMeddraRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			let (version, code) = (rule.values)(item);
			eval_meddra_values(
				issues,
				vocabulary,
				rule.version_code,
				rule.code_code,
				(rule.version_path)(idx),
				(rule.code_path)(idx),
				version,
				code,
			);
		}
	}
}

pub(crate) struct IndexedTrueMarkerRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> (Option<bool>, Option<&'a str>),
}

pub(crate) fn eval_indexed_true_markers<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedTrueMarkerRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			let (value, null_flavor) = (rule.value)(item);
			if invalid_true_marker(rule.code, value, null_flavor) {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) fn eval_indexed_allowed_codes<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedAllowedCodeRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if invalid_code(rule.code, (rule.value)(item)) {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) struct IndexedRepeatedAllowedCodeRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub values: fn(&T) -> Vec<String>,
}

pub(crate) struct IndexedNumericTextRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_indexed_numeric_text<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedNumericTextRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if invalid_numeric_text(rule.code, (rule.value)(item)) {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) fn eval_indexed_repeated_allowed_codes<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedRepeatedAllowedCodeRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if (rule.values)(item)
				.iter()
				.any(|value| invalid_code(rule.code, Some(value)))
			{
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) struct NestedAllowedCodeRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) struct NestedVocabularyRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) struct NestedMeddraRule<T> {
	pub version_code: &'static str,
	pub code_code: &'static str,
	pub version_path: fn(usize, usize) -> String,
	pub code_path: fn(usize, usize) -> String,
	pub values: for<'a> fn(&'a T) -> (Option<&'a str>, Option<&'a str>),
}

pub(crate) fn eval_nested_meddra<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	vocabulary: &VocabularyContext,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedMeddraRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			let (version, code) = (rule.values)(item);
			eval_meddra_values(
				issues,
				vocabulary,
				rule.version_code,
				rule.code_code,
				(rule.version_path)(parent_idx, item_idx),
				(rule.code_path)(parent_idx, item_idx),
				version,
				code,
			);
		}
	}
}

pub(crate) fn eval_nested_vocabulary<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedVocabularyRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if invalid_vocabulary(rule.code, (rule.value)(item)) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

pub(crate) fn eval_nested_allowed_codes<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedAllowedCodeRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if invalid_code(rule.code, (rule.value)(item)) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

pub(crate) struct LengthRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_length<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[LengthRule<T>],
) {
	for rule in rules {
		let Some(value) = (rule.value)(item) else {
			continue;
		};
		let max_length = max_length_for_rule(rule.code)
			.expect("length rule code should exist in catalog");
		if value.chars().count() > max_length {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) struct IndexedLengthRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_indexed_length<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedLengthRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			let Some(value) = (rule.value)(item) else {
				continue;
			};
			let max_length = max_length_for_rule(rule.code)
				.expect("length rule code should exist in catalog");
			if value.chars().count() > max_length {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) struct DerivedLengthRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: fn(&T) -> Option<String>,
}

pub(crate) fn eval_derived_length<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[DerivedLengthRule<T>],
) {
	for rule in rules {
		let Some(value) = (rule.value)(item) else {
			continue;
		};
		let max_length = max_length_for_rule(rule.code)
			.expect("length rule code should exist in catalog");
		if value.chars().count() > max_length {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) struct IndexedDerivedLengthRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: fn(&T) -> Option<String>,
}

pub(crate) fn eval_indexed_derived_length<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedDerivedLengthRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			let Some(value) = (rule.value)(item) else {
				continue;
			};
			let max_length = max_length_for_rule(rule.code)
				.expect("length rule code should exist in catalog");
			if value.chars().count() > max_length {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
		}
	}
}

pub(crate) struct NestedDerivedLengthRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: fn(&T) -> Option<String>,
}

pub(crate) fn eval_nested_derived_length<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedDerivedLengthRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			let Some(value) = (rule.value)(item) else {
				continue;
			};
			let max_length = max_length_for_rule(rule.code)
				.expect("length rule code should exist in catalog");
			if value.chars().count() > max_length {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
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

pub(crate) struct ConditionalIndexedRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub trigger: fn(&T) -> bool,
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

pub(crate) fn eval_conditional_indexed<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[ConditionalIndexedRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if !(rule.trigger)(item) {
				continue;
			}
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

pub(crate) struct IndexedFutureDateRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub dates: fn(&T) -> DateValues,
}

pub(crate) fn eval_indexed_future_dates<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedFutureDateRule<T>],
) {
	for (idx, item) in items.iter().enumerate() {
		for rule in rules {
			if (rule.dates)(item).any_future() {
				push_issue_by_code(issues, rule.code, (rule.path)(idx));
			}
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

/// Companion rules for a child collection nested under a parent collection.
/// The evaluator maps each child to its owning parent index and still preserves
/// the child's own display index for paths like `parents.{p}.pastDrugs.{i}`.
pub(crate) struct NestedCompanionRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub trigger: fn(&T) -> bool,
	pub required: fn(&T) -> bool,
}

pub(crate) fn eval_nested_companions<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedCompanionRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if (rule.trigger)(item) && !(rule.required)(item) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

pub(crate) struct NestedFutureDateRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub dates: fn(&T) -> DateValues,
}

pub(crate) fn eval_nested_future_dates<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedFutureDateRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if (rule.dates)(item).any_future() {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

pub(crate) struct NestedLengthRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_nested_length<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedLengthRule<T>],
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent_key(parent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&parent_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			let Some(value) = (rule.value)(item) else {
				continue;
			};
			let max_length = max_length_for_rule(rule.code)
				.expect("length rule code should exist in catalog");
			if value.chars().count() > max_length {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

pub(crate) struct GrandchildLengthRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) fn eval_grandchild_length<G, P, T, GK, PK>(
	issues: &mut Vec<ValidationIssue>,
	grandparents: &[G],
	parents: &[P],
	items: &[T],
	grandparent_key: fn(&G) -> GK,
	parent_key: fn(&P) -> PK,
	parent_grandparent_key: fn(&P) -> GK,
	item_parent_key: fn(&T) -> PK,
	parent_idx: fn(&P, usize) -> usize,
	item_idx: fn(&T, usize) -> usize,
	rules: &[GrandchildLengthRule<T>],
) where
	GK: Copy + Eq + Hash,
	PK: Copy + Eq + Hash,
{
	let grandparent_indices = grandparents
		.iter()
		.enumerate()
		.map(|(idx, grandparent)| (grandparent_key(grandparent), idx))
		.collect::<HashMap<_, _>>();
	let mut fallback_parent_idx_by_grandparent = HashMap::<GK, usize>::new();
	let parent_indices = parents
		.iter()
		.map(|parent| {
			let grandparent_key = parent_grandparent_key(parent);
			let fallback_idx = fallback_parent_idx_by_grandparent
				.entry(grandparent_key)
				.or_insert(0);
			let parent_idx = parent_idx(parent, *fallback_idx);
			*fallback_idx += 1;
			(parent_key(parent), (grandparent_key, parent_idx))
		})
		.collect::<HashMap<_, _>>();
	let mut fallback_item_idx_by_parent = HashMap::<PK, usize>::new();
	for item in items {
		let parent_key = item_parent_key(item);
		let fallback_idx =
			fallback_item_idx_by_parent.entry(parent_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		let Some((grandparent_key, parent_idx)) =
			parent_indices.get(&parent_key).copied()
		else {
			continue;
		};
		let Some(grandparent_idx) =
			grandparent_indices.get(&grandparent_key).copied()
		else {
			continue;
		};
		for rule in rules {
			let Some(value) = (rule.value)(item) else {
				continue;
			};
			let max_length = max_length_for_rule(rule.code)
				.expect("length rule code should exist in catalog");
			if value.chars().count() > max_length {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(grandparent_idx, parent_idx, item_idx),
				);
			}
		}
	}
}

#[cfg(test)]
mod vocabulary_rule_tests {
	use super::{eval_vocabulary, VocabularyRule};
	use crate::ValidationIssue;

	struct Item {
		country: Option<&'static str>,
	}

	#[test]
	fn iso3166_rule_accepts_standard_and_ich_eu_codes() {
		let rules = [VocabularyRule {
			code: "ICH.C.3.4.5.VOCABULARY",
			path: "senderInformation.countryCode",
			value: |item: &Item| item.country,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		for country in [None, Some("KR"), Some("EU")] {
			eval_vocabulary(&mut issues, &Item { country }, &rules);
		}

		assert!(issues.is_empty());
	}

	#[test]
	fn iso3166_rule_rejects_unknown_and_wrong_case_codes() {
		let rules = [VocabularyRule {
			code: "ICH.C.3.4.5.VOCABULARY",
			path: "senderInformation.countryCode",
			value: |item: &Item| item.country,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		for country in [Some("ZZ"), Some("kr")] {
			eval_vocabulary(&mut issues, &Item { country }, &rules);
		}

		assert_eq!(issues.len(), 2);
		assert!(issues
			.iter()
			.all(|issue| issue.code == "ICH.C.3.4.5.VOCABULARY"));
	}
}

#[cfg(test)]
mod allowed_code_rule_tests {
	use super::{
		eval_allowed_codes, eval_nested_allowed_codes, AllowedCodeRule,
		NestedAllowedCodeRule,
	};
	use crate::ValidationIssue;

	struct Item {
		value: Option<&'static str>,
		parent_id: u8,
	}

	struct Parent {
		id: u8,
	}

	#[test]
	fn allowed_code_rule_reads_values_from_catalog() {
		let rules = [AllowedCodeRule {
			code: "ICH.C.1.3.ALLOWED.VALUE",
			path: "safetyReportIdentification.reportType",
			value: |item: &Item| item.value,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_allowed_codes(
			&mut issues,
			&Item {
				value: Some("2"),
				parent_id: 1,
			},
			&rules,
		);
		assert!(issues.is_empty());

		eval_allowed_codes(
			&mut issues,
			&Item {
				value: Some("9"),
				parent_id: 1,
			},
			&rules,
		);
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.C.1.3.ALLOWED.VALUE");
	}

	#[test]
	fn nested_allowed_rule_does_not_fallback_to_parent_zero() {
		let parents = [Parent { id: 1 }];
		let items = [Item {
			value: Some("9"),
			parent_id: 2,
		}];
		let rules =
			[NestedAllowedCodeRule {
				code: "ICH.G.k.9.i.4.ALLOWED.VALUE",
				path: |parent_idx, idx| {
					format!("drugs.{parent_idx}.reactionAssessments.{idx}.reactionRecurred")
				},
				value: |item: &Item| item.value,
			}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_nested_allowed_codes(
			&mut issues,
			&parents,
			&items,
			|parent| parent.id,
			|item| item.parent_id,
			|_, idx| idx,
			&rules,
		);

		assert!(issues.is_empty());
	}
}

#[cfg(test)]
mod date_rule_tests {
	use super::{
		eval_future_dates, eval_indexed_future_dates, eval_nested_future_dates,
		DateValues, FutureDateRule, IndexedFutureDateRule, NestedFutureDateRule,
	};
	use crate::ValidationIssue;
	use sqlx::types::time::Date;
	use time::Month;

	#[derive(Clone, Copy)]
	struct Item {
		date: Option<Date>,
		other_date: Option<Date>,
		parent_id: u8,
		sequence_number: i32,
	}

	#[derive(Clone, Copy)]
	struct Parent {
		id: u8,
	}

	fn future_date() -> Date {
		Date::from_calendar_date(2999, Month::January, 1).unwrap()
	}

	fn past_date() -> Date {
		Date::from_calendar_date(2000, Month::January, 1).unwrap()
	}

	#[test]
	fn future_date_rule_emits_once_when_any_date_is_future() {
		let item = Item {
			date: Some(past_date()),
			other_date: Some(future_date()),
			parent_id: 1,
			sequence_number: 1,
		};
		let rules: [FutureDateRule<Item>; 1] = [FutureDateRule {
			code: "ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
			path: "patientInformation.patientBirthDate",
			dates: |item| DateValues::Two(item.date, item.other_date),
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_future_dates(&mut issues, &item, &rules);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.D.2.1.FUTURE_DATE.FORBIDDEN");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("patientInformation.patientBirthDate")
		);
	}

	#[test]
	fn indexed_future_date_rule_preserves_item_index_path() {
		let items = [
			Item {
				date: Some(past_date()),
				other_date: None,
				parent_id: 1,
				sequence_number: 1,
			},
			Item {
				date: Some(future_date()),
				other_date: None,
				parent_id: 1,
				sequence_number: 2,
			},
		];
		let rules: [IndexedFutureDateRule<Item>; 1] = [IndexedFutureDateRule {
			code: "ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
			path: |idx| format!("testResults.{idx}.testDate"),
			dates: |item| DateValues::One(item.date),
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_indexed_future_dates(&mut issues, &items, &rules);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.F.r.1.FUTURE_DATE.FORBIDDEN");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("testResults.1.testDate")
		);
	}

	#[test]
	fn nested_future_date_rule_preserves_parent_and_child_indices() {
		let parents = [Parent { id: 10 }, Parent { id: 20 }];
		let items = [
			Item {
				date: Some(past_date()),
				other_date: None,
				parent_id: 10,
				sequence_number: 1,
			},
			Item {
				date: Some(future_date()),
				other_date: None,
				parent_id: 20,
				sequence_number: 3,
			},
		];
		let rules: [NestedFutureDateRule<Item>; 1] = [NestedFutureDateRule {
			code: "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
			path: |parent_idx, idx| {
				format!(
					"patientInformation.parents.{parent_idx}.medicalHistory.{idx}.dateRange"
				)
			},
			dates: |item| DateValues::One(item.date),
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_nested_future_dates(
			&mut issues,
			&parents,
			&items,
			|parent| parent.id,
			|item| item.parent_id,
			|item, fallback_idx| {
				if item.sequence_number > 0 {
					(item.sequence_number - 1) as usize
				} else {
					fallback_idx
				}
			},
			&rules,
		);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("patientInformation.parents.1.medicalHistory.2.dateRange")
		);
	}
}

#[cfg(test)]
mod length_rule_tests {
	use super::{
		eval_indexed_length, eval_length, eval_nested_length, IndexedLengthRule,
		LengthRule, NestedLengthRule,
	};
	use crate::ValidationIssue;

	struct Item {
		value: Option<&'static str>,
		parent_id: u8,
		sequence_number: i32,
	}

	#[derive(Clone, Copy)]
	struct Parent {
		id: u8,
	}

	#[test]
	fn length_rule_uses_catalog_limit_and_emits_when_value_exceeds_it() {
		let item = Item {
			value: Some("ABC"),
			parent_id: 1,
			sequence_number: 1,
		};
		let rules: [LengthRule<Item>; 1] = [LengthRule {
			code: "ICH.N.1.1.LENGTH.MAX",
			path: "messageHeader.messageType",
			value: |item| item.value,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_length(&mut issues, &item, &rules);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.N.1.1.LENGTH.MAX");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("messageHeader.messageType")
		);
	}

	#[test]
	fn length_rule_is_silent_for_missing_or_within_limit_values() {
		let rules: [LengthRule<Item>; 1] = [LengthRule {
			code: "ICH.N.1.1.LENGTH.MAX",
			path: "messageHeader.messageType",
			value: |item| item.value,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_length(
			&mut issues,
			&Item {
				value: None,
				parent_id: 1,
				sequence_number: 1,
			},
			&rules,
		);
		eval_length(
			&mut issues,
			&Item {
				value: Some("AB"),
				parent_id: 1,
				sequence_number: 1,
			},
			&rules,
		);

		assert!(issues.is_empty());
	}

	#[test]
	fn indexed_length_rule_preserves_item_index_path() {
		let items = [
			Item {
				value: Some("AB"),
				parent_id: 1,
				sequence_number: 1,
			},
			Item {
				value: Some("ABC"),
				parent_id: 1,
				sequence_number: 2,
			},
		];
		let rules: [IndexedLengthRule<Item>; 1] = [IndexedLengthRule {
			code: "ICH.N.1.1.LENGTH.MAX",
			path: |idx| format!("messageHeaders.{idx}.messageType"),
			value: |item| item.value,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_indexed_length(&mut issues, &items, &rules);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.N.1.1.LENGTH.MAX");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("messageHeaders.1.messageType")
		);
	}

	#[test]
	fn nested_length_rule_preserves_parent_and_child_indices() {
		let parents = [Parent { id: 10 }, Parent { id: 20 }];
		let items = [
			Item {
				value: Some("AB"),
				parent_id: 10,
				sequence_number: 1,
			},
			Item {
				value: Some("ABC"),
				parent_id: 20,
				sequence_number: 3,
			},
		];
		let rules: [NestedLengthRule<Item>; 1] = [NestedLengthRule {
			code: "ICH.N.1.1.LENGTH.MAX",
			path: |parent_idx, idx| {
				format!("parents.{parent_idx}.messageHeaders.{idx}.messageType")
			},
			value: |item| item.value,
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_nested_length(
			&mut issues,
			&parents,
			&items,
			|parent| parent.id,
			|item| item.parent_id,
			|item, fallback_idx| {
				if item.sequence_number > 0 {
					(item.sequence_number - 1) as usize
				} else {
					fallback_idx
				}
			},
			&rules,
		);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.N.1.1.LENGTH.MAX");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("parents.1.messageHeaders.2.messageType")
		);
	}
}
