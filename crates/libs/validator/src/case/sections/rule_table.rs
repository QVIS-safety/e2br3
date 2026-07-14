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

use crate::allowed_value::{
	is_allowed_value_valid, is_named_vocabulary_value_valid, ConstraintValue,
};
use crate::context::VocabularyContext;
use crate::{
	max_length_for_rule, push_issue_by_code, push_issue_if_rule_invalid,
	vocabulary_for_rule, vocabulary_variant_for_rule, RuleFacts, ValidationIssue,
};
use sqlx::types::time::{Date, OffsetDateTime};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

#[cfg(test)]
pub(crate) trait HasRuleCode {
	fn rule_code(&self) -> &'static str;
}

#[cfg(test)]
pub(crate) fn table_rule_codes<T: HasRuleCode>(
	rules: &[T],
) -> impl Iterator<Item = &'static str> + '_ {
	rules.iter().map(HasRuleCode::rule_code)
}

#[cfg(test)]
macro_rules! impl_has_rule_code {
	($($rule:ident),+ $(,)?) => {
		$(impl<T> HasRuleCode for $rule<T> {
			fn rule_code(&self) -> &'static str {
				self.code
			}
		})+
	};
}

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

pub(crate) struct ConstraintRule<T> {
	pub code: &'static str,
	pub path: &'static str,
	pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) struct IndexedConstraintRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) struct NestedConstraintRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) struct IndexedVocabularyVariantRule<T> {
	pub code: &'static str,
	pub path: fn(usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

pub(crate) struct NestedVocabularyVariantRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> Option<&'a str>,
}

fn vocabulary_variant_value_is_invalid(
	code: &str,
	receiver: Option<&str>,
	value: Option<&str>,
	vocabulary: &VocabularyContext,
) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return false;
	};
	let Some(variant) = receiver
		.map(str::trim)
		.filter(|receiver| !receiver.is_empty())
		.and_then(|receiver| vocabulary_variant_for_rule(code, receiver))
	else {
		return false;
	};
	!is_named_vocabulary_value_valid(
		variant.vocabulary,
		variant.scope,
		value,
		vocabulary,
	)
}

