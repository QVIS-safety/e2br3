use super::support::*;

#[serial]
#[tokio::test]
async fn test_role_privilege_matrix_update_grants_effective_case_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "QA effective role",
				"description": "Starts without effective case permissions",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();

	let custom_user = insert_user(
		&mm,
		seed.org_id,
		&profile_id,
		system_user_id(),
		Some("custompwd"),
	)
	.await?;
	let custom_token =
		generate_web_token(&custom_user.email, custom_user.token_salt)?;
	let custom_cookie = cookie_header(&custom_token.to_string());

	let (status, _value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({
			"data": {
				"privileges": [
					{
						"menu_key": "case",
						"can_read": true,
						"can_edit": false,
						"can_review": false,
						"can_lock": false
					}
				]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &custom_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, profile) = request_json(
		&app,
		"GET",
		&custom_cookie,
		"/api/users/me/profile".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{profile:?}");
	assert_eq!(
		profile["data"]["privileges"],
		json!([{
			"menu_key": "case",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": false
		}]),
		"the authenticated profile must expose the stored menu intent without reverse-mapping permissions"
	);

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_home_workflow_matrix_privileges_grant_effective_case_list_access(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let none_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_hw_none_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let read_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_hw_read_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let (_none_user, none_cookie) =
		custom_role_user(&mm, seed.org_id, &none_id).await?;
	let (_read_user, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_id).await?;

	// Unchecked: no case access.
	assert!(!has_permission(&none_id, CASE_LIST));
	assert_get_status(
		&app,
		&none_cookie,
		"/api/cases/list-view",
		StatusCode::FORBIDDEN,
	)
	.await?;

	// home_workflow read grants case view + list, but not write.
	update_role_privileges(
		&app,
		&admin_cookie,
		&read_id,
		json!([{
			"menu_key": "home_workflow",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(has_permission(&read_id, CASE_READ));
	assert!(has_permission(&read_id, CASE_LIST));
	assert!(!has_permission(&read_id, CASE_CREATE));
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/cases/list-view",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}

// Gap coverage: audit read (or review) privilege must grant effective audit-log
