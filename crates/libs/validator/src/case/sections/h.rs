use super::rule_table::{
	eval_catalog_values, eval_companions, eval_indexed_constraints,
	eval_indexed_length, eval_indexed_meddra, eval_length, CatalogValueRule,
	CompanionRule, IndexedConstraintRule, IndexedLengthRule, IndexedMeddraRule,
	LengthRule, RuleValue,
};
use crate::allowed_value::ConstraintValue;
use crate::{
	has_text, should_require_case_narrative, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
};
use std::borrow::Cow;

struct HNarrativePresenceView {
	value: Option<String>,
}

const H_NARRATIVE_PRESENCE_RULES: &[CatalogValueRule<HNarrativePresenceView>] =
	&[CatalogValueRule {
		code: "ICH.H.1.REQUIRED",
		path: |_| "narrative.caseNarrative".to_string(),
		value: |item| RuleValue::borrowed(item.value.as_deref(), None),
		facts: |_| RuleFacts::default(),
	}];

const H_NARRATIVE_LENGTH_RULES: &[LengthRule<NarrativeInformation>] = &[
	LengthRule {
		code: "ICH.H.1.LENGTH.MAX",
		path: "narrative.caseNarrative",
		value: |narrative| Some(narrative.case_narrative.as_str()),
	},
	LengthRule {
		code: "ICH.H.2.LENGTH.MAX",
		path: "narrative.reporterComments",
		value: |narrative| narrative.reporter_comments.as_deref(),
	},
	LengthRule {
		code: "ICH.H.4.LENGTH.MAX",
		path: "narrative.senderComments",
		value: |narrative| narrative.sender_comments.as_deref(),
	},
];

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

const H_SENDER_DIAGNOSIS_LENGTH_RULES: &[IndexedLengthRule<SenderDiagnosis>] = &[
	IndexedLengthRule {
		code: "ICH.H.3.r.1a.LENGTH.MAX",
		path: |idx| {
			format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion")
		},
		value: |diagnosis| diagnosis.diagnosis_meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.H.3.r.1b.LENGTH.MAX",
		path: |idx| format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode"),
		value: |diagnosis| diagnosis.diagnosis_meddra_code.as_deref(),
	},
];

const H_SENDER_DIAGNOSIS_MEDDRA_RULES: &[IndexedMeddraRule<SenderDiagnosis>] =
	&[IndexedMeddraRule {
		version_allowed_code: "ICH.H.3.r.1a.ALLOWED.VALUE",
		version_code: "ICH.H.3.r.1a.VOCABULARY",
		code_allowed_code: "ICH.H.3.r.1b.ALLOWED.VALUE",
		code_code: "ICH.H.3.r.1b.VOCABULARY",
		version_path: |idx| {
			format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion")
		},
		code_path: |idx| {
			format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode")
		},
		values: |diagnosis| {
			(
				diagnosis.diagnosis_meddra_version.as_deref(),
				diagnosis.diagnosis_meddra_code.as_deref(),
			)
		},
	}];

const H_CASE_SUMMARY_COMPANIONS: &[CompanionRule<CaseSummaryInformation>] =
	&[CompanionRule {
		code: "ICH.H.5.r.1b.REQUIRED",
		path: |idx| format!("narrative.caseSummaries.{idx}.languageCode"),
		trigger: |summary| has_text(summary.summary_text.as_deref()),
		required: |summary| has_text(summary.language_code.as_deref()),
	}];

const H_CASE_SUMMARY_LENGTH_RULES: &[IndexedLengthRule<CaseSummaryInformation>] = &[
	IndexedLengthRule {
		code: "ICH.H.5.r.1a.LENGTH.MAX",
		path: |idx| format!("narrative.caseSummaries.{idx}.summaryText"),
		value: |summary| summary.summary_text.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.H.5.r.1b.LENGTH.MAX",
		path: |idx| format!("narrative.caseSummaries.{idx}.languageCode"),
		value: |summary| summary.language_code.as_deref(),
	},
];

