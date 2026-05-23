use crate::validation::{
	has_text, push_issue_by_code, push_issue_if_rule_invalid,
	should_require_case_narrative, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};

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

	validation_ctx.sender_diagnoses.iter().enumerate().for_each(
		|(idx, diagnosis)| {
			if has_text(diagnosis.diagnosis_meddra_code.as_deref())
				&& !has_text(diagnosis.diagnosis_meddra_version.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.H.3.r.1a.REQUIRED",
					format!(
						"narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion"
					),
				);
			}
			if has_text(diagnosis.diagnosis_meddra_version.as_deref())
				&& !has_text(diagnosis.diagnosis_meddra_code.as_deref())
			{
				push_issue_by_code(
					issues,
					"ICH.H.3.r.1b.REQUIRED",
					format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode"),
				);
			}
		},
	);

	validation_ctx
		.case_summaries
		.iter()
		.enumerate()
		.for_each(|(idx, summary)| {
			if has_text(summary.summary_type.as_deref()) {
				let _ = push_issue_if_rule_invalid(
					issues,
					"ICH.H.5.r.1b.REQUIRED",
					format!("narrative.caseSummaries.{idx}.languageCode"),
					summary.language_code.as_deref(),
					None,
					RuleFacts::default(),
				);
			}
		});
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.H.1.REQUIRED" => Some("narrative.caseNarrative"),
		"ICH.H.3.r.1a.REQUIRED" => {
			Some("narrative.senderDiagnoses.0.diagnosisMeddraVersion")
		}
		"ICH.H.3.r.1b.REQUIRED" => {
			Some("narrative.senderDiagnoses.0.diagnosisMeddraCode")
		}
		"ICH.H.5.r.1b.REQUIRED" => Some("caseSummaryInformation.0.languageCode"),
		_ => None,
	}
}
