use crate::{Error, Result};
use base64::engine::{general_purpose, Engine};
use input_contracts::{FieldInput, InputIssue, InputValue};

pub(crate) fn string<F>(
	field: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
	_check_field: F,
) -> Result<()>
where
	F: for<'a> Fn(FieldInput<'a>) -> Vec<InputIssue>,
{
	reject_value_and_null_flavor(field, value.is_some(), null_flavor)?;
	reject_input_issues(
		field,
		_check_field(FieldInput::new(
				match value {
					Some(value) => InputValue::String(value),
					None => InputValue::Missing,
				},
			null_flavor,
		)),
	)
}

pub(crate) fn base64(field: &str, value: Option<&str>) -> Result<()> {
	if value.is_some_and(|value| general_purpose::STANDARD.decode(value).is_err()) {
		return Err(Error::InvalidXml {
			message: format!("{field}: invalid Base64 value"),
			line: None,
			column: None,
		});
	}
	Ok(())
}

pub(crate) fn boolean<F>(
	field: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
	_check_field: F,
) -> Result<()>
where
	F: for<'a> Fn(FieldInput<'a>) -> Vec<InputIssue>,
{
	reject_value_and_null_flavor(field, value.is_some(), null_flavor)?;
	reject_input_issues(
		field,
		_check_field(FieldInput::new(
				match value {
					Some(value) => InputValue::Boolean(value),
					None => InputValue::Missing,
				},
			null_flavor,
		)),
	)
}

fn reject_value_and_null_flavor(
	field: &str,
	has_value: bool,
	null_flavor: Option<&str>,
) -> Result<()> {
	if has_value && null_flavor.is_some() {
		return Err(Error::InvalidXml {
			message: format!("{field}: value and nullFlavor cannot both be present"),
			line: None,
			column: None,
		});
	}
	Ok(())
}

fn reject_input_issues(field: &str, issues: Vec<InputIssue>) -> Result<()> {
	if let Some(issue) = issues.into_iter().next() {
		return Err(Error::InvalidXml {
			message: format!("{field}: {}: {}", issue.code, issue.message),
			line: None,
			column: None,
		});
	}
	Ok(())
}

pub(crate) fn number_string<F>(
	field: &str,
	value: Option<&str>,
	check_field: F,
) -> Result<()>
where
	F: for<'a> Fn(FieldInput<'a>) -> Vec<InputIssue>,
{
	let number = value
		.map(normalize_decimal_lexeme)
		.map(|value| value.parse::<serde_json::Number>())
		.transpose()
		.map_err(|_| Error::InvalidXml {
			message: format!("{field}: invalid numeric value"),
			line: None,
			column: None,
		})?;
	reject_input_issues(
		field,
		check_field(FieldInput::new(
				match number.as_ref() {
					Some(number) => InputValue::Number(number),
					None => InputValue::Missing,
				},
			None,
		)),
	)
}

pub(crate) fn normalize_decimal_lexeme(value: &str) -> String {
	let trimmed = value.trim();
	match trimmed.as_bytes() {
		[b'.', ..] => format!("0{trimmed}"),
		[b'-', b'.', ..] => format!("-0{}", &trimmed[1..]),
		[b'+', b'.', ..] => format!("+0{}", &trimmed[1..]),
		_ => trimmed.to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::{base64, normalize_decimal_lexeme, string};

	#[test]
	fn rejects_invalid_base64_storage_input() {
		assert!(base64("includedDocument", Some("A")).is_err());
		base64("includedDocument", Some("QUJD")).expect("valid Base64");
	}

	#[test]
	fn business_rules_do_not_block_import_parsing() {
		string(
			"safetyReportId",
			Some("invalid"),
			None,
			input_contracts::generated::c::c_1_1,
		)
		.expect("business validation runs after import");
	}

	#[test]
	fn storage_contracts_block_unstorable_values() {
		let err = string(
			"safetyReportId",
			Some(&"X".repeat(101)),
			None,
			input_contracts::generated::c::c_1_1,
		)
		.unwrap_err();
		assert!(err.to_string().contains("ICH.C.1.1.LENGTH.MAX"));
	}

	#[test]
	fn rejects_value_with_null_flavor() {
		let err = string(
			"testDate",
			Some("20260810"),
			Some("UNK"),
			input_contracts::generated::f::f_r_1,
		)
		.unwrap_err();
		assert!(err
			.to_string()
			.contains("value and nullFlavor cannot both be present"));
	}

	#[test]
	fn accepts_xml_decimal_leading_point() {
		assert_eq!(normalize_decimal_lexeme(".5"), "0.5");
		assert_eq!(normalize_decimal_lexeme("-.5"), "-0.5");
		assert_eq!(normalize_decimal_lexeme(" 1.5 "), "1.5");
	}
}
