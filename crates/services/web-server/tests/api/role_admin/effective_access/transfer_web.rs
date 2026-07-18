use super::support::*;

#[serial]
#[tokio::test]
async fn test_export_submission_matrix_privileges_grant_effective_xml_export_permission(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let no_export_profile_name =
		format!("qa_export_none_{}", Uuid::new_v4().simple());
	let read_profile_name = format!("qa_export_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_export_edit_{}", Uuid::new_v4().simple());

	let no_export_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&no_export_profile_name,
	)
	.await?;
	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_no_export_user_id, no_export_cookie) =
		custom_role_user(&mm, seed.org_id, &no_export_profile_id).await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT),
		"empty custom role must not grant XML_EXPORT"
	);
	assert!(
		!has_permission(&no_export_profile_id, XML_EXPORT_READ),
		"empty custom role must not grant XML_EXPORT_READ"
	);
	assert_get_status(
		&app,
		&no_export_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_access(
		&app,
		&read_cookie,
		&[
			("exportSubmission", "read", true),
			("exportSubmission", "execute", false),
		],
	)
	.await?;
	assert!(
		has_permission(&read_profile_id, XML_EXPORT_READ),
		"export_submission.can_read must grant XML_EXPORT_READ"
	);
	assert!(
		!has_permission(&read_profile_id, XML_EXPORT),
		"export_submission.can_read must not grant XML_EXPORT"
	);
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&read_cookie,
		"/api/cases/export/xml".to_string(),
		Some(json!({ "case_ids": [] })),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"export_submission.can_read must not execute XML export: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "export_submission",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_access(
		&app,
		&edit_cookie,
		&[
			("exportSubmission", "read", false),
			("exportSubmission", "execute", true),
		],
	)
	.await?;
	assert!(
		has_permission(&edit_profile_id, XML_EXPORT),
		"export_submission.can_edit must independently grant XML_EXPORT"
	);
	assert!(
		!has_permission(&edit_profile_id, XML_EXPORT_READ),
		"export_submission.can_edit must not grant history read without can_read"
	);
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/exports/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&edit_cookie,
		"/api/cases/export/xml".to_string(),
		Some(json!({ "case_ids": [] })),
	)
	.await?;
	assert_ne!(
		status,
		StatusCode::FORBIDDEN,
		"export_submission.can_edit should pass XML export permission check: {value:?}"
	);

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_import_matrix_privileges_split_files_edit_from_history_read(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let read_profile_name = format!("qa_import_read_{}", Uuid::new_v4().simple());
	let edit_profile_name = format!("qa_import_edit_{}", Uuid::new_v4().simple());

	let read_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&read_profile_name,
	)
	.await?;
	let edit_profile_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&edit_profile_name,
	)
	.await?;
	let (_read_user_id, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_profile_id).await?;
	let (_edit_user_id, edit_cookie) =
		custom_role_user(&mm, seed.org_id, &edit_profile_id).await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&read_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_access(
		&app,
		&read_cookie,
		&[("import", "read", true), ("import", "execute", false)],
	)
	.await?;
	assert!(has_permission(&read_profile_id, XML_IMPORT_READ));
	assert!(!has_permission(&read_profile_id, XML_IMPORT));
	assert_get_status(
		&app,
		&read_cookie,
		"/api/import/xml/history",
		StatusCode::OK,
	)
	.await?;
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", read_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(
		res.status(),
		StatusCode::FORBIDDEN,
		"import.can_read must not execute XML import"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&edit_profile_id,
		json!([
			{
				"menu_key": "import",
				"can_read": false,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_access(
		&app,
		&edit_cookie,
		&[("import", "read", false), ("import", "execute", true)],
	)
	.await?;
	assert!(has_permission(&edit_profile_id, XML_IMPORT));
	assert!(!has_permission(&edit_profile_id, XML_IMPORT_READ));
	assert_get_status(
		&app,
		&edit_cookie,
		"/api/import/xml/history",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header("cookie", edit_cookie.clone())
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_ne!(
		res.status(),
		StatusCode::FORBIDDEN,
		"import.can_edit should pass XML import permission check"
	);

	Ok(())
}
