use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_lacks_issue,
	assert_lacks_xml_rule, create_case_with_safety_report, read_base_xml_fixture,
	validate_business_xml, validate_case,
};
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DosageInformationForUpdate,
	DrugActiveSubstanceBmc, DrugActiveSubstanceForCreate, DrugIndicationBmc,
	DrugIndicationForCreate, DrugIndicationForUpdate, DrugInformationBmc,
	DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	DrugReactionAssessmentForUpdate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::xml::validate::ValidationProfile;
use rust_decimal::Decimal;
use serial_test::serial;
use sqlx::types::Uuid;

fn blank_drug_update() -> DrugInformationForUpdate {
	DrugInformationForUpdate {
		medicinal_product: None,
		drug_characterization: None,
		brand_name: None,
		drug_generic_name: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		dosage_text: None,
		action_taken: None,
		rechallenge: None,
		investigational_product_blinded: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		obtain_drug_country: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
		drug_additional_info_codes_json: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
	}
}

fn blank_substance_create(drug_id: Uuid) -> DrugActiveSubstanceForCreate {
	DrugActiveSubstanceForCreate {
		drug_id,
		sequence_number: 1,
		substance_name: None,
		substance_termid: None,
		substance_termid_version: None,
		strength_value: None,
		strength_unit: None,
	}
}

fn blank_dosage_create(drug_id: Uuid) -> DosageInformationForCreate {
	DosageInformationForCreate {
		drug_id,
		sequence_number: 1,
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_value: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_time: None,
		last_administration_date: None,
		last_administration_time: None,
		duration_value: None,
		duration_unit: None,
		batch_lot_number: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	}
}

fn blank_dosage_update() -> DosageInformationForUpdate {
	DosageInformationForUpdate {
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_value: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_time: None,
		last_administration_date: None,
		last_administration_time: None,
		duration_value: None,
		duration_unit: None,
		batch_lot_number: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	}
}

fn blank_indication_update() -> DrugIndicationForUpdate {
	DrugIndicationForUpdate {
		indication_text: None,
		indication_meddra_version: None,
		indication_meddra_code: None,
	}
}

fn blank_assessment_update() -> DrugReactionAssessmentForUpdate {
	DrugReactionAssessmentForUpdate {
		administration_start_interval_value: None,
		administration_start_interval_unit: None,
		last_dose_interval_value: None,
		last_dose_interval_unit: None,
		recurrence_action: None,
		recurrence_meddra_version: None,
		recurrence_meddra_code: None,
		reaction_recurred: None,
	}
}

async fn create_drug_case(
	drug_characterization: &str,
	medicinal_product: &str,
) -> Result<(
	lib_core::ctx::Ctx,
	lib_core::model::ModelManager,
	Uuid,
	Uuid,
)> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let drug_id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: drug_characterization.to_string(),
			medicinal_product: medicinal_product.to_string(),
		},
	)
	.await?;

	Ok((ctx, mm, case_id, drug_id))
}

