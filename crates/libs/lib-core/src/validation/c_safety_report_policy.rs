// Shared Section C policy used by exporter + case validators.
use super::{
	is_rule_condition_satisfied, should_clear_null_flavor_on_value, RuleFacts,
};

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
	fn c_section_null_flavor_clear_policy_tracks_catalog() {
		assert!(should_clear_local_criteria_null_flavor_on_value());
		assert!(should_clear_combination_product_null_flavor_on_value());
	}
}
