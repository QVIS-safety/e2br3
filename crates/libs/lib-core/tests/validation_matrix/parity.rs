use lib_core::xml::validate::rule_test_matrix::{
	RuleTestSpec, CASE_RULE_TEST_MATRIX,
};
use lib_core::xml::validate::{
	canonical_rules_for_phase, find_canonical_rule_for_phase, RuleCategory,
	ValidationPhase,
};
use std::collections::HashSet;

fn matrix_codes(matrix: &[RuleTestSpec]) -> HashSet<&'static str> {
	matrix.iter().map(|spec| spec.code).collect()
}

#[test]
fn case_rule_matrix_matches_case_business_catalog() {
	let expected: HashSet<&str> =
		canonical_rules_for_phase(ValidationPhase::CaseValidate)
			.into_iter()
			.filter(|rule| rule.category == RuleCategory::CaseBusiness)
			.map(|rule| rule.code)
			.collect();
	let actual = matrix_codes(CASE_RULE_TEST_MATRIX);
	assert_eq!(
		actual, expected,
		"case rule matrix must track case business canonical rules"
	);
}

#[test]
fn case_rule_matrix_entries_are_unique_and_catalog_backed() {
	let mut seen = HashSet::new();
	for spec in CASE_RULE_TEST_MATRIX {
		assert!(
			seen.insert(spec.code),
			"duplicate matrix code: {}",
			spec.code
		);
		assert!(
			find_canonical_rule_for_phase(spec.code, ValidationPhase::CaseValidate)
				.is_some(),
			"matrix code missing from case_validate catalog: {}",
			spec.code
		);
	}
}
