use super::golden_f_required_tests::{empty_ctx, test_result};
use super::*;
use lib_core::model::safety_report::SafetyReportIdentification;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::field_tests::record_rule_result(&invalid, &repaired);
	let emitted_code = if code == "ICH.F.r.2.2a.VOCABULARY" {
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

fn rule_issues(test: &TestResult) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	f_r_2(0, test, &mut issues);
	f_r_1(0, test, &mut issues);
	f_r_2_1(0, test, &mut issues);
	f_r_2_2a(0, test, &mut issues);
	f_r_2_2b(0, test, &mut issues);
	f_r_3_3(0, test, &mut issues);
	f_r_3_1(0, test, &mut issues);
	f_r_3_2(0, test, &mut issues);
	f_r_3_4(0, test, &mut issues);
	issues
}

fn meddra_issues(
	test: &TestResult,
	vocabulary: crate::context::VocabularyContext,
) -> Vec<ValidationIssue> {
	let mut ctx = empty_ctx();
	ctx.vocabulary = vocabulary;
	ctx.tests.push(test.clone());
	let mut issues = Vec::new();
	f_r_2_2_meddra(0, test, &ctx, &mut issues);
	super::super::collect_meddra_version_issues(&ctx, &mut issues);
	issues
}

fn report(additional_documents_available: bool) -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 1,
		transmission_date: None,
		report_type: None,
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: Some(additional_documents_available),
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn verify(code: &str) {
	let (path, invalid, repaired) = match code {
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN" => {
			let mut bad = test_result();
			bad.test_date = Some("29990101".into());
			let mut good = bad.clone();
			good.test_date = Some("20200101".into());
			(
				"testResults.0.testDate",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.1.REQUIRED" => {
			let mut bad = test_result();
			bad.test_name = "ALT".into();
			let mut good = bad.clone();
			good.test_date = Some("20200101".into());
			(
				"testResults.0.testDate",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.2.REQUIRED" => {
			let mut bad = test_result();
			bad.test_result_code = Some("1".into());
			let mut good = bad.clone();
			good.test_name = "ALT".into();
			(
				"testResults.0.testName",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.2.1.REQUIRED" => {
			let mut bad = test_result();
			bad.test_date = Some("20200101".into());
			let mut good = bad.clone();
			good.test_name = "ALT".into();
			(
				"testResults.0.testName",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.2.2a.REQUIRED" => {
			let mut bad = test_result();
			bad.test_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			good.test_meddra_version = Some("26.1".into());
			(
				"testResults.0.testMeddraVersion",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.2.2b.REQUIRED" => {
			let mut bad = test_result();
			bad.test_date = Some("20200101".into());
			let mut good = bad.clone();
			good.test_meddra_code = Some("10000001".into());
			(
				"testResults.0.testMeddraCode",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.2.2a.ALLOWED.VALUE" => {
			let mut bad = test_result();
			bad.test_meddra_version = Some("bad".into());
			bad.test_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			good.test_meddra_version = Some("26.1".into());
			(
				"testResults.0.testMeddraVersion",
				meddra_issues(&bad, Default::default()),
				meddra_issues(&good, Default::default()),
			)
		}
		"ICH.F.r.2.2a.VOCABULARY" => {
			let vocab = || {
				crate::context::VocabularyContext::for_meddra(&[(
					"26.1", "10000001",
				)])
			};
			let mut bad = test_result();
			bad.test_meddra_version = Some("27.0".into());
			bad.test_meddra_code = Some("10000001".into());
			let mut good = bad.clone();
			good.test_meddra_version = Some("26.1".into());
			(
				"testResults.0.testMeddraVersion",
				meddra_issues(&bad, vocab()),
				meddra_issues(&good, vocab()),
			)
		}
		"ICH.F.r.2.2b.VOCABULARY" => {
			let vocab = || {
				crate::context::VocabularyContext::for_meddra(&[(
					"26.1", "10000001",
				)])
			};
			let mut bad = test_result();
			bad.test_meddra_version = Some("26.1".into());
			bad.test_meddra_code = Some("99999999".into());
			let mut good = bad.clone();
			good.test_meddra_code = Some("10000001".into());
			(
				"testResults.0.testMeddraCode",
				meddra_issues(&bad, vocab()),
				meddra_issues(&good, vocab()),
			)
		}
		"ICH.F.r.3.1.REQUIRED" | "ICH.F.r.3.2.REQUIRED" | "ICH.F.r.3.4.REQUIRED" => {
			let mut bad = test_result();
			bad.test_name = "ALT".into();
			let mut good = bad.clone();
			let path = if code.contains("3.1") {
				good.test_result_code = Some("1".into());
				"testResults.0.testResultCode"
			} else if code.contains("3.2") {
				good.test_result_value = Some("1".into());
				"testResults.0.testResultValue"
			} else if code == "ICH.F.r.3.4.REQUIRED" {
				good.result_unstructured = Some("normal".into());
				"testResults.0.resultUnstructured"
			} else {
				panic!("missing result fixture: {code}")
			};
			(path, rule_issues(&bad), rule_issues(&good))
		}
		"ICH.F.r.3.3.REQUIRED" => {
			let mut bad = test_result();
			bad.test_result_value = Some("1".into());
			let mut good = bad.clone();
			good.test_result_unit = Some("mg".into());
			(
				"testResults.0.testResultUnit",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"ICH.F.r.3.3.VOCABULARY" => {
			let mut bad = test_result();
			bad.test_result_unit = Some("not ucum".into());
			let mut good = bad.clone();
			good.test_result_unit = Some("mg".into());
			(
				"testResults.0.testResultUnit",
				rule_issues(&bad),
				rule_issues(&good),
			)
		}
		"MFDS.F.r.1.NULLFLAVOR.VOCABULARY" => {
			let mut bad = empty_ctx();
			let mut row = test_result();
			row.test_date_null_flavor = Some("MSK".into());
			bad.tests.push(row);
			let mut good = bad.clone();
			good.tests[0].test_date_null_flavor = Some("UNK".into());
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				collect(&mut issues, RegulatoryAuthority::Mfds, ctx);
				issues
			};
			("testResults.0.testDateNullFlavor", run(&bad), run(&good))
		}
		"MFDS.F.r.7.C.1.6.1.REQUIRED" => {
			let mut bad = empty_ctx();
			let mut row = test_result();
			row.more_info_available = Some(true);
			bad.tests.push(row);
			let mut good = bad.clone();
			good.safety_report = Some(report(true));
			let run = |ctx: &ValidationContext| {
				let mut issues = Vec::new();
				f_r_7(ctx, &mut issues);
				issues
			};
			(
				"safetyReportIdentification.additionalDocumentsAvailable",
				run(&bad),
				run(&good),
			)
		}
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_ich_f_r_1_future_date_forbidden: "ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
	rule_ich_f_r_1_required: "ICH.F.r.1.REQUIRED",
	rule_ich_f_r_2_1_required: "ICH.F.r.2.1.REQUIRED",
	rule_ich_f_r_2_2a_allowed_value: "ICH.F.r.2.2a.ALLOWED.VALUE",
	rule_ich_f_r_2_2a_required: "ICH.F.r.2.2a.REQUIRED",
	rule_ich_f_r_2_2a_vocabulary: "ICH.F.r.2.2a.VOCABULARY",
	rule_ich_f_r_2_2b_required: "ICH.F.r.2.2b.REQUIRED",
	rule_ich_f_r_2_2b_vocabulary: "ICH.F.r.2.2b.VOCABULARY",
	rule_ich_f_r_2_required: "ICH.F.r.2.REQUIRED",
	rule_ich_f_r_3_1_required: "ICH.F.r.3.1.REQUIRED",
	rule_ich_f_r_3_2_required: "ICH.F.r.3.2.REQUIRED",
	rule_ich_f_r_3_3_required: "ICH.F.r.3.3.REQUIRED",
	rule_ich_f_r_3_3_vocabulary: "ICH.F.r.3.3.VOCABULARY",
	rule_ich_f_r_3_4_required: "ICH.F.r.3.4.REQUIRED",
	rule_mfds_f_r_1_nullflavor_vocabulary: "MFDS.F.r.1.NULLFLAVOR.VOCABULARY",
	rule_mfds_f_r_7_c_1_6_1_required: "MFDS.F.r.7.C.1.6.1.REQUIRED",
}

include!("rule_test_bindings.rs");
