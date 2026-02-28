use lib_core::xml::validate::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts,
};

#[test]
fn non_executable_ich_case_required_rules_are_value_strict() {
	let rules = [
		"ICH.C.1.2.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.4.REQUIRED",
		"ICH.C.1.5.REQUIRED",
		"ICH.C.1.7.REQUIRED",
	];

	for code in rules {
		assert!(
			!is_rule_value_valid(code, None, None, RuleFacts::default()),
			"{code} should reject missing value in policy lane"
		);
		assert!(
			!is_rule_value_valid(code, Some(""), None, RuleFacts::default()),
			"{code} should reject blank value in policy lane"
		);
		assert!(
			is_rule_value_valid(code, Some("1"), None, RuleFacts::default()),
			"{code} should accept non-empty value in policy lane"
		);
	}
}

#[test]
fn non_executable_fda_c112_required_is_still_policy_validated() {
	let code = "FDA.C.1.12.REQUIRED";
	assert!(!is_rule_value_valid(code, None, None, RuleFacts::default()));
	assert!(!is_rule_value_valid(
		code,
		Some(""),
		None,
		RuleFacts::default()
	));
	assert!(is_rule_value_valid(
		code,
		None,
		Some("NI"),
		RuleFacts::default()
	));
}

#[test]
fn non_executable_fda_required_intervention_condition_and_value_policy() {
	let code = "FDA.E.i.3.2h.REQUIRED";
	let facts = RuleFacts {
		fda_reaction_other_medically_important: Some(true),
		..RuleFacts::default()
	};
	assert!(is_rule_condition_satisfied(code, facts));
	assert!(!is_rule_value_valid(code, None, None, RuleFacts::default()));
	assert!(is_rule_value_valid(
		code,
		None,
		Some("NI"),
		RuleFacts::default()
	));
}
