use super::helpers::{
	max_length, reject_future_date, reject_when, valid_code, valid_decimal,
	valid_dotted_version, valid_meddra_term, valid_meddra_version, valid_ucum,
	DateValues,
};
use crate::{
	has_test_payload, has_text, RegulatoryAuthority, ValidationContext,
	ValidationIssue,
};
use lib_core::model::test_result::TestResult;

const SECTION: &str = "tests";
const MAX_LENGTH_MESSAGE: &str = "Dictionary max length exceeded.";
const ALLOWED_VALUE_MESSAGE: &str = "Dictionary allowed values constraint.";
const VOCABULARY_MESSAGE: &str = "Dictionary vocabulary constraint.";

/// ICH.F.r.1.REQUIRED
/// ICH.F.r.1.FUTURE_DATE.FORBIDDEN
fn f_r_1(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testDate");
	reject_when(
		issues,
		"ICH.F.r.1.REQUIRED",
		&path,
		SECTION,
		"[F.r.1] Test date is required when [F.r.2] is populated.",
		has_text(Some(test.test_name.as_str()))
			&& test.test_date.is_none()
			&& !has_text(test.test_date_null_flavor.as_deref()),
	);
	reject_future_date(
		issues,
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		&path,
		SECTION,
		"[F.r.1] Test date must not be later than today.",
		DateValues::One(test.test_date),
	);
}

/// ICH.F.r.2.REQUIRED
fn f_r_2(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	reject_when(
		issues,
		"ICH.F.r.2.REQUIRED",
		&format!("testResults.{idx}.testName"),
		SECTION,
		"[F.r.2] is required when test payload is present.",
		has_test_payload(test) && !has_text(Some(test.test_name.as_str())),
	);
}

/// ICH.F.r.2.1.REQUIRED
/// ICH.F.r.2.1.LENGTH.MAX
fn f_r_2_1(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testName");
	reject_when(
		issues,
		"ICH.F.r.2.1.REQUIRED",
		&path,
		SECTION,
		"[F.r.2.1] Test name (free text) is required when [F.r.1] is populated and [F.r.2.2b] is not populated.",
		test.test_date.is_some()
			&& !has_text(test.test_meddra_code.as_deref())
			&& !has_text(Some(test.test_name.as_str())),
	);
	max_length(
		issues,
		"ICH.F.r.2.1.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		Some(test.test_name.as_str()),
		250,
	);
}

/// ICH.F.r.2.2a.REQUIRED
/// ICH.F.r.2.2a.LENGTH.MAX
fn f_r_2_2a(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testMeddraVersion");
	reject_when(
		issues,
		"ICH.F.r.2.2a.REQUIRED",
		&path,
		SECTION,
		"[F.r.2.2a] Test name MedDRA version is required when [F.r.2.2b] is populated.",
		has_text(test.test_meddra_code.as_deref())
			&& !has_text(test.test_meddra_version.as_deref()),
	);
	max_length(
		issues,
		"ICH.F.r.2.2a.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.test_meddra_version.as_deref(),
		4,
	);
}

/// ICH.F.r.2.2b.REQUIRED
/// ICH.F.r.2.2b.LENGTH.MAX
fn f_r_2_2b(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testMeddraCode");
	reject_when(
		issues,
		"ICH.F.r.2.2b.REQUIRED",
		&path,
		SECTION,
		"[F.r.2.2b] Test name MedDRA code is required when [F.r.1] is populated and [F.r.2.1] is not populated.",
		test.test_date.is_some()
			&& !has_text(Some(test.test_name.as_str()))
			&& !has_text(test.test_meddra_code.as_deref()),
	);
	max_length(
		issues,
		"ICH.F.r.2.2b.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.test_meddra_code.as_deref(),
		8,
	);
}

/// ICH.F.r.2.2a.ALLOWED.VALUE
/// ICH.F.r.2.2a.VOCABULARY
/// ICH.F.r.2.2b.ALLOWED.VALUE
/// ICH.F.r.2.2b.VOCABULARY
fn f_r_2_2_meddra(
	idx: usize,
	test: &TestResult,
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let version_path = format!("testResults.{idx}.testMeddraVersion");
	let code_path = format!("testResults.{idx}.testMeddraCode");
	let version = test.test_meddra_version.as_deref();
	let code = test.test_meddra_code.as_deref();
	reject_when(
		issues,
		"ICH.F.r.2.2a.ALLOWED.VALUE",
		&version_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_dotted_version(version),
	);
	reject_when(
		issues,
		"ICH.F.r.2.2b.ALLOWED.VALUE",
		&code_path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_decimal(code),
	);
	reject_when(
		issues,
		"ICH.F.r.2.2a.VOCABULARY",
		&version_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_version(&validation_ctx.vocabulary, version),
	);
	reject_when(
		issues,
		"ICH.F.r.2.2b.VOCABULARY",
		&code_path,
		SECTION,
		VOCABULARY_MESSAGE,
		!valid_meddra_term(&validation_ctx.vocabulary, version, code),
	);
}

