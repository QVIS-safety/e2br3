use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_workflow_transition_updates_case_and_persists_event() -> Result<()> {
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
	create_message_header(&app, &cookie, case_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "Ready for review",
				"due_at": "2026-04-20T09:00:00Z"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(
		body["data"]["workflow_status"].as_str(),
		Some("To be reviewed")
	);
	assert_eq!(body["data"]["workflow_assigned_role"].as_str(), Some("pvm"));
	assert_eq!(
		body["data"]["workflow_due_at"].as_str(),
		Some("2026-04-20T09:00:00Z")
	);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/cases/{case_id}/editor/shell"))
		.header("cookie", &cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let shell = serde_json::from_slice::<Value>(&body)?;
	assert_eq!(status, StatusCode::OK, "{shell:?}");
	assert_eq!(
		shell["workflowDueAt"].as_str(),
		Some("2026-04-20T09:00:00Z")
	);

	let (status, events) = get_workflow_events(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"].as_array().map(|rows| rows.len()), Some(1));
	assert_eq!(events["data"][0]["fromStatus"].as_str(), Some("Saved"));
	assert_eq!(
		events["data"][0]["toStatus"].as_str(),
		Some("To be reviewed")
	);
	assert_eq!(events["data"][0]["targetRole"].as_str(), Some("pvm"));
	assert_eq!(
		events["data"][0]["comment"].as_str(),
		Some("Ready for review")
	);
	assert!(events["data"][0].get("fromRole").is_some());
	assert!(events["data"][0].get("fromUserId").is_some());
	assert!(events["data"][0].get("targetUserDisplay").is_some());
	assert!(
		events["data"][0]["actedByDisplay"].as_str().is_some(),
		"{events:?}"
	);
	assert_eq!(
		events["data"][0]["dateOfMostRecent"]
			.as_str()
			.unwrap_or("N/A"),
		"N/A"
	);
	assert!(
		events["data"][0]["delay"]
			.as_str()
			.is_some_and(|value| value == "N/A" || value.ends_with('d')),
		"{events:?}"
	);
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_assignment_updates_owner_without_changing_status(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Initial state",
							"allowed_roles": ["user"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = assign_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"target_role": "user",
				"target_user_id": seed.user.id,
				"comment": "Assign authoring owner",
				"due_at": "2026-04-21T09:00:00Z"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["workflow_status"].as_str(), Some("Saved"));
	assert_eq!(
		body["data"]["workflow_assigned_role"].as_str(),
		Some("user")
	);
	assert_eq!(body["data"]["can_act_on_workflow"].as_bool(), Some(true));
	assert_eq!(
		body["data"]["workflow_block_reason"].as_str(),
		Some("workflow_admin_override_allowed")
	);
	let assigned_user_id = seed.user.id.to_string();
	assert_eq!(
		body["data"]["workflow_assigned_user_id"].as_str(),
		Some(assigned_user_id.as_str())
	);

	let (status, events) = get_workflow_events(&app, &admin_cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"].as_array().map(|rows| rows.len()), Some(1));
	assert_eq!(events["data"][0]["fromStatus"].as_str(), Some("Saved"));
	assert_eq!(events["data"][0]["toStatus"].as_str(), Some("Saved"));
	assert_eq!(
		events["data"][0]["comment"].as_str(),
		Some("Assign authoring owner")
	);
	assert!(events["data"][0].get("fromRole").is_some());
	assert!(events["data"][0].get("fromUserId").is_some());
	assert!(
		events["data"][0]["targetUserDisplay"].as_str().is_some(),
		"{events:?}"
	);
	assert!(
		events["data"][0]["actedByDisplay"].as_str().is_some(),
		"{events:?}"
	);
	assert!(
		events["data"][0]["delay"]
			.as_str()
			.is_some_and(|value| value == "N/A" || value.ends_with('d')),
		"{events:?}"
	);
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_workflow_transition_rejects_user_outside_current_step_role(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let user_cookie = cookie_header(&user_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Owned by PVS only",
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

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "PVM",
				"comment": "User should not own Saved"
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
async fn test_workflow_transition_rejects_user_outside_current_assignee(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let other_user =
		insert_user(&mm, seed.org_id, ROLE_USER, system_user_id(), None).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let other_user_token =
		generate_web_token(&other_user.email, other_user.token_salt)?;
	let user_token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let other_user_cookie = cookie_header(&other_user_token.to_string());
	let user_cookie = cookie_header(&user_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Initial state",
							"allowed_roles": ["user"],
							"due_days": 0
						},
						{
							"name": "Assigned",
							"editable": true,
							"description": "Owned by one assigned user",
							"allowed_roles": ["user"],
							"due_days": 0
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending internal review",
							"allowed_roles": ["manager"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "Assigned",
				"target_role": "user",
				"target_user_id": seed.user.id,
				"comment": "Assign to specific user"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = transition_case_workflow(
		&app,
		&other_user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Wrong user should be blocked"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"), "{body:?}");

	let (status, body) = transition_case_workflow(
		&app,
		&user_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Assigned user can transition"
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
async fn test_workflow_admin_override_is_allowed_and_audited() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"description": "Owned by user role",
							"allowed_roles": ["user"],
							"due_days": 0
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"description": "Pending manager review",
							"allowed_roles": ["manager"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager",
				"comment": "Admin override handoff"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["can_act_on_workflow"].as_bool(), Some(true));
	assert_eq!(
		body["data"]["workflow_block_reason"].as_str(),
		Some("workflow_admin_override_allowed")
	);

	let (status, events) = get_workflow_events(&app, &admin_cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{events:?}");
	assert_eq!(events["data"][0]["usedAdminOverride"].as_bool(), Some(true));
	assert_eq!(
		events["data"][0]["actorRoleId"].as_str(),
		Some("sponsor_admin_cro")
	);
	assert!(
		events["data"][0]["overrideReason"]
			.as_str()
			.unwrap_or_default()
			.contains("audited admin policy"),
		"{events:?}"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_locked_case_blocks_workflow_transition_even_for_admin_override(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_all_roles(&mm).await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, body) = update_admin_settings(
		&app,
		&admin_cookie,
		json!({
			"data": {
				"workflow_enabled": true,
				"workflow": {
					"statuses": [
						{
							"name": "Saved",
							"editable": true,
							"allowed_roles": ["user"],
							"due_days": 0
						},
						{
							"name": "To be reviewed",
							"editable": false,
							"allowed_roles": ["manager"],
							"due_days": 0
						}
					]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let (status, body) =
		toggle_case_action(&app, &admin_cookie, case_id, "lock").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = transition_case_workflow(
		&app,
		&admin_cookie,
		case_id,
		json!({
			"data": {
				"to_status": "To be reviewed",
				"target_role": "manager"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body:?}");
	assert!(body.to_string().contains("PERMISSION_DENIED"), "{body:?}");
	Ok(())
}
