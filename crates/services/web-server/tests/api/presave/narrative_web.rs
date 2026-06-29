use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

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
			"data": {
				"case_narrative": "REST minimal narrative"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(value["data"].get("name").is_none(), "{value:?}");
	assert!(value["data"].get("comments").is_none(), "{value:?}");

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		"/api/presaves/narratives".to_string(),
		Some(json!({
			"data": {
				"case_narrative": "REST auto narrative {D.2.2a} {D.5}",
				"case_narrative_notation": "REST notation",
				"additional_information": "REST sponsor additional information"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	assert!(value["data"].get("name").is_none(), "{value:?}");
	assert!(value["data"].get("comments").is_none(), "{value:?}");
	assert_eq!(
		value["data"]["case_narrative"].as_str(),
		Some("REST auto narrative {D.2.2a} {D.5}")
	);
	assert_eq!(
		value["data"]["additional_information"].as_str(),
		Some("REST sponsor additional information")
	);
	let narrative_id = data_id(&value)?;

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
			.any(|row| row["id"].as_str() == Some(&narrative_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/narratives/{narrative_id}"),
		Some(json!({
			"data": {
				"case_narrative": "REST auto narrative updated {D.2.2a} {D.5}",
				"additional_information": "REST sponsor additional information updated"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(
		value["data"]["case_narrative"].as_str(),
		Some("REST auto narrative updated {D.2.2a} {D.5}")
	);
	assert_eq!(
		value["data"]["additional_information"].as_str(),
		Some("REST sponsor additional information updated")
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/narratives/{narrative_id}/sender-diagnoses"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"diagnosis_meddra_version": "26.1",
				"diagnosis_meddra_code": "10000001"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let diagnosis_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!(
			"/api/presaves/narratives/{narrative_id}/sender-diagnoses/{diagnosis_id}"
		),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::POST,
		format!("/api/presaves/narratives/{narrative_id}/case-summaries"),
		Some(json!({
			"data": {
				"sequence_number": 1,
				"summary_type": "sender",
				"language_code": "en",
				"summary_text": "REST summary"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let summary_id = data_id(&value)?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::GET,
		format!("/api/presaves/narratives/{narrative_id}/case-summaries"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(
		value["data"]
			.as_array()
			.ok_or("case summary list data is not array")?
			.iter()
			.any(|row| row["id"].as_str() == Some(&summary_id.to_string())),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!(
			"/api/presaves/narratives/{narrative_id}/case-summaries/{summary_id}"
		),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["deleted"].as_bool(), Some(true));

	let narrative_delete_uris = [
		format!(
			"/api/presaves/narratives/{narrative_id}/case-summaries/{summary_id}"
		),
		format!(
			"/api/presaves/narratives/{narrative_id}/sender-diagnoses/{diagnosis_id}"
		),
		format!("/api/presaves/narratives/{narrative_id}"),
	];
	for uri in narrative_delete_uris {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::DELETE, uri.clone(), None)
				.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");

		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::OK, "{value:?}");
		assert_eq!(value["data"]["deleted"].as_bool(), Some(true));
	}

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
		.find(|row| row["id"].as_str() == Some(&narrative_id.to_string()))
		.ok_or("deleted narrative missing from list")?;
	assert_eq!(deleted_row["deleted"].as_bool(), Some(true));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_graph_load_save_and_delete() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id =
		create_narrative_presave_with_authority_via_api(&app, &admin_cookie, "ich")
			.await?;
	let diagnosis_id = create_narrative_sender_diagnosis_with_code_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"10000001",
	)
	.await?;
	let summary_id = create_narrative_case_summary_with_text_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Summary old",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], narrative_id.to_string());
	assert_eq!(
		details["data"]["sender_diagnoses"][0]["id"],
		diagnosis_id.to_string()
	);
	assert_eq!(
		details["data"]["case_summaries"][0]["id"],
		summary_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"parent": { "case_narrative": "Narrative graph updated" },
				"sender_diagnoses": [
					{
						"id": diagnosis_id,
						"sequence_number": 2,
						"diagnosis_meddra_version": "26.1",
						"diagnosis_meddra_code": "10000002"
					},
					{
						"sequence_number": 3,
						"diagnosis_meddra_version": "26.1",
						"diagnosis_meddra_code": "10000003"
					}
				],
				"case_summaries": [
					{
						"id": summary_id,
						"sequence_number": 2,
						"summary_type": "sender",
						"language_code": "en",
						"summary_text": "Summary updated"
					},
					{
						"sequence_number": 3,
						"summary_type": "reporter",
						"language_code": "en",
						"summary_text": "Summary created"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["case_narrative"],
		"Narrative graph updated"
	);
	assert_eq!(
		saved["data"]["sender_diagnoses"].as_array().unwrap().len(),
		2
	);
	assert_eq!(saved["data"]["case_summaries"].as_array().unwrap().len(), 2);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"case_summaries": [{ "id": summary_id, "_delete": true }]
			}
		}),
	)
	.await?;
	let after_delete = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	let deleted_summary = after_delete["data"]["case_summaries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"].as_str() == Some(&summary_id.to_string()))
		.ok_or("missing deleted summary")?
		.clone();
	assert_eq!(deleted_summary["deleted"].as_bool(), Some(true));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_graph_load_and_save() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"10000001",
	)
	.await?;
	let summary_id = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Summary 1",
	)
	.await?;

	let details = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(details["data"]["parent"]["id"], narrative_id.to_string());
	assert_eq!(
		details["data"]["sender_diagnoses"][0]["id"],
		diagnosis_id.to_string()
	);
	assert_eq!(
		details["data"]["case_summaries"][0]["id"],
		summary_id.to_string()
	);

	let saved = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"parent": { "case_narrative": "updated by narrative graph" },
				"sender_diagnoses": [
					{
						"id": diagnosis_id,
						"sequence_number": 2,
						"diagnosis_meddra_version": "27.0",
						"diagnosis_meddra_code": "10000002"
					},
					{
						"sequence_number": 3,
						"diagnosis_meddra_version": "27.0",
						"diagnosis_meddra_code": "10000003"
					}
				],
				"case_summaries": [
					{
						"id": summary_id,
						"sequence_number": 2,
						"summary_type": "company",
						"language_code": "en",
						"summary_text": "Summary 2"
					},
					{
						"sequence_number": 3,
						"summary_type": "sender",
						"language_code": "ko",
						"summary_text": "Summary 3"
					}
				]
			}
		}),
	)
	.await?;
	assert_eq!(
		saved["data"]["parent"]["case_narrative"],
		"updated by narrative graph"
	);
	assert_eq!(
		saved["data"]["sender_diagnoses"].as_array().unwrap().len(),
		2
	);
	assert_eq!(saved["data"]["case_summaries"].as_array().unwrap().len(), 2);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_requires_explicit_child_delete() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_id = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_delete_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"DELETE",
	)
	.await?;
	let diagnosis_keep_id = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		2,
		"KEEP",
	)
	.await?;
	let summary_id = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_id,
		1,
		"Keep summary",
	)
	.await?;

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({ "data": { "parent": { "case_narrative": "omit children" } } }),
	)
	.await?;
	let after_omit = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(
		after_omit["data"]["sender_diagnoses"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_omit["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({ "data": { "sender_diagnoses": [], "case_summaries": [] } }),
	)
	.await?;
	let after_empty = get_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
	)
	.await?;
	assert_eq!(
		after_empty["data"]["sender_diagnoses"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(
		after_empty["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.len(),
		1
	);

	let after_delete = put_json_ok(
		&app,
		&admin_cookie,
		format!("/api/presaves/narratives/{narrative_id}/details"),
		json!({
			"data": {
				"sender_diagnoses": [
					{ "id": diagnosis_delete_id, "_delete": true }
				]
			}
		}),
	)
	.await?;
	let diagnoses = after_delete["data"]["sender_diagnoses"].as_array().unwrap();
	let deleted_diagnosis = diagnoses
		.iter()
		.find(|row| row["id"].as_str() == Some(&diagnosis_delete_id.to_string()))
		.ok_or("missing deleted diagnosis")?;
	assert_eq!(deleted_diagnosis["deleted"].as_bool(), Some(true));
	let kept_diagnosis = diagnoses
		.iter()
		.find(|row| row["id"].as_str() == Some(&diagnosis_keep_id.to_string()))
		.ok_or("missing kept diagnosis")?;
	assert_eq!(kept_diagnosis["deleted"].as_bool(), Some(false));
	assert!(
		after_delete["data"]["case_summaries"]
			.as_array()
			.unwrap()
			.iter()
			.any(|row| row["id"].as_str() == Some(&summary_id.to_string())),
		"{after_delete:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_details_rejects_invalid_child_operations(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let narrative_a = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let narrative_b = create_narrative_presave_via_api(&app, &admin_cookie).await?;
	let diagnosis_b = create_narrative_sender_diagnosis_via_api(
		&app,
		&admin_cookie,
		narrative_b,
		1,
		"OTHER",
	)
	.await?;
	let summary_b = create_narrative_case_summary_via_api(
		&app,
		&admin_cookie,
		narrative_b,
		1,
		"Other summary",
	)
	.await?;

	for body in [
		json!({ "data": { "sender_diagnoses": [{ "_delete": true }] } }),
		json!({ "data": { "case_summaries": [{ "_delete": true }] } }),
		json!({ "data": { "sender_diagnoses": [{ "id": diagnosis_b, "_delete": true }] } }),
		json!({ "data": { "case_summaries": [{ "id": summary_b, "_delete": true }] } }),
		json!({ "data": { "sender_diagnoses": [{ "id": diagnosis_b, "sequence_number": 2, "diagnosis_meddra_code": "WRONG" }] } }),
		json!({ "data": { "case_summaries": [{ "id": summary_b, "sequence_number": 2, "summary_text": "Wrong parent" }] } }),
	] {
		let (status, value) = request_json(
			&app,
			&admin_cookie,
			Method::PUT,
			format!("/api/presaves/narratives/{narrative_a}/details"),
			Some(body),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	}

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/presaves/narratives/{narrative_a}/details"),
		Some(json!({ "data": { "parent": { "metadata_name": " " } } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(value["data"]["parent"].get("name").is_none(), "{value:?}");
	assert!(value["data"]["parent"].get("comments").is_none(), "{value:?}");

	Ok(())
}
