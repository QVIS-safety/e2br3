use crate::context::VocabularyContext;
use crate::{
	allowed_value_code_for_vocabulary_rule, allowed_value_constraint_for_rule,
	vocabulary_for_rule, AllowedValueConstraint, AllowedValueConstraintKind,
	FormatName, IdentifierProfile, NumericShape,
};
use base64::engine::{general_purpose, Engine};
use sqlx::types::time::Date;
use sqlx::types::Decimal;
use std::borrow::Cow;

pub(crate) enum ConstraintValue<'a> {
	Text(Option<Cow<'a, str>>),
	Texts(Vec<Cow<'a, str>>),
	Boolean(Option<bool>),
	#[allow(dead_code)]
	Decimal(Option<Decimal>),
	#[allow(dead_code)]
	Date(Option<Date>),
}

impl ConstraintValue<'_> {
	fn is_empty(&self) -> bool {
		match self {
			Self::Text(value) => {
				value.as_deref().map(str::trim).is_none_or(str::is_empty)
			}
			Self::Texts(values) => values.is_empty(),
			Self::Boolean(value) => value.is_none(),
			Self::Decimal(value) => value.is_none(),
			Self::Date(value) => value.is_none(),
		}
	}
}

pub(crate) fn true_marker_value<'a>(
	value: Option<bool>,
	null_flavor: Option<&'a str>,
) -> ConstraintValue<'a> {
	if null_flavor.is_some_and(|value| !value.trim().is_empty()) {
		ConstraintValue::Boolean(None)
	} else {
		ConstraintValue::Boolean(value)
	}
}

pub(crate) fn is_allowed_value_valid(
	rule_code: &str,
	value: ConstraintValue<'_>,
	vocabulary: &VocabularyContext,
) -> bool {
	if let ConstraintValue::Texts(values) = value {
		return values.into_iter().all(|value| {
			is_allowed_value_valid(
				rule_code,
				ConstraintValue::Text(Some(value)),
				vocabulary,
			)
		});
	}
	let constraint = constraint_for_rule(rule_code);
	if value.is_empty() {
		return true;
	}

	match constraint.kind {
		AllowedValueConstraintKind::CodeSet => validate_code_set(constraint, value),
		AllowedValueConstraintKind::Boolean => {
			matches!(value, ConstraintValue::Boolean(Some(_)))
		}
		AllowedValueConstraintKind::TrueMarker => {
			matches!(value, ConstraintValue::Boolean(Some(true)))
		}
		AllowedValueConstraintKind::Numeric => validate_numeric(constraint, value),
		AllowedValueConstraintKind::Format => {
			validate_format(constraint, value, vocabulary)
		}
		AllowedValueConstraintKind::Vocabulary => {
			validate_vocabulary(rule_code, constraint, value, vocabulary)
		}
		AllowedValueConstraintKind::Descriptive => true,
	}
}

fn constraint_for_rule(rule_code: &str) -> &'static AllowedValueConstraint {
	if let Some(constraint) = allowed_value_constraint_for_rule(rule_code) {
		return constraint;
	}
	if vocabulary_for_rule(rule_code).is_some() {
		let constraint_code = allowed_value_code_for_vocabulary_rule(rule_code)
			.expect("vocabulary rule should have an allowed-value companion");
		return allowed_value_constraint_for_rule(constraint_code)
			.expect("vocabulary companion should have a constraint");
	}
	panic!("missing allowed-value constraint: {rule_code}")
}

fn text_value<'a>(value: ConstraintValue<'a>, rule_kind: &str) -> Cow<'a, str> {
	match value {
		ConstraintValue::Text(Some(value)) => value,
		_ => panic!("{rule_kind} constraint requires a text value"),
	}
}

fn validate_code_set(
	constraint: &AllowedValueConstraint,
	value: ConstraintValue<'_>,
) -> bool {
	let value = text_value(value, "code_set");
	constraint
		.values
		.iter()
		.any(|allowed| allowed == value.trim())
}