pub(crate) fn eval_indexed_vocabulary_variants<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedVocabularyVariantRule<T>],
	receiver: Option<&str>,
	vocabulary: &VocabularyContext,
) {
	for (index, item) in items.iter().enumerate() {
		for rule in rules {
			if vocabulary_variant_value_is_invalid(
				rule.code,
				receiver,
				(rule.value)(item),
				vocabulary,
			) {
				push_issue_by_code(issues, rule.code, (rule.path)(index));
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn eval_nested_vocabulary_variants<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T) -> Option<usize>,
	rules: &[NestedVocabularyVariantRule<T>],
	receiver: Option<&str>,
	vocabulary: &VocabularyContext,
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(index, parent)| (parent_key(parent), index))
		.collect::<HashMap<_, _>>();
	for item in items {
		let Some(parent_idx) = parent_indices.get(&item_parent_key(item)).copied()
		else {
			continue;
		};
		let Some(item_idx) = item_idx(item) else {
			continue;
		};
		for rule in rules {
			if vocabulary_variant_value_is_invalid(
				rule.code,
				receiver,
				(rule.value)(item),
				vocabulary,
			) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

#[allow(dead_code)]
pub(crate) struct GrandchildConstraintRule<T> {
	pub code: &'static str,
	pub path: fn(usize, usize, usize) -> String,
	pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) fn eval_constraints<T>(
	issues: &mut Vec<ValidationIssue>,
	item: &T,
	rules: &[ConstraintRule<T>],
	vocabulary: &VocabularyContext,
) {
	for rule in rules {
		if !is_allowed_value_valid(rule.code, (rule.value)(item), vocabulary) {
			push_issue_by_code(issues, rule.code, rule.path);
		}
	}
}

pub(crate) fn eval_indexed_constraints<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[IndexedConstraintRule<T>],
	vocabulary: &VocabularyContext,
) {
	for (index, item) in items.iter().enumerate() {
		for rule in rules {
			if !is_allowed_value_valid(rule.code, (rule.value)(item), vocabulary) {
				push_issue_by_code(issues, rule.code, (rule.path)(index));
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn eval_nested_constraints<P, T, K>(
	issues: &mut Vec<ValidationIssue>,
	parents: &[P],
	items: &[T],
	parent_key: fn(&P) -> K,
	item_parent_key: fn(&T) -> K,
	item_idx: fn(&T, usize) -> usize,
	rules: &[NestedConstraintRule<T>],
	vocabulary: &VocabularyContext,
) where
	K: Copy + Eq + Hash,
{
	let parent_indices = parents
		.iter()
		.enumerate()
		.map(|(index, parent)| (parent_key(parent), index))
		.collect::<HashMap<_, _>>();
	let mut fallback_idx_by_parent = HashMap::<K, usize>::new();
	for item in items {
		let owner_key = item_parent_key(item);
		let Some(parent_idx) = parent_indices.get(&owner_key).copied() else {
			continue;
		};
		let fallback_idx = fallback_idx_by_parent.entry(owner_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if !is_allowed_value_valid(rule.code, (rule.value)(item), vocabulary) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(parent_idx, item_idx),
				);
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn eval_grandchild_constraints<G, P, T, GK, PK>(
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
	rules: &[GrandchildConstraintRule<T>],
	vocabulary: &VocabularyContext,
) where
	GK: Copy + Eq + Hash,
	PK: Copy + Eq + Hash,
{
	let grandparent_indices = grandparents
		.iter()
		.enumerate()
		.map(|(index, grandparent)| (grandparent_key(grandparent), index))
		.collect::<HashMap<_, _>>();
	let mut fallback_parent_idx_by_grandparent = HashMap::<GK, usize>::new();
	let parent_indices = parents
		.iter()
		.filter_map(|parent| {
			let owner_key = parent_grandparent_key(parent);
			grandparent_indices.get(&owner_key)?;
			let fallback_idx = fallback_parent_idx_by_grandparent
				.entry(owner_key)
				.or_insert(0);
			let concrete_idx = parent_idx(parent, *fallback_idx);
			*fallback_idx += 1;
			Some((parent_key(parent), (owner_key, concrete_idx)))
		})
		.collect::<HashMap<_, _>>();
	let mut fallback_item_idx_by_parent = HashMap::<PK, usize>::new();
	for item in items {
		let owner_key = item_parent_key(item);
		let Some((grandparent_key, parent_idx)) =
			parent_indices.get(&owner_key).copied()
		else {
			continue;
		};
		let grandparent_idx = grandparent_indices[&grandparent_key];
		let fallback_idx = fallback_item_idx_by_parent.entry(owner_key).or_insert(0);
		let item_idx = item_idx(item, *fallback_idx);
		*fallback_idx += 1;
		for rule in rules {
			if !is_allowed_value_valid(rule.code, (rule.value)(item), vocabulary) {
				push_issue_by_code(
					issues,
					rule.code,
					(rule.path)(grandparent_idx, parent_idx, item_idx),
				);
			}
		}
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

/// A prepared value whose concrete path and rule facts are supplied by the
/// section. The catalog remains responsible for condition and value policy.
pub(crate) struct CatalogValueRule<T> {
	pub code: &'static str,
	pub path: fn(&T) -> String,
	pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
	pub facts: fn(&T) -> RuleFacts,
}

pub(crate) fn eval_catalog_values<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[CatalogValueRule<T>],
) {
	for item in items {
		for rule in rules {
			let RuleValue::Text { value, null_flavor } = (rule.value)(item);
			let _ = push_issue_if_rule_invalid(
				issues,
				rule.code,
				(rule.path)(item),
				value.as_deref(),
				null_flavor,
				(rule.facts)(item),
			);
		}
	}
}

/// A prepared rule for algorithmic invalidity that cannot be expressed as a
/// catalog value policy. The section supplies only the predicate and path.
pub(crate) struct ViolationRule<T> {
	pub code: &'static str,
	pub path: fn(&T) -> String,
	pub violated: fn(&T) -> bool,
}

pub(crate) fn eval_violations<T>(
	issues: &mut Vec<ValidationIssue>,
	items: &[T],
	rules: &[ViolationRule<T>],
) {
	for item in items {
		for rule in rules {
			if (rule.violated)(item) {
				push_issue_by_code(issues, rule.code, (rule.path)(item));
			}
		}
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

pub(crate) struct IndexedMeddraRule<T> {
	pub version_allowed_code: &'static str,
	pub code_allowed_code: &'static str,
	pub version_code: &'static str,
	pub code_code: &'static str,
	pub version_path: fn(usize) -> String,
	pub code_path: fn(usize) -> String,
	pub values: for<'a> fn(&'a T) -> (Option<&'a str>, Option<&'a str>),
}

#[cfg(test)]
pub(crate) fn indexed_meddra_constraint_codes<T>(
	rules: &[IndexedMeddraRule<T>],
) -> Vec<&'static str> {
	rules
		.iter()
		.flat_map(|rule| [rule.version_allowed_code, rule.code_allowed_code])
		.collect()
}

fn eval_meddra_values(
	issues: &mut Vec<ValidationIssue>,
	vocabulary: &VocabularyContext,
	version_allowed_code: &str,
	code_allowed_code: &str,
	version_code: &str,
	code_code: &str,
	version_path: String,
	code_path: String,
	version: Option<&str>,
	code: Option<&str>,
) {
	for (allowed_value_code, vocabulary_code, path, value) in [
		(
			version_allowed_code,
			version_code,
			version_path.clone(),
			version,
		),
		(code_allowed_code, code_code, code_path.clone(), code),
	] {
		if !is_allowed_value_valid(
			vocabulary_code,
			ConstraintValue::Text(value.map(Cow::Borrowed)),
			vocabulary,
		) {
			push_issue_by_code(issues, allowed_value_code, path);
		}
	}
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
				rule.version_allowed_code,
				rule.code_allowed_code,
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

pub(crate) struct NestedMeddraRule<T> {
	pub version_allowed_code: &'static str,
	pub code_allowed_code: &'static str,
	pub version_code: &'static str,
	pub code_code: &'static str,
	pub version_path: fn(usize, usize) -> String,
	pub code_path: fn(usize, usize) -> String,
	pub values: for<'a> fn(&'a T) -> (Option<&'a str>, Option<&'a str>),
}

#[cfg(test)]
pub(crate) fn nested_meddra_constraint_codes<T>(
	rules: &[NestedMeddraRule<T>],
) -> Vec<&'static str> {
	rules
		.iter()
		.flat_map(|rule| [rule.version_allowed_code, rule.code_allowed_code])
		.collect()
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
				rule.version_allowed_code,
				rule.code_allowed_code,
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

#[cfg(test)]
impl_has_rule_code!(
	ConstraintRule,
	IndexedConstraintRule,
	NestedConstraintRule,
	GrandchildConstraintRule,
	IndexedVocabularyVariantRule,
	NestedVocabularyVariantRule,
	ValueRule,
	CatalogValueRule,
	ViolationRule,
	ConditionalValueRule,
	FutureDateRule,
	LengthRule,
	IndexedLengthRule,
	DerivedLengthRule,
	IndexedDerivedLengthRule,
	NestedDerivedLengthRule,
	IndexedRule,
	ConditionalIndexedRule,
	IndexedFutureDateRule,
	CompanionRule,
	NestedCompanionRule,
	NestedFutureDateRule,
	NestedLengthRule,
	GrandchildLengthRule,
);

#[cfg(test)]
pub(crate) fn indexed_meddra_rule_codes<T>(
	rules: &[IndexedMeddraRule<T>],
) -> impl Iterator<Item = &'static str> + '_ {
	rules.iter().flat_map(|rule| {
		[
			rule.version_allowed_code,
			rule.code_allowed_code,
			rule.version_code,
			rule.code_code,
		]
	})
}

#[cfg(test)]
pub(crate) fn nested_meddra_rule_codes<T>(
	rules: &[NestedMeddraRule<T>],
) -> impl Iterator<Item = &'static str> + '_ {
	rules.iter().flat_map(|rule| {
		[
			rule.version_allowed_code,
			rule.code_allowed_code,
			rule.version_code,
			rule.code_code,
		]
	})
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
mod catalog_value_rule_tests {
	use super::{eval_catalog_values, CatalogValueRule, RuleValue};
	use crate::{RuleFacts, ValidationIssue};

	struct PreparedValue {
		path: String,
		value: Option<String>,
		facts: RuleFacts,
	}

	const RULES: &[CatalogValueRule<PreparedValue>] = &[CatalogValueRule {
		code: "MFDS.C.5.4.KR.1.REQUIRED",
		path: |item| item.path.clone(),
		value: |item| RuleValue::borrowed(item.value.as_deref(), None),
		facts: |item| item.facts,
	}];

	fn prepared(condition: bool, value: Option<&str>) -> PreparedValue {
		PreparedValue {
			path: "studyInformation.2.studyTypeReactionKr1".to_string(),
			value: value.map(str::to_string),
			facts: RuleFacts {
				mfds_study_type_reaction_is_three: Some(condition),
				..RuleFacts::default()
			},
		}
	}

	#[test]
	fn catalog_condition_and_value_policy_control_emission() {
		let items = [
			prepared(false, None),
			prepared(true, None),
			prepared(true, Some("value")),
		];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_catalog_values(&mut issues, &items, RULES);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "MFDS.C.5.4.KR.1.REQUIRED");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("studyInformation.2.studyTypeReactionKr1")
		);
	}
}

#[cfg(test)]
mod violation_rule_tests {
	use super::{eval_violations, ViolationRule};
	use crate::ValidationIssue;

	struct PreparedViolation {
		path: String,
		violated: bool,
	}

	const RULES: &[ViolationRule<PreparedViolation>] = &[ViolationRule {
		code: "ICH.D.8.MPID_PHPID.EXCLUSIVE",
		path: |item| item.path.clone(),
		violated: |item| item.violated,
	}];

	#[test]
	fn violation_predicate_controls_emission_and_preserves_path() {
		let items = [
			PreparedViolation {
				path: "parents.0.pastDrugs.0.mpid".to_string(),
				violated: false,
			},
			PreparedViolation {
				path: "parents.1.pastDrugs.3.mpid".to_string(),
				violated: true,
			},
		];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_violations(&mut issues, &items, RULES);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("parents.1.pastDrugs.3.mpid")
		);
	}
}

#[cfg(test)]
mod constraint_rule_tests {
	use super::{
		eval_constraints, eval_grandchild_constraints, eval_indexed_constraints,
		eval_nested_constraints, ConstraintRule, GrandchildConstraintRule,
		IndexedConstraintRule, NestedConstraintRule,
	};
	use crate::allowed_value::ConstraintValue;
	use crate::context::VocabularyContext;
	use crate::ValidationIssue;
	use std::borrow::Cow;

	struct Item {
		parent_id: u8,
		stored_idx: usize,
		value: Option<&'static str>,
	}

	struct Parent {
		id: u8,
		grandparent_id: u8,
		stored_idx: usize,
	}

	struct Grandparent {
		id: u8,
	}

	#[test]
	fn scalar_constraint_uses_catalog_vocabulary_semantics() {
		let rule = [ConstraintRule {
			code: "ICH.C.3.4.5.VOCABULARY",
			path: "senderInformation.countryCode",
			value: |item: &Item| {
				ConstraintValue::Text(item.value.map(Cow::Borrowed))
			},
		}];
		let mut issues = Vec::<ValidationIssue>::new();
		let vocabulary = VocabularyContext::for_active_codes(&[
			("ISO3166", crate::VocabularyScope::All, "KR"),
			("ISO3166", crate::VocabularyScope::All, "EU"),
		]);

		for value in [None, Some("KR"), Some("EU")] {
			eval_constraints(
				&mut issues,
				&Item {
					parent_id: 0,
					stored_idx: 0,
					value,
				},
				&rule,
				&vocabulary,
			);
		}
		assert!(issues.is_empty());

		eval_constraints(
			&mut issues,
			&Item {
				parent_id: 0,
				stored_idx: 0,
				value: Some("ZZ"),
			},
			&rule,
			&vocabulary,
		);
		assert_eq!(issues.len(), 1);
	}

	#[test]
	fn indexed_constraint_retains_actual_index() {
		let items = [
			Item {
				parent_id: 0,
				stored_idx: 0,
				value: Some("1"),
			},
			Item {
				parent_id: 0,
				stored_idx: 0,
				value: Some("99"),
			},
		];
		let rules = [IndexedConstraintRule {
			code: "ICH.E.i.7.ALLOWED.VALUE",
			path: |index| format!("reactions.{index}.outcome"),
			value: |item: &Item| {
				ConstraintValue::Text(item.value.map(Cow::Borrowed))
			},
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_indexed_constraints(
			&mut issues,
			&items,
			&rules,
			&VocabularyContext::default(),
		);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].field_path.as_deref(), Some("reactions.1.outcome"));
	}

	#[test]
	fn nested_constraint_never_falls_back_to_parent_zero() {
		let parents = [Parent {
			id: 1,
			grandparent_id: 0,
			stored_idx: 0,
		}];
		let items = [Item {
			parent_id: 2,
			stored_idx: 3,
			value: Some("99"),
		}];
		let rules = [NestedConstraintRule {
			code: "ICH.E.i.7.ALLOWED.VALUE",
			path: |parent_idx, item_idx| {
				format!("parents.{parent_idx}.items.{item_idx}.outcome")
			},
			value: |item: &Item| {
				ConstraintValue::Text(item.value.map(Cow::Borrowed))
			},
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_nested_constraints(
			&mut issues,
			&parents,
			&items,
			|parent| parent.id,
			|item| item.parent_id,
			|item, fallback| {
				if item.stored_idx == 0 {
					fallback
				} else {
					item.stored_idx
				}
			},
			&rules,
			&VocabularyContext::default(),
		);

		assert!(issues.is_empty());
	}

	#[test]
	fn grandchild_constraint_retains_all_concrete_indexes() {
		let grandparents = [Grandparent { id: 1 }, Grandparent { id: 2 }];
		let parents = [Parent {
			id: 8,
			grandparent_id: 2,
			stored_idx: 2,
		}];
		let items = [Item {
			parent_id: 8,
			stored_idx: 3,
			value: Some("99"),
		}];
		let rules = [GrandchildConstraintRule {
			code: "ICH.E.i.7.ALLOWED.VALUE",
			path: |grandparent_idx, parent_idx, item_idx| {
				format!("drugs.{grandparent_idx}.dosages.{parent_idx}.intervals.{item_idx}.unit")
			},
			value: |item: &Item| {
				ConstraintValue::Text(item.value.map(Cow::Borrowed))
			},
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_grandchild_constraints(
			&mut issues,
			&grandparents,
			&parents,
			&items,
			|grandparent| grandparent.id,
			|parent| parent.id,
			|parent| parent.grandparent_id,
			|item| item.parent_id,
			|parent, fallback| {
				if parent.stored_idx == 0 {
					fallback
				} else {
					parent.stored_idx
				}
			},
			|item, fallback| {
				if item.stored_idx == 0 {
					fallback
				} else {
					item.stored_idx
				}
			},
			&rules,
			&VocabularyContext::default(),
		);

		assert_eq!(issues.len(), 1);
		assert_eq!(
			issues[0].field_path.as_deref(),
			Some("drugs.1.dosages.2.intervals.3.unit")
		);
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

#[cfg(test)]
mod vocabulary_variant_rule_tests {
	use super::{eval_indexed_vocabulary_variants, IndexedVocabularyVariantRule};
	use crate::context::VocabularyContext;
	use crate::{ValidationIssue, VocabularyScope};

	struct Item {
		code: Option<&'static str>,
	}

	const RULES: &[IndexedVocabularyVariantRule<Item>] =
		&[IndexedVocabularyVariantRule {
			code: "MFDS.G.k.2.1.KR.1b.VOCABULARY",
			path: |idx| format!("drugs.{idx}.mfdsMpid"),
			value: |item| item.code,
		}];

	#[test]
	fn receiver_variant_uses_matching_active_vocabulary_without_fallback() {
		let vocabulary = VocabularyContext::for_active_codes(&[
			("MFDS_PRODUCT", VocabularyScope::ItemSeq, "KR123"),
			("WHODrug", VocabularyScope::All, "FR456"),
		]);
		let items = [Item {
			code: Some("KR123"),
		}];
		let mut issues = Vec::<ValidationIssue>::new();

		eval_indexed_vocabulary_variants(
			&mut issues,
			&items,
			RULES,
			Some("KR"),
			&vocabulary,
		);
		assert!(issues.is_empty());

		eval_indexed_vocabulary_variants(
			&mut issues,
			&items,
			RULES,
			Some("FR"),
			&vocabulary,
		);
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "MFDS.G.k.2.1.KR.1b.VOCABULARY");

		issues.clear();
		eval_indexed_vocabulary_variants(
			&mut issues,
			&items,
			RULES,
			Some("UNKNOWN"),
			&vocabulary,
		);
		assert!(issues.is_empty());
	}
}
