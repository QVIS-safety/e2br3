use lib_core::validation::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts,
};

#[test]
fn mfds_c_2_r_4_kr_1_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.C.2.r.4.KR.1.REQUIRED",
		RuleFacts {
			mfds_primary_source_qualification_is_three: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_c_2_r_4_kr_1_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.C.2.r.4.KR.1.REQUIRED",
		RuleFacts {
			mfds_primary_source_qualification_is_three: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_c_3_1_kr_1_required_false() {
	assert!(!is_rule_condition_satisfied(
		"MFDS.C.3.1.KR.1.REQUIRED",
		RuleFacts {
			mfds_sender_type_disallowed: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_c_3_1_kr_1_required_true() {
	assert!(is_rule_condition_satisfied(
		"MFDS.C.3.1.KR.1.REQUIRED",
		RuleFacts {
			mfds_sender_type_disallowed: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn mfds_c_5_4_kr_1_required_false() {
	assert!(!is_rule_value_valid(
		"MFDS.C.5.4.KR.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn mfds_c_5_4_kr_1_required_true() {
	assert!(is_rule_value_valid(
		"MFDS.C.5.4.KR.1.REQUIRED",
		Some("KR1"),
		None,
		RuleFacts::default(),
	));
}
