use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_drug,
	create_drug_indication, create_drug_reaction_assessment, create_message_header,
	create_message_header_with_receiver, create_patient, create_primary_source,
	create_reaction, create_safety_report, create_sender, put_json, setup_case,
	update_drug, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.G.k.1.REQUIRED",
		"ICH.G.k.2.1.1a.REQUIRED",
		"ICH.G.k.2.1.2a.REQUIRED",
		"ICH.G.k.2.2.REQUIRED",
		"ICH.G.k.5a.REQUIRED",
		"ICH.G.k.5b.REQUIRED",
		"ICH.G.k.6a.REQUIRED",
		"ICH.G.k.6b.REQUIRED",
		"ICH.G.k.7.r.2a.REQUIRED",
		"ICH.G.k.7.r.2b.REQUIRED",
		"ICH.G.k.9.i.3.2a.REQUIRED",
		"ICH.G.k.9.i.3.2b.REQUIRED",
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		"MFDS.G.k.2.1.KR.1b.REQUIRED",
		"MFDS.KR.FOREIGN.WHOMPID.REQUIRED",
		"MFDS.G.k.2.1.KR.1a.REQUIRED",
	]
}

#[test]
fn g_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('G', tested_rule_codes());
}

async fn setup_drug_case() -> Result<(
	super::validation_common::ValidationCtx,
	uuid::Uuid,
	uuid::Uuid,
)> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let reaction_id =
		create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	Ok((ctx, drug_id, reaction_id))
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_reaction(&ctx.app, &ctx.cookie, ctx.case_id, 1, "Headache").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_2_1_1a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "mpid": "WHOMPID-001", "mpid_version": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.2.1.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_2_1_2a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "phpid": "WHOPHPID-001", "phpid_version": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.2.1.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_2_2_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "medicinal_product": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.2.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_5a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "cumulative_dose_first_reaction_value": null, "cumulative_dose_first_reaction_unit": "mg" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.5a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_5b_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "cumulative_dose_first_reaction_value": "10", "cumulative_dose_first_reaction_unit": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.5b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_6a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "gestation_period_exposure_value": null, "gestation_period_exposure_unit": "wk" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.6a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_6b_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "gestation_period_exposure_value": "10", "gestation_period_exposure_unit": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.6b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_7_r_2a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	let indication_id = create_drug_indication(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		1,
		Some("Indication"),
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/drugs/{drug_id}/indications/{indication_id}", ctx.case_id),
		json!({"data": { "indication_meddra_version": null, "indication_meddra_code": "10012345" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.7.r.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_7_r_2b_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, _) = setup_drug_case().await?;
	let indication_id = create_drug_indication(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		1,
		Some("Indication"),
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/drugs/{drug_id}/indications/{indication_id}", ctx.case_id),
		json!({"data": { "indication_meddra_version": "27.0", "indication_meddra_code": "" }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.7.r.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_9_i_3_2a_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, reaction_id) = setup_drug_case().await?;
	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		reaction_id,
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/drugs/{drug_id}/reaction-assessments/{assessment_id}", ctx.case_id),
		json!({"data": { "last_dose_interval_unit": "d", "last_dose_interval_value": null }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.9.i.3.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_ich_g_k_9_i_3_2b_required_returns_banner_issue() -> Result<()> {
	let (ctx, drug_id, reaction_id) = setup_drug_case().await?;
	let assessment_id = create_drug_reaction_assessment(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		reaction_id,
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/drugs/{drug_id}/reaction-assessments/{assessment_id}", ctx.case_id),
		json!({"data": { "last_dose_interval_unit": "", "last_dose_interval_value": 3.0 }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.G.k.9.i.3.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_mfds_kr_domestic_productcode_required_returns_banner_issue() -> Result<()>
{
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
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "obtain_drug_country": "KR" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_mfds_g_k_2_1_kr_1b_required_returns_banner_issue() -> Result<()> {
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
	let drug_id =
		create_drug(&ctx.app, &ctx.cookie, ctx.case_id, 1, "1", "Drug A").await?;
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "mpid": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.G.k.2.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_mfds_kr_foreign_whompid_recommended_returns_banner_issue() -> Result<()> {
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
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "obtain_drug_country": "US" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.KR.FOREIGN.WHOMPID.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn g_mfds_g_k_2_1_kr_1a_required_returns_banner_issue() -> Result<()> {
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
	update_drug(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		drug_id,
		json!({"data": { "mpid": "WHOMPID-001", "mpid_version": "" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.G.k.2.1.KR.1a.REQUIRED");
	Ok(())
}