const H_CASE_SUMMARY_CONSTRAINT_RULES: &[IndexedConstraintRule<
	CaseSummaryInformation,
>] = &[IndexedConstraintRule {
	code: "ICH.H.5.r.1b.ALLOWED.VALUE",
	path: |idx| format!("narrative.caseSummaries.{idx}.languageCode"),
	value: |summary| {
		ConstraintValue::Text(summary.language_code.as_deref().map(Cow::Borrowed))
	},
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
	if let Some(narrative) = validation_ctx.narrative.as_ref() {
		eval_length(issues, narrative, H_NARRATIVE_LENGTH_RULES);
	}
	let narrative_presence = HNarrativePresenceView {
		value: validation_ctx.narrative.as_ref().and_then(|narrative| {
			if should_require_case_narrative(narrative) {
				Some(narrative.case_narrative.clone())
			} else {
				Some("present".to_string())
			}
		}),
	};
	eval_catalog_values(
		issues,
		std::slice::from_ref(&narrative_presence),
		H_NARRATIVE_PRESENCE_RULES,
	);

	eval_companions(
		issues,
		&validation_ctx.sender_diagnoses,
		H_SENDER_DIAGNOSIS_COMPANIONS,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.sender_diagnoses,
		H_SENDER_DIAGNOSIS_LENGTH_RULES,
	);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.sender_diagnoses,
		H_SENDER_DIAGNOSIS_MEDDRA_RULES,
	);
	eval_companions(
		issues,
		&validation_ctx.case_summaries,
		H_CASE_SUMMARY_COMPANIONS,
	);
	eval_indexed_length(
		issues,
		&validation_ctx.case_summaries,
		H_CASE_SUMMARY_LENGTH_RULES,
	);
	eval_indexed_constraints(
		issues,
		&validation_ctx.case_summaries,
		H_CASE_SUMMARY_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	H_CASE_SUMMARY_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			H_SENDER_DIAGNOSIS_MEDDRA_RULES,
		))
		.collect()
}

