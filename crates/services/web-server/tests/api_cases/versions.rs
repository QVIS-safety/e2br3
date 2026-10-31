use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use crate::validation_common::{get_json, post_json};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::audit::{CaseVersionBmc, CaseVersionForCreate};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_case_versions_returns_saved_snapshot() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_no = format!("VERSION-READ-{}", Uuid::new_v4().simple());
	let (status, created) = post_json(
		&app,
		&cookie,
		"/api/cases".to_string(),
		json!({"data": {
			"safetyReportIdentification": {"safetyReportId": case_no}, "status": "draft"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{created:?}");
	let case_id =
		Uuid::parse_str(created["data"]["id"].as_str().ok_or("missing case id")?)?;
	let snapshot = json!({"caseNo": case_no, "marker": "persisted-version"});
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	CaseVersionBmc::create(
		&ctx,
		&mm,
		CaseVersionForCreate {
			case_id,
			version: 6,
			snapshot: json!({"marker": "older-version"}),
			change_reason: None,
		},
	)
	.await?;
	CaseVersionBmc::create(
		&ctx,
		&mm,
		CaseVersionForCreate {
			case_id,
			version: 7,
			snapshot: snapshot.clone(),
			change_reason: Some("contract read".to_string()),
		},
	)
	.await?;

	let (status, body) =
		get_json(&app, &cookie, format!("/api/cases/{case_id}/versions")).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let versions = body["data"].as_array().ok_or("missing versions")?;
	assert_eq!(versions.len(), 2, "{body:?}");
	assert_eq!(versions[0]["version"], 7);
	assert_eq!(versions[1]["version"], 6);
	let version = &versions[0];
	assert_eq!(version["case_id"], case_id.to_string());
	assert_eq!(version["version"], 7);
	assert_eq!(version["snapshot"], snapshot);
	assert_eq!(version["change_reason"], "contract read");
	assert_eq!(version["changed_by"], seed.admin.id.to_string());
	Ok(())
}
