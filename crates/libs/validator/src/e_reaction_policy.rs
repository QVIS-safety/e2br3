pub use lib_core::xml::export::policy::{
	normalize_outcome_code, outcome_display_name,
	should_case_validation_require_required_intervention,
	should_emit_required_intervention_null_flavor_ni,
};

#[cfg(test)]
pub use lib_core::xml::export::policy::DEFAULT_OUTCOME_DISPLAY;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_outcome_code_rejects_missing_or_invalid() {
		assert_eq!(normalize_outcome_code(None), None);
		assert_eq!(normalize_outcome_code(Some("")), None);
		assert_eq!(normalize_outcome_code(Some("99")), None);
	}

	#[test]
	fn normalize_outcome_code_preserves_valid_values() {
		assert_eq!(normalize_outcome_code(Some("1")), Some("1"));
		assert_eq!(normalize_outcome_code(Some("5")), Some("5"));
	}

	#[test]
	fn display_name_mapping_is_stable() {
		assert_eq!(outcome_display_name("1"), "recovered/resolved");
		assert_eq!(outcome_display_name("3"), DEFAULT_OUTCOME_DISPLAY);
	}

	#[test]
	fn required_intervention_policy_tracks_export_policy() {
		assert!(should_emit_required_intervention_null_flavor_ni());
		assert!(should_case_validation_require_required_intervention());
	}
}
