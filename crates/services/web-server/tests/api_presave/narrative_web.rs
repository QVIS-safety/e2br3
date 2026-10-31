use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;

#[tokio::test]
async fn test_section_presave_narrative_rest_contract() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({
			"data": { "rows": { "narrative": {
				"caseNarrative": "REST minimal narrative"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["narrative"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["narrative"].get("comments").is_none(),
		"{value:?}"
	);
	let (legacy_status, _) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({ "data": { "case_narrative": "legacy" } })),
	)
	.await?;
	assert_eq!(legacy_status, StatusCode::UNPROCESSABLE_ENTITY);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({
			"data": { "rows": { "narrative": {
				"caseNarrative": "REST auto narrative {D.2.2a} {D.5}",
				"caseNarrativeNotation": "REST notation",
				"additionalInformation": "REST sponsor additional information"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(
		value["data"]["rows"]["narrative"].get("name").is_none(),
		"{value:?}"
	);
	assert!(
		value["data"]["rows"]["narrative"].get("comments").is_none(),
		"{value:?}"
	);
	assert_eq!(
		value["data"]["rows"]["narrative"]["caseNarrative"].as_str(),
		Some("REST auto narrative {D.2.2a} {D.5}")
	);
	assert_eq!(
		value["data"]["rows"]["narrative"]["additionalInformation"].as_str(),
		Some("REST sponsor additional information")
	);
	let narrative_id = data_rows_id(&value, "narrative")?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		"/api/presaves/narratives".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("narrative list data is not array")?
			.iter()
			.any(|row| row["rows"]["narrative"]["id"].as_str()
				== Some(&narrative_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/narratives/{narrative_id}"),
		Some(json!({
			"data": { "rows": { "narrative": {
				"caseNarrative": "REST auto narrative updated {D.2.2a} {D.5}",
				"additionalInformation": "REST sponsor additional information updated"
			} } }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["narrative"]["caseNarrative"].as_str(),
		Some("REST auto narrative updated {D.2.2a} {D.5}")
	);
	assert_eq!(
		value["data"]["rows"]["narrative"]["additionalInformation"].as_str(),
		Some("REST sponsor additional information updated")
	);

	let uri = format!("/api/presaves/narratives/{narrative_id}");
	let (status, value) =
		request_json(&app, &admin_cookie, Method::DELETE, uri.clone(), None).await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");

	let (status, value) =
		request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["rows"]["narrative"]["deleted"].as_bool(),
		Some(true)
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		"/api/presaves/narratives".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let deleted_row = value["data"]
		.as_array()
		.ok_or("narrative list data is not array")?
		.iter()
		.find(|row| {
			row["rows"]["narrative"]["id"].as_str()
				== Some(&narrative_id.to_string())
		})
		.ok_or("deleted narrative missing from list")?;
	assert_eq!(
		deleted_row["rows"]["narrative"]["deleted"].as_bool(),
		Some(true)
	);

	Ok(())
}