fn validate_numeric(
	constraint: &AllowedValueConstraint,
	value: ConstraintValue<'_>,
) -> bool {
	let shape = constraint
		.numeric_shape
		.expect("numeric constraint should declare numeric_shape");
	match (shape, value) {
		(NumericShape::Decimal, ConstraintValue::Decimal(Some(_))) => true,
		(NumericShape::Integer, ConstraintValue::Decimal(Some(value))) => {
			value.fract().is_zero()
		}
		(shape, ConstraintValue::Text(Some(value))) => {
			let value = value.trim();
			match shape {
				NumericShape::Decimal => value.parse::<Decimal>().is_ok(),
				NumericShape::Integer => {
					!value.is_empty()
						&& value.bytes().all(|byte| byte.is_ascii_digit())
				}
				NumericShape::DottedVersion => {
					let mut parts = value.split('.');
					let valid_part = |part: &str| {
						!part.is_empty()
							&& part.bytes().all(|byte| byte.is_ascii_digit())
					};
					parts.next().is_some_and(valid_part)
						&& parts.next().is_some_and(valid_part)
						&& parts.next().is_none()
				}
			}
		}
		_ => panic!("numeric constraint received an incompatible value"),
	}
}

fn validate_format(
	constraint: &AllowedValueConstraint,
	value: ConstraintValue<'_>,
	vocabulary: &VocabularyContext,
) -> bool {
	let format = constraint
		.format_name
		.expect("format constraint should declare format_name");
	match (format, value) {
		(FormatName::E2bDatetime, ConstraintValue::Date(Some(_))) => true,
		(format, ConstraintValue::Text(Some(value))) => match format {
			FormatName::E2bDatetime => valid_e2b_datetime(value.trim()),
			FormatName::Base64 => {
				general_purpose::STANDARD.decode(value.trim()).is_ok()
			}
			FormatName::IchIdentifier => {
				valid_ich_identifier(value.trim(), vocabulary)
			}
		},
		_ => panic!("format constraint received an incompatible value"),
	}
}

fn valid_ich_identifier(value: &str, vocabulary: &VocabularyContext) -> bool {
	if value.chars().count() > 100 || value.chars().any(char::is_control) {
		return false;
	}
	let Some((country, remainder)) = value.split_once('-') else {
		return false;
	};
	let Some((organization, report_number)) = remainder.rsplit_once('-') else {
		return false;
	};
	let valid_country = vocabulary.contains_snapshot_code(
		"ISO3166",
		crate::VocabularyScope::All,
		country,
	);
	valid_country
		&& !organization.trim().is_empty()
		&& !report_number.trim().is_empty()
}

fn valid_e2b_datetime(value: &str) -> bool {
	let (local, offset) = match value
		.char_indices()
		.skip(4)
		.find(|(_, char)| matches!(char, '+' | '-'))
	{
		Some((index, _)) => (&value[..index], Some(&value[index..])),
		None => (value, None),
	};
	if let Some(offset) = offset {
		let bytes = offset.as_bytes();
		if bytes.len() != 5
			|| !matches!(bytes[0], b'+' | b'-')
			|| !bytes[1..].iter().all(u8::is_ascii_digit)
		{
			return false;
		}
		let hour = offset[1..3].parse::<u8>().ok();
		let minute = offset[3..5].parse::<u8>().ok();
		if !matches!((hour, minute), (Some(0..=14), Some(0..=59))) {
			return false;
		}
	}

	let (digits, fraction) = match local.split_once('.') {
		Some((digits, fraction)) => (digits, Some(fraction)),
		None => (local, None),
	};
	if !matches!(digits.len(), 4 | 6 | 8 | 10 | 12 | 14)
		|| !digits.bytes().all(|byte| byte.is_ascii_digit())
	{
		return false;
	}
	if let Some(fraction) = fraction {
		if digits.len() != 14
			|| fraction.is_empty()
			|| fraction.len() > 4
			|| !fraction.bytes().all(|byte| byte.is_ascii_digit())
		{
			return false;
		}
	}

	let number = |range: std::ops::Range<usize>| {
		digits.get(range).and_then(|value| value.parse::<u8>().ok())
	};
	if digits.len() >= 6 && !matches!(number(4..6), Some(1..=12)) {
		return false;
	}
	if digits.len() >= 8
		&& lib_core::serde::flex_date::e2b_datetime_date(digits).is_none()
	{
		return false;
	}
	if digits.len() >= 10 && !matches!(number(8..10), Some(0..=23)) {
		return false;
	}
	if digits.len() >= 12 && !matches!(number(10..12), Some(0..=59)) {
		return false;
	}
	if digits.len() >= 14 && !matches!(number(12..14), Some(0..=59)) {
		return false;
	}
	true
}

