use crate::{FieldInput, InputIssue, InputValue};

pub type Check = for<'a> fn(FieldInput<'a>) -> Vec<InputIssue>;

enum Value {
	String(Option<String>),
	Boolean {
		raw: Option<String>,
		value: Option<bool>,
	},
	Number(Option<String>),
}

struct Field {
	path: String,
	value: Value,
	null_flavor: Option<String>,
	check: Check,
}

#[derive(Default)]
pub struct InputRecord {
	fields: Vec<Field>,
}

#[derive(Debug)]
pub struct RecordIssue {
	pub path: String,
	pub issue: InputIssue,
}

/// The shared value/nullFlavor invariant applies before field-specific rules.
pub fn check_input(
	input: FieldInput<'_>,
	check: impl FnOnce(FieldInput<'_>) -> Vec<InputIssue>,
) -> Vec<InputIssue> {
	if !matches!(input.value, InputValue::Missing | InputValue::Null)
		&& input.null_flavor.is_some()
	{
		return vec![InputIssue {
			code: "INPUT.VALUE_NULLFLAVOR.CONFLICT",
			message: "value and nullFlavor cannot both be present".into(),
		}];
	}
	check(input)
}

impl InputRecord {
	pub fn string(
		&mut self,
		path: &str,
		value: Option<&str>,
		null_flavor: Option<&str>,
		check: Check,
	) {
		self.fields.push(Field {
			path: path.into(),
			value: Value::String(value.map(str::to_owned)),
			null_flavor: null_flavor.map(str::to_owned),
			check,
		});
	}

	pub fn boolean(
		&mut self,
		path: &str,
		raw: Option<&str>,
		value: Option<bool>,
		null_flavor: Option<&str>,
		check: Check,
	) {
		self.fields.push(Field {
			path: path.into(),
			value: Value::Boolean {
				raw: raw.map(str::to_owned),
				value,
			},
			null_flavor: null_flavor.map(str::to_owned),
			check,
		});
	}

	pub fn number_string(&mut self, path: &str, value: Option<&str>, check: Check) {
		self.fields.push(Field {
			path: path.into(),
			value: Value::Number(value.map(str::to_owned)),
			null_flavor: None,
			check,
		});
	}

	pub fn base64(&mut self, path: &str, value: Option<&str>) {
		self.string(path, value, None, |input| {
			let mut issues = Vec::new();
			crate::helpers::base64(&mut issues, "INPUT.BASE64", input.value);
			issues
		});
	}

	pub fn validate(&self) -> Result<(), RecordIssue> {
		for field in &self.fields {
			let number;
			let input = match &field.value {
				Value::String(value) => match value.as_deref() {
					Some(value) => InputValue::String(value),
					None => InputValue::Missing,
				},
				Value::Boolean { raw, value } => match (raw, value) {
					(_, Some(value)) => InputValue::Boolean(*value),
					(Some(_), None) => InputValue::InvalidType,
					(None, None) => InputValue::Missing,
				},
				Value::Number(value) => match value {
					Some(value) => {
						number = normalize_decimal_lexeme(value)
							.parse::<serde_json::Number>();
						match &number {
							Ok(number) => InputValue::Number(number),
							Err(_) => InputValue::InvalidType,
						}
					}
					None => InputValue::Missing,
				},
			};
			if matches!(input, InputValue::InvalidType) {
				return Err(RecordIssue {
					path: field.path.clone(),
					issue: InputIssue {
						code: "INPUT.TYPE",
						message: "invalid boolean or numeric value".into(),
					},
				});
			}
			if let Some(issue) = check_input(
				FieldInput::new(input, field.null_flavor.as_deref()),
				field.check,
			)
			.into_iter()
			.next()
			{
				return Err(RecordIssue {
					path: field.path.clone(),
					issue,
				});
			}
		}
		Ok(())
	}
}

pub fn normalize_decimal_lexeme(value: &str) -> String {
	let value = value.trim();
	let value = match value.strip_prefix('+') {
		Some(value) => value,
		None => value,
	};
	match value.as_bytes() {
		[b'.', ..] => format!("0{value}"),
		[b'-', b'.', ..] => format!("-0{}", &value[1..]),
		_ => value.to_owned(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::generated::{d, g, h};

	#[test]
	fn record_preserves_distinct_supplied_values_and_uses_field_rules() {
		let mut input = InputRecord::default();
		input.string("empty product", Some(""), None, g::g_k_2_2);
		input.string("text", Some(" text & <xml> "), None, h::h_1);
		input.string("sex", Some("0"), None, d::d_5);
		input.string("parent sex", Some("0"), None, d::d_10_6);
		input.boolean("autopsy", Some("false"), Some(false), None, d::d_9_3);
		input.number_string("age", Some("0"), d::d_2_2a);
		input.string("initials", None, Some("NASK"), d::d_1);
		input.validate().expect("storable draft inputs");
		input.string("sex", Some("9"), None, d::d_5);
		let error = input
			.validate()
			.expect_err("field's allowed values still apply");
		assert_eq!(error.path, "sex");
		assert_eq!(error.issue.code, "ICH.D.5.ALLOWED.VALUE");
	}

	#[test]
	fn record_rejects_conflicts_and_invalid_null_flavors() {
		for value in ["", "given"] {
			let mut input = InputRecord::default();
			input.string("initials", Some(value), Some("NASK"), d::d_1);
			assert_eq!(
				input.validate().unwrap_err().issue.code,
				"INPUT.VALUE_NULLFLAVOR.CONFLICT"
			);
		}
		let mut input = InputRecord::default();
		input.boolean("autopsy", Some("false"), Some(false), Some("UNK"), d::d_9_3);
		assert_eq!(
			input.validate().unwrap_err().issue.code,
			"INPUT.VALUE_NULLFLAVOR.CONFLICT"
		);
		let mut input = InputRecord::default();
		input.string("initials", None, Some("INVALID"), d::d_1);
		assert_eq!(
			input.validate().unwrap_err().issue.code,
			"ICH.D.1.NULLFLAVOR.ALLOWED"
		);
	}

	#[test]
	fn malformed_scalars_cannot_become_missing() {
		let mut input = InputRecord::default();
		input.boolean("autopsy", Some("invalid"), None, None, d::d_9_3);
		assert_eq!(input.validate().unwrap_err().issue.code, "INPUT.TYPE");
		let mut input = InputRecord::default();
		input.number_string("age", Some("invalid"), d::d_2_2a);
		assert_eq!(input.validate().unwrap_err().issue.code, "INPUT.TYPE");
		let mut input = InputRecord::default();
		input.base64("document", Some("A"));
		assert_eq!(input.validate().unwrap_err().issue.code, "INPUT.BASE64");
	}
}
