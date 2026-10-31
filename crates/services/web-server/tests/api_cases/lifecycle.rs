use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_update_case_rejects_invalid_status() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "not-a-status").await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(body.to_string().contains("invalid case status"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_create_case_rejects_every_protected_lifecycle_status() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for protected_status in [
		"reviewed",
		"validated",
		"locked",
		"submitted",
		"deleted",
		"archived",
		"nullified",
	] {
		let (status, body) = create_case_with_payload(
			&app,
			&cookie,
			json!({
				"data": {
					"safetyReportIdentification": {
						"safetyReportId": format!("PROTECTED-{protected_status}-{}", Uuid::new_v4())
					},
					"status": protected_status
				}
			}),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::BAD_REQUEST,
			"{protected_status}: {body}"
		);
		assert!(body
			.to_string()
			.contains("case creation only accepts draft"));
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_status_transition_prevents_regression_after_submitted(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) =
		update_case_status(&app, &cookie, case_id, "submitted").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_case_status(&app, &cookie, case_id, "draft").await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_save_does_not_auto_transition_status_when_updating_fields(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"dg_prd_key": "UPDATED-DG-PRD-001"
				},
				"reason_for_change": "update case scope"
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = serde_json::from_slice::<Value>(
		&to_bytes(res.into_body(), usize::MAX).await?,
	)?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("draft"));
	assert_eq!(
		body["data"]["dg_prd_key"].as_str(),
		Some("UPDATED-DG-PRD-001")
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_review_toggle_allows_validated_to_draft_transition() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	force_validated_case_fixture(&mm, seed.admin.id, seed.org_id, case_id).await?;

	let (status, body) =
		toggle_case_action(&app, &cookie, case_id, "review").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(body.get("data").is_some(), "{body:?}");

	Ok(())
}
