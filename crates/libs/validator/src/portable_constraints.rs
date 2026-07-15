use crate::allowed_value::{
	is_allowed_value_valid, true_marker_value, ConstraintValue,
};
use crate::context::VocabularyContext;
use crate::{
	allowed_value_constraint_for_rule, find_canonical_rule, null_flavors_for_rule,
	AllowedValueConstraintKind, FormatName, NumericShape, RegulatoryAuthority,
	ALLOWED_VALUE_RULES, MAX_LENGTH_RULES, NULL_FLAVOR_RULES,
};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableConstraintKind {
	MaxLength,
	Numeric,
	Format,
	InlineAllowedValues,
	NullFlavor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableConstraint {
	pub code: String,
	pub kind: PortableConstraintKind,
	pub max_length: Option<usize>,
	pub values: Vec<String>,
	pub numeric_shape: Option<NumericShape>,
	pub format_name: Option<FormatName>,
	pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableConstraintViolation {
	pub code: String,
	pub message: String,
}

pub fn portable_ich_constraints() -> Vec<PortableConstraint> {
	let mut rules = Vec::new();

	for rule in MAX_LENGTH_RULES
		.iter()
		.filter(|rule| rule.authority == RegulatoryAuthority::Ich)
	{
		rules.push(PortableConstraint {
			code: rule.code.to_string(),
			kind: PortableConstraintKind::MaxLength,
			max_length: Some(rule.max_length),
			values: Vec::new(),
			numeric_shape: None,
			format_name: None,
			message: message_for_rule(rule.code),
		});
	}

	for rule in ALLOWED_VALUE_RULES
		.iter()
		.filter(|rule| rule.authority == RegulatoryAuthority::Ich)
	{
		let Some(constraint) = allowed_value_constraint_for_rule(rule.code) else {
			continue;
		};
		let (kind, values) = match constraint.kind {
			AllowedValueConstraintKind::CodeSet => (
				PortableConstraintKind::InlineAllowedValues,
				constraint.values.clone(),
			),
			AllowedValueConstraintKind::Boolean => (
				PortableConstraintKind::InlineAllowedValues,
				vec!["false".to_string(), "true".to_string()],
			),
			AllowedValueConstraintKind::TrueMarker => (
				PortableConstraintKind::InlineAllowedValues,
				vec!["true".to_string()],
			),
			AllowedValueConstraintKind::Numeric => {
				(PortableConstraintKind::Numeric, Vec::new())
			}
			AllowedValueConstraintKind::Format
				if matches!(
					constraint.format_name,
					Some(FormatName::E2bDatetime | FormatName::Base64)
				) =>
			{
				(PortableConstraintKind::Format, Vec::new())
			}
			AllowedValueConstraintKind::Format
			| AllowedValueConstraintKind::Vocabulary
			| AllowedValueConstraintKind::Descriptive => continue,
		};
		rules.push(PortableConstraint {
			code: rule.code.to_string(),
			kind,
			max_length: None,
			values,
			numeric_shape: constraint.numeric_shape,
			format_name: constraint.format_name,
			message: message_for_rule(rule.code),
		});
	}

	for rule in NULL_FLAVOR_RULES
		.iter()
		.filter(|rule| rule.authority == RegulatoryAuthority::Ich)
	{
		let Some(values) = null_flavors_for_rule(rule.code) else {
			continue;
		};
		rules.push(PortableConstraint {
			code: rule.code.to_string(),
			kind: PortableConstraintKind::NullFlavor,
			max_length: None,
			values: values.to_vec(),
			numeric_shape: None,
			format_name: None,
			message: message_for_rule(rule.code),
		});
	}

	rules.sort_by(|left, right| left.code.cmp(&right.code));
	debug_assert_eq!(
		rules
			.iter()
			.map(|rule| rule.code.as_str())
			.collect::<HashSet<_>>()
			.len(),
		rules.len(),
		"portable constraint codes must be unique"
	);
	rules
}

pub fn validate_portable_value(
	rule_code: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
) -> Result<(), PortableConstraintViolation> {
	let Some(rule) = portable_ich_constraints()
		.into_iter()
		.find(|rule| rule.code == rule_code)
	else {
		return Ok(());
	};
	let value = value.map(str::trim).filter(|value| !value.is_empty());
	let valid = match rule.kind {
		PortableConstraintKind::MaxLength => value.is_none_or(|value| {
			value.chars().count() <= rule.max_length.expect("max length is present")
		}),
		PortableConstraintKind::NullFlavor => null_flavor
			.or(value)
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.is_none_or(|value| rule.values.iter().any(|allowed| allowed == value)),
		PortableConstraintKind::InlineAllowedValues
		| PortableConstraintKind::Numeric
		| PortableConstraintKind::Format => {
			let Some(value) = value else {
				return Ok(());
			};
			let constraint = allowed_value_constraint_for_rule(rule_code)
				.expect("portable allowed-value rule should exist in catalog");
			let constraint_value = match constraint.kind {
				AllowedValueConstraintKind::Boolean => {
					ConstraintValue::Boolean(value.parse::<bool>().ok())
				}
				AllowedValueConstraintKind::TrueMarker => {
					true_marker_value(value.parse::<bool>().ok(), null_flavor)
				}
				_ => ConstraintValue::Text(Some(Cow::Borrowed(value))),
			};
			is_allowed_value_valid(
				rule_code,
				constraint_value,
				&VocabularyContext::default(),
			)
		}
	};

	if valid {
		Ok(())
	} else {
		Err(PortableConstraintViolation {
			code: rule.code,
			message: rule.message,
		})
	}
}

fn message_for_rule(code: &str) -> String {
	find_canonical_rule(code)
		.map(|rule| rule.message.to_string())
		.unwrap_or_else(|| code.to_string())
}
