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
#[path = "tests/c_safety_report_policy.rs"]
mod tests;
