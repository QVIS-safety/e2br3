use crate::allowed_value::{
	is_allowed_value_valid, true_marker_value, ConstraintValue,
};
use crate::context::VocabularyContext;
use crate::{
	allowed_value_constraint_for_rule, find_canonical_rule, null_flavors_for_rule,
	AllowedValueConstraintKind, FormatName, NumericShape, ALLOWED_VALUE_RULES,
	MAX_LENGTH_RULES, NULL_FLAVOR_RULES,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortableInputValue<'a> {
	Missing,
	String(&'a str),
	Boolean(bool),
	Number(&'a serde_json::Number),
	InvalidType,
}

pub fn portable_constraints() -> Vec<PortableConstraint> {
	let mut rules = Vec::new();

	for rule in MAX_LENGTH_RULES {
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

	for rule in ALLOWED_VALUE_RULES {
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

	for rule in NULL_FLAVOR_RULES {
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
	value: PortableInputValue<'_>,
	null_flavor: Option<&str>,
) -> Result<(), PortableConstraintViolation> {
	let Some(rule) = portable_constraints()
		.into_iter()
		.find(|rule| rule.code == rule_code)
	else {
		return Ok(());
	};
	let value = match value {
		PortableInputValue::String(value) => {
			let value = value.trim();
			if value.is_empty() {
				PortableInputValue::Missing
			} else {
				PortableInputValue::String(value)
			}
		}
		value => value,
	};
	let valid = match rule.kind {
		PortableConstraintKind::MaxLength => match value {
			PortableInputValue::Missing => true,
			PortableInputValue::String(value) => {
				value.chars().count()
					<= rule.max_length.expect("max length is present")
			}
			PortableInputValue::Number(value) => {
				value.to_string().chars().count()
					<= rule.max_length.expect("max length is present")
			}
			_ => false,
		},
		PortableConstraintKind::NullFlavor => null_flavor
			.or(match value {
				PortableInputValue::String(value) => Some(value),
				PortableInputValue::Missing => None,
				_ => return portable_violation(rule),
			})
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.is_none_or(|value| rule.values.iter().any(|allowed| allowed == value)),
		PortableConstraintKind::InlineAllowedValues
		| PortableConstraintKind::Numeric
		| PortableConstraintKind::Format => {
			if value == PortableInputValue::Missing {
				return Ok(());
			}
			let constraint = allowed_value_constraint_for_rule(rule_code)
				.expect("portable allowed-value rule should exist in catalog");
			let constraint_value = match (constraint.kind, value) {
				(
					AllowedValueConstraintKind::Boolean,
					PortableInputValue::Boolean(value),
				) => ConstraintValue::Boolean(Some(value)),
				(
					AllowedValueConstraintKind::TrueMarker,
					PortableInputValue::Boolean(value),
				) => true_marker_value(Some(value), null_flavor),
				(
					AllowedValueConstraintKind::Numeric,
					PortableInputValue::Number(value),
				) => ConstraintValue::Text(Some(Cow::Owned(value.to_string()))),
				(
					AllowedValueConstraintKind::CodeSet
					| AllowedValueConstraintKind::Numeric
					| AllowedValueConstraintKind::Format,
					PortableInputValue::String(value),
				) => ConstraintValue::Text(Some(Cow::Borrowed(value))),
				_ => return portable_violation(rule),
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
		portable_violation(rule)
	}
}

fn portable_violation(
	rule: PortableConstraint,
) -> Result<(), PortableConstraintViolation> {
	Err(PortableConstraintViolation {
		code: rule.code,
		message: rule.message,
	})
}

fn message_for_rule(code: &str) -> String {
	find_canonical_rule(code)
		.map(|rule| rule.message.to_string())
		.unwrap_or_else(|| code.to_string())
}
