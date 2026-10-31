use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_case_can_be_marked_locked() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("locked"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_locked_case_rejects_content_updates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({
				"data": {
					"dg_prd_key": "LOCKED-EDIT-BLOCKED"
				}
			})
			.to_string(),
		))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let body: Value = serde_json::from_slice(&body)?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_non_editable_workflow_status_blocks_subresource_write() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS"],
							"due_days": 0
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["PVM"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = transition_case_workflow(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "Hand off"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string()
			.contains("workflow status 'To be reviewed' is read-only"),
		"{body:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_qced_case_blocks_content_updates_even_when_workflow_saved_is_editable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) =
		toggle_case_action(&app, &cookie, case_id, "review").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"), "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_validated_case_blocks_content_updates_even_when_workflow_saved_is_editable(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = update_admin_settings(
		&app,
		&cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Default authoring state",
							"allowed_roles": ["PVS", "PVM"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;
	force_validated_case_fixture(&mm, seed.admin.id, seed.org_id, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"report_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"), "{body:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_read_returns_separate_qc_and_lock_axes() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("Pending"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(false));

	let (status, body) =
		toggle_case_action(&app, &cookie, case_id, "review").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("QCed"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(false));
	assert_eq!(
		body["data"]["case_write_block_reason"].as_str(),
		Some("case_qced")
	);

	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("QCed"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(true));
	assert_eq!(
		body["data"]["case_write_block_reason"].as_str(),
		Some("case_locked")
	);
	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["qc_state"].as_str(), Some("QCed"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(true));

	let (status, body) = toggle_case_action(&app, &cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("reviewed"));
	assert_eq!(body["data"]["qc_state"].as_str(), Some("QCed"));
	assert_eq!(body["data"]["is_locked"].as_bool(), Some(false));
	assert_eq!(
		body["data"]["case_write_block_reason"].as_str(),
		Some("case_qced")
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_lifecycle_toggle_rejects_stale_screen_status() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;

	let (status, body) =
		toggle_case_action(&app, &cookie, case_id, "review").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let stale_lock_req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/lock/toggle"))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(
			json!({"data":{"expected_status":"draft"}}).to_string(),
		))?;
	let stale_lock_res = app.clone().oneshot(stale_lock_req).await?;
	assert_eq!(stale_lock_res.status(), StatusCode::CONFLICT);

	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("reviewed"));
	Ok(())
}
