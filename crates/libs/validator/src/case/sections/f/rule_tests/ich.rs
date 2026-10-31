use super::*;

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
	super::super::super::collect_meddra_version_issues(&ctx, &mut issues);
	issues
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
		_ => panic!("missing rule fixture: {code}"),
	};
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rules { ($($name:ident: $code:literal,)+) => { $(#[test] fn $name() { verify($code); })+ }; }
rules! {
	rule_ich_f_r_2_2a_allowed_value: "ICH.F.r.2.2a.ALLOWED.VALUE",
	rule_ich_f_r_2_2a_required: "ICH.F.r.2.2a.REQUIRED",
	rule_ich_f_r_2_2a_vocabulary: "ICH.F.r.2.2a.VOCABULARY",
	rule_ich_f_r_2_2b_vocabulary: "ICH.F.r.2.2b.VOCABULARY",
	rule_ich_f_r_3_3_required: "ICH.F.r.3.3.REQUIRED",
	rule_ich_f_r_3_3_vocabulary: "ICH.F.r.3.3.VOCABULARY",
}

#[test]
fn rule_ich_f_r_1_future_date_forbidden() {
	verify("ICH.F.r.1.FUTURE_DATE.FORBIDDEN");

	let today = time::OffsetDateTime::now_utc().date();
	for offset in [-1, 0, 1] {
		let date = today + time::Duration::days(offset);
		let value = format!(
			"{:04}{:02}{:02}",
			date.year(),
			date.month() as u8,
			date.day()
		);

		let mut row = test_result();
		row.test_date = Some(value);
		assert_eq!(
			rule_issues(&row)
				.iter()
				.any(|i| i.code == "ICH.F.r.1.FUTURE_DATE.FORBIDDEN"),
			offset > 0
		);
	}
}

#[test]
fn rule_ich_f_r_1_required() {
	verify("ICH.F.r.1.REQUIRED");

	let mut row = test_result();
	row.test_name = "ALT".into();
	row.test_date_null_flavor = Some("UNK".into());
	assert!(!rule_issues(&row)
		.iter()
		.any(|i| i.code == "ICH.F.r.1.REQUIRED"));
}

#[test]
fn rule_ich_f_r_2_1_required() {
	verify("ICH.F.r.2.1.REQUIRED");

	let mut row = test_result();
	row.test_date = Some("20200101".into());
	row.test_meddra_code = Some("10000001".into());
	assert!(!rule_issues(&row)
		.iter()
		.any(|i| i.code == "ICH.F.r.2.1.REQUIRED"));
}

#[test]
fn rule_ich_f_r_2_2b_required() {
	verify("ICH.F.r.2.2b.REQUIRED");

	let mut row = test_result();
	row.test_date = Some("20200101".into());
	row.test_name = "ALT".into();
	assert!(!rule_issues(&row)
		.iter()
		.any(|i| i.code == "ICH.F.r.2.2b.REQUIRED"));
}

#[test]
fn rule_ich_f_r_2_required() {
	verify("ICH.F.r.2.REQUIRED");

	for more_info in [None, Some(false), Some(true)] {
		let mut row = test_result();
		row.more_info_available = more_info;
		row.test_result_code = Some("1".into());
		assert!(rule_issues(&row)
			.iter()
			.any(|i| i.code == "ICH.F.r.2.REQUIRED"));
		row.test_meddra_code = Some("10000001".into());
		assert!(!rule_issues(&row)
			.iter()
			.any(|i| i.code == "ICH.F.r.2.REQUIRED"));
	}
}

#[test]
fn rule_ich_f_r_3_1_required() {
	verify("ICH.F.r.3.1.REQUIRED");

	for alternative in 0..3 {
		let mut row = test_result();
		row.test_name = "ALT".into();
		match alternative {
			0 => row.test_result_code = Some("1".into()),
			1 => row.test_result_value = Some("0".into()),
			2 => row.result_unstructured = Some("Normal".into()),
			_ => unreachable!(),
		}
		let issues = rule_issues(&row);
		for code in [
			"ICH.F.r.3.1.REQUIRED",
			"ICH.F.r.3.2.REQUIRED",
			"ICH.F.r.3.4.REQUIRED",
		] {
			assert!(
				!issues.iter().any(|i| i.code == code),
				"alternative={alternative}, code={code}"
			);
		}
	}
}

#[test]
fn rule_ich_f_r_3_2_required() {
	verify("ICH.F.r.3.2.REQUIRED");

	for alternative in 0..3 {
		let mut row = test_result();
		row.test_name = "ALT".into();
		match alternative {
			0 => row.test_result_code = Some("1".into()),
			1 => row.test_result_value = Some("0".into()),
			2 => row.result_unstructured = Some("Normal".into()),
			_ => unreachable!(),
		}
		let issues = rule_issues(&row);
		for code in [
			"ICH.F.r.3.1.REQUIRED",
			"ICH.F.r.3.2.REQUIRED",
			"ICH.F.r.3.4.REQUIRED",
		] {
			assert!(
				!issues.iter().any(|i| i.code == code),
				"alternative={alternative}, code={code}"
			);
		}
	}
}

#[test]
fn rule_ich_f_r_3_4_required() {
	verify("ICH.F.r.3.4.REQUIRED");

	for alternative in 0..3 {
		let mut row = test_result();
		row.test_name = "ALT".into();
		match alternative {
			0 => row.test_result_code = Some("1".into()),
			1 => row.test_result_value = Some("0".into()),
			2 => row.result_unstructured = Some("Normal".into()),
			_ => unreachable!(),
		}
		let issues = rule_issues(&row);
		for code in [
			"ICH.F.r.3.1.REQUIRED",
			"ICH.F.r.3.2.REQUIRED",
			"ICH.F.r.3.4.REQUIRED",
		] {
			assert!(
				!issues.iter().any(|i| i.code == code),
				"alternative={alternative}, code={code}"
			);
		}
	}
}

#[test]
fn rule_ich_f_r_2_2a_meddra_version_consistent() {
	let mut ctx = empty_ctx();
	let mut a = test_result();
	a.test_meddra_version = Some("26.1".into());
	let mut b = a.clone();
	b.test_meddra_version = Some("27.0".into());
	ctx.tests = vec![a, b];
	let mut invalid = Vec::new();
	super::super::super::collect_meddra_version_issues(&ctx, &mut invalid);
	assert!(invalid
		.iter()
		.any(|i| i.code == "ICH.F.r.2.2a.MEDDRA.VERSION.CONSISTENT"
			&& i.path == "testResults.1.testMeddraVersion"));
	for version in [Some("26.1"), Some(" 26.1 "), Some(""), None] {
		ctx.tests[1].test_meddra_version = version.map(str::to_owned);
		let mut repaired = Vec::new();
		super::super::super::collect_meddra_version_issues(&ctx, &mut repaired);
		assert_edge(
			"ICH.F.r.2.2a.MEDDRA.VERSION.CONSISTENT",
			"testResults.0.testMeddraVersion",
			invalid.clone(),
			repaired,
		);
	}
}
