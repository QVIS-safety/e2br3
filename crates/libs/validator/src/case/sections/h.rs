use super::helpers::{
	max_length, reject_when, require, valid_decimal, valid_dotted_version,
	valid_iso639, valid_meddra_term, valid_meddra_version,
};
use crate::{
	has_text, push_business_issue, RegulatoryAuthority, ValidationContext,
	ValidationIssue,
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
	let present = narrative
		.is_some_and(|narrative| has_text(Some(narrative.case_narrative.as_str())));
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
#[path = "h/rule_tests.rs"]
mod rule_tests;

#[cfg(test)]
#[path = "h/tests.rs"]
mod tests;
