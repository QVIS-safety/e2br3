use super::helpers::{
	max_length, reject_when, require, valid_decimal, valid_dotted_version,
	valid_iso639, valid_meddra_term, valid_meddra_version,
};
use crate::{
	has_text, push_business_issue, should_require_case_narrative,
	RegulatoryAuthority, ValidationContext, ValidationIssue,
};
use lib_core::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
};
use lib_core::regulatory::{
	is_mfds_clinical_trial_receiver, is_mfds_domestic_receiver,
};

const SECTION: &str = "narrative";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

/// ICH.H.1.REQUIRED
/// ICH.H.1.LENGTH.MAX
fn h_1(narrative: Option<&NarrativeInformation>, issues: &mut Vec<ValidationIssue>) {
	const PATH: &str = "narrative.caseNarrative";
	let present = narrative.is_some_and(|narrative| {
		!should_require_case_narrative(narrative)
			|| has_text(Some(narrative.case_narrative.as_str()))
	});
	require(
		issues,
		"ICH.H.1.REQUIRED",
		PATH,
		SECTION,
		"[H.1] This Element is required.",
		present,
	);
	max_length(
		issues,
		"ICH.H.1.LENGTH.MAX",
		PATH,
		SECTION,
		MAX_LENGTH_MESSAGE,
		narrative.map(|narrative| narrative.case_narrative.as_str()),
		100000,
	);
}

/// ICH.H.2.LENGTH.MAX
fn h_2(narrative: &NarrativeInformation, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.H.2.LENGTH.MAX",
		"narrative.reporterComments",
		SECTION,
		MAX_LENGTH_MESSAGE,
		narrative.reporter_comments.as_deref(),
		20000,
	);
}

/// ICH.H.3.r.1a.REQUIRED
/// ICH.H.3.r.1a.LENGTH.MAX
fn h_3_r_1a(
	idx: usize,
	diagnosis: &SenderDiagnosis,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion");
	reject_when(
		issues,
		"ICH.H.3.r.1a.REQUIRED",
		&path,
		SECTION,
		"[H.3.r.1a] Sender diagnosis MedDRA version is required when [H.3.r.1b] is populated.",
		has_text(diagnosis.diagnosis_meddra_code.as_deref())
			&& !has_text(diagnosis.diagnosis_meddra_version.as_deref()),
	);
	max_length(
		issues,
		"ICH.H.3.r.1a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		diagnosis.diagnosis_meddra_version.as_deref(),
		4,
	);
}

/// ICH.H.3.r.1b.REQUIRED
/// ICH.H.3.r.1b.LENGTH.MAX
fn h_3_r_1b(
	idx: usize,
	diagnosis: &SenderDiagnosis,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode");
	reject_when(
		issues,
		"ICH.H.3.r.1b.REQUIRED",
		&path,
		SECTION,
		"[H.3.r.1b] Sender diagnosis MedDRA code is required when [H.3.r.1a] is populated.",
		has_text(diagnosis.diagnosis_meddra_version.as_deref())
			&& !has_text(diagnosis.diagnosis_meddra_code.as_deref()),
	);
	max_length(
		issues,
		"ICH.H.3.r.1b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		diagnosis.diagnosis_meddra_code.as_deref(),
		8,
	);
}

/// ICH.H.3.r.1a.ALLOWED.VALUE
/// ICH.H.3.r.1a.VOCABULARY
/// ICH.H.3.r.1b.ALLOWED.VALUE
/// ICH.H.3.r.1b.VOCABULARY
fn h_3_r_1(
	idx: usize,
	diagnosis: &SenderDiagnosis,
	vocabulary: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let version_path =
		format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion");
	let code_path = format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraCode");
	let version = diagnosis.diagnosis_meddra_version.as_deref();
	let code = diagnosis.diagnosis_meddra_code.as_deref();
	reject_when(
		issues,
		"ICH.H.3.r.1a.ALLOWED.VALUE",
		&version_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_dotted_version(version),
	);
	reject_when(
		issues,
		"ICH.H.3.r.1b.ALLOWED.VALUE",
		&code_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_decimal(code),
	);
	reject_when(
		issues,
		"ICH.H.3.r.1a.VOCABULARY",
		&version_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_version(vocabulary, version),
	);
	reject_when(
		issues,
		"ICH.H.3.r.1b.VOCABULARY",
		&code_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_term(vocabulary, version, code),
	);
}

/// ICH.H.4.LENGTH.MAX
fn h_4(narrative: &NarrativeInformation, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.H.4.LENGTH.MAX",
		"narrative.senderComments",
		SECTION,
		MAX_LENGTH_MESSAGE,
		narrative.sender_comments.as_deref(),
		20000,
	);
}

