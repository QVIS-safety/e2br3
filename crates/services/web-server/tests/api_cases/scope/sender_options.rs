use super::support::*;

#[serial]
#[tokio::test]
async fn test_routing_profile_sender_options_include_info_sender_masters(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Client A Sender Master",
		"SEND-MASTER-A",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
	)
	.await?;

	let (status, admin_profile) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{admin_profile:?}");
	let admin_senders = admin_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing admin senders")?;
	let admin_master = admin_senders
		.iter()
		.find(|row| row["senderIdentifier"] == sender_id.to_string())
		.ok_or("INFO sender master missing from admin routing options")?;
	assert_eq!(admin_master["caseCount"], 0);

	let (status, viewer_profile) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{viewer_profile:?}");
	let viewer_senders = viewer_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing viewer senders")?;
	assert_eq!(viewer_senders.len(), 1);
	assert_eq!(viewer_senders[0]["senderIdentifier"], sender_id.to_string());
	assert_eq!(viewer_senders[0]["caseCount"], 0);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_unset_sender_scope_lists_all_sender_presaves() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let info_user = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("viewpwd"),
	)
	.await?;
	let viewer_token = generate_web_token(&info_user.email, info_user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Unset Scope Sender Master",
		"SEND-UNSET-SCOPE",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/presaves/senders".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("missing sender rows")?
		.iter()
		.filter_map(|row| row["rows"]["sender"].as_object())
		.collect::<Vec<_>>();
	assert!(
		rows.iter().any(|row| row["id"] == sender_id.to_string()),
		"unset sender scope must list all sender presaves: {value:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_uuid_scope_lists_matching_sender_presave() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let info_user = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("viewpwd"),
	)
	.await?;
	let viewer_token = generate_web_token(&info_user.email, info_user.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	let allowed_id = create_sender_presave(
		&app,
		&admin_cookie,
		"UUID Scope Sender A",
		"SEND-UUID-A",
	)
	.await?;
	let denied_id = create_sender_presave(
		&app,
		&admin_cookie,
		"UUID Scope Sender B",
		"SEND-UUID-B",
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		info_user.id,
		json!({ "access_sender_ids": [allowed_id.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/presaves/senders".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let rows = value["data"]
		.as_array()
		.ok_or("missing sender rows")?
		.iter()
		.filter_map(|row| row["rows"]["sender"].as_object())
		.collect::<Vec<_>>();
	assert!(rows.iter().any(|row| row["id"] == allowed_id.to_string()));
	assert!(!rows.iter().any(|row| row["id"] == denied_id.to_string()));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_routing_profile_sender_options_follow_role_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-ROUTE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-ROUTE-B")
			.await?;

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ROUTE-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ROUTE-B-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_a, "SEND-A").await?;
	create_message_header(&app, &admin_cookie, case_b, "SEND-B").await?;
	create_sender_information(&app, &admin_cookie, case_a, "Sender Org A").await?;
	create_sender_information(&app, &admin_cookie, case_b, "Sender Org B").await?;
	link_case_presave_sources(
		&mm,
		case_a,
		seed.admin.id,
		seed.org_id,
		Some(sender_a),
		None,
		None,
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_b,
		seed.admin.id,
		seed.org_id,
		Some(sender_b),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_a.to_string()] }),
	)
	.await?;

	let (status, admin_profile) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{admin_profile:?}");
	let admin_senders = admin_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing admin senders")?;
	assert!(admin_senders
		.iter()
		.any(|row| row["senderIdentifier"] == sender_a.to_string()));
	assert!(admin_senders
		.iter()
		.any(|row| row["senderIdentifier"] == sender_b.to_string()));

	let (status, viewer_profile) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{viewer_profile:?}");
	let viewer_senders = viewer_profile["data"]["availableSenders"]
		.as_array()
		.ok_or("missing viewer senders")?;
	assert_eq!(viewer_senders.len(), 1);
	assert_eq!(viewer_senders[0]["senderIdentifier"], sender_a.to_string());

	Ok(())
}
