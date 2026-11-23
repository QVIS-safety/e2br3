pub const EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE: &str =
	"ICH.XML.STRUCTURAL.EMPTY.PRUNE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPolicyDirective {
	RemoveEmptyStructuralNodes,
}
impl ExportPolicyDirective {
	pub fn as_str(self) -> &'static str {
		"remove_empty_structural_nodes"
	}
}
pub fn export_policy_directive_for_rule(
	code: &str,
) -> Option<ExportPolicyDirective> {
	match code {
		EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE => {
			Some(ExportPolicyDirective::RemoveEmptyStructuralNodes)
		}
		_ => None,
	}
}
pub fn has_export_policy_directive(
	code: &str,
	directive: ExportPolicyDirective,
) -> bool {
	export_policy_directive_for_rule(code) == Some(directive)
}

pub fn normalize_time_unit(value: &str) -> Option<&'static str> {
	match value.trim() {
		"800" | "10.a" | "decade" => Some("10.a"),
		"801" | "a" | "year" => Some("a"),
		"802" | "mo" | "month" => Some("mo"),
		"803" | "wk" | "week" => Some("wk"),
		"804" | "d" | "day" => Some("d"),
		"805" | "h" | "hour" => Some("h"),
		"min" | "minute" => Some("min"),
		"s" | "second" => Some("s"),
		_ => None,
	}
}

pub fn normalize_gestation_unit(value: &str) -> Option<&'static str> {
	match value.trim() {
		"{Trimester}" | "{trimester}" => Some("{trimester}"),
		value => match normalize_time_unit(value) {
			Some(unit @ ("mo" | "wk" | "d")) => Some(unit),
			_ => None,
		},
	}
}

pub fn export_xpath_for_rule(_code: &str) -> Option<&'static str> {
	None
}

pub fn export_xpaths_for_rule(code: &str) -> &'static [&'static str] {
	match code {
		EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE => &[
			"//hl7:outboundRelationship2",
			"//hl7:component",
			"//hl7:component1",
			"//hl7:subjectOf2",
			"//hl7:observation",
			"//hl7:organizer",
		],
		_ => &[],
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn export_policy_does_not_rewrite_invalid_draft_codes() {
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.PLACEHOLDER.VALUE.PRUNE"),
			None
		);
		assert!(export_xpaths_for_rule("ICH.XML.PLACEHOLDER.VALUE.PRUNE").is_empty());
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.STRUCTURAL.EMPTY.PRUNE"),
			Some(ExportPolicyDirective::RemoveEmptyStructuralNodes)
		);
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED"),
			None
		);
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED"),
			None
		);
		assert!(export_xpaths_for_rule("ICH.XML.STRUCTURAL.EMPTY.PRUNE")
			.contains(&"//hl7:observation"));
	}

	#[test]
	fn time_units_are_exported_as_constrained_ucum() {
		assert_eq!(normalize_time_unit("801"), Some("a"));
		assert_eq!(normalize_time_unit("day"), Some("d"));
		assert_eq!(normalize_gestation_unit("804"), Some("d"));
		assert_eq!(normalize_gestation_unit("{Trimester}"), Some("{trimester}"));
		assert_eq!(normalize_gestation_unit("{trimester}"), Some("{trimester}"));
		assert_eq!(normalize_gestation_unit("801"), None);
		assert_eq!(normalize_time_unit("fortnight"), None);
	}
}
