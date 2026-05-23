mod c_reporter_policy;
mod c_safety_report_policy;
pub mod case;
mod catalog;
mod context;
mod d_patient_policy;
mod e_reaction_policy;
mod f_test_result_policy;
mod fda_context;
mod g_drug_policy;
mod h_narrative_policy;
mod mfds_context;
pub mod xml;

pub use self::xml::export_rules::*;
pub use self::xml::shared_specs::*;
pub use crate::regulatory::*;
pub use c_reporter_policy::has_any_primary_source_content;
pub use c_safety_report_policy::{
	has_report_type, should_clear_combination_product_null_flavor_on_value,
	should_clear_local_criteria_null_flavor_on_value,
	should_require_fda_local_criteria_report_type,
	should_warn_fda_combination_product_indicator_missing,
};
pub use case::{validate_case_for_authority, validate_case_for_authorities};
pub use catalog::*;
pub use context::{load_base_validation_context, ValidationContext};
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
pub use f_test_result_policy::{has_test_name, has_test_payload};
pub use fda_context::{
	list_drug_characteristics, list_study_registrations,
	load_fda_validation_context, FdaValidationContext,
};
pub use g_drug_policy::{
	drug_characterization_display_name, has_drug_characterization,
	has_medicinal_product, normalize_drug_characterization,
};
pub use h_narrative_policy::{
	has_case_narrative, has_narrative_payload, should_require_case_narrative,
};
pub use mfds_context::{
	load_mfds_validation_context, MfdsValidationContext, ParentPastDrugByCase,
	PastDrugByCase, RelatednessWithDrug,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
	pub code: String,
	pub message: String,
	pub path: String,
	pub field_path: Option<String>,
	pub section: String,
	pub subsection: String,
	pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSectionSummary {
	pub section: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSubsectionSummary {
	pub section: String,
	pub subsection: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseValidationReport {
	#[serde(default)]
	pub authority: String,
	pub case_id: Uuid,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub section_summaries: Vec<ValidationSectionSummary>,
	pub subsection_summaries: Vec<ValidationSubsectionSummary>,
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
		let field_path =
			case::sections::resolve_validation_field_path(code, Some(&path));
		let subsection =
			case::sections::resolve_validation_subsection(code, Some(&path));
		issues.push(ValidationIssue {
			code: rule.code.to_string(),
			message: rule.message.to_string(),
			field_path,
			path,
			section: rule.section.to_string(),
			subsection,
			blocking: rule.blocking,
		});
	} else {
		let field_path =
			case::sections::resolve_validation_field_path(code, Some(&path));
		let subsection =
			case::sections::resolve_validation_subsection(code, Some(&path));
		issues.push(ValidationIssue {
			code: code.to_string(),
			message: code.to_string(),
			field_path,
			path,
			section: "unknown".to_string(),
			subsection,
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
	authority: RegulatoryAuthority,
	case_id: Uuid,
	issues: Vec<ValidationIssue>,
) -> CaseValidationReport {
	let blocking_count = issues.iter().filter(|issue| issue.blocking).count();
	let non_blocking_count = issues.len().saturating_sub(blocking_count);
	let mut by_section: BTreeMap<String, (usize, usize)> = BTreeMap::new();
	let mut by_subsection: BTreeMap<(String, String), (usize, usize)> =
		BTreeMap::new();
	for issue in &issues {
		let section_counts = by_section.entry(issue.section.clone()).or_default();
		let subsection_counts = by_subsection
			.entry((issue.section.clone(), issue.subsection.clone()))
			.or_default();
		if issue.blocking {
			section_counts.0 += 1;
			subsection_counts.0 += 1;
		} else {
			section_counts.1 += 1;
			subsection_counts.1 += 1;
		}
	}
	let section_summaries = by_section
		.into_iter()
		.map(|(section, (blocking_count, non_blocking_count))| {
			ValidationSectionSummary {
				section,
				blocking_count,
				non_blocking_count,
			}
		})
		.collect();
	let subsection_summaries = by_subsection
		.into_iter()
		.map(
			|((section, subsection), (blocking_count, non_blocking_count))| {
				ValidationSubsectionSummary {
					section,
					subsection,
					blocking_count,
					non_blocking_count,
				}
			},
		)
		.collect();
	let authority = authority.as_str().to_string();
	CaseValidationReport {
		authority,
		case_id,
		ok: blocking_count == 0,
		blocking_count,
		non_blocking_count,
		section_summaries,
		subsection_summaries,
		issues,
	}
}
