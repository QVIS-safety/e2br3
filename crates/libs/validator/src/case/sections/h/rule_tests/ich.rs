use super::*;

use lib_core::model::narrative::{CaseSummaryInformation, SenderDiagnosis};

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
	super::super::super::collect_meddra_version_issues(&ctx, &mut issues);
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
	for text in [None, Some(""), Some("   ")] {
		let mut row = case_summary();
		row.summary_text = text.map(str::to_owned);
		assert!(!summary_issues(&row)
			.iter()
			.any(|i| i.code == "ICH.H.5.r.1b.REQUIRED"));
	}

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
fn rule_ich_h_3_r_1a_meddra_version_consistent() {
	let mut ctx = empty_ctx();
	let mut a = sender_diagnosis();
	a.diagnosis_meddra_version = Some("26.1".into());
	let mut b = a.clone();
	b.diagnosis_meddra_version = Some("27.0".into());
	ctx.sender_diagnoses = vec![a, b];
	let mut invalid = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid
		.iter()
		.any(|i| i.code == "ICH.H.3.r.1a.MEDDRA.VERSION.CONSISTENT"
			&& i.path == "narrative.senderDiagnoses.1.diagnosisMeddraVersion"));
	for version in [Some("26.1"), Some(" 26.1 "), Some(""), None] {
		ctx.sender_diagnoses[1].diagnosis_meddra_version =
			version.map(str::to_owned);
		let mut repaired = Vec::new();
		super::super::super::collect_meddra_version_issues(&ctx, &mut repaired);
		assert_edge(
			"ICH.H.3.r.1a.MEDDRA.VERSION.CONSISTENT",
			"narrative.senderDiagnoses.0.diagnosisMeddraVersion",
			invalid.clone(),
			repaired,
		);
	}
}