fn vocabulary_name_for_allowed_rule(rule_code: &str) -> Option<&'static str> {
	if rule_code.ends_with(".VOCABULARY") {
		return vocabulary_for_rule(rule_code);
	}
	let prefix = rule_code.strip_suffix(".ALLOWED.VALUE")?;
	vocabulary_for_rule(&format!("{prefix}.VOCABULARY"))
}

pub(crate) fn is_named_vocabulary_value_valid(
	vocabulary_name: &str,
	scope: crate::VocabularyScope,
	value: &str,
	vocabulary: &VocabularyContext,
) -> bool {
	match vocabulary_name {
		"MFDS_PRODUCT" | "WHODrug" => {
			vocabulary.contains_snapshot_code(vocabulary_name, scope, value.trim())
		}
		_ => false,
	}
}

fn validate_vocabulary(
	rule_code: &str,
	constraint: &AllowedValueConstraint,
	value: ConstraintValue<'_>,
	vocabulary: &VocabularyContext,
) -> bool {
	let value = text_value(value, "vocabulary");
	let value = value.trim();
	if let Some(profile) = constraint.identifier_profile {
		return validate_identifier(profile, value);
	}

	let scope = constraint.vocabulary_scope.expect(
		"vocabulary constraint should declare a scope or identifier profile",
	);
	match vocabulary_name_for_allowed_rule(rule_code) {
		Some("ISO3166") => {
			vocabulary.contains_snapshot_code("ISO3166", scope, value)
		}
		Some("ISO639") => {
			vocabulary.contains_snapshot_code("ISO639-2", scope, value)
		}
		Some("UCUM") if scope == crate::VocabularyScope::All => {
			octofhir_ucum::validate(value).is_ok()
		}
		Some("UCUM") => vocabulary.contains_snapshot_code("ICH-UCUM", scope, value),
		Some("EDQM") if rule_code.starts_with("ICH.G.k.4.r.9.2a.") => {
			vocabulary.contains_vocabulary_version("EDQM", value)
		}
		Some("EDQM") => vocabulary.contains_snapshot_code("EDQM", scope, value),
		Some(name) => panic!("unsupported vocabulary {name}: {rule_code}"),
		None => panic!("missing vocabulary metadata: {rule_code}"),
	}
}

