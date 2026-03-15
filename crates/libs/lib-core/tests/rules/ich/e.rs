use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_lacks_issue,
	assert_lacks_xml_rule, create_case_with_safety_report, read_base_xml_fixture,
	validate_business_xml, validate_case,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use lib_core::xml::validate::{find_canonical_rule, ValidationProfile};
use rust_decimal::Decimal;
use serial_test::serial;

fn blank_reaction_update() -> ReactionForUpdate {
	ReactionForUpdate {
		primary_source_reaction: None,
		primary_source_reaction_translation: None,
		reaction_language: None,
		reaction_meddra_code: None,
		reaction_meddra_version: None,
		term_highlighted: None,
		serious: None,
		criteria_death: None,
		criteria_death_null_flavor: None,
		criteria_life_threatening: None,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: None,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: None,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: None,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: None,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		duration_value: None,
		duration_unit: None,
		outcome: None,
		medical_confirmation: None,
		country_code: None,
	}
}

#[test]
fn ich_e_i_0_relationship_code_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\"/>",
		"<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_0_relationship_code_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(
		&report,
		"ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.FORBIDDEN",
	);
	Ok(())
}

#[test]
fn ich_e_i_0_relationship_code_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\"/>",
		"<code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_0_relationship_code_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\"/>",
		"<code codeSystem=\"2.16.840.1.113883.3.989.2.1.1.22\" displayName=\"sourceReport\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(
		&report,
		"ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.REQUIRED",
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_1_1a_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.1.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_1_1a_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.1.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_1_1b_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.1.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_1_1b_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.reaction_language = Some("en".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.1.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_e_i_1_2_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"ED\">THROMBOSE VEINEUSE PROFONDE</value>",
		"<value xsi:type=\"ED\" nullFlavor=\"UNK\">THROMBOSE VEINEUSE PROFONDE</value>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.1.2.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_1_2_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.1.2.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_1_2_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"ED\">THROMBOSE VEINEUSE PROFONDE</value>",
		"<value xsi:type=\"ED\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.1.2.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_1_2_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<value xsi:type=\"ED\">THROMBOSE VEINEUSE PROFONDE</value>",
		"<value xsi:type=\"ED\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.1.2.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_2_1a_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.reaction_meddra_code = Some("10027940".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.2.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_2_1a_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.reaction_meddra_code = Some("10027940".to_string());
	reaction_u.reaction_meddra_version = Some("27.0".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.2.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_2_1b_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.2.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_2_1b_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.reaction_meddra_code = Some("10027940".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.2.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_e_i_2_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.2.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_2_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.2.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_2_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.2.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_2_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.2.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_4_5_low_high_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<low value=\"20141010\"/>", "<low/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_4_5_low_high_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed =
		xml.replacen("<low value=\"20141010\"/>", "<low nullFlavor=\"UNK\"/>", 1);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_4_6_conditional_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let reaction_id = "<id root=\"154eb889-958b-45f2-a02f-42d4d6f4657f\"/>";
	let broken = if let Some(reaction_idx) = xml.find(reaction_id) {
		if let Some(rel_eff_start) = xml[reaction_idx..].find("<effectiveTime") {
			let eff_start = reaction_idx + rel_eff_start;
			if let Some(rel_eff_end) = xml[eff_start..].find("</effectiveTime>") {
				let eff_end = eff_start + rel_eff_end + "</effectiveTime>".len();
				let mut out =
					String::with_capacity(xml.len() - (eff_end - eff_start));
				out.push_str(&xml[..eff_start]);
				out.push_str(&xml[eff_end..]);
				out
			} else {
				xml.clone()
			}
		} else {
			xml.clone()
		}
	} else {
		xml.clone()
	};

	let report = validate_business_xml(&broken)?;
	let rule = find_canonical_rule("ICH.E.i.4-6.CONDITIONAL")
		.expect("ICH.E.i.4-6.CONDITIONAL should exist in catalog");

	assert!(
		!rule.blocking,
		"ICH.E.i.4-6.CONDITIONAL is expected to remain nonblocking in XML validation"
	);
	assert_lacks_xml_rule(&report, "ICH.E.i.4-6.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_e_i_4_6_conditional_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.4-6.CONDITIONAL");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_6a_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.duration_unit = Some("d".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_6a_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.duration_value = Some(Decimal::new(5, 0));
	reaction_u.duration_unit = Some("d".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_6b_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.duration_value = Some(Decimal::new(5, 0));
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_6b_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.duration_value = Some(Decimal::new(5, 0));
	reaction_u.duration_unit = Some("d".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_e_i_7_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"3\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\"/>",
		"<value xsi:type=\"CE\" code=\"3\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.7.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_7_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.7.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_e_i_7_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"3\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\"/>",
		"<value xsi:type=\"CE\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.7.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_7_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<value xsi:type=\"CE\" code=\"3\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\"/>",
		"<value xsi:type=\"CE\" displayName=\"not recovered/not resolved/ongoing\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.7.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_7_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.E.i.7.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_e_i_7_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let reaction_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let mut reaction_u = blank_reaction_update();
	reaction_u.outcome = Some("3".to_string());
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.E.i.7.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_e_i_9_country_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/>",
		"<code codeSystem=\"1.0.3166.1.2.2\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.E.i.9.COUNTRY.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_e_i_9_country_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/>",
		"<code codeSystem=\"1.0.3166.1.2.2\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.E.i.9.COUNTRY.NULLFLAVOR.REQUIRED");
	Ok(())
}
