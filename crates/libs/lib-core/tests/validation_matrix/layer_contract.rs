use lib_core::xml::validate::rule_layer_contract::rule_layer_contract;
use lib_core::xml::validate::rule_test_matrix::{
	CASE_RULE_TEST_MATRIX, XSD_RULE_TEST_MATRIX,
};
use lib_core::xml::validate::{
	canonical_rules_for_phase, ValidationPhase, CASE_VALIDATOR_RULE_CODES,
};
use std::collections::HashSet;

#[test]
fn contract_covers_all_case_matrix_codes_as_case_validator() {
	for spec in CASE_RULE_TEST_MATRIX {
		let contract = rule_layer_contract(spec.code)
			.unwrap_or_else(|| panic!("missing layer contract for {}", spec.code));
		assert!(
			contract.case_validator,
			"{} must be case_validator in layer contract",
			spec.code
		);
	}
}

#[test]
fn contract_covers_all_xsd_matrix_codes_as_xsd_layer() {
	for spec in XSD_RULE_TEST_MATRIX {
		let contract = rule_layer_contract(spec.code)
			.unwrap_or_else(|| panic!("missing layer contract for {}", spec.code));
		assert!(contract.xsd, "{} must be xsd in layer contract", spec.code);
	}
}

#[test]
fn contract_case_validator_set_matches_registry() {
	let expected: HashSet<&str> =
		CASE_VALIDATOR_RULE_CODES.iter().copied().collect();
	let actual: HashSet<&str> = CASE_VALIDATOR_RULE_CODES
		.iter()
		.copied()
		.filter(|code| {
			rule_layer_contract(code)
				.map(|contract| contract.case_validator)
				.unwrap_or(false)
		})
		.collect();
	assert_eq!(
		actual, expected,
		"layer contract case_validator ownership must match case registry"
	);
}

#[test]
fn contract_captures_known_xml_business_structural_rules() {
	for code in [
		"ICH.XML.BL.NULLFLAVOR.REQUIRED",
		"ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED",
	] {
		let contract = rule_layer_contract(code)
			.unwrap_or_else(|| panic!("missing layer contract for {code}"));
		assert!(contract.xml_business, "{code} must be xml_business");
		assert!(
			!contract.case_validator,
			"{code} must not be case_validator"
		);
	}
}

#[test]
fn contract_covers_all_blocking_import_rules() {
	for rule in canonical_rules_for_phase(ValidationPhase::Import) {
		if !rule.blocking {
			continue;
		}
		let contract = rule_layer_contract(rule.code).unwrap_or_else(|| {
			panic!(
				"missing layer contract for blocking import rule {}",
				rule.code
			)
		});
		assert!(
			contract.xml_business || contract.xsd,
			"blocking import rule must be xml_business or xsd in contract: {}",
			rule.code
		);
	}
}