fn validate_identifier(profile: IdentifierProfile, value: &str) -> bool {
	let max_length = match profile {
		IdentifierProfile::Mpid => 1000,
		IdentifierProfile::Phpid | IdentifierProfile::SubstanceId => 250,
	};
	!value.is_empty()
		&& value.len() <= max_length
		&& !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
	use super::{
		is_allowed_value_valid, is_named_vocabulary_value_valid, ConstraintValue,
	};
	use crate::context::VocabularyContext;
	use std::borrow::Cow;

	fn text(value: &str) -> ConstraintValue<'_> {
		ConstraintValue::Text(Some(Cow::Borrowed(value)))
	}

	#[test]
	fn release_backed_product_vocabularies_fail_closed() {
		let context = VocabularyContext::for_active_codes(&[
			("MFDS_PRODUCT", crate::VocabularyScope::ItemSeq, "KR123"),
			("WHODrug", crate::VocabularyScope::All, "FR456"),
		]);

		assert!(is_named_vocabulary_value_valid(
			"MFDS_PRODUCT",
			crate::VocabularyScope::ItemSeq,
			"KR123",
			&context,
		));
		assert!(is_named_vocabulary_value_valid(
			"WHODrug",
			crate::VocabularyScope::All,
			"FR456",
			&context,
		));
		assert!(!is_named_vocabulary_value_valid(
			"MFDS_PRODUCT",
			crate::VocabularyScope::ItemSeq,
			"missing",
			&context,
		));
	}

	#[test]
	fn empty_optional_values_are_valid() {
		assert!(is_allowed_value_valid(
			"ICH.F.r.3.2.ALLOWED.VALUE",
			ConstraintValue::Text(None),
			&VocabularyContext::default(),
		));
	}

	#[test]
	fn validates_catalog_code_set_and_true_marker() {
		let vocabulary = VocabularyContext::default();
		assert!(is_allowed_value_valid(
			"ICH.E.i.7.ALLOWED.VALUE",
			text("1"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.E.i.7.ALLOWED.VALUE",
			text("99"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.D.7.3.ALLOWED.VALUE",
			ConstraintValue::Boolean(Some(false)),
			&vocabulary,
		));
	}

	#[test]
	fn validates_numeric_shapes_without_partial_parses() {
		let vocabulary = VocabularyContext::default();
		assert!(is_allowed_value_valid(
			"ICH.F.r.3.2.ALLOWED.VALUE",
			text("12.5"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.F.r.3.2.ALLOWED.VALUE",
			text("12mg"),
			&vocabulary,
		));
		assert!(is_allowed_value_valid(
			"ICH.E.i.2.1a.ALLOWED.VALUE",
			text("27.1"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.E.i.2.1a.ALLOWED.VALUE",
			text("27"),
			&vocabulary,
		));
	}

	#[test]
	fn validates_datetime_base64_and_snapshot_vocabulary() {
		let vocabulary = VocabularyContext::default();
		assert!(is_allowed_value_valid(
			"ICH.N.2.r.4.ALLOWED.VALUE",
			text("20260713123000+0900"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.N.2.r.4.ALLOWED.VALUE",
			text("20260713123000+0900junk"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.N.2.r.4.ALLOWED.VALUE",
			text("20260713253000+0900"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.N.2.r.4.ALLOWED.VALUE",
			text("20260713123000+0960"),
			&vocabulary,
		));
		assert!(is_allowed_value_valid(
			"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
			text("SGVsbG8="),
			&vocabulary,
		));
		assert!(is_allowed_value_valid(
			"ICH.E.i.1.1b.ALLOWED.VALUE",
			text("eng"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.E.i.1.1b.ALLOWED.VALUE",
			text("en"),
			&vocabulary,
		));
	}

	#[test]
	fn validates_composed_ucum_expression() {
		assert!(is_allowed_value_valid(
			"ICH.F.r.3.3.ALLOWED.VALUE",
			text("mg/kg"),
			&VocabularyContext::default(),
		));
		assert!(!is_allowed_value_valid(
			"ICH.F.r.3.3.ALLOWED.VALUE",
			text("not-a-unit"),
			&VocabularyContext::default(),
		));
	}

	#[test]
	fn constrained_ucum_fails_closed_without_official_scope_snapshot() {
		assert!(!is_allowed_value_valid(
			"ICH.D.2.2b.ALLOWED.VALUE",
			text("a"),
			&VocabularyContext::default(),
		));
	}

	#[test]
	fn edqm_fails_closed_without_approved_snapshot() {
		assert!(!is_allowed_value_valid(
			"ICH.G.k.4.r.9.2b.ALLOWED.VALUE",
			text("example"),
			&VocabularyContext::default(),
		));
		assert!(!is_allowed_value_valid(
			"ICH.G.k.4.r.9.2a.ALLOWED.VALUE",
			text("2026-01-01"),
			&VocabularyContext::default(),
		));
		assert!(is_allowed_value_valid(
			"ICH.G.k.4.r.9.2a.ALLOWED.VALUE",
			text("2026-01-01"),
			&VocabularyContext::for_active_versions(&[("EDQM", "2026-01-01")]),
		));
	}

	#[test]
	fn country_and_ich_identifier_use_explicit_active_terminology() {
		let vocabulary = VocabularyContext::for_active_codes(&[
			("ISO3166", crate::VocabularyScope::All, "KR"),
			("ISO3166", crate::VocabularyScope::All, "EU"),
		]);
		assert!(is_allowed_value_valid(
			"ICH.C.2.r.3.VOCABULARY",
			text("KR"),
			&vocabulary,
		));
		assert!(!is_allowed_value_valid(
			"ICH.C.2.r.3.VOCABULARY",
			text("ZZ"),
			&vocabulary,
		));
		assert!(is_allowed_value_valid(
			"ICH.C.1.8.1.ALLOWED.VALUE",
			text("KR-ACME-2026-001"),
			&vocabulary,
		));
	}

	#[test]
	fn ich_identifiers_require_country_organization_and_report_components() {
		let vocabulary = VocabularyContext::for_active_codes(&[(
			"ISO3166",
			crate::VocabularyScope::All,
			"KR",
		)]);
		assert!(is_allowed_value_valid(
			"ICH.C.1.8.1.ALLOWED.VALUE",
			text("KR-ACME-2026-001"),
			&vocabulary,
		));
		for invalid in ["ACME-2026-001", "ZZ-ACME-001", "KR--001", "KR-ACME-"] {
			assert!(!is_allowed_value_valid(
				"ICH.C.1.8.1.ALLOWED.VALUE",
				text(invalid),
				&vocabulary,
			));
		}
	}
}
