use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	assert_has_issue, assert_has_xml_rule, assert_lacks_issue,
	assert_lacks_xml_rule, blank_safety_report_update,
	create_case_with_safety_report, read_base_xml_fixture, update_safety_report,
	validate_business_xml, validate_case,
};
use lib_core::model::drug::{
	DrugDeviceCharacteristicBmc, DrugDeviceCharacteristicForCreate,
	DrugInformationBmc, DrugInformationForCreate,
};
use lib_core::validation::ValidationProfile;
use serial_test::serial;
use sqlx::types::Uuid;

fn device_char(
	drug_id: Uuid,
	sequence_number: i32,
	code: &str,
	value_code: &str,
) -> DrugDeviceCharacteristicForCreate {
	DrugDeviceCharacteristicForCreate {
		drug_id,
		sequence_number,
		code: Some(code.to_string()),
		code_system: None,
		code_display_name: None,
		value_type: Some("CE".to_string()),
		value_value: None,
		value_code: Some(value_code.to_string()),
		value_code_system: None,
		value_display_name: None,
	}
}

async fn create_fda_drug_case(
	drug_characterization: &str,
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
			drug_generic_name: None,
			medicinal_product: "Device Product".to_string(),
			..Default::default()
		},
	)
	.await?;

	Ok((ctx, mm, case_id, drug_id))
}

fn fda_gk10a_base_xml() -> Result<String> {
	let xml = read_base_xml_fixture()?;
	let fda_xml = xml
		.replacen(
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER_IND_EXEMPT_BA_BE\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA_PREMKT\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let pre_anda = "<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1>";
	Ok(fda_xml.replacen(
		"</investigationEvent>",
		&format!("{pre_anda}</investigationEvent>"),
		1,
	))
}

fn replace_nth(
	haystack: &str,
	needle: &str,
	replacement: &str,
	nth: usize,
) -> String {
	let mut start = 0usize;
	let mut seen = 0usize;
	while let Some(offset) = haystack[start..].find(needle) {
		if seen == nth {
			let idx = start + offset;
			let mut out = String::with_capacity(
				haystack.len() - needle.len() + replacement.len(),
			);
			out.push_str(&haystack[..idx]);
			out.push_str(replacement);
			out.push_str(&haystack[idx + needle.len()..]);
			return out;
		}
		seen += 1;
		start += offset + needle.len();
	}
	haystack.to_string()
}

#[serial]
#[tokio::test]
async fn fda_g_k_1_a_conditional_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.1.a", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_has_issue(&report, "FDA.G.K.1.A.CONDITIONAL");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_1_a_conditional_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("4").await?;
	let mut report_u = blank_safety_report_update();
	report_u.combination_product_report_indicator = Some("1".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 2, "FDA.G.k.1.a", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_lacks_issue(&report, "FDA.G.K.1.A.CONDITIONAL");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_r_11_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("4".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_has_issue(&report, "FDA.G.K.12.R.11.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_r_11_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("4".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 2, "FDA.G.k.12.r.11", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_lacks_issue(&report, "FDA.G.K.12.R.11.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_r_3_required_false() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_has_issue(&report, "FDA.G.K.12.R.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_r_3_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 2, "FDA.G.k.12.r.3", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_lacks_issue(&report, "FDA.G.K.12.R.3.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_required_false() -> Result<()> {
	let (ctx, mm, case_id, _drug_id) = create_fda_drug_case("1").await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("5".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_has_issue(&report, "FDA.G.K.12.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_g_k_12_required_true() -> Result<()> {
	let (ctx, mm, case_id, drug_id) = create_fda_drug_case("1").await?;
	let mut report_u = blank_safety_report_update();
	report_u.local_criteria_report_type = Some("5".to_string());
	update_safety_report(&ctx, &mm, case_id, report_u).await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		device_char(drug_id, 1, "FDA.G.k.12.r.1", "1"),
	)
	.await?;

	let report = validate_case(&ctx, &mm, case_id, ValidationProfile::Fda).await?;

	assert_lacks_issue(&report, "FDA.G.K.12.REQUIRED");
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[test]
fn fda_g_k_10a_required_false() -> Result<()> {
	let xml = fda_gk10a_base_xml()?;
	let bad_gk10a = "<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"9\"/></observation></outboundRelationship2>";
	let broken = replace_nth(
		&xml,
		"</substanceAdministration>",
		&format!("{bad_gk10a}</substanceAdministration>"),
		2,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.G.k.10a.REQUIRED");
	Ok(())
}

#[test]
fn fda_g_k_10a_required_true() -> Result<()> {
	let xml = fda_gk10a_base_xml()?;
	let good_gk10a = "<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"1\"/></observation></outboundRelationship2>";
	let fixed = replace_nth(
		&xml,
		"</substanceAdministration>",
		&format!("{good_gk10a}</substanceAdministration>"),
		2,
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "FDA.G.k.10a.REQUIRED");
	Ok(())
}
