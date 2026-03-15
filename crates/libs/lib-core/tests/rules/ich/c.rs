use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_lacks_issue,
	assert_lacks_xml_rule, blank_safety_report_update,
	create_case_with_safety_report, read_base_xml_fixture, update_safety_report,
	validate_business_xml, validate_case,
};
use lib_core::model::e_signature::{ESignatureBmc, ESignatureForCreate};
use lib_core::xml::validate::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts, ValidationProfile,
};
use serial_test::serial;
use time::OffsetDateTime;

async fn create_nullification_signature(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<sqlx::types::Uuid> {
	Ok(ESignatureBmc::create(
		ctx,
		mm,
		ESignatureForCreate {
			case_id: Some(case_id),
			signer_user_id: demo_user_id(),
			signer_username: "demo_user".to_string(),
			action: "nullification".to_string(),
			meaning: "Nullification status transition".to_string(),
			reason: "nullification validation test".to_string(),
			signature_method: Some("password".to_string()),
			signed_at: Some(OffsetDateTime::now_utc()),
		},
	)
	.await?)
}

#[test]
fn ich_c_1_1_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_1_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.1.REQUIRED",
		Some("CASE-001"),
		None,
		RuleFacts::default(),
	));
}

#[serial]
#[tokio::test]
async fn ich_c_1_11_2_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let signature_id = create_nullification_signature(&ctx, &mm, case_id).await?;
	let compliance_ctx = ctx.with_compliance(
		Some("nullification validation test".to_string()),
		Some(signature_id),
	);
	let mut report_u = blank_safety_report_update();
	report_u.nullification_code = Some("1".to_string());
	update_safety_report(&compliance_ctx, &mm, case_id, report_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.C.1.11.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_c_1_11_2_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let signature_id = create_nullification_signature(&ctx, &mm, case_id).await?;
	let compliance_ctx = ctx.with_compliance(
		Some("nullification validation test".to_string()),
		Some(signature_id),
	);
	let mut report_u = blank_safety_report_update();
	report_u.nullification_code = Some("1".to_string());
	report_u.nullification_reason = Some("duplicate report".to_string());
	update_safety_report(&compliance_ctx, &mm, case_id, report_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.C.1.11.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_c_1_2_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.2.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_2_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.2.REQUIRED",
		Some("20260313"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_3_conditional_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let premarket_xml = xml
		.replacen(
			"<id extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"/>",
			"<id extension=\"CDER_IND_EXEMPT_BA_BE\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"/>",
			1,
		)
		.replacen(
			"<id extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
			"<id extension=\"ZZFDA_PREMKT\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
			1,
		);
	let broken = premarket_xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.17\"/><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.1.3.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_c_1_3_conditional_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let premarket_xml = xml
		.replacen(
			"<id extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"/>",
			"<id extension=\"CDER_IND_EXEMPT_BA_BE\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"/>",
			1,
		)
		.replacen(
			"<id extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
			"<id extension=\"ZZFDA_PREMKT\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
			1,
		)
		.replacen(
			"<value xsi:type=\"CE\" code=\"1\" displayName=\"Spontaneous report\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"/>",
			"<value xsi:type=\"CE\" code=\"2\" displayName=\"Report from study\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"/>",
			1,
		);
	let valid = premarket_xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.17\"/><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.1.3.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_c_1_3_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.3.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_3_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.3.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_4_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.4.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_4_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.4.REQUIRED",
		Some("20260313"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_5_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.5.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_5_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.5.REQUIRED",
		Some("20260313"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_6_1_r_1_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.6.1.r.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_6_1_r_1_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.6.1.r.1.REQUIRED",
		Some("document description"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_7_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.7.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_7_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.7.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_9_1_conditional_false() {
	assert!(!is_rule_condition_satisfied(
		"ICH.C.1.9.1.CONDITIONAL",
		RuleFacts {
			ich_case_history_true_missing_prior_ids: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn ich_c_1_9_1_conditional_true() {
	assert!(is_rule_condition_satisfied(
		"ICH.C.1.9.1.CONDITIONAL",
		RuleFacts {
			ich_case_history_true_missing_prior_ids: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn ich_c_1_9_1_r_1_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.9.1.r.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_9_1_r_1_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.9.1.r.1.REQUIRED",
		Some("source"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_9_1_r_2_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.1.9.1.r.2.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_1_9_1_r_2_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.1.9.1.r.2.REQUIRED",
		Some("CASE-ALT-001"),
		None,
		RuleFacts::default(),
	));
}

#[serial]
#[tokio::test]
async fn ich_c_1_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.C.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_c_1_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.C.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id extension=\"REPORTER-1\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.1.ID.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id extension=\"REPORTER-1\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.1.ID.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_root_3_6_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id root=\"2.16.840.1.113883.3.989.2.1.3.6\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_1_id_root_3_6_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<primaryRole classCode=\"INVSBJ\">",
		"<primaryRole classCode=\"INVSBJ\"><id root=\"2.16.840.1.113883.3.989.2.1.3.6\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_2_name_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<name>JD</name>",
		"<name><prefix nullFlavor=\"UNK\">Doctor</prefix></name>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.2.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_2_name_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.2.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_2_name_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<name>JD</name>", "<name><prefix/></name>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_2_name_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<name>JD</name>",
		"<name><prefix nullFlavor=\"UNK\"/></name>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_3_org_name_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<name>Management</name>",
		"<name nullFlavor=\"UNK\">Management</name>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_3_org_name_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_2_r_3_org_name_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<name>Management</name>", "<name/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_3_org_name_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid =
		xml.replacen("<name>Management</name>", "<name nullFlavor=\"UNK\"/>", 1);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_2_r_4_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.2.r.4.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_2_r_4_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.2.r.4.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_3_1_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.3.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_3_1_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.3.1.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_3_2_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.3.2.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_3_2_required_true() {
	assert!(is_rule_value_valid(
		"ICH.C.3.2.REQUIRED",
		Some("Sender Org"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_5_4_required_value_false() {
	assert!(!is_rule_value_valid(
		"ICH.C.5.4.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_5_4_required_value_true() {
	assert!(is_rule_value_valid(
		"ICH.C.5.4.REQUIRED",
		Some("Receiver Org"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_5_4_required_false() {
	assert!(is_rule_condition_satisfied(
		"ICH.C.5.4.REQUIRED",
		RuleFacts {
			ich_report_type_is_study: Some(true),
			..RuleFacts::default()
		},
	));
	assert!(!is_rule_value_valid(
		"ICH.C.5.4.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_5_4_required_true() {
	assert!(is_rule_condition_satisfied(
		"ICH.C.5.4.REQUIRED",
		RuleFacts {
			ich_report_type_is_study: Some(true),
			..RuleFacts::default()
		},
	));
	assert!(is_rule_value_valid(
		"ICH.C.5.4.REQUIRED",
		Some("Receiver Org"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_c_5_4_required_condition_false() {
	assert!(!is_rule_condition_satisfied(
		"ICH.C.5.4.REQUIRED",
		RuleFacts {
			ich_report_type_is_study: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn ich_c_5_4_required_condition_true() {
	assert!(is_rule_condition_satisfied(
		"ICH.C.5.4.REQUIRED",
		RuleFacts {
			ich_report_type_is_study: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn ich_c_5_title_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><title nullFlavor=\"UNK\">Study A</title></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_5_title_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><title>Study A</title></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_c_5_title_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><title/></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_c_5_title_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</investigationEvent>",
		"<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><title nullFlavor=\"UNK\"/></researchStudy></subjectOf1></investigationEvent>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED");
	Ok(())
}
