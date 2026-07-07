// Shared Section C policy used by exporter + case validators.
use super::{is_rule_condition_satisfied, RuleFacts};
use lib_core::xml::export::policy::should_clear_null_flavor_on_value;

pub fn has_report_type(value: &str) -> bool {
	!value.trim().is_empty()
}

pub fn should_require_fda_local_criteria_report_type(
	fulfil_expedited_criteria: bool,
) -> bool {
	is_rule_condition_satisfied(
		"FDA.C.1.7.1.REQUIRED",
		RuleFacts {
			fda_fulfil_expedited_criteria: Some(fulfil_expedited_criteria),
			..RuleFacts::default()
		},
	)
}

pub fn should_warn_fda_combination_product_indicator_missing() -> bool {
	is_rule_condition_satisfied("FDA.C.1.12.RECOMMENDED", RuleFacts::default())
}

pub fn should_clear_local_criteria_null_flavor_on_value() -> bool {
	should_clear_null_flavor_on_value("FDA.C.1.7.1.REQUIRED")
}

pub fn should_clear_combination_product_null_flavor_on_value() -> bool {
	should_clear_null_flavor_on_value("FDA.C.1.12.REQUIRED")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::is_rule_value_valid;

	fn is_fda_local_criteria_report_type_allowed(value: &str) -> bool {
		is_rule_value_valid(
			"FDA.C.1.7.1.REQUIRED",
			Some(value),
			None,
			RuleFacts {
				fda_fulfil_expedited_criteria: Some(true),
				..RuleFacts::default()
			},
		)
	}

	fn is_fda_combination_product_indicator_allowed(value: &str) -> bool {
		is_rule_value_valid(
			"FDA.C.1.12.REQUIRED",
			Some(value),
			None,
			RuleFacts::default(),
		)
	}

	#[test]
	fn report_type_presence_is_trim_aware() {
		assert!(has_report_type("1"));
		assert!(!has_report_type(""));
		assert!(!has_report_type("   "));
	}

	#[test]
	fn local_criteria_requirement_is_conditional_on_expedited() {
		assert!(should_require_fda_local_criteria_report_type(true));
		assert!(!should_require_fda_local_criteria_report_type(false));
	}

	#[test]
	fn c_section_null_flavor_clear_policy_tracks_export_policy() {
		assert!(should_clear_local_criteria_null_flavor_on_value());
		assert!(should_clear_combination_product_null_flavor_on_value());
	}

	#[test]
	fn fda_local_criteria_report_type_uses_reference_value_contract() {
		for value in ["1", "2", "4", "5", "6"] {
			assert!(is_fda_local_criteria_report_type_allowed(value), "{value}");
		}

		assert!(!is_fda_local_criteria_report_type_allowed("3"));
		assert!(!is_fda_local_criteria_report_type_allowed(""));
	}

	#[test]
	fn fda_combination_product_indicator_uses_boolean_string_contract() {
		assert!(is_fda_combination_product_indicator_allowed("false"));
		assert!(is_fda_combination_product_indicator_allowed("true"));

		for value in ["1", "2", "3", ""] {
			assert!(
				!is_fda_combination_product_indicator_allowed(value),
				"{value}"
			);
		}
	}
}
