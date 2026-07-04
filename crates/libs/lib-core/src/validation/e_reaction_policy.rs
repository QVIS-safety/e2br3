// Shared Section E policy used by exporter + case validators.
// Keep behavior in one place to avoid drift when FDA submission behavior changes.
use crate::xml::export::policy::{
	has_export_policy_directive, ExportPolicyDirective,
};

pub const DEFAULT_OUTCOME_DISPLAY: &str = "not recovered/not resolved/ongoing";

pub fn normalize_outcome_code(value: Option<&str>) -> Option<&'static str> {
	match value.map(str::trim).filter(|v| !v.is_empty()) {
		Some("1") => Some("1"),
		Some("2") => Some("2"),
		Some("3") => Some("3"),
		Some("4") => Some("4"),
		Some("5") => Some("5"),
		_ => None,
	}
}

pub fn outcome_display_name(code: &str) -> &'static str {
	match code {
		"1" => "recovered/resolved",
		"2" => "recovering/resolving",
		"3" => "not recovered/not resolved/ongoing",
		"4" => "recovered/resolved with sequelae",
		"5" => "fatal",
		_ => DEFAULT_OUTCOME_DISPLAY,
	}
}

pub fn should_emit_required_intervention_null_flavor_ni() -> bool {
	has_export_policy_directive(
		"FDA.E.i.3.2h.REQUIRED",
		ExportPolicyDirective::RequiredInterventionNullFlavorNi,
	)
}

pub fn should_case_validator_require_required_intervention() -> bool {
	true
}

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
		assert!(should_case_validator_require_required_intervention());
	}
}
