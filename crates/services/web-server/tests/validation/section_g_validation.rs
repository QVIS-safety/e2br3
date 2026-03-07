use super::validation_common::{
	assert_has_code, create_active_substance, create_drug,
	create_drug_device_characteristic, create_drug_reaction_assessment,
	create_message_header, create_message_header_with_receiver, create_patient,
	create_primary_source, create_reaction, create_relatedness_assessment,
	create_safety_report, create_safety_report_with, create_sender, put_json,
	setup_case, update_drug, update_safety_report, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn g_section_drug_characterization_rejects_blank_value_at_api_layer(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/drugs/{drug_id}", ctx.case_id),
		json!({"data": { "drug_characterization": "" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_medicinal_product_rule_is_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "medicinal_product": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.G.k.2.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_domestic_product_code_rule_is_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZMFDS"))
		.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "obtain_drug_country": "KR", "mpid": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_fda_gk12_required_when_local_criteria_is_5() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "combination_product_report_indicator": "1", "local_criteria_report_type": "5" }}),
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.G.K.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_fda_gk12r3_required_when_malfunction_true() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "combination_product_report_indicator": "1", "local_criteria_report_type": "1" }}),
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let _ = create_drug_device_characteristic(
		&ctx,
		drug_id,
		1,
		Some("FDA.G.k.12.r.1"),
		Some("BL"),
		Some("1"),
		Some("true"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.G.K.12.R.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_fda_gk12r11_required_when_local_criteria_4_and_malfunction(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "combination_product_report_indicator": "1", "local_criteria_report_type": "4" }}),
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let _ = create_drug_device_characteristic(
		&ctx,
		drug_id,
		1,
		Some("FDA.G.k.12.r.1"),
		Some("BL"),
		Some("1"),
		Some("true"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.G.K.12.R.11.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_fda_gk1a_requires_combination_malfunction_and_role4() -> Result<()>
{
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "combination_product_report_indicator": "2", "local_criteria_report_type": "1" }}),
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let _ = create_drug_device_characteristic(
		&ctx,
		drug_id,
		1,
		Some("FDA.G.k.1.a"),
		Some("BL"),
		Some("1"),
		Some("1"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.G.K.1.A.CONDITIONAL");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_relatedness_kr2_required_for_clinical_krct_method(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "2", false)
		.await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"CT",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		reaction_id,
	)
	.await?;
	create_relatedness_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		assessment_id,
		1,
		Some("1"),
		Some("2"),
		None,
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_method_required_for_ct_receiver_even_without_source(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "2", false)
		.await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"CT",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		reaction_id,
	)
	.await?;
	create_relatedness_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		assessment_id,
		1,
		None,
		None,
		None,
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_method_profile_restriction_rejects_wrong_value_for_kr(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"KR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		reaction_id,
	)
	.await?;
	create_relatedness_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		assessment_id,
		1,
		Some("1"),
		Some("2"),
		None,
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_product_code_required_for_kr_receiver() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"KR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let _ =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.G.k.2.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_section_mfds_substance_version_required_for_fr_when_substance_code_present(
) -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"FR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	let _ = create_active_substance(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		1,
		Some("Substance"),
		Some("CAS-001"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_has_code(&report, "MFDS.G.k.2.3.r.1.KR.1a.REQUIRED");
	Ok(())
}
