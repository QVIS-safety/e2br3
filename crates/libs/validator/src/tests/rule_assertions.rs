use crate::ValidationIssue;
use std::collections::BTreeSet;

#[derive(serde::Deserialize)]
struct Catalog {
	rules: Vec<RuleBinding>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleBinding {
	rule_code: String,
	expected_issue_code: String,
	expected_issue_paths: Vec<String>,
}

fn path_template(path: &str) -> String {
	let mut out = String::new();
	for part in path.split('.') {
		if !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()) {
			out.push_str("[]");
		} else {
			if !out.is_empty() {
				out.push('.');
			}
			out.push_str(part);
		}
	}
	out
}

pub(crate) fn assert_rule_result(
	code: &str,
	invalid: &[ValidationIssue],
	repaired: &[ValidationIssue],
) {
	static CATALOG: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
	let catalog = CATALOG.get_or_init(|| {
		serde_json::from_str(include_str!("../../rules.json"))
			.expect("valid rule catalog")
	});
	let binding = catalog
		.rules
		.iter()
		.find(|rule| rule.rule_code == code)
		.expect("rule test must have a catalog binding");
	let actual: BTreeSet<_> = invalid
		.iter()
		.filter(|issue| issue.code == binding.expected_issue_code)
		.map(|issue| path_template(&issue.path))
		.collect();
	let expected: BTreeSet<_> =
		binding.expected_issue_paths.iter().cloned().collect();
	assert!(!expected.is_empty(), "rule must declare tested field paths");
	assert_eq!(actual, expected, "stale paths or missing issue for {code}");
	assert!(
		!repaired
			.iter()
			.any(|issue| issue.code == binding.expected_issue_code),
		"{code} remained after repair: {repaired:?}"
	);
}

#[test]
fn catalog_assertion_rejects_wrong_code_path_and_failed_repair() {
	let code = "ICH.C.1.1.REQUIRED";
	let mut invalid = Vec::new();
	crate::push_business_issue(
		&mut invalid,
		code,
		"safetyReportIdentification.safetyReportId",
		"required",
	);
	assert_rule_result(code, &invalid, &[]);
	assert!(std::panic::catch_unwind(|| {
		assert_rule_result(code, &invalid, &invalid)
	})
	.is_err());
	let mut wrong_path = invalid.clone();
	wrong_path[0].path = "wrong.field".into();
	assert!(std::panic::catch_unwind(|| {
		assert_rule_result(code, &wrong_path, &[])
	})
	.is_err());
	let mut wrong_code = invalid;
	wrong_code[0].code = "ICH.C.1.2.REQUIRED".into();
	assert!(std::panic::catch_unwind(|| {
		assert_rule_result(code, &wrong_code, &[])
	})
	.is_err());
}
