use super::tests::{case_summary, empty_ctx, narrative, sender_diagnosis};
use super::*;
use lib_core::model::narrative::{CaseSummaryInformation, SenderDiagnosis};

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
	let emitted_code = if code == "ICH.H.3.r.1a.VOCABULARY" {
		"ICH.MEDDRA.VERSION.UNAVAILABLE"
	} else {
		code
	};
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == emitted_code && issue.path == path),
		"{emitted_code} missing at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == emitted_code),
		"{emitted_code} remained after repair: {repaired:?}"
	);
	if emitted_code != code {
		assert!(!invalid.iter().any(|issue| issue.code == code));
		assert!(!repaired.iter().any(|issue| issue.code == code));
	}
}

fn diagnosis_issues(diagnosis: &SenderDiagnosis) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	h_3_r_1a(0, diagnosis, &mut issues);
	h_3_r_1b(0, diagnosis, &mut issues);
	h_3_r_1(0, diagnosis, &Default::default(), &mut issues);
	issues
}

fn diagnosis_vocabulary_issues(diagnosis: &SenderDiagnosis) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	let vocabulary =
		crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
	h_3_r_1(0, diagnosis, &vocabulary, &mut issues);
	let mut ctx = empty_ctx();
	ctx.vocabulary = vocabulary;
	ctx.sender_diagnoses.push(diagnosis.clone());
	super::super::collect_meddra_version_issues(&ctx, &mut issues);
	issues
}

fn summary_issues(summary: &CaseSummaryInformation) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	h_5_r_1b(0, summary, &Default::default(), &mut issues);
	issues
}

#[test]
fn rule_ich_h_1_required() {
	let mut good = narrative();
	good.case_narrative = "Narrative".into();
	let (mut invalid, mut repaired) = (Vec::new(), Vec::new());
	h_1(None, &mut invalid);
	h_1(Some(&good), &mut repaired);
	assert_edge(
		"ICH.H.1.REQUIRED",
		"narrative.caseNarrative",
		invalid,
		repaired,
	);
}

#[test]
fn rule_ich_h_3_r_1a_allowed_value() {
	let mut bad = sender_diagnosis();
	bad.diagnosis_meddra_version = Some("bad".into());
	bad.diagnosis_meddra_code = Some("10000001".into());
	let mut good = bad.clone();
	good.diagnosis_meddra_version = Some("26.1".into());
	assert_edge(
		"ICH.H.3.r.1a.ALLOWED.VALUE",
		"narrative.senderDiagnoses.0.diagnosisMeddraVersion",
		diagnosis_issues(&bad),
		diagnosis_issues(&good),
	);
}

#[test]
fn rule_ich_h_3_r_1a_required() {
	let mut bad = sender_diagnosis();
	bad.diagnosis_meddra_code = Some("10000001".into());
	let mut good = bad.clone();
	good.diagnosis_meddra_version = Some("26.1".into());
	assert_edge(
		"ICH.H.3.r.1a.REQUIRED",
		"narrative.senderDiagnoses.0.diagnosisMeddraVersion",
		diagnosis_issues(&bad),
		diagnosis_issues(&good),
	);
}

#[test]
fn rule_ich_h_3_r_1a_vocabulary() {
	let mut bad = sender_diagnosis();
	bad.diagnosis_meddra_version = Some("27.0".into());
	bad.diagnosis_meddra_code = Some("10000001".into());
	let mut good = bad.clone();
	good.diagnosis_meddra_version = Some("26.1".into());
	assert_edge(
		"ICH.H.3.r.1a.VOCABULARY",
		"narrative.senderDiagnoses.0.diagnosisMeddraVersion",
		diagnosis_vocabulary_issues(&bad),
		diagnosis_vocabulary_issues(&good),
	);
}

#[test]
fn rule_ich_h_3_r_1b_required() {
	let mut bad = sender_diagnosis();
	bad.diagnosis_meddra_version = Some("26.1".into());
	let mut good = bad.clone();
	good.diagnosis_meddra_code = Some("10000001".into());
	assert_edge(
		"ICH.H.3.r.1b.REQUIRED",
		"narrative.senderDiagnoses.0.diagnosisMeddraCode",
		diagnosis_issues(&bad),
		diagnosis_issues(&good),
	);
}

#[test]
fn rule_ich_h_3_r_1b_vocabulary() {
	let mut bad = sender_diagnosis();
	bad.diagnosis_meddra_version = Some("26.1".into());
	bad.diagnosis_meddra_code = Some("99999999".into());
	let mut good = bad.clone();
	good.diagnosis_meddra_code = Some("10000001".into());
	assert_edge(
		"ICH.H.3.r.1b.VOCABULARY",
		"narrative.senderDiagnoses.0.diagnosisMeddraCode",
		diagnosis_vocabulary_issues(&bad),
		diagnosis_vocabulary_issues(&good),
	);
}

#[test]
fn rule_ich_h_5_r_1b_required() {
	let mut bad = case_summary();
	bad.summary_text = Some("Summary".into());
	let mut good = bad.clone();
	good.language_code = Some("eng".into());
	assert_edge(
		"ICH.H.5.r.1b.REQUIRED",
		"narrative.caseSummaries.0.languageCode",
		summary_issues(&bad),
		summary_issues(&good),
	);
}

#[test]
fn rule_ich_h_5_r_1b_vocabulary() {
	let mut bad = case_summary();
	bad.summary_text = Some("Summary".into());
	bad.language_code = Some("zzz".into());
	let mut good = bad.clone();
	good.language_code = Some("eng".into());
	assert_edge(
		"ICH.H.5.r.1b.VOCABULARY",
		"narrative.caseSummaries.0.languageCode",
		summary_issues(&bad),
		summary_issues(&good),
	);
}

#[test]
fn rule_mfds_h_4_korean_required() {
	let mut bad = narrative();
	bad.sender_comments = Some("English only".into());
	let mut good = bad.clone();
	good.sender_comments = Some("한국어".into());
	let (mut invalid, mut repaired) = (Vec::new(), Vec::new());
	mfds_h_4(&bad, &mut invalid);
	mfds_h_4(&good, &mut repaired);
	assert_edge(
		"MFDS.H.4.KOREAN.REQUIRED",
		"narrative.senderComments",
		invalid,
		repaired,
	);
}
