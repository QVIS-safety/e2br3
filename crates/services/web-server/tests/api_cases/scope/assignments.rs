use super::support::*;

#[serial]
#[tokio::test]
async fn test_user_scope_assignment_rejects_cross_sender_product() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Scope Sender A", "SCOPE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Scope Sender B", "SCOPE-B")
			.await?;
	let product_b =
		create_product_presave(&app, &admin_cookie, sender_b, "Scope Product B")
			.await?;
	let study_b =
		create_study_presave(&app, &admin_cookie, product_b, "SCOPE-STUDY-B")
			.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_a.to_string()],
				"access_product_ids": [product_b.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(value.to_string().contains("access_product_ids"));

	let (status, value) = request_json(
		&app,
		"PUT",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_a.to_string()],
				"access_study_ids": [study_b.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	assert!(value.to_string().contains("access_study_ids"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_scope_empty_array_clears_existing_assignment() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Clear Sender", "CLEAR").await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&admin_cookie,
		format!("/api/users/{}", seed.viewer.id),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["scope"]["assignedSenderIds"], json!([]));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_does_not_treat_e2b_blinded_field_as_user_scope() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-BLIND")
			.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Demo Product")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-BLIND").await?;

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-BLIND-{}", Uuid::new_v4()),
		Some("PROD-BLIND"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-A").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org A").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-BLIND").await?;
	create_drug(&app, &admin_cookie, case_id, true).await?;
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_id),
		Some(study_id),
	)
	.await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_id.to_string()],
			"access_study_ids": [study_id.to_string()],
			"access_blind_allowed": false
		}),
	)
	.await?;

	let (status, _value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_company_sponsor_admin_can_assign_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_company_org_with_users(&mm, "companypwd", "viewpwd").await?;
	let company_admin = seed.admin;
	let company_token =
		generate_web_token(&company_admin.email, company_admin.token_salt)?;
	let company_cookie = cookie_header(&company_token.to_string());
	let app = web_server::app(mm);
	let sender_id = create_sender_presave(
		&app,
		&company_cookie,
		"Company Sender Scope",
		"COMPANY-SCOPE",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&company_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({
			"data": {
				"access_sender_ids": [sender_id.to_string()]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}
