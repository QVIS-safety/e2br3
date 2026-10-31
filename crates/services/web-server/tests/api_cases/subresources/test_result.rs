use super::support::*;

#[serial]
#[tokio::test]
async fn test_test_results_support_50_character_normal_ranges() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let low = "L".repeat(50);
	let high = "H".repeat(50);

	let body = json!({"data": {
		"case_id": case_id,
		"sequence_number": 1,
		"test_name": "ALT",
		"normal_low_value": low,
		"normal_high_value": high
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], low);
	assert_eq!(value["data"]["normal_high_value"], high);
	let test_result_id = value["data"]["id"]
		.as_str()
		.ok_or("missing test result id")?;

	let updated_low = "A".repeat(50);
	let updated_high = "B".repeat(50);
	let body = json!({"data": {
		"normal_low_value": updated_low,
		"normal_high_value": updated_high
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/test-results/{test_result_id}"),
		body,
		"test normal range widening",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["normal_low_value"], updated_low);
	assert_eq!(value["data"]["normal_high_value"], updated_high);

	Ok(())
}

#[serial]
#[tokio::test]
async fn validation_contract_accepts_meddra_test_name_alternative() -> Result<()> {
	use crate::common::seed_active_test_meddra_term;
	use crate::validation_common::{
		create_message_header, create_primary_source, create_safety_report,
		create_sender, create_test_result, put_json, setup_case,
	};

	let ctx = setup_case().await?;
	seed_active_test_meddra_term(&ctx.mm).await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let test_id =
		create_test_result(&ctx.app, &ctx.cookie, ctx.case_id, 1, "LFT").await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/test-results/{test_id}", ctx.case_id),
		json!({"data": {
			"test_date": "20210303",
			"test_name": "",
			"test_meddra_version": "26.0",
			"test_meddra_code": "10000001"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	Ok(())
}
