use super::support::*;

#[serial]
#[tokio::test]
async fn test_info_matrix_privileges_grant_effective_presave_permissions(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let profile_id = format!("qa_info_matrix_{}", Uuid::new_v4().simple());

	let profile_id =
		create_empty_custom_role(&app, &admin_cookie, &profile_id).await?;
	let (custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;
	let seed_sender_name = format!("Info Matrix Seed {}", Uuid::new_v4().simple());
	let editable_sender_name =
		format!("Info Matrix Editable {}", Uuid::new_v4().simple());
	let deletable_sender_name =
		format!("Info Matrix Deletable {}", Uuid::new_v4().simple());
	let template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&seed_sender_name,
		"INFO-MATRIX-SEED",
	)
	.await?;
	let editable_template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&editable_sender_name,
		"INFO-MATRIX-EDITABLE",
	)
	.await?;
	let deletable_template_id = create_sender_presave(
		&app,
		&admin_cookie,
		&deletable_sender_name,
		"INFO-MATRIX-DELETABLE",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		custom_user_id,
		json!({
			"access_sender_ids": [
				template_id.to_string(),
				editable_template_id.to_string(),
				deletable_template_id.to_string()
			]
		}),
	)
	.await?;

	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presaves/senders",
		StatusCode::FORBIDDEN,
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
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
			("info", "read", true),
			("info", "create", false),
			("info", "update", false),
			("info", "delete", false),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_READ));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_LIST));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_CREATE));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_UPDATE));
	assert!(!has_permission(&profile_id, PRESAVE_TEMPLATE_DELETE));
	assert_get_status(
		&app,
		&custom_cookie,
		"/api/presaves/senders",
		StatusCode::OK,
	)
	.await?;
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presaves/senders/{template_id}"),
		StatusCode::OK,
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presaves/senders".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"sender_type": "2",
				"organization_name": "Info Matrix Sender",
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presaves/senders/{editable_template_id}"),
		Some(json!({
			"data": {
				"organization_name": "Info Matrix Readonly Patch"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presaves/senders/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "info",
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
			("info", "read", true),
			("info", "create", true),
			("info", "update", true),
			("info", "delete", true),
		],
	)
	.await?;
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_CREATE));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_UPDATE));
	assert!(has_permission(&profile_id, PRESAVE_TEMPLATE_DELETE));

	let (status, value) = request_json(
		&app,
		"POST",
		&custom_cookie,
		"/api/presaves/senders".to_string(),
		Some(json!({
			"data": {
				"authority": "fda",
				"sender_type": "2",
				"organization_name": "INFO-MATRIX-EDIT",
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let created_template_id = extract_id(&value)?;

	let (status, value) = request_json(
		&app,
		"PATCH",
		&custom_cookie,
		format!("/api/presaves/senders/{editable_template_id}"),
		Some(json!({
			"data": {
				"sender_type": "2",
				"organization_name": editable_sender_name,
				"person_given_name": "Safety"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert!(value["data"].get("name").is_none(), "{value:?}");
	assert!(value["data"].get("comments").is_none(), "{value:?}");
	assert_eq!(
		value["data"]["organization_name"].as_str(),
		Some(editable_sender_name.as_str()),
		"{value:?}"
	);

	let (status, value) = request_json(
		&app,
		"DELETE",
		&custom_cookie,
		format!("/api/presaves/senders/{deletable_template_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert_get_status(
		&app,
		&custom_cookie,
		&format!("/api/presaves/senders/{created_template_id}"),
		StatusCode::FORBIDDEN,
	)
	.await?;

	Ok(())
}
