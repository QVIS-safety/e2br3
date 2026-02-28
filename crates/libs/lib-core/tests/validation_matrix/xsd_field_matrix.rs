use lib_core::xml::validate::rule_test_matrix::{RuleLayer, XSD_RULE_TEST_MATRIX};
use lib_core::xml::validate::{find_canonical_rule_for_phase, ValidationPhase};

#[test]
fn xsd_matrix_codes_are_import_catalog_backed() {
	for spec in XSD_RULE_TEST_MATRIX {
		assert_eq!(spec.layer, RuleLayer::Xsd);
		let rule = find_canonical_rule_for_phase(spec.code, ValidationPhase::Import)
			.unwrap_or_else(|| panic!("missing import rule for {}", spec.code));
		assert!(
			rule.blocking,
			"xsd/import rule must be blocking: {}",
			spec.code
		);
	}
}
