use super::support::*;

#[serial]
#[tokio::test]
async fn test_data_matrix_privileges_grant_effective_terminology_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_data_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("data", "read", true),
			("data", "import", false),
			("data", "approve", false),
		],
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/terminology/countries",
		StatusCode::OK,
	)
	.await?;
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"read-only DATA must not grant terminology import permission"
	);
	assert!(
		!has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"read-only DATA must not grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(
		res.status(),
		StatusCode::FORBIDDEN,
		"read-only DATA must not import terminology"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"read-only DATA must not approve terminology releases: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_capabilities(
		&app,
		&custom_cookie,
		&[
			("data", "read", true),
			("data", "import", true),
			("data", "approve", true),
		],
	)
	.await?;
	assert!(
		has_permission(&profile_id, TERMINOLOGY_IMPORT),
		"editable DATA must grant terminology import permission"
	);
	assert!(
		has_permission(&profile_id, TERMINOLOGY_APPROVE),
		"editable DATA must grant terminology approve permission"
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", custom_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_ne!(
		res.status(),
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology import permission check"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/terminology/releases/meddra/TEST/approve".to_string(),
		None,
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"editable DATA should pass terminology approve permission check: {value:?}"
	);

	Ok(())
}
