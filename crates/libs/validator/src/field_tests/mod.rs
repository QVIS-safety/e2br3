//! JSON-owned field tests reuse the actual rule fixtures and inspect their issues.
use crate::case::sections::{c, d, e, f, g, h};
use crate::ValidationIssue;
use std::cell::RefCell;
use std::collections::BTreeSet;

struct RuleResult {
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
}

thread_local! {
	static CAPTURE: RefCell<Option<Vec<RuleResult>>> = const { RefCell::new(None) };
}

pub(crate) fn record_rule_result(
	invalid: &[ValidationIssue],
	repaired: &[ValidationIssue],
) {
	CAPTURE.with(|slot| {
		if let Some(results) = slot.borrow_mut().as_mut() {
			results.push(RuleResult {
				invalid: invalid.to_vec(),
				repaired: repaired.to_vec(),
			});
		}
	});
}

fn rule_tests() -> impl Iterator<Item = &'static (&'static str, fn())> {
	[
		c::rule_tests::RULE_TESTS,
		d::rule_tests::RULE_TESTS,
		e::rule_tests::RULE_TESTS,
		f::rule_tests::RULE_TESTS,
		g::rule_tests::RULE_TESTS,
		h::rule_tests::RULE_TESTS,
	]
	.into_iter()
	.flatten()
}

fn capture(test: fn()) -> RuleResult {
	CAPTURE.with(|slot| {
		assert!(
			slot.replace(Some(Vec::new())).is_none(),
			"nested rule capture"
		)
	});
	let outcome = std::panic::catch_unwind(test);
	let results =
		CAPTURE.with(|slot| slot.take().expect("rule capture was initialized"));
	if let Err(payload) = outcome {
		std::panic::resume_unwind(payload);
	}
	assert_eq!(
		results.len(),
		1,
		"rule fixture must report exactly one invalid/repair pair"
	);
	results
		.into_iter()
		.next()
		.expect("checked exactly one rule result")
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldManifest {
	fields: Vec<FieldBinding>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldBinding {
	field_path: String,
	rule_codes: Vec<String>,
}

fn catalog() -> &'static Catalog {
	static CATALOG: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
	CATALOG.get_or_init(|| {
		serde_json::from_str(include_str!("../../rules.json"))
			.expect("valid rule catalog")
	})
}

fn check_field(manifest: &str, path: &str) {
	let manifest: FieldManifest =
		serde_json::from_str(manifest).expect("valid field-rule JSON");
	let matches: Vec<_> = manifest
		.fields
		.iter()
		.filter(|field| field.field_path == path)
		.collect();
	assert_eq!(
		matches.len(),
		1,
		"expected exactly one field binding for {path}"
	);
	let field = matches[0];
	assert!(
		!field.rule_codes.is_empty(),
		"field test must execute at least one rule: {path}"
	);
	let unique: BTreeSet<_> = field.rule_codes.iter().collect();
	assert_eq!(
		unique.len(),
		field.rule_codes.len(),
		"duplicate rule binding: {path}"
	);
	for code in &field.rule_codes {
		let binding = catalog()
			.rules
			.iter()
			.find(|rule| &rule.rule_code == code)
			.expect("known rule binding");
		let (_, test) = rule_tests()
			.find(|(registered, _)| *registered == code)
			.expect("executable Rust rule test");
		let result = capture(*test);
		assert!(
			result
				.invalid
				.iter()
				.any(|issue| issue.code == binding.expected_issue_code
					&& path_template(&issue.path) == path),
			"{code}: invalid fixture did not report field {path}: {:?}",
			result.invalid
		);
		assert!(
			!result
				.repaired
				.iter()
				.any(|issue| issue.code == binding.expected_issue_code
					&& path_template(&issue.path) == path),
			"{code}: field {path} remained invalid after repair: {:?}",
			result.repaired
		);
	}
}

#[test]
fn catalog_matches_rule_fixtures() {
	let registered: BTreeSet<_> = rule_tests().map(|(code, _)| *code).collect();
	let declared: BTreeSet<_> = catalog()
		.rules
		.iter()
		.map(|rule| rule.rule_code.as_str())
		.collect();
	assert_eq!(
		registered, declared,
		"catalog must bind every executable rule test"
	);
	assert_eq!(
		rule_tests().count(),
		registered.len(),
		"duplicate registered rule test"
	);
	assert_eq!(
		catalog().rules.len(),
		declared.len(),
		"duplicate catalog rule"
	);
	for binding in &catalog().rules {
		let (_, test) = rule_tests()
			.find(|(code, _)| *code == binding.rule_code)
			.expect("checked rule binding");
		let result = capture(*test);
		let actual: BTreeSet<_> = result
			.invalid
			.iter()
			.filter(|issue| issue.code == binding.expected_issue_code)
			.map(|issue| path_template(&issue.path))
			.collect();
		let expected: BTreeSet<_> =
			binding.expected_issue_paths.iter().cloned().collect();
		assert!(!expected.is_empty(), "rule must declare tested field paths");
		assert_eq!(actual, expected, "stale paths for {}", binding.rule_code);
	}
}

#[test]
fn json_binding_rejects_an_unrelated_rule_and_empty_rule_list() {
	let path = "safetyReportIdentification.safetyReportId";
	for rules in [vec!["ICH.C.1.2.REQUIRED"], vec![]] {
		let manifest =
			serde_json::json!({"fields":[{"fieldPath":path,"ruleCodes":rules}]})
				.to_string();
		assert!(std::panic::catch_unwind(|| check_field(&manifest, path)).is_err());
	}
	check_field(include_str!("../../field-rules/ci.json"), path);
}

macro_rules! field_validation_test {
	($name:ident, $path:literal) => {
		#[test]
		fn $name() {
			super::check_field(MANIFEST, $path);
		}
	};
}

include!("generated.rs");
