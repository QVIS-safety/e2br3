use lib_core::xml::validate::rule_test_matrix::{RuleLayer, CASE_RULE_TEST_MATRIX};

#[test]
fn case_rule_matrix_has_entries_for_all_profiles() {
	let ich = CASE_RULE_TEST_MATRIX
		.iter()
		.filter(|spec| spec.layer == RuleLayer::Ich)
		.count();
	let fda = CASE_RULE_TEST_MATRIX
		.iter()
		.filter(|spec| spec.layer == RuleLayer::Fda)
		.count();
	let mfds = CASE_RULE_TEST_MATRIX
		.iter()
		.filter(|spec| spec.layer == RuleLayer::Mfds)
		.count();

	assert!(ich > 0, "ICH matrix must not be empty");
	assert!(fda > 0, "FDA matrix must not be empty");
	assert!(mfds > 0, "MFDS matrix must not be empty");
}

#[test]
fn case_rule_matrix_paths_are_not_empty() {
	for spec in CASE_RULE_TEST_MATRIX {
		assert!(
			!spec.field_path.trim().is_empty(),
			"matrix field_path must be set for {}",
			spec.code
		);
	}
}
