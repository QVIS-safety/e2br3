use super::support::*;

#[serial]
#[tokio::test]
async fn test_home_notice_matrix_privileges_surface_in_current_user_capabilities(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_home_notice_{}", Uuid::new_v4().simple());
	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		"/api/admin/notices".to_string(),
		Some(json!({
			"data": {
				"notices": [{
					"id": "pdf-notice-read",
					"title": "PDF permission notice"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("homeNotice", "read", false),
			("homeNotice", "update", false),
		],
	)
	.await?;
	let runtime = assert_get_status(
		&app,
		&custom_cookie,
		"/api/settings/runtime",
		StatusCode::OK,
	)
	.await?;
	assert_eq!(runtime["notices"], json!([]), "{runtime:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "home_notice",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("homeNotice", "read", true),
			("homeNotice", "update", true),
			("settings", "update", false),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_READ));
	assert!(has_permission(&profile_id, DASHBOARD_NOTICE_UPDATE));
	assert!(!has_permission(&profile_id, SETTINGS_UPDATE));
	let runtime = assert_get_status(
		&app,
		&custom_cookie,
		"/api/settings/runtime",
		StatusCode::OK,
	)
	.await?;
	assert_eq!(runtime["notices"][0]["title"], "PDF permission notice");

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_report_due_mail_rows_persist_but_grant_nothing_while_reserved(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_email_{}", Uuid::new_v4().simple()),
	)
	.await?;
	assert!(!has_permission(&profile_id, EMAIL_NOTIFICATION_SEND));

	// The PDF row remains representable for migration, but reserved grants never
	// compile to an operational permission.
	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([{
			"menu_key": "email_report_due",
			"can_read": false,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(!has_permission(&profile_id, EMAIL_NOTIFICATION_SEND));

	Ok(())
}
