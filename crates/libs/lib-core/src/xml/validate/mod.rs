// Case-level validation contract shared by regional validators.

mod c_reporter_policy;
mod c_safety_report_policy;
mod case_detector_registry;
mod catalog;
mod d_patient_policy;
mod e_reaction_policy;
mod export_transform_registry;
mod f_test_result_policy;
mod g_drug_policy;
mod h_narrative_policy;
pub mod rule_layer_contract;
pub mod rule_test_matrix;
mod xml_detector_registry;

pub use c_reporter_policy::has_any_primary_source_content;
pub use c_safety_report_policy::{
	has_report_type, should_clear_combination_product_null_flavor_on_value,
	should_clear_local_criteria_null_flavor_on_value,
	should_require_fda_local_criteria_report_type,
	should_warn_fda_combination_product_indicator_missing,
};
pub use case_detector_registry::*;
pub use catalog::{
	canonical_rules_all, canonical_rules_for_phase, canonical_rules_for_profile,
	canonical_rules_for_profile_phase, canonical_rules_version,
	export_attribute_strip_spec_for_rule, export_directive_for_rule,
	export_normalization_spec_for_rule, export_xpath_for_rule,
	export_xpaths_for_rule, find_canonical_rule, find_canonical_rule_for_phase,
	has_export_directive, is_rule_condition_satisfied, is_rule_presence_valid,
	is_rule_value_valid, should_clear_null_flavor_on_value, CanonicalRule,
	ExportAttributeStripSpec, ExportDirective, ExportNormalizationSpec,
	ExportNormalizeKind, RuleCategory, RuleCondition, RuleFacts, RuleSeverity,
	ValidationPhase, ValidationRuleMetadata, CANONICAL_RULES, VALIDATION_RULES,
};
pub use d_patient_policy::{
	has_fda_ethnicity, has_fda_race, has_patient_initials, has_patient_payload,
	should_require_fda_ethnicity, should_require_fda_race,
	should_require_patient_initials,
};
pub use e_reaction_policy::{
	normalize_outcome_code, outcome_display_name,
	should_case_validator_require_required_intervention,
	should_emit_required_intervention_null_flavor_ni,
};
pub use export_transform_registry::*;
pub use f_test_result_policy::{has_test_name, has_test_payload};
pub use g_drug_policy::{
	drug_characterization_display_name, has_drug_characterization,
	has_medicinal_product, normalize_drug_characterization,
};
pub use h_narrative_policy::{
	has_case_narrative, has_narrative_payload, should_require_case_narrative,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
pub use xml_detector_registry::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationProfile {
	Ich,
	Fda,
	Mfds,
}

impl ValidationProfile {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Ich => "ich",
			Self::Fda => "fda",
			Self::Mfds => "mfds",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value.trim().to_ascii_lowercase().as_str() {
			"ich" => Some(Self::Ich),
			"fda" => Some(Self::Fda),
			"mfds" => Some(Self::Mfds),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
	pub code: String,
	pub message: String,
	pub path: String,
	pub section: String,
	pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseValidationReport {
	pub profile: String,
	pub case_id: Uuid,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub issues: Vec<ValidationIssue>,
}

pub fn has_text(value: Option<&str>) -> bool {
	value.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

pub fn push_issue_by_code(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: impl Into<String>,
) {
	let path = path.into();
	if let Some(rule) =
		find_canonical_rule_for_phase(code, ValidationPhase::CaseValidate)
	{
		issues.push(ValidationIssue {
			code: rule.code.to_string(),
			message: rule.message.to_string(),
			path,
			section: rule.section.to_string(),
			blocking: rule.blocking,
		});
	} else {
		issues.push(ValidationIssue {
			code: code.to_string(),
			message: code.to_string(),
			path,
			section: "unknown".to_string(),
			blocking: false,
		});
	}
}

pub fn push_issue_if_rule_invalid(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: impl Into<String>,
	value_code: Option<&str>,
	null_flavor: Option<&str>,
	facts: RuleFacts,
) -> bool {
	if is_rule_condition_satisfied(code, facts)
		&& !is_rule_value_valid(code, value_code, null_flavor, facts)
	{
		push_issue_by_code(issues, code, path);
		return true;
	}
	false
}

pub fn push_issue_if_conditioned_value_invalid(
	issues: &mut Vec<ValidationIssue>,
	condition_code: &str,
	value_rule_code: &str,
	issue_code: &str,
	path: impl Into<String>,
	value_code: Option<&str>,
	null_flavor: Option<&str>,
	condition_facts: RuleFacts,
	value_facts: RuleFacts,
) -> bool {
	if is_rule_condition_satisfied(condition_code, condition_facts)
		&& !is_rule_value_valid(
			value_rule_code,
			value_code,
			null_flavor,
			value_facts,
		) {
		push_issue_by_code(issues, issue_code, path);
		return true;
	}
	false
}

pub fn push_issue_if_condition_violated(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: impl Into<String>,
	facts: RuleFacts,
) -> bool {
	if is_rule_condition_satisfied(code, facts) {
		push_issue_by_code(issues, code, path);
		return true;
	}
	false
}

pub fn build_report(
	profile: ValidationProfile,
	case_id: Uuid,
	issues: Vec<ValidationIssue>,
) -> CaseValidationReport {
	let blocking_count = issues.iter().filter(|issue| issue.blocking).count();
	let non_blocking_count = issues.len().saturating_sub(blocking_count);
	CaseValidationReport {
		profile: profile.as_str().to_string(),
		case_id,
		ok: blocking_count == 0,
		blocking_count,
		non_blocking_count,
		issues,
	}
}
