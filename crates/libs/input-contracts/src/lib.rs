mod helpers;

pub mod generated;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputValue<'a> {
	Missing,
	Null,
	String(&'a str),
	Boolean(bool),
	Number(&'a serde_json::Number),
	InvalidType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldInput<'a> {
	pub value: InputValue<'a>,
	pub null_flavor: Option<&'a str>,
}

impl<'a> FieldInput<'a> {
	pub const fn new(value: InputValue<'a>, null_flavor: Option<&'a str>) -> Self {
		Self { value, null_flavor }
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputIssue {
	pub code: &'static str,
	pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum NumericShape {
	Decimal,
	DottedVersion,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FormatName {
	E2bDatetime,
}

#[cfg(test)]
mod tests {
	use super::{generated, FieldInput, InputValue};

	#[test]
	fn study_type_accepts_e2d_r1_codes_but_not_unknown_codes() {
		for value in ["1", "2", "3", "4", "5", "6", ""] {
			assert!(
				generated::c::c_5_4(FieldInput::new(
					InputValue::String(value),
					None
				))
				.is_empty(),
				"{value}"
			);
		}
		for value in [InputValue::Missing, InputValue::Null] {
			assert!(generated::c::c_5_4(FieldInput::new(value, None)).is_empty());
		}
		for value in ["0", "7", "10"] {
			assert!(
				generated::c::c_5_4(FieldInput::new(
					InputValue::String(value),
					None
				))
				.iter()
				.any(|issue| issue.code == "ICH.C.5.4.ALLOWED.VALUE"),
				"{value}"
			);
		}
	}

	#[test]
	fn f_r_2_1_blank_is_left_to_conditional_business_validator() {
		assert!(
			generated::f::f_r_2_1(FieldInput::new(InputValue::Null, None,))
				.is_empty()
		);
	}

	#[test]
	fn migrated_business_rules_do_not_reject_input() {
		assert!(generated::c::c_1_6_1_r_2(FieldInput::new(
			InputValue::String("not-base64"),
			None,
		))
		.is_empty());
		assert!(!generated::c::c_4_r_2(FieldInput::new(
			InputValue::String("not-base64"),
			None,
		))
		.is_empty());
		assert!(generated::c::c_4_r_2(FieldInput::new(
			InputValue::String("UEZERg=="),
			None,
		))
		.is_empty());
		assert!(!generated::d::d_6(FieldInput::new(
			InputValue::Missing,
			Some("NASK"),
		))
		.is_empty());
		assert!(generated::d::d_6(FieldInput::new(
			InputValue::Missing,
			Some("MSK"),
		))
		.is_empty());
		assert!(generated::e::e_i_2_1a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::d::d_7_1_r_1a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::d::d_10_7_1_r_1a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::g::g_k_7_r_2a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::f::f_r_2_2a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::h::h_3_r_1a(FieldInput::new(
			InputValue::String("15"),
			None,
		))
		.is_empty());
		assert!(generated::e::e_i_4(FieldInput::new(
			InputValue::String("200509211242-08"),
			None,
		))
		.is_empty());
		assert!(!generated::c::c_1_4(FieldInput::new(
			InputValue::String("202206"),
			None,
		))
		.is_empty());
	}

	#[test]
	fn non_nullable_storage_fields_reject_null_and_blank() {
		for check in [
			generated::c::c_1_1,
			generated::g::g_k_1,
			generated::g::g_k_2_2,
			generated::h::h_1,
		] {
			let issues = check(FieldInput::new(InputValue::Null, None));
			assert_eq!(issues.len(), 1);
			assert_eq!(issues[0].code, "INPUT.NULL.REJECTED");
			assert_eq!(
				check(FieldInput::new(InputValue::String("  "), None))[0].code,
				"INPUT.NULL.REJECTED"
			);
			assert!(check(FieldInput::new(InputValue::Missing, None)).is_empty());
		}
	}

	#[test]
	fn business_required_editor_fields_allow_draft_clear() {
		for check in [
			generated::c::c_1_6_1,
			generated::c::c_1_8_1,
			generated::c::c_1_8_2,
		] {
			assert!(check(FieldInput::new(InputValue::Null, None)).is_empty());
			assert!(check(FieldInput::new(InputValue::Missing, None)).is_empty());
		}
		assert!(
			generated::d::d_1(FieldInput::new(InputValue::Null, None)).is_empty()
		);
		assert!(
			generated::d::d_1(FieldInput::new(InputValue::Null, Some("UNK"),))
				.is_empty()
		);
		assert!(
			generated::e::e_i_2_1a(FieldInput::new(InputValue::Null, None))
				.is_empty()
		);
		assert!(
			generated::e::e_i_2_1a(FieldInput::new(InputValue::Missing, None))
				.is_empty()
		);
	}

	#[test]
	fn device_code_rows_reject_empty_values_and_mfds_device_text_matches_storage() {
		for check in [
			generated::g::fda_g_k_12_r_2_r,
			generated::g::fda_g_k_12_r_3_r,
			generated::g::fda_g_k_12_r_11_r,
		] {
			let issues = check(FieldInput::new(InputValue::String(""), None));
			assert!(issues
				.iter()
				.any(|issue| issue.code == "INPUT.EMPTY.REJECTED"));
		}
		for check in [
			generated::e::local_mfds_device_cause_other,
			generated::e::local_mfds_device_action_reason,
			generated::e::local_mfds_device_action_other,
		] {
			assert!(check(FieldInput::new(
				InputValue::String(&"x".repeat(20000)),
				None,
			))
			.is_empty());
			assert!(!check(FieldInput::new(
				InputValue::String(&"x".repeat(20001)),
				None,
			))
			.is_empty());
		}
	}
}
