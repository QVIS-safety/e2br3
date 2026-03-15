use lib_core::xml::validate::{is_rule_condition_satisfied, RuleFacts};

#[test]
fn mfds_d_8_r_1_kr_1a_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.D.8.r.1.KR.1a.REQUIRED",
		RuleFacts {
			mfds_past_drug_version_required_context: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_8_r_1_kr_1a_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.D.8.r.1.KR.1a.REQUIRED",
		RuleFacts {
			mfds_past_drug_version_required_context: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_8_r_1_kr_1b_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.D.8.r.1.KR.1b.REQUIRED",
		RuleFacts {
			mfds_past_drug_code_required_context: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_8_r_1_kr_1b_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.D.8.r.1.KR.1b.REQUIRED",
		RuleFacts {
			mfds_past_drug_code_required_context: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_10_8_r_1_kr_1a_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		RuleFacts {
			mfds_parent_past_drug_version_required_context: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_10_8_r_1_kr_1a_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		RuleFacts {
			mfds_parent_past_drug_version_required_context: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_10_8_r_1_kr_1b_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		RuleFacts {
			mfds_parent_past_drug_code_required_context: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_d_10_8_r_1_kr_1b_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		RuleFacts {
			mfds_parent_past_drug_code_required_context: Some(true),
			..RuleFacts::default()
		},
	));
}
