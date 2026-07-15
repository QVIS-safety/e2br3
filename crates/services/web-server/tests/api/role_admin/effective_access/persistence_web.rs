use super::support::*;

#[serial]
#[tokio::test]
async fn test_role_admin_api_persists_privilege_matrix_menu_keys() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		"POST",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "QA matrix role",
				"description": "Created before privilege matrix toggles",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let profile_id = value["id"].as_str().ok_or("missing role id")?.to_string();

	let matrix_privileges = json!([
		{
			"menu_key": "home_workflow",
			"can_read": true,
			"can_edit": false,
			"can_review": true,
			"can_lock": false
		},
		{
			"menu_key": "home_notice",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		},
		{
			"menu_key": "case",
			"can_read": true,
			"can_edit": true,
			"can_review": true,
			"can_lock": true
		},
		{
			"menu_key": "info",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		},
		{
			"menu_key": "import",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}
	]);

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{profile_id}"),
		Some(json!({ "data": { "privileges": matrix_privileges } })),
	)
	.await?;

	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/admin/permission-profiles".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let roles = value.as_array().ok_or("roles response should be array")?;
	let persisted_role = roles
		.iter()
		.find(|role| role["id"] == profile_id)
		.ok_or("missing persisted matrix role")?;
	let privileges = persisted_role["privileges"]
		.as_array()
		.ok_or("persisted role privileges should be an array")?;
	for (menu_key, can_read, can_edit, can_review, can_lock) in [
		("home_workflow", true, false, true, false),
		("home_notice", true, true, false, false),
		("case", true, true, true, true),
		("info", true, true, false, false),
		("import", true, true, false, false),
	] {
		let row = privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing persisted privilege for {menu_key}"))?;
		assert_eq!(row["can_read"].as_bool(), Some(can_read), "{menu_key}");
		assert_eq!(row["can_edit"].as_bool(), Some(can_edit), "{menu_key}");
		assert_eq!(row["can_review"].as_bool(), Some(can_review), "{menu_key}");
		assert_eq!(row["can_lock"].as_bool(), Some(can_lock), "{menu_key}");
	}
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_READ));
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_UPDATE));
	for menu_key in [
		"report_due_mail",
		"monitoring",
		"sync",
		"sync_mapping",
		// Organization management is system-admin only, not a matrix privilege.
		"organization",
		"organizations",
	] {
		let invalid_privileges = json!([
			{
				"menu_key": menu_key,
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]);
		let (status, value) = request_json(
			&app,
			"PUT",
			&admin_cookie,
			format!("/api/admin/permission-profiles/{profile_id}"),
			Some(json!({ "data": { "privileges": invalid_privileges } })),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
		assert!(
			value
				.to_string()
				.contains(&format!("unknown role privilege menu '{menu_key}'")),
			"unexpected unsupported privilege body for {menu_key}: {value:?}"
		);
	}

	Ok(())
}