#[serial]
#[tokio::test]
async fn ich_g_k_1_required_false() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, _drug_id) = create_drug_case("1", "Drug A").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_1_1a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.mpid = Some("WHOMPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.1.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_1_1a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.mpid = Some("WHOMPID-001".to_string());
	drug_u.mpid_version = Some("2024Q4".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.1.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_1_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.phpid = Some("WHOPHPID-001".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.1.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_1_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.phpid = Some("WHOPHPID-001".to_string());
	drug_u.phpid_version = Some("2024Q4".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.1.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_2_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.medicinal_product = Some(String::new());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_2_required_true() -> Result<()> {
	let (ctx, mm, case_id, _drug_id) = create_drug_case("1", "Drug A").await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.2.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_g_k_2_3_name_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<name>Ingredient A</name>",
		"<name nullFlavor=\"UNK\">Ingredient A</name>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.2.3.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_g_k_2_3_name_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.2.3.NAME.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_g_k_2_3_name_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<name>Ingredient A</name>", "<name/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.2.3.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_2_3_name_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed =
		xml.replacen("<name>Ingredient A</name>", "<name nullFlavor=\"UNK\"/>", 1);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.2.3.NAME.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_1_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let substance_id =
		DrugActiveSubstanceBmc::create(&ctx, &mm, blank_substance_create(drug_id))
			.await?;
	let _ = substance_id;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.3.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_1_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.3.r.1.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.3.r.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: Some("TERM-001".to_string()),
			substance_termid_version: Some("27.0".to_string()),
			strength_value: None,
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.3.r.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_3b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: Some(Decimal::new(10, 0)),
			strength_unit: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.2.3.r.3b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_2_3_r_3b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: 1,
			substance_name: Some("Ingredient A".to_string()),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: Some(Decimal::new(10, 0)),
			strength_unit: Some("mg".to_string()),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.2.3.r.3b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_10_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.route_of_administration = Some("001".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.10.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_10_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.route_of_administration = Some("001".to_string());
	dosage_u.route_termid_version = Some("2014.10.30".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.10.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_g_k_4_r_10_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<formCode codeSystem=\"0.4.0.127.0.16.1.1.2.1\">",
		"<formCode>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_4_r_10_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<formCode codeSystem=\"0.4.0.127.0.16.1.1.2.1\">",
		"<formCode nullFlavor=\"UNK\">",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_11_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.parent_route_termid = Some("ROUTE-001".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.11.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_11_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.parent_route_termid = Some("ROUTE-001".to_string());
	dosage_u.parent_route_termid_version = Some("27.0".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.11.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_g_k_4_r_11_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		"<routeCode codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		1,
	);
	let broken = broken.replacen("<originalText>Oral</originalText>", "", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_4_r_11_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		"<routeCode nullFlavor=\"UNK\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		1,
	);
	let fixed = fixed.replacen("<originalText>Oral</originalText>", "", 1);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.dose_value = Some(Decimal::new(5, 0));
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.dose_value = Some(Decimal::new(5, 0));
	dosage_u.dose_unit = Some("mg".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_3_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.frequency_value = Some(Decimal::new(2, 0));
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_3_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.frequency_value = Some(Decimal::new(2, 0));
	dosage_u.frequency_unit = Some("day".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_g_k_4_r_4_5_low_high_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<effectiveTime nullFlavor=\"ASKU\"/>",
		"<effectiveTime xsi:type=\"IVL_TS\"><low/></effectiveTime>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_4_r_4_5_low_high_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<effectiveTime nullFlavor=\"ASKU\"/>",
		"<effectiveTime xsi:type=\"IVL_TS\"><low nullFlavor=\"UNK\"/></effectiveTime>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_4_r_4_8_conditional_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<effectiveTime nullFlavor=\"ASKU\"/>",
		"<effectiveTime xsi:type=\"IVL_TS\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.4.r.4-8.CONDITIONAL");
	Ok(())
}

#[test]
fn ich_g_k_4_r_4_8_conditional_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.4.r.4-8.CONDITIONAL");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_6a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.duration_unit = Some("day".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_6a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.duration_value = Some(Decimal::new(4, 0));
	dosage_u.duration_unit = Some("day".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_6b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.duration_value = Some(Decimal::new(4, 0));
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_6b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.duration_value = Some(Decimal::new(4, 0));
	dosage_u.duration_unit = Some("day".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_9_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.dose_form_termid = Some("DF-001".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.4.r.9.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_4_r_9_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let dosage_id =
		DosageInformationBmc::create(&ctx, &mm, blank_dosage_create(drug_id))
			.await?;
	let mut dosage_u = blank_dosage_update();
	dosage_u.dose_form_termid = Some("DF-001".to_string());
	dosage_u.dose_form_termid_version = Some("27.0".to_string());
	DosageInformationBmc::update(&ctx, &mm, dosage_id, dosage_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.4.r.9.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_5a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.cumulative_dose_first_reaction_unit = Some("mg".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.5a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_5a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.cumulative_dose_first_reaction_value = Some(Decimal::new(10, 0));
	drug_u.cumulative_dose_first_reaction_unit = Some("mg".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.5a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_5b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.cumulative_dose_first_reaction_value = Some(Decimal::new(10, 0));
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.5b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_5b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.cumulative_dose_first_reaction_value = Some(Decimal::new(10, 0));
	drug_u.cumulative_dose_first_reaction_unit = Some("mg".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.5b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_6a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.gestation_period_exposure_unit = Some("wk".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_6a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.gestation_period_exposure_value = Some(Decimal::new(2, 0));
	drug_u.gestation_period_exposure_unit = Some("wk".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.6a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_6b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.gestation_period_exposure_value = Some(Decimal::new(2, 0));
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_6b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let mut drug_u = blank_drug_update();
	drug_u.gestation_period_exposure_value = Some(Decimal::new(2, 0));
	drug_u.gestation_period_exposure_unit = Some("wk".to_string());
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.6b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_7_r_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Headache".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: Some("10019211".to_string()),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.7.r.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_7_r_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Headache".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("10019211".to_string()),
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.7.r.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_7_r_2b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Headache".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: None,
		},
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.7.r.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_7_r_2b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
	let indication_id = DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Headache".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("10019211".to_string()),
		},
	)
	.await?;
	let indication_u = blank_indication_update();
	DrugIndicationBmc::update(&ctx, &mm, indication_id, indication_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.7.r.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn ich_g_k_9_i_2_id_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">",
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\"><id extension=\"A1\" nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.9.i.2.ID.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_g_k_9_i_2_id_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.9.i.2.ID.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_g_k_9_i_2_id_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">",
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\"><id/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.G.k.9.i.2.ID.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_g_k_9_i_2_id_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fixed = xml.replacen(
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">",
		"<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\"><id nullFlavor=\"UNK\"/>",
		1,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "ICH.G.k.9.i.2.ID.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_1a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.administration_start_interval_unit = Some("805".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.9.i.3.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_1a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.administration_start_interval_value = Some(Decimal::new(12, 0));
	assessment_u.administration_start_interval_unit = Some("805".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.9.i.3.1a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_1b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.administration_start_interval_value = Some(Decimal::new(12, 0));
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.9.i.3.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_1b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.administration_start_interval_value = Some(Decimal::new(12, 0));
	assessment_u.administration_start_interval_unit = Some("805".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.9.i.3.1b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_2a_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.last_dose_interval_unit = Some("804".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.9.i.3.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_2a_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.last_dose_interval_value = Some(Decimal::new(3, 0));
	assessment_u.last_dose_interval_unit = Some("804".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.9.i.3.2a.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_2b_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.last_dose_interval_value = Some(Decimal::new(3, 0));
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_has_issue(&report, "ICH.G.k.9.i.3.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn ich_g_k_9_i_3_2b_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_drug_case("1", "Drug A").await?;
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
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id,
			reaction_id,
		},
	)
	.await?;
	let mut assessment_u = blank_assessment_update();
	assessment_u.last_dose_interval_value = Some(Decimal::new(3, 0));
	assessment_u.last_dose_interval_unit = Some("804".to_string());
	DrugReactionAssessmentBmc::update(&ctx, &mm, assessment_id, assessment_u)
		.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Ich).await?;

	assert_lacks_issue(&report, "ICH.G.k.9.i.3.2b.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}
