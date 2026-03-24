use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_issue_metadata,
	assert_lacks_issue, assert_lacks_xml_rule, blank_safety_report_update,
	create_case_with_safety_report, read_base_xml_fixture, update_safety_report,
	validate_business_xml, validate_case,
};
use lib_core::validation::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts, ValidationProfile,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn fda_c_1_12_recommended_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_has_issue(&report, "FDA.C.1.12.RECOMMENDED");
	assert_issue_metadata(
		&report,
		"FDA.C.1.12.RECOMMENDED",
		"C",
		Some("safetyReportIdentification.combinationProductReportIndicator"),
	);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_c_1_12_recommended_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let mut report_u = blank_safety_report_update();
	report_u.combination_product_report_indicator = Some("1".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_lacks_issue(&report, "FDA.C.1.12.RECOMMENDED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn fda_c_1_12_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.C.1.12.REQUIRED",
		None,
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_1_12_required_true() {
	assert!(is_rule_value_valid(
		"FDA.C.1.12.REQUIRED",
		Some("true"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_1_7_1_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.C.1.7.1.REQUIRED",
		Some("5"),
		None,
		RuleFacts {
			fda_combination_product_true: Some(false),
			fda_fulfil_expedited_criteria: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_c_1_7_1_required_true() {
	assert!(is_rule_value_valid(
		"FDA.C.1.7.1.REQUIRED",
		Some("4"),
		None,
		RuleFacts {
			fda_combination_product_true: Some(true),
			fda_fulfil_expedited_criteria: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_c_1_7_1_required_missing_code_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let broken = fda_xml.replacen(
		"</investigationEvent>",
		"<subjectOf2 typeCode=\"SUBJ\"><investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\"/></investigationCharacteristic></subjectOf2></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.C.1.7.1.REQUIRED.MISSING_CODE");
	Ok(())
}

#[test]
fn fda_c_1_7_1_required_missing_code_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let valid = fda_xml.replacen(
		"</investigationEvent>",
		"<subjectOf2 typeCode=\"SUBJ\"><investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"1\"/></investigationCharacteristic></subjectOf2></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "FDA.C.1.7.1.REQUIRED.MISSING_CODE");
	Ok(())
}

#[test]
fn fda_c_2_r_2_email_required_false() {
	assert!(!is_rule_condition_satisfied(
		"FDA.C.2.r.2.EMAIL.REQUIRED",
		RuleFacts {
			fda_primary_source_present: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_c_2_r_2_email_required_true() {
	assert!(is_rule_condition_satisfied(
		"FDA.C.2.r.2.EMAIL.REQUIRED",
		RuleFacts {
			fda_primary_source_present: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_c_5_5a_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.C.5.5a.REQUIRED",
		Some("ABC123"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_5_5a_required_true() {
	assert!(is_rule_value_valid(
		"FDA.C.5.5a.REQUIRED",
		Some("123456"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_5_5b_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.C.5.5b.REQUIRED",
		Some("A23456"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_5_5b_required_true() {
	assert!(is_rule_value_valid(
		"FDA.C.5.5b.REQUIRED",
		Some("234567"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_c_5_5b_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let with_pre_anda = xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1></investigationEvent>",
		1,
	);
	let broken = with_pre_anda
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.C.5.5b.FORBIDDEN");
	Ok(())
}

#[test]
fn fda_c_5_5b_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;
	assert_lacks_xml_rule(&report, "FDA.C.5.5b.FORBIDDEN");
	Ok(())
}

#[test]
fn fda_c_5_6_r_required_false() {
	assert!(!is_rule_condition_satisfied(
		"FDA.C.5.6.r.REQUIRED",
		RuleFacts {
			fda_has_ind_number: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_c_5_6_r_required_true() {
	assert!(is_rule_condition_satisfied(
		"FDA.C.5.6.r.REQUIRED",
		RuleFacts {
			fda_has_ind_number: Some(true),
			..RuleFacts::default()
		},
	));
}