/// ICH.F.r.3.1.REQUIRED
/// ICH.F.r.3.1.ALLOWED.VALUE
/// ICH.F.r.3.1.LENGTH.MAX
fn f_r_3_1(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testResultCode");
	reject_when(
		issues,
		"ICH.F.r.3.1.REQUIRED",
		&path,
		SECTION,
		"[F.r.3.1] Test result (coded) is required when [F.r.2] is populated and neither [F.r.3.2] nor [F.r.3.4] is populated.",
		has_text(Some(test.test_name.as_str()))
			&& !has_text(test.test_result_value.as_deref())
			&& !has_text(test.result_unstructured.as_deref())
			&& !has_text(test.test_result_code.as_deref()),
	);
	reject_when(
		issues,
		"ICH.F.r.3.1.ALLOWED.VALUE",
		&path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_code(test.test_result_code.as_deref(), &["1", "2", "3", "4"]),
	);
	max_length(
		issues,
		"ICH.F.r.3.1.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.test_result_code.as_deref(),
		1,
	);
}

/// ICH.F.r.3.2.REQUIRED
/// ICH.F.r.3.2.ALLOWED.VALUE
/// ICH.F.r.3.2.LENGTH.MAX
fn f_r_3_2(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testResultValue");
	reject_when(
		issues,
		"ICH.F.r.3.2.REQUIRED",
		&path,
		SECTION,
		"[F.r.3.2] Test result (value/finding) is required when [F.r.2] is populated and [F.r.3.1] and [F.r.3.4] are not populated.",
		has_text(Some(test.test_name.as_str()))
			&& !has_text(test.test_result_code.as_deref())
			&& !has_text(test.result_unstructured.as_deref())
			&& !has_text(test.test_result_value.as_deref()),
	);
	reject_when(
		issues,
		"ICH.F.r.3.2.ALLOWED.VALUE",
		&path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_decimal(test.test_result_value.as_deref()),
	);
	max_length(
		issues,
		"ICH.F.r.3.2.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.test_result_value.as_deref(),
		50,
	);
}

/// ICH.F.r.3.3.REQUIRED
/// ICH.F.r.3.3.ALLOWED.VALUE
/// ICH.F.r.3.3.LENGTH.MAX
fn f_r_3_3(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.testResultUnit");
	reject_when(
		issues,
		"ICH.F.r.3.3.REQUIRED",
		&path,
		SECTION,
		"[F.r.3.3] Test result unit is required when [F.r.3.2] is populated.",
		has_text(test.test_result_value.as_deref())
			&& !has_text(test.test_result_unit.as_deref()),
	);
	reject_when(
		issues,
		"ICH.F.r.3.3.VOCABULARY",
		&path,
		SECTION,
		ALLOWED_VALUE_MESSAGE,
		!valid_ucum(test.test_result_unit.as_deref()),
	);
	max_length(
		issues,
		"ICH.F.r.3.3.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.test_result_unit.as_deref(),
		50,
	);
}

/// ICH.F.r.3.4.REQUIRED
/// ICH.F.r.3.4.LENGTH.MAX
fn f_r_3_4(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	let path = format!("testResults.{idx}.resultUnstructured");
	reject_when(
		issues,
		"ICH.F.r.3.4.REQUIRED",
		&path,
		SECTION,
		"[F.r.3.4] Result unstructured data is required when [F.r.2] is populated and [F.r.3] is not populated.",
		has_text(Some(test.test_name.as_str()))
			&& !has_text(test.test_result_code.as_deref())
			&& !has_text(test.test_result_value.as_deref())
			&& !has_text(test.result_unstructured.as_deref()),
	);
	max_length(
		issues,
		"ICH.F.r.3.4.LENGTH.MAX",
		&path,
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.result_unstructured.as_deref(),
		2000,
	);
}

/// ICH.F.r.4.LENGTH.MAX
fn f_r_4(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.F.r.4.LENGTH.MAX",
		&format!("testResults.{idx}.normalLowValue"),
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.normal_low_value.as_deref(),
		50,
	);
}

/// ICH.F.r.5.LENGTH.MAX
fn f_r_5(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.F.r.5.LENGTH.MAX",
		&format!("testResults.{idx}.normalHighValue"),
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.normal_high_value.as_deref(),
		50,
	);
}

