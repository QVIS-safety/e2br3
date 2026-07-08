use super::rule_table::{eval_companions, CompanionRule};
use crate::{
	has_text, push_issue_by_code, push_issue_if_rule_invalid,
	should_require_case_narrative, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::model::narrative::{CaseSummaryInformation, SenderDiagnosis};

const H_SENDER_DIAGNOSIS_COMPANIONS: &[CompanionRule<SenderDiagnosis>] = &[
	CompanionRule {
		code: "ICH.H.3.r.1a.REQUIRED",
		path: |idx| {
			format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion")
		},
		trigger: |diagnosis| has_text(diagnosis.diagnosis_meddra_code.as_deref()),
		required: |diagnosis| {
			has_text(diagnosis.diagnosis_meddra_version.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.H.3.r.1b.REQUIRED",
		path: |idx| format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode"),
		trigger: |diagnosis| has_text(diagnosis.diagnosis_meddra_version.as_deref()),
		required: |diagnosis| has_text(diagnosis.diagnosis_meddra_code.as_deref()),
	},
];

const H_CASE_SUMMARY_COMPANIONS: &[CompanionRule<CaseSummaryInformation>] =
	&[CompanionRule {
		code: "ICH.H.5.r.1b.REQUIRED",
		path: |idx| format!("narrative.caseSummaries.{idx}.languageCode"),
		trigger: |summary| has_text(summary.summary_text.as_deref()),
		required: |summary| has_text(summary.language_code.as_deref()),
	}];

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
) {
	let _ = authority;
	collect_ich_issues(validation_ctx, issues);
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	if validation_ctx.narrative.is_none() {
		push_issue_by_code(issues, "ICH.H.1.REQUIRED", "narrative.caseNarrative");
	}

	if let Some(narrative) = validation_ctx.narrative.as_ref() {
		if should_require_case_narrative(narrative) {
			let _ = push_issue_if_rule_invalid(
				issues,
				"ICH.H.1.REQUIRED",
				"narrative.caseNarrative",
				Some(narrative.case_narrative.as_str()),
				None,
				RuleFacts::default(),
			);
		}
	}

	eval_companions(
		issues,
		&validation_ctx.sender_diagnoses,
		H_SENDER_DIAGNOSIS_COMPANIONS,
	);
	eval_companions(
		issues,
		&validation_ctx.case_summaries,
		H_CASE_SUMMARY_COMPANIONS,
	);
}
