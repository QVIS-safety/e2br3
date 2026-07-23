use super::support::*;

#[serial]
#[tokio::test]
async fn test_admin_edit_cannot_manage_roles_or_assign_roles() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_users_edit_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([{
			"menu_key": "admin",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({ "data": { "comments": "allowed user-field edit" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": { "name": "Escalated Role", "privileges": [] }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({ "data": { "role": profile_id } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_matrix_privileges_grant_user_operations_but_not_role_identity(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_users_roles_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "read", false),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/admin/permission-profiles",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "admin",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&profile_id, USER_READ));
	assert!(has_permission(&profile_id, USER_LIST));
	assert!(!has_permission(&profile_id, USER_CREATE));
	assert!(!has_permission(&profile_id, USER_UPDATE));
	assert!(!has_permission(&profile_id, USER_DELETE));
	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("admin", "read", true),
			("admin", "update", false),
			("users", "read", true),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"comments": "users read must not update"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"users.can_read must not update users: {value:?}"
	);

	let roles_profile_id = format!("qa_roles_matrix_{}", Uuid::new_v4().simple());
	let roles_profile_id =
		create_empty_custom_role(&app, &admin_cookie, &roles_profile_id).await?;
	let (_roles_user_id, roles_cookie) =
		custom_role_user(&mm, seed.org_id, &roles_profile_id).await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/admin/permission-profiles/{roles_profile_id}"),
		Some(json!({ "data": { "privileges": [{
				"menu_key": "roles",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}] } })),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(!has_permission(&roles_profile_id, USER_CREATE));
	assert!(!has_permission(&roles_profile_id, USER_UPDATE));
	assert!(!has_permission(&roles_profile_id, USER_DELETE));
	assert_profile_access(
		&app,
		&roles_cookie,
		&[
			("admin", "read", false),
			("admin", "update", false),
			("users", "create", false),
			("users", "update", false),
			("users", "delete", false),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&roles_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Roles Matrix Child",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"roles alias must not create permission profiles: {value:?}"
	);

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "admin",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(has_permission(&profile_id, USER_CREATE));
	assert!(has_permission(&profile_id, USER_UPDATE));
	assert!(has_permission(&profile_id, USER_DELETE));
	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("admin", "read", true),
			("admin", "update", true),
			("users", "read", true),
			("users", "create", true),
			("users", "update", true),
			("users", "delete", true),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
		],
	)
	.await?;
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/admin/permission-profiles".to_string(),
		Some(json!({
			"data": {
				"name": "Users Roles Child",
				"privileges": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_pdf_admin_read_and_edit_grant_registered_admin_entitlements(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_settings_admin_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::FORBIDDEN)
		.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/admin/settings",
		StatusCode::FORBIDDEN,
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-empty-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"empty settings role must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "admin",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		value.get("sponsor_admin_capable").is_none(),
		"derived admin summaries must not be exposed: {value:?}"
	);
	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("admin", "read", true),
			("admin", "update", false),
			("users", "read", true),
			("users", "create", false),
			("roles", "read", false),
			("roles", "create", false),
		],
	)
	.await?;
	assert!(
		!has_permission(&profile_id, CASE_CREATE),
		"settings.can_read alone must not grant raw CASE_CREATE permission"
	);
	assert!(
		!has_permission(&profile_id, USER_CREATE),
		"settings.can_read alone must not grant raw USER_CREATE permission"
	);
	assert_get_status(&app, &custom_cookie, "/api/admin/settings", StatusCode::OK)
		.await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		"/api/admin/settings".to_string(),
		Some(json!({
			"data": {
				"idle_session_minutes": 45,
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not update admin settings: {value:?}"
	);
	assert!(has_permission(&profile_id, SETTINGS_READ));
	assert!(!has_permission(&profile_id, SETTINGS_UPDATE));
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SETTINGS-READ-{}", Uuid::new_v4().simple())
				},
				"status": "draft"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create cases via raw permissions: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-read-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_read alone must not create users: {value:?}"
	);

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "admin",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert!(
		value.get("sponsor_admin_capable").is_none(),
		"derived admin summaries must not be exposed: {value:?}"
	);
	assert_profile_access(
		&app,
		&custom_cookie,
		&[
			("admin", "read", true),
			("admin", "update", true),
			("users", "read", true),
			("users", "create", true),
			("users", "update", true),
			("users", "delete", true),
			("roles", "read", false),
			("roles", "create", false),
			("roles", "update", false),
			("roles", "delete", false),
			("settings", "read", true),
			("settings", "update", true),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, SETTINGS_READ));
	assert!(has_permission(&profile_id, SETTINGS_UPDATE));
	assert!(has_permission(&profile_id, USER_CREATE));
	assert!(has_permission(&profile_id, USER_UPDATE));
	assert!(has_permission(&profile_id, USER_DELETE));
	assert_get_status(&app, &custom_cookie, "/api/users", StatusCode::OK).await?;
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		"/api/admin/settings".to_string(),
		Some(json!({
			"data": {
				"idle_session_minutes": 45,
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"settings.can_edit should update admin settings: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/admin/permission-profiles/{ROLE_SYSTEM_ADMIN}"),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"settings.can_edit must not delete roles: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/users".to_string(),
		Some(json!({
			"data": {
				"organization_id": seed.org_id,
				"email": format!("settings-admin-edit-{}@example.com", Uuid::new_v4()),
				"role": "viewer"
			}
		})),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::FORBIDDEN,
		"admin.can_edit grants the entitlement but must not create a privileged administrator identity: {value:?}"
	);

	Ok(())
}

// Gap coverage: home_workflow read privilege must grant effective case-list
#[serial]
#[tokio::test]
async fn test_admin_read_grants_effective_audit_log_access() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let none_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_audit_none_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let read_id = create_empty_custom_role_with_generated_id(
		&app,
		&admin_cookie,
		&format!("qa_audit_read_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let (_none_user, none_cookie) =
		custom_role_user(&mm, seed.org_id, &none_id).await?;
	let (_read_user, read_cookie) =
		custom_role_user(&mm, seed.org_id, &read_id).await?;

	// Unchecked: no audit access.
	assert!(!has_permission(&none_id, AUDIT_LIST));
	assert_get_status(&app, &none_cookie, "/api/audit-logs", StatusCode::FORBIDDEN)
		.await?;

	// PDF ADMIN Read includes the registered audit-read entitlement.
	update_role_privileges(
		&app,
		&admin_cookie,
		&read_id,
		json!([{
			"menu_key": "admin",
			"can_read": true,
			"can_edit": false,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	assert!(has_permission(&read_id, AUDIT_READ));
	assert!(has_permission(&read_id, AUDIT_LIST));
	assert_get_not_status(
		&app,
		&read_cookie,
		"/api/audit-logs",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}
#[serial]
#[tokio::test]
async fn test_organization_management_requires_system_admin() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	// seed.admin is a sponsor admin (ROLE_SPONSOR_ADMIN_CRO), not a system admin.
	assert_get_status(
		&app,
		&admin_cookie,
		"/api/organizations",
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}
