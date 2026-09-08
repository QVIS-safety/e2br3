// Shared Section C policy used by exporter + case validators.
pub fn has_report_type(value: &str) -> bool {
	!value.trim().is_empty()
}

pub fn should_require_fda_local_criteria_report_type(
	fulfil_expedited_criteria: bool,
) -> bool {
	fulfil_expedited_criteria
}

pub fn should_clear_local_criteria_null_flavor_on_value() -> bool {
	true
}

pub fn should_clear_combination_product_null_flavor_on_value() -> bool {
	true
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
	fn c_section_null_flavor_clear_policy_tracks_export_policy() {
		assert!(should_clear_local_criteria_null_flavor_on_value());
		assert!(should_clear_combination_product_null_flavor_on_value());
	}
}
