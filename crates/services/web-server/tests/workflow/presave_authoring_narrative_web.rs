use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use crate::presave_authoring::{
	apply_authoring_presave, create_case, create_template, get_template_data,
	request_json,
};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_narrative_presave_imports_into_case_fields_and_persists() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let narrative_data = json!({
		"caseNarrative": "Narrative text",
		"reporterComments": "Reporter comments",
		"senderComments": "Sender comments",
		"caseSummary": "Case summary text",
		"senderDiagnoses": [{
			"diagnosisMeddraCode": "10028813",
			"diagnosisMeddraVersion": "27.0"
		}]
	});
	let (template_id, _) = create_template(
		&app,
		&cookie,
		"narrative",
		"narrative-authoring",
		narrative_data,
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "narrative", &saved_data)
		.await?;

	let (status, narrative) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{narrative:?}");
	assert_eq!(
		narrative["data"]["case_narrative"].as_str(),
		Some("Narrative text")
	);
	assert_eq!(
		narrative["data"]["reporter_comments"].as_str(),
		Some("Reporter comments")
	);
	assert_eq!(
		narrative["data"]["sender_comments"].as_str(),
		Some("Sender comments")
	);

	let (status, summaries) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative/summaries"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{summaries:?}");
	let summary = summaries["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing case summary row")?;
	assert_eq!(summary["summary_text"].as_str(), Some("Case summary text"));

	let (status, diagnoses) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{diagnoses:?}");
	let diagnosis = diagnoses["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.ok_or("missing sender diagnosis row")?;
	assert_eq!(
		diagnosis["diagnosis_meddra_code"].as_str(),
		Some("10028813")
	);
	assert_eq!(diagnosis["diagnosis_meddra_version"].as_str(), Some("27.0"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_imports_minimal_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"narrative",
		"narrative-minimal",
		json!({
			"caseNarrative": "Minimal narrative"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "narrative", &saved_data)
		.await?;

	let (status, narrative) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{narrative:?}");
	assert_eq!(
		narrative["data"]["case_narrative"].as_str(),
		Some("Minimal narrative")
	);
	assert!(narrative["data"]["reporter_comments"].is_null());
	assert!(narrative["data"]["sender_comments"].is_null());

	let (status, summaries) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative/summaries"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{summaries:?}");
	assert_eq!(summaries["data"].as_array().map(|rows| rows.len()), Some(0));

	let (status, diagnoses) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{diagnoses:?}");
	assert_eq!(diagnoses["data"].as_array().map(|rows| rows.len()), Some(0));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_presave_updates_comments_and_merges_nested_rows(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::POST,
		format!("/api/cases/{case_id}/narrative"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"case_narrative": "Existing narrative"
			}
		})),
	)
	.await?;
	assert!(
		status == StatusCode::CREATED || status == StatusCode::OK,
		"{value:?}"
	);
	let narrative_id = value["data"]["id"].as_str().ok_or("missing narrative id")?;

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/narrative"),
		Some(json!({
			"data": {
				"reporter_comments": "Existing reporter comments",
				"sender_comments": "Existing sender comments"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::POST,
		format!("/api/cases/{case_id}/narrative/summaries"),
		Some(json!({
			"data": {
				"narrative_id": narrative_id,
				"sequence_number": 2,
				"summary_text": "Existing summary"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	let (template_id, _) = create_template(
		&app,
		&cookie,
		"narrative",
		"narrative-overwrite",
		json!({
			"caseNarrative": "Imported narrative",
			"reporterComments": "Imported reporter comments",
			"senderComments": "Imported sender comments",
			"caseSummary": "Imported summary"
		}),
	)
	.await?;
	let saved_data = get_template_data(&app, &cookie, template_id).await?;
	apply_authoring_presave(&app, &cookie, case_id, "narrative", &saved_data)
		.await?;

	let (status, narrative) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{narrative:?}");
	assert_eq!(
		narrative["data"]["case_narrative"].as_str(),
		Some("Existing narrative")
	);
	assert_eq!(
		narrative["data"]["reporter_comments"].as_str(),
		Some("Imported reporter comments")
	);
	assert_eq!(
		narrative["data"]["sender_comments"].as_str(),
		Some("Imported sender comments")
	);

	let (status, summaries) = request_json(
		&app,
		&cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative/summaries"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{summaries:?}");
	let rows = summaries["data"].as_array().ok_or("missing summary rows")?;
	assert_eq!(rows.len(), 2, "{summaries:?}");
	assert!(rows
		.iter()
		.any(|row| row["summary_text"].as_str() == Some("Existing summary")));
	assert!(rows
		.iter()
		.any(|row| row["summary_text"].as_str() == Some("Imported summary")));
	Ok(())
}
