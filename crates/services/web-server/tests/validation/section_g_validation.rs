use super::validation_common::{
	assert_has_code, create_drug, create_message_header, create_patient,
	create_primary_source, create_reaction, create_safety_report, create_sender,
	put_json, setup_case, update_drug, validate_case,
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