/// ICH.F.r.6.LENGTH.MAX
fn f_r_6(idx: usize, test: &TestResult, issues: &mut Vec<ValidationIssue>) {
	max_length(
		issues,
		"ICH.F.r.6.LENGTH.MAX",
		&format!("testResults.{idx}.comments"),
		SECTION,
		MAX_LENGTH_MESSAGE,
		test.comments.as_deref(),
		2000,
	);
}

/// MFDS.F.r.7: more test information means additional documents are available.
fn f_r_7(validation_ctx: &ValidationContext, issues: &mut Vec<ValidationIssue>) {
	if validation_ctx
		.tests
		.iter()
		.any(|test| test.more_info_available == Some(true))
		&& validation_ctx
			.safety_report
			.as_ref()
			.is_none_or(|report| report.additional_documents_available != Some(true))
	{
		crate::push_field_issue(
			issues,
			"MFDS.F.r.7.C.1.6.1.REQUIRED",
			"safetyReportIdentification.additionalDocumentsAvailable",
			"case-identification",
			"Additional documents must be marked available when more test information is available",
		);
	}
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
) {
	collect_ich_issues(validation_ctx, issues);
	if authority == RegulatoryAuthority::Mfds {
		for (idx, test) in validation_ctx.tests.iter().enumerate() {
			if test
				.test_date_null_flavor
				.as_deref()
				.map(str::trim)
				.is_some_and(|value| value != "MSK")
			{
				crate::push_business_issue(
					issues,
					"MFDS.F.r.1.NULLFLAVOR.VOCABULARY",
					format!("testResults.{idx}.testDateNullFlavor"),
					"MFDS only allows MSK as the [F.r.1] nullFlavor.",
				);
			}
		}
		f_r_7(validation_ctx, issues);
	}
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	for (idx, test) in validation_ctx.tests.iter().enumerate() {
		f_r_2(idx, test, issues);
		f_r_1(idx, test, issues);
		f_r_2_1(idx, test, issues);
		f_r_2_2a(idx, test, issues);
		f_r_2_2b(idx, test, issues);
		f_r_3_3(idx, test, issues);
		f_r_3_1(idx, test, issues);
		f_r_3_2(idx, test, issues);
		f_r_3_4(idx, test, issues);
		f_r_4(idx, test, issues);
		f_r_5(idx, test, issues);
		f_r_6(idx, test, issues);
		f_r_2_2_meddra(idx, test, validation_ctx, issues);
	}
}

#[cfg(test)]
mod golden_f_required_tests {
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::test_result::TestResult;
	use sqlx::types::time::{Date, OffsetDateTime};
	use sqlx::types::Uuid;
	use time::Month;

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

	fn test_result() -> TestResult {
		TestResult {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: String::new(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: None,
			test_result_value: None,
			test_result_qualifier: None,
			test_result_unit: None,
			result_unstructured: None,
			normal_low_value: None,
			normal_high_value: None,
			comments: None,
			more_info_available: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	#[test]
	fn mfds_only_allows_msk_test_date_null_flavor() {
		let mut ctx = empty_ctx();
		let mut row = test_result();
		row.test_date_null_flavor = Some("UNK".to_string());
		ctx.tests = vec![row];
		let mut issues = Vec::new();
		collect(&mut issues, RegulatoryAuthority::Mfds, &ctx);
		assert!(issues
			.iter()
			.any(|issue| issue.code == "MFDS.F.r.1.NULLFLAVOR.VOCABULARY"));
	}

	fn codes_for(test: TestResult) -> Vec<String> {
		let mut ctx = empty_ctx();
		ctx.tests.push(test);
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		issues.into_iter().map(|issue| issue.code).collect()
	}

	fn length_issue(code: &str, path: &str) -> (String, String) {
		(code.to_string(), path.to_string())
	}

	fn length_issues_for(test: TestResult) -> Vec<(String, String)> {
		let mut ctx = empty_ctx();
		ctx.tests.push(test);
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();
		out
	}

	#[test]
	fn allowed_value_rule_flags_invalid_test_result_code() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		test.test_result_code = Some("9".to_string());

		assert!(codes_for(test).contains(&"ICH.F.r.3.1.ALLOWED.VALUE".to_string()));
	}

	#[test]
	fn numeric_rule_flags_non_numeric_test_result_value() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		test.test_result_value = Some("not-numeric".to_string());

		assert!(codes_for(test).contains(&"ICH.F.r.3.2.ALLOWED.VALUE".to_string()));
	}

	#[test]
	fn empty_test_result_is_silent() {
		assert!(codes_for(test_result()).is_empty());
	}

	#[test]
	fn test_payload_without_name_flags_test_name() {
		let mut test = test_result();
		test.test_result_code = Some("1".to_string());

		assert_eq!(codes_for(test), vec!["ICH.F.r.2.REQUIRED".to_string()]);
	}

	#[test]
	fn test_name_without_date_flags_date_and_result_group() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.1.REQUIRED".to_string(),
				"ICH.F.r.3.1.REQUIRED".to_string(),
				"ICH.F.r.3.2.REQUIRED".to_string(),
				"ICH.F.r.3.4.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn test_date_without_name_or_meddra_code_flags_name_variants() {
		let mut test = test_result();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.2.REQUIRED".to_string(),
				"ICH.F.r.2.1.REQUIRED".to_string(),
				"ICH.F.r.2.2b.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn meddra_code_without_version_flags_version() {
		let mut test = test_result();
		test.test_meddra_code = Some("10000001".to_string());

		assert_eq!(codes_for(test), vec!["ICH.F.r.2.2a.REQUIRED".to_string()]);
	}