fn contains_korean(value: &str) -> bool {
	value.chars().any(|ch| {
		matches!(ch, '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '\u{ac00}'..='\u{d7a3}')
	})
}

/// MFDS.H.4: Korean is required for domestic and clinical reports.
fn mfds_h_4(narrative: &NarrativeInformation, issues: &mut Vec<ValidationIssue>) {
	if narrative
		.sender_comments
		.as_deref()
		.is_some_and(|comments| {
			has_text(Some(comments)) && !contains_korean(comments)
		}) {
		push_business_issue(
			issues,
			"MFDS.H.4.KOREAN.REQUIRED",
			"narrative.senderComments",
			"Sender comments must include Korean for domestic and clinical reports",
		);
	}
}

/// ICH.H.5.r.1a.LENGTH.MAX
fn h_5_r_1a(
	idx: usize,
	summary: &CaseSummaryInformation,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("narrative.caseSummaries.{idx}.summaryText");
	max_length(
		issues,
		"ICH.H.5.r.1a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		summary.summary_text.as_deref(),
		100000,
	);
}

/// ICH.H.5.r.1b.REQUIRED
/// ICH.H.5.r.1b.LENGTH.MAX
/// ICH.H.5.r.1b.ALLOWED.VALUE
fn h_5_r_1b(
	idx: usize,
	summary: &CaseSummaryInformation,
	vocabulary: &crate::context::VocabularyContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let path = format!("narrative.caseSummaries.{idx}.languageCode");
	reject_when(
		issues,
		"ICH.H.5.r.1b.REQUIRED",
		&path,
		SECTION,
		"[H.5.r.1b] Case summary language is required when [H.5.r.1a] summary text is populated.",
		has_text(summary.summary_text.as_deref())
			&& !has_text(summary.language_code.as_deref()),
	);
	max_length(
		issues,
		"ICH.H.5.r.1b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		summary.language_code.as_deref(),
		3,
	);
	reject_when(
		issues,
		"ICH.H.5.r.1b.VOCABULARY",
		&path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_iso639(vocabulary, summary.language_code.as_deref()),
	);
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
) {
	collect_ich_issues(validation_ctx, issues);
	if authority == RegulatoryAuthority::Mfds {
		let receiver = validation_ctx
			.message_header
			.as_ref()
			.map(|header| header.message_receiver_identifier.as_str());
		if is_mfds_domestic_receiver(receiver)
			|| is_mfds_clinical_trial_receiver(receiver)
		{
			if let Some(narrative) = validation_ctx.narrative.as_ref() {
				mfds_h_4(narrative, issues);
			}
		}
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	h_1(validation_ctx.narrative.as_ref(), issues);
	if let Some(narrative) = validation_ctx.narrative.as_ref() {
		h_2(narrative, issues);
		h_4(narrative, issues);
	}
	for (idx, diagnosis) in validation_ctx.sender_diagnoses.iter().enumerate() {
		h_3_r_1a(idx, diagnosis, issues);
		h_3_r_1b(idx, diagnosis, issues);
		h_3_r_1(idx, diagnosis, &validation_ctx.vocabulary, issues);
	}
	for (idx, summary) in validation_ctx.case_summaries.iter().enumerate() {
		h_5_r_1a(idx, summary, issues);
		h_5_r_1b(idx, summary, &validation_ctx.vocabulary, issues);
	}
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
			status_before_lock: None,
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
		assert!(!codes.iter().any(|code| code.ends_with(".VOCABULARY")));
		ctx.sender_diagnoses[0].diagnosis_meddra_version = Some("26.1".to_string());
		let codes = codes_for(&ctx);
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

	#[test]
	fn korean_sender_comments_are_required_for_mfds_routes() {
		let mut narrative = narrative();
		narrative.sender_comments = Some("English only".to_string());
		let mut issues = Vec::new();
		mfds_h_4(&narrative, &mut issues);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "MFDS.H.4.KOREAN.REQUIRED"));
	}

	#[test]
	fn golden_h_issue_metadata() {
		let mut ctx = empty_ctx();
		ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		let mut diagnosis = sender_diagnosis();
		diagnosis.diagnosis_meddra_version = Some("bad".to_string());
		diagnosis.diagnosis_meddra_code = Some("10000001".to_string());
		ctx.sender_diagnoses = vec![diagnosis];

		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| {
				matches!(
					issue.code.as_str(),
					"ICH.H.1.REQUIRED" | "ICH.H.3.r.1a.ALLOWED.VALUE"
				)
			})
			.map(|issue| {
				(
					issue.code,
					issue.message,
					issue.path,
					issue.field_path,
					issue.section,
					issue.subsection,
				)
			})
			.collect::<Vec<_>>();
		out.sort_by(|left, right| left.0.cmp(&right.0));

		assert_eq!(
			out,
			vec![
				(
					"ICH.H.1.REQUIRED".to_string(),
					"[H.1] This Element is required.".to_string(),
					"narrative.caseNarrative".to_string(),
					Some("narrative.caseNarrative".to_string()),
					"narrative".to_string(),
					"H".to_string(),
				),
				(
					"ICH.H.3.r.1a.ALLOWED.VALUE".to_string(),
					"Dictionary allowed values constraint.".to_string(),
					"narrative.senderDiagnoses.0.diagnosisMeddraVersion".to_string(),
					Some(
						"narrative.senderDiagnoses.0.diagnosisMeddraVersion"
							.to_string()
					),
					"narrative".to_string(),
					"H".to_string(),
				),
			],
		);
	}
}
