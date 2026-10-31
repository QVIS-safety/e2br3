use super::helpers::{
	e2b_ts_date, max_length, reject_future_date, reject_when, valid_code,
	valid_decimal, valid_dotted_version, valid_meddra_term, valid_meddra_version,
	valid_ucum, DateValues,
};
use crate::{
	has_test_name, has_test_payload, has_text, RegulatoryAuthority,
	ValidationContext, ValidationIssue,
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
		has_test_name(test)
			&& test.test_date.is_none()
			&& !has_text(test.test_date_null_flavor.as_deref()),
	);
	reject_future_date(
		issues,
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		&path,
		SECTION,
		"[F.r.1] Test date must not be later than today.",
		DateValues::One(e2b_ts_date(test.test_date.as_deref())),
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
		has_test_payload(test) && !has_test_name(test),
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
		has_test_name(test)
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
		has_test_name(test)
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
		has_test_name(test)
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
				.is_some_and(|value| value != "UNK")
			{
				crate::push_business_issue(
					issues,
					"MFDS.F.r.1.NULLFLAVOR.VOCABULARY",
					format!("testResults.{idx}.testDateNullFlavor"),
					"MFDS only allows UNK as the [F.r.1] nullFlavor.",
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
#[path = "f/rule_tests.rs"]
mod rule_tests;

#[cfg(test)]
#[path = "f/golden_f_required_tests.rs"]
mod golden_f_required_tests;
