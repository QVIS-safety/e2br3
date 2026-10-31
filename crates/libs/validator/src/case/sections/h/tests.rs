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

pub(super) fn empty_ctx() -> ValidationContext {
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

pub(super) fn narrative() -> NarrativeInformation {
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

pub(super) fn sender_diagnosis() -> SenderDiagnosis {
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

pub(super) fn case_summary() -> CaseSummaryInformation {
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
fn h_1_is_required_independently_of_other_narrative_content() {
	let required_count = |ctx: &ValidationContext| {
		codes_for(ctx)
			.into_iter()
			.filter(|code| code == "ICH.H.1.REQUIRED")
			.count()
	};

	assert_eq!(required_count(&empty_ctx()), 1);

	let mut empty_row = empty_ctx();
	empty_row.narrative = Some(narrative());
	assert_eq!(required_count(&empty_row), 1);

	let mut whitespace = narrative();
	whitespace.case_narrative = " \t\n ".to_string();
	let mut whitespace_ctx = empty_ctx();
	whitespace_ctx.narrative = Some(whitespace);
	assert_eq!(required_count(&whitespace_ctx), 1);

	let mut h_5_only = empty_ctx();
	h_5_only.narrative = Some(narrative());
	let mut summary = case_summary();
	summary.summary_text = Some("Case summary only".to_string());
	summary.language_code = Some("eng".to_string());
	h_5_only.case_summaries = vec![summary];
	assert_eq!(required_count(&h_5_only), 1);

	let mut populated = narrative();
	populated.case_narrative = "Required narrative".to_string();
	let mut populated_ctx = empty_ctx();
	populated_ctx.narrative = Some(populated);
	assert_eq!(required_count(&populated_ctx), 0);
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
					"narrative.senderDiagnoses.0.diagnosisMeddraVersion".to_string()
				),
				"narrative".to_string(),
				"H".to_string(),
			),
		],
	);
}
