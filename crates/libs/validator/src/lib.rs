mod c_reporter_policy;
mod c_safety_report_policy;
pub mod case;
mod context;
mod d_patient_policy;
mod f_test_result_policy;
mod fda_context;
mod h_narrative_policy;
mod mfds_context;
pub use c_reporter_policy::has_any_primary_source_content;
pub use c_safety_report_policy::{
	has_report_type, should_clear_combination_product_null_flavor_on_value,
	should_clear_local_criteria_null_flavor_on_value,
	should_require_fda_local_criteria_report_type,
};
pub use case::{validate_case_for_authorities, validate_case_for_authority};
pub use context::{
	load_base_validation_context, ValidationContext, VocabularyScope,
};
pub use d_patient_policy::{
	has_fda_ethnicity, has_fda_race, has_patient_initials, has_patient_payload,
	should_require_fda_ethnicity, should_require_fda_race,
	should_require_patient_initials,
};
pub use f_test_result_policy::{has_test_name, has_test_payload};
pub use fda_context::{
	list_fda_devices, list_study_registrations, load_fda_validation_context,
	FdaValidationContext,
};
pub use h_narrative_policy::{
	has_case_narrative, has_narrative_payload, should_require_case_narrative,
};
pub use lib_core::regulatory::*;
pub use lib_core::validation_report::{
	CaseValidationReport, ValidationIssue, ValidationSectionSummary,
	ValidationSubsectionSummary,
};
pub use mfds_context::{
	load_mfds_validation_context, MfdsValidationContext, ParentPastDrugByCase,
	PastDrugByCase, RelatednessWithDrug,
};
use sqlx::types::Uuid;
use std::collections::BTreeMap;

pub fn has_text(value: Option<&str>) -> bool {
	value.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

pub(crate) fn push_business_issue(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: impl Into<String>,
	message: impl Into<String>,
) {
	let path = path.into();
	let section = case::sections::resolve_validation_section(code, Some(&path));
	push_field_issue(issues, code, path, section, message);
}

pub(crate) fn push_field_issue(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: impl Into<String>,
	section: impl Into<String>,
	message: impl Into<String>,
) {
	let path = path.into();
	let field_path = case::sections::resolve_validation_field_path(Some(&path));
	let subsection =
		case::sections::resolve_validation_subsection(code, Some(&path));
	issues.push(ValidationIssue {
		code: code.to_string(),
		message: message.into(),
		field_path,
		path,
		section: section.into(),
		subsection,
	});
}

pub fn build_report(
	authority: RegulatoryAuthority,
	case_id: Uuid,
	mut issues: Vec<ValidationIssue>,
) -> CaseValidationReport {
	let mut seen = std::collections::HashSet::new();
	issues.retain(|issue| seen.insert((issue.code.clone(), issue.path.clone())));
	let issue_count = issues.len();
	let mut by_section: BTreeMap<String, usize> = BTreeMap::new();
	let mut by_subsection: BTreeMap<(String, String), usize> = BTreeMap::new();
	for issue in &issues {
		let section_counts = by_section.entry(issue.section.clone()).or_default();
		let subsection_counts = by_subsection
			.entry((issue.section.clone(), issue.subsection.clone()))
			.or_default();
		*section_counts += 1;
		*subsection_counts += 1;
	}
	let section_summaries = by_section
		.into_iter()
		.map(|(section, issue_count)| ValidationSectionSummary {
			section,
			issue_count,
		})
		.collect();
	let subsection_summaries = by_subsection
		.into_iter()
		.map(
			|((section, subsection), issue_count)| ValidationSubsectionSummary {
				section,
				subsection,
				issue_count,
			},
		)
		.collect();
	let authority = authority.as_str().to_string();
	CaseValidationReport {
		authority,
		case_id,
		ok: issues.is_empty(),
		issue_count,
		section_summaries,
		subsection_summaries,
		issues,
	}
}

#[cfg(test)]
mod direct_business_issue_tests {
	use super::*;

	#[test]
	fn report_deduplicates_rules_without_collapsing_distinct_rows() {
		let mut issues = Vec::new();
		for path in [
			"reactions.0.reactionStartDate",
			"reactions.0.reactionStartDate",
			"reactions.1.reactionStartDate",
		] {
			push_business_issue(
				&mut issues,
				"ICH.E.i.4.REQUIRED",
				path,
				"Date required",
			);
		}
		let report = build_report(RegulatoryAuthority::Ich, Uuid::nil(), issues);
		assert_eq!(report.issue_count, 2);
		assert_eq!(report.section_summaries[0].issue_count, 2);
	}

	#[test]
	fn direct_business_issue_fails_report_without_catalog_metadata() {
		let mut issues = Vec::new();
		push_business_issue(
			&mut issues,
			"FDA.R0011",
			"safetyReportIdentification.safetyReportId",
			"invalid identifier profile",
		);

		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].section, "C");
		assert_eq!(issues[0].subsection, "C.1");
		assert_eq!(issues[0].message, "invalid identifier profile");
		let report = build_report(RegulatoryAuthority::Fda, Uuid::nil(), issues);
		assert!(!report.ok);
		assert_eq!(report.issue_count, 1);
		assert_eq!(report.section_summaries[0].issue_count, 1);
		assert_eq!(report.subsection_summaries[0].issue_count, 1);
		let json = serde_json::to_value(&report).unwrap();
		assert_eq!(
			json["issue_count"],
			json["issues"].as_array().unwrap().len()
		);
		assert!(json.get("blocking_count").is_none());
		assert!(json.get("non_blocking_count").is_none());
		assert!(json["issues"][0].get("blocking").is_none());
	}
}