#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> {
	let mut codes = Vec::new();
	codes.extend(super::rule_table::table_rule_codes(
		H_NARRATIVE_LENGTH_RULES,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_SENDER_DIAGNOSIS_COMPANIONS,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_SENDER_DIAGNOSIS_LENGTH_RULES,
	));
	codes.extend(super::rule_table::indexed_meddra_rule_codes(
		H_SENDER_DIAGNOSIS_MEDDRA_RULES,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_CASE_SUMMARY_COMPANIONS,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_CASE_SUMMARY_LENGTH_RULES,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_CASE_SUMMARY_CONSTRAINT_RULES,
	));
	codes.extend(super::rule_table::table_rule_codes(
		H_NARRATIVE_PRESENCE_RULES,
	));
	codes
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::narrative::{
		CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
	};
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
			review_receivers_json: None,
			workflow_routes_json: None,
			workflow_status: String::new(),
			workflow_assigned_role: None,
			workflow_assigned_user_id: None,
			workflow_due_at: None,
			workflow_description: None,
			workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			created_by: Uuid::nil(),
			updated_by: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: false,
			dirty_d: false,
			dirty_e: false,
			dirty_f: false,
			dirty_g: false,
			dirty_h: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn empty_ctx() -> ValidationContext {
		ValidationContext {
			vocabulary: Default::default(),
			case: dummy_case(),
			safety_report: None,
			message_header: None,
			sender: None,
			patient: None,
			narrative: None,
			sender_diagnoses: Vec::new(),
			case_summaries: Vec::new(),
			medical_history: Vec::new(),
			past_drugs: Vec::new(),
			death_info: None,
			reported_causes_of_death: Vec::new(),
			autopsy_causes_of_death: Vec::new(),
			parents: Vec::new(),
			parent_medical_history: Vec::new(),
			parent_past_drugs: Vec::new(),
			primary_sources: Vec::new(),
			documents_held_by_sender: Vec::new(),
			literature_references: Vec::new(),
			other_case_identifiers: Vec::new(),
			linked_report_numbers: Vec::new(),
			studies: Vec::new(),
			study_registrations: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			relatedness_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn narrative() -> NarrativeInformation {
		NarrativeInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_narrative_presave_id: None,
			case_narrative: String::new(),
			reporter_comments: None,
			sender_comments: None,
			additional_information: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn sender_diagnosis() -> SenderDiagnosis {
		SenderDiagnosis {
			id: Uuid::nil(),
			narrative_id: Uuid::nil(),
			sequence_number: 1,
			deleted: false,
			diagnosis_meddra_version: None,
			diagnosis_meddra_code: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn case_summary() -> CaseSummaryInformation {
		CaseSummaryInformation {
			id: Uuid::nil(),
			narrative_id: Uuid::nil(),
			sequence_number: 1,
			deleted: false,
			summary_type: None,
			language_code: None,
			summary_text: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn length_issue(code: &str, path: &str) -> (String, String) {
		(code.to_string(), path.to_string())
	}

	fn length_issues(ctx: &ValidationContext) -> Vec<(String, String)> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();
		out
	}

	fn codes_for(ctx: &ValidationContext) -> Vec<String> {
		let mut issues = Vec::new();
		collect_ich_issues(ctx, &mut issues);
		issues.into_iter().map(|issue| issue.code).collect()
	}

	#[test]
	fn meddra_vocabulary_rules_cover_sender_diagnosis_codes() {
		let mut ctx = empty_ctx();
		ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		let mut diagnosis = sender_diagnosis();
		diagnosis.diagnosis_meddra_version = Some("99.9".to_string());
		diagnosis.diagnosis_meddra_code = Some("99999999".to_string());
		ctx.sender_diagnoses = vec![diagnosis];

		let codes = codes_for(&ctx);
		assert!(codes.contains(&"ICH.H.3.r.1a.VOCABULARY".to_string()));
		assert!(codes.contains(&"ICH.H.3.r.1b.VOCABULARY".to_string()));
	}

	#[test]
	fn max_length_rules_cover_h_narrative_text_fields() {
		let mut narrative = narrative();
		narrative.case_narrative = "N".repeat(100001);
		narrative.reporter_comments = Some("R".repeat(20001));
		narrative.sender_comments = Some("S".repeat(20001));
		let mut diagnosis = sender_diagnosis();
		diagnosis.diagnosis_meddra_version = Some("V".repeat(5));
		diagnosis.diagnosis_meddra_code = Some("C".repeat(9));
		let mut summary = case_summary();
		summary.summary_text = Some("T".repeat(100001));
		summary.language_code = Some("LANG".to_string());
		let mut ctx = empty_ctx();
		ctx.narrative = Some(narrative);
		ctx.sender_diagnoses = vec![diagnosis];
		ctx.case_summaries = vec![summary];

		assert_eq!(
			length_issues(&ctx),
			vec![
				length_issue("ICH.H.1.LENGTH.MAX", "narrative.caseNarrative"),
				length_issue("ICH.H.2.LENGTH.MAX", "narrative.reporterComments"),
				length_issue(
					"ICH.H.3.r.1a.LENGTH.MAX",
					"narrative.senderDiagnoses.0.diagnosisMeddraVersion"
				),
				length_issue(
					"ICH.H.3.r.1b.LENGTH.MAX",
					"narrative.senderDiagnoses.0.diagnosisMeddraCode"
				),
				length_issue("ICH.H.4.LENGTH.MAX", "narrative.senderComments"),
				length_issue(
					"ICH.H.5.r.1a.LENGTH.MAX",
					"narrative.caseSummaries.0.summaryText"
				),
				length_issue(
					"ICH.H.5.r.1b.LENGTH.MAX",
					"narrative.caseSummaries.0.languageCode"
				),
			]
		);
	}
}
