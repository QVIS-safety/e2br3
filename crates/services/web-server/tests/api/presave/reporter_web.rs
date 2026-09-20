use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
async fn test_reporter_presave_round_trips_mfds_qualification_detail() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/reporters".to_string(),
		Some(json!({
			"data": { "rows": { "reporter": {
				"reporterGivenName": "Min",
				"organization": "MFDS Reporter Org",
				"countryCode": "KR",
				"qualification": "3",
				"qualificationKr1": "1",
				"primarySourceRegulatory": "1"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let reporter_id = data_rows_id(&value, "reporter")?;
	assert_eq!(
		value["data"]["rows"]["reporter"]["qualificationKr1"].as_str(),
		Some("1")
	);
	let (legacy_status, _) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/reporters".to_string(),
		Some(json!({ "data": { "reporter_given_name": "legacy" } })),
	)
	.await?;
	assert_eq!(legacy_status, StatusCode::UNPROCESSABLE_ENTITY);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/reporters/{reporter_id}"),
		Some(json!({
			"data": { "rows": { "reporter": {
				"qualificationKr1": "2"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["reporter"]["qualificationKr1"].as_str(),
		Some("2")
	);
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/reporters/{reporter_id}"),
		Some(json!({
			"data": { "rows": { "reporter": {
				"qualificationKr1": null
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["reporter"]["qualificationKr1"].as_str(),
		Some("2")
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/reporters/{reporter_id}"),
		Some(json!({
			"data": { "rows": { "reporter": {
				"qualification": "1",
				"qualificationKr1": ""
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["reporter"]["qualification"].as_str(),
		Some("1")
	);
	assert_eq!(
		value["data"]["rows"]["reporter"].get("qualificationKr1"),
		Some(&json!(null))
	);
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		format!("/api/presaves/reporters/{reporter_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["reporter"].get("qualificationKr1"),
		Some(&json!(null))
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reporter_presave_accepts_mfds_qualification_detail_input() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/reporters".to_string(),
		Some(json!({
			"data": { "rows": { "reporter": {
				"reporterGivenName": "Min",
				"organization": "MFDS Reporter Invalid Org",
				"qualification": "1",
				"qualificationKr1": "1"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["reporter"]["qualificationKr1"].as_str(),
		Some("1")
	);

	Ok(())
}
