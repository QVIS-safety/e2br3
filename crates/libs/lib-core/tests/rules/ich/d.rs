use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_lacks_issue,
	assert_lacks_xml_rule, create_case_with_safety_report, read_base_xml_fixture,
	validate_business_xml, validate_case,
};
use lib_core::model::parent_history::{
	ParentPastDrugHistoryBmc, ParentPastDrugHistoryForCreate,
	ParentPastDrugHistoryForUpdate,
};
use lib_core::model::patient::{
	ParentInformationBmc, ParentInformationForCreate, PastDrugHistoryBmc,
	PastDrugHistoryForCreate, PastDrugHistoryForUpdate, PatientInformationBmc,
	PatientInformationForCreate, PatientInformationForUpdate,
};
use lib_core::xml::validate::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts, ValidationProfile,
};
use rust_decimal::Decimal;
use serial_test::serial;

fn blank_patient_update() -> PatientInformationForUpdate {
	PatientInformationForUpdate {
		patient_initials: None,
		patient_given_name: None,
		patient_family_name: None,
		patient_initials_null_flavor: None,
		birth_date: None,
		birth_date_null_flavor: None,
		age_at_time_of_onset: None,
		age_at_time_of_onset_null_flavor: None,
		age_unit: None,
		gestation_period: None,
		gestation_period_unit: None,
		age_group: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		sex_null_flavor: None,
		race_code: None,
		ethnicity_code: None,
		last_menstrual_period_date: None,
		last_menstrual_period_date_null_flavor: None,
		medical_history_text: None,
		concomitant_therapy: None,
	}
}

#[test]
fn ich_d_1_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.1.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_1_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.1.REQUIRED",
		Some("JD"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_2_2a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.2.2a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_2_2a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.2.2a.REQUIRED",
		Some("40"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_2_2b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.2.2b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_2_2b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.2.2b.REQUIRED",
		Some("a"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_6_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.6.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_6_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.6.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_7_1_r_1a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.7.1.r.1a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_7_1_r_1a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.7.1.r.1a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_7_1_r_1b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.7.1.r.1b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_7_1_r_1b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.7.1.r.1b.REQUIRED",
		Some("10012345"),
		None,
		RuleFacts::default(),
	));
}

