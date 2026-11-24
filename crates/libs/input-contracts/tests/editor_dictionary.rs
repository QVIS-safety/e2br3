use input_contracts::{FieldInput, InputIssue, InputValue};
use serde_json::Value;
use std::{error::Error, fs, path::Path};

type TestResult = Result<(), Box<dyn Error>>;

fn verify(
	page: &str,
	field_code: &str,
	source_code: &str,
	property: &str,
	validate: fn(FieldInput<'_>) -> Vec<InputIssue>,
) -> TestResult {
	let path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../../registry/editor-contracts")
		.join(format!("{}.json", page.to_lowercase()));
	let document: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
	let field = document["fields"]
		.as_array()
		.ok_or("missing fields")?
		.iter()
		.find(|field| field["code"] == field_code)
		.ok_or("missing field")?;
	let binding = field["dictionaryInputs"]
		.as_array()
		.ok_or("missing dictionaryInputs")?
		.iter()
		.find(|binding| binding["code"] == source_code)
		.ok_or("missing binding")?;
	let check = &binding["checks"][property];
	let rule = check["ruleCode"].as_str().ok_or("missing ruleCode")?;
	for (group, rejected) in [("valid", false), ("invalid", true)] {
		let cases = check[group].as_array().ok_or("missing boundary cases")?;
		assert!(!cases.is_empty(), "{page}/{source_code}/{property}/{group}");
		for (index, case) in cases.iter().enumerate() {
			let raw = case.get("value").ok_or("missing value")?;
			let value = if raw.is_object() {
				let text = raw["repeat"].as_str().ok_or("missing repeat string")?;
				let count = usize::try_from(
					raw["count"].as_u64().ok_or("invalid repeat count")?,
				)?;
				Value::String(text.repeat(count))
			} else {
				raw.clone()
			};
			let input = match &value {
				Value::Null => InputValue::Null,
				Value::String(value) => InputValue::String(value),
				Value::Bool(value) => InputValue::Boolean(*value),
				Value::Number(value) => InputValue::Number(value),
				_ => InputValue::InvalidType,
			};
			let null_flavor =
				match case.get("nullFlavor").ok_or("missing nullFlavor")? {
					Value::Null => None,
					Value::String(value) => Some(value.as_str()),
					_ => return Err("invalid nullFlavor fixture".into()),
				};
			let issues = validate(FieldInput::new(input, null_flavor));
			// Boundaries isolate one rule: a max-length string need not be a valid code-set member.
			assert_eq!(issues.iter().any(|issue| issue.code == rule), rejected,
                "{page}/{source_code}/{property}/{group}[{index}]: expected {rule}; got {issues:?}");
		}
	}
	Ok(())
}

macro_rules! dictionary_test {
	($name:ident, $page:literal, $field:literal, $source:literal, $property:literal, $function:path) => {
		#[test]
		fn $name() -> TestResult {
			verify($page, $field, $source, $property, $function)
		}
	};
}

include!("editor_dictionary/generated.rs");

fn verify_fda_max_length(
	code: &str,
	validate: fn(FieldInput<'_>) -> Vec<InputIssue>,
) -> TestResult {
	let path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../../registry/dictionary/ich-e2br3.json");
	let document: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
	let entry = document["entries"]
		.as_array()
		.ok_or("missing entries")?
		.iter()
		.find(|entry| entry["code"] == code)
		.ok_or("missing dictionary entry")?;
	assert_eq!(entry["profile_max_lengths"]["fda"], "20");
	let valid = "x".repeat(20);
	let invalid = "x".repeat(21);
	assert!(validate(FieldInput::new(InputValue::String(&valid), None)).is_empty());
	assert!(
		validate(FieldInput::new(InputValue::String(&invalid), None))
			.iter()
			.any(|issue| issue.code == format!("FDA.{code}.LENGTH.MAX"))
	);
	Ok(())
}

#[test]
fn fda_mpid_version_limits_match_dictionary() -> TestResult {
	verify_fda_max_length(
		"G.k.2.1.1a",
		input_contracts::generated::g::fda_g_k_2_1_1a,
	)?;
	verify_fda_max_length("D.8.r.2a", input_contracts::generated::d::fda_d_8_r_2a)
}

#[test]
fn official_seriousness_markers_reject_false_at_input() {
	use input_contracts::generated::{c, e};
	for validate in [
		e::e_i_3_2a,
		e::e_i_3_2b,
		e::e_i_3_2c,
		e::e_i_3_2d,
		e::e_i_3_2e,
		e::e_i_3_2f,
	] {
		assert!(
			!validate(FieldInput::new(InputValue::Boolean(false), None)).is_empty()
		);
	}
	for validate in [c::c_1_9_1, e::fda_e_i_3_2h] {
		assert!(
			validate(FieldInput::new(InputValue::Boolean(false), None)).is_empty()
		);
	}
	assert!(c::c_1_8_1(FieldInput::new(
		InputValue::String("legacy/received-id"),
		None
	))
	.is_empty());
}