	#[test]
	fn result_value_without_unit_flags_unit() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_result_value = Some("15".to_string());

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.1.REQUIRED".to_string(),
				"ICH.F.r.3.3.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_f_test_result_text_fields() {
		let mut test = test_result();
		test.test_name = "T".repeat(251);
		test.test_meddra_version = Some("V".repeat(5));
		test.test_meddra_code = Some("M".repeat(9));
		test.test_result_code = Some("RC".to_string());
		test.test_result_value = Some("V".repeat(51));
		test.test_result_unit = Some("U".repeat(51));
		test.result_unstructured = Some("R".repeat(2001));
		test.normal_low_value = Some("L".repeat(51));
		test.normal_high_value = Some("H".repeat(51));
		test.comments = Some("C".repeat(2001));

		assert_eq!(
			length_issues_for(test),
			vec![
				length_issue("ICH.F.r.2.1.LENGTH.MAX", "testResults.0.testName"),
				length_issue(
					"ICH.F.r.2.2a.LENGTH.MAX",
					"testResults.0.testMeddraVersion"
				),
				length_issue(
					"ICH.F.r.2.2b.LENGTH.MAX",
					"testResults.0.testMeddraCode"
				),
				length_issue(
					"ICH.F.r.3.1.LENGTH.MAX",
					"testResults.0.testResultCode"
				),
				length_issue(
					"ICH.F.r.3.2.LENGTH.MAX",
					"testResults.0.testResultValue"
				),
				length_issue(
					"ICH.F.r.3.3.LENGTH.MAX",
					"testResults.0.testResultUnit"
				),
				length_issue(
					"ICH.F.r.3.4.LENGTH.MAX",
					"testResults.0.resultUnstructured"
				),
				length_issue("ICH.F.r.4.LENGTH.MAX", "testResults.0.normalLowValue"),
				length_issue(
					"ICH.F.r.5.LENGTH.MAX",
					"testResults.0.normalHighValue"
				),
				length_issue("ICH.F.r.6.LENGTH.MAX", "testResults.0.comments"),
			]
		);
	}

	#[test]
	fn more_information_requires_additional_documents() {
		let mut ctx = empty_ctx();
		let mut test = test_result();
		test.more_info_available = Some(true);
		ctx.tests = vec![test];
		let mut issues = Vec::new();
		f_r_7(&ctx, &mut issues);
		let issue = issues
			.iter()
			.find(|issue| issue.code == "MFDS.F.r.7.C.1.6.1.REQUIRED")
			.expect("MFDS F.r.7 issue");
		assert_eq!(
			issue.path,
			"safetyReportIdentification.additionalDocumentsAvailable"
		);
		assert_eq!(
			issue.field_path.as_deref(),
			Some("safetyReportIdentification.additionalDocumentsAvailable")
		);
		assert_eq!(issue.section, "case-identification");
	}

	#[test]
	fn golden_f_issue_metadata() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_result_code = Some("9".to_string());
		let mut ctx = empty_ctx();
		ctx.tests.push(test);
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| {
				matches!(
					issue.code.as_str(),
					"ICH.F.r.1.REQUIRED" | "ICH.F.r.3.1.ALLOWED.VALUE"
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
					"ICH.F.r.1.REQUIRED".to_string(),
					"[F.r.1] Test date is required when [F.r.2] is populated."
						.to_string(),
					"testResults.0.testDate".to_string(),
					Some("testResults.0.testDate".to_string()),
					"tests".to_string(),
					"F.r".to_string(),
				),
				(
					"ICH.F.r.3.1.ALLOWED.VALUE".to_string(),
					"Dictionary allowed values constraint.".to_string(),
					"testResults.0.testResultCode".to_string(),
					Some("testResults.0.testResultCode".to_string()),
					"tests".to_string(),
					"F.r".to_string(),
				),
			],
		);
	}
}