#[serial]
#[tokio::test]
async fn ich_d_10_8_mpid_phpid_exclusive_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: Some("2".to_string()),
			medical_history_text: None,
		},
	)
	.await?;
	let parent_past_id = ParentPastDrugHistoryBmc::create(
		&ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id,
			sequence_number: 1,
			drug_name: Some("Parent Past Drug".to_string()),
			drug_name_null_flavor: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	ParentPastDrugHistoryBmc::update(
		&ctx,
		&mm,
		parent_past_id,
		ParentPastDrugHistoryForUpdate {
			drug_name: None,
			drug_name_null_flavor: None,
			mpid: Some("MPID-2".to_string()),
			mpid_version: Some("2024Q4".to_string()),
			phpid: Some("PHPID-2".to_string()),
			phpid_version: Some("2024Q4".to_string()),
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_10_8_mpid_phpid_exclusive_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: Some("2".to_string()),
			medical_history_text: None,
		},
	)
	.await?;
	let parent_past_id = ParentPastDrugHistoryBmc::create(
		&ctx,
		&mm,
		ParentPastDrugHistoryForCreate {
			parent_id,
			sequence_number: 1,
			drug_name: Some("Parent Past Drug".to_string()),
			drug_name_null_flavor: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date_null_flavor: Some("UNK".to_string()),
		},
	)
	.await?;
	ParentPastDrugHistoryBmc::update(
		&ctx,
		&mm,
		parent_past_id,
		ParentPastDrugHistoryForUpdate {
			drug_name: None,
			drug_name_null_flavor: None,
			mpid: Some("MPID-2".to_string()),
			mpid_version: Some("2024Q4".to_string()),
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_d_10_8_r_2a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.2a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_2a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.2a.REQUIRED",
		Some("2024Q4"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_3a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.3a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_3a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.3a.REQUIRED",
		Some("2024Q4"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_6a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.6a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_6a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.6a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_6b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.6b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_6b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.6b.REQUIRED",
		Some("10054321"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_7a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.7a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_7a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.7a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_7b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.10.8.r.7b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_10_8_r_7b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.10.8.r.7b.REQUIRED",
		Some("10067890"),
		None,
		RuleFacts::default(),
	));
}

#[serial]
#[tokio::test]
async fn ich_d_2_2_1a_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.gestation_period_unit = Some("wk".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.2.2.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2_1a_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.gestation_period = Some(Decimal::new(12, 0));
	update.gestation_period_unit = Some("wk".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.2.2.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2_1b_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.gestation_period = Some(Decimal::new(12, 0));
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.2.2.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2_1b_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.gestation_period = Some(Decimal::new(12, 0));
	update.gestation_period_unit = Some("wk".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.2.2.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2a_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.age_unit = Some("a".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.2.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2a_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.age_at_time_of_onset = Some(Decimal::new(42, 0));
	update.age_unit = Some("a".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.2.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2b_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.age_at_time_of_onset = Some(Decimal::new(42, 0));
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.2.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_2_2b_required_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let mut update = blank_patient_update();
	update.age_at_time_of_onset = Some(Decimal::new(42, 0));
	update.age_unit = Some("a".to_string());
	PatientInformationBmc::update(&ctx, &mm, patient_id, update).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.2.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_d_2_birthtime_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<birthTime value=\"20010615\"/>",
		"<birthTime value=\"20010615\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.2.BIRTHTIME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_2_birthtime_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;
	assert_lacks_xml_rule(&report, "ICH.D.2.BIRTHTIME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_2_birthtime_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<birthTime value=\"20010615\"/>", "<birthTime/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_2_birthtime_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<birthTime value=\"20010615\"/>",
		"<birthTime nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_5_sex_conditional_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<administrativeGenderCode code=\"1\" displayName=\"Male\" codeSystem=\"1.0.5218\"/>",
		"<administrativeGenderCode/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.5.SEX.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_d_5_sex_conditional_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;
	assert_lacks_xml_rule(&report, "ICH.D.5.SEX.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_d_7_1_r_1a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.7.1.r.1a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_7_1_r_1a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.7.1.r.1a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_7_1_r_1b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.7.1.r.1b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_7_1_r_1b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.7.1.r.1b.REQUIRED",
		Some("10011111"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_7_2_conditional_false() {
	assert!(!is_rule_condition_satisfied(
		"ICH.D.7.2.CONDITIONAL",
		RuleFacts {
			ich_medical_history_missing_d72_text: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn ich_d_7_2_conditional_true() {
	assert!(is_rule_condition_satisfied(
		"ICH.D.7.2.CONDITIONAL",
		RuleFacts {
			ich_medical_history_missing_d72_text: Some(true),
			..RuleFacts::default()
		},
	));
}

#[serial]
#[tokio::test]
async fn ich_d_8_mpid_phpid_exclusive_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let past_id = PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: Some("Past Drug".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID-1".to_string()),
			mpid_version: Some("2024Q4".to_string()),
			phpid: Some("PHPID-1".to_string()),
			phpid_version: Some("2024Q4".to_string()),
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date: None,
			end_date_null_flavor: Some("UNK".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;
	let _ = past_id;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_d_8_mpid_phpid_exclusive_true() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let past_id = PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: Some("Past Drug".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID-1".to_string()),
			mpid_version: Some("2024Q4".to_string()),
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date: None,
			end_date_null_flavor: Some("UNK".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;
	PastDrugHistoryBmc::update(
		&ctx,
		&mm,
		past_id,
		PastDrugHistoryForUpdate {
			drug_name: None,
			drug_name_null_flavor: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_d_8_r_2a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.2a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_2a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.2a.REQUIRED",
		Some("2024Q4"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_3a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.3a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_3a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.3a.REQUIRED",
		Some("2024Q4"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_6a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.6a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_6a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.6a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_6b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.6b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_6b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.6b.REQUIRED",
		Some("10022222"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_7a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.7a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_7a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.7a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_7b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.8.r.7b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_8_r_7b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.8.r.7b.REQUIRED",
		Some("10033333"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_1a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.2.r.1a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_1a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.2.r.1a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_1b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.2.r.1b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_1b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.2.r.1b.REQUIRED",
		Some("10044444"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_2_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.2.r.2.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_2_r_2_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.2.r.2.REQUIRED",
		Some("text"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_3_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.3.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_3_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.3.REQUIRED",
		Some("1"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_1a_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.4.r.1a.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_1a_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.4.r.1a.REQUIRED",
		Some("27.0"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_1b_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.4.r.1b.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_1b_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.4.r.1b.REQUIRED",
		Some("10055555"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_2_required_false() {
	assert!(!is_rule_value_valid(
		"ICH.D.9.4.r.2.REQUIRED",
		Some(""),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_9_4_r_2_required_true() {
	assert!(is_rule_value_valid(
		"ICH.D.9.4.r.2.REQUIRED",
		Some("text"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn ich_d_effectivetime_low_high_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<low value=\"20220614101010-0500\"/>", "<low/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_effectivetime_low_high_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<low value=\"20220614101010-0500\"/>",
		"<low nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(
		&report,
		"ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
	);
	Ok(())
}

#[test]
fn ich_d_parent_birthtime_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><birthTime value=\"19700101\" nullFlavor=\"UNK\"/></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_parent_birthtime_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><birthTime value=\"19700101\"/></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_parent_birthtime_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><birthTime/></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_parent_birthtime_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><birthTime nullFlavor=\"UNK\"/></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_parent_name_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name><given nullFlavor=\"UNK\">Parent</given></name></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_parent_name_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name><given>Parent</given></name></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_d_parent_name_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name><given/></name></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_d_parent_name_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"</primaryRole>",
		"<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\"><subject1 typeCode=\"SBJ\"><associatedPerson classCode=\"PSN\" determinerCode=\"INSTANCE\"><name><given nullFlavor=\"UNK\"/></name></associatedPerson></subject1></observation></subjectOf2></primaryRole>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}
