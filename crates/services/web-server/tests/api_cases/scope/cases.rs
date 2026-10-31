use super::support::*;

#[serial]
#[tokio::test]
async fn test_case_list_is_filtered_by_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-A").await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-B").await?;

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-B-{}", Uuid::new_v4()),
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
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	dbx.execute(
		sqlx::query(
			"UPDATE cases SET created_at = NOW() + INTERVAL '1 hour' WHERE id = $1",
		)
		.bind(case_b),
	)
	.await?;
	dbx.commit_txn().await?;

	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_a.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/cases?list_options%5Blimit%5D=1&list_options%5Boffset%5D=0"
			.to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert_eq!(cases.len(), 1, "{value:?}");
	assert_eq!(cases[0]["id"], case_a.to_string());
	assert_eq!(cases[0]["can_act_on_workflow"], false);
	assert_eq!(cases[0]["workflow_block_reason"], "workflow_not_enabled");
	assert!(cases[0]["case_write_block_reason"].is_null());

	let (status, list_view) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		"/api/cases/list-view?list_options%5Blimit%5D=1&list_options%5Boffset%5D=0&list_options%5Border_bys%5D=%21created_at".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{list_view:?}");
	let rows = list_view["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	assert_eq!(rows.len(), 1, "{list_view:?}");
	assert_eq!(rows[0]["caseId"], case_a.to_string());
	assert_eq!(rows[0]["no"], 1);

	let (status, query) = request_json(
		&app,
		"POST",
		&viewer_cookie,
		"/api/cases/query".to_string(),
		Some(json!({
			"conditions": [],
			"resultPages": ["CASE"],
			"limit": 1,
			"offset": 0
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{query:?}");
	assert_eq!(query["data"]["total"], 1);
	assert_eq!(query["data"]["caseIds"][0], case_a.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sender_scope_follows_product_relationship() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Product-linked Sender",
		"SEND-PRODUCT-LINK",
	)
	.await?;
	let product_id = create_product_presave(
		&app,
		&admin_cookie,
		sender_id,
		"Product-linked Brand",
	)
	.await?;
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-PRODUCT-SENDER-{}", Uuid::new_v4()),
		Some("PRODUCT-LINK"),
	)
	.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Product-linked Brand")
		.await?;
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		None,
		Some(product_id),
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_does_not_match_sender_scope_by_message_header_only(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let scoped_sender =
		create_sender_presave(&app, &admin_cookie, "Scoped Sender", "SCOPED")
			.await?;
	let case_sender =
		create_sender_presave(&app, &admin_cookie, "Case Sender", "CASE-SENDER")
			.await?;
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-SENDER-HEADER-ONLY-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "MSG-ONLY").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org B").await?;
	link_case_presave_sources(
		&mm,
		case_id,
		seed.admin.id,
		seed.org_id,
		Some(case_sender),
		None,
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [scoped_sender.to_string()] }),
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
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_case_without_source_when_user_has_sender_scope(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);
	let scoped_sender = create_sender_presave(
		&app,
		&admin_cookie,
		"Scoped Sender",
		"SCOPED-NO-SOURCE",
	)
	.await?;

	// Case carries no sender organization (and no product/study/blind data).
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-NO-SENDER-ORG-{}", Uuid::new_v4()),
		None,
	)
	.await?;

	// Viewer has a sender scope, but the case has no sender org to match against.
	// Sender scope must behave like product/study: an absent case value is allowed,
	// not blocked. (required_scope_matches semantics, not optional_scope_matches.)
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [scoped_sender.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_unset_scope_even_when_case_has_values() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm);

	// Case carries sender org, product brand, and study number.
	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UNSET-ALL-{}", Uuid::new_v4()),
		Some("PROD-UNSET"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-UNSET").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Unset Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Unset").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-UNSET").await?;

	// Viewer has NO scope configured at all. Unset scope means "allow all":
	// the case must be visible even though it carries sender/product/study values.
	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["id"], case_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_update_requires_matching_sender_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-UPD-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-UPD-B")
			.await?;
	let product_a =
		create_product_presave(&app, &admin_cookie, sender_a, "Product A").await?;
	let product_b =
		create_product_presave(&app, &admin_cookie, sender_b, "Product B").await?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let product_b_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(product_b),
		)
		.await?
		.0;
	dbx.commit_txn().await?;

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UPD-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-UPD-B-{}", Uuid::new_v4()),
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
		json!({
			"access_sender_ids": [sender_a.to_string()],
			"access_product_ids": [product_a.to_string()]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		format!("/api/cases/{case_b}"),
		Some(json!({
			"data": {
				"dg_prd_key": "UNAUTHORIZED-PRODUCT-EDIT"
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		format!("/api/cases/{case_a}"),
		Some(json!({ "data": { "dg_prd_key": product_b_key } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let stored_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (Option<String>,)>(
				"SELECT dg_prd_key FROM cases WHERE id = $1",
			)
			.bind(case_a),
		)
		.await?
		.0;
	dbx.commit_txn().await?;
	assert_eq!(stored_product_key, None);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_requires_matching_product_and_study_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-SCOPE")
			.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Alpha")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-ALPHA").await?;

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-SCOPE-{}", Uuid::new_v4()),
		Some("PROD-ALPHA"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-A").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Org A").await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Alpha").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-ALPHA").await?;
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
			"access_study_ids": [study_id.to_string()]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["data"]["id"], case_id.to_string());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_get_allows_empty_product_or_study_scope_but_blocks_mismatch(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id = create_sender_presave(
		&app,
		&admin_cookie,
		"Sender Strict Org",
		"SEND-STRICT",
	)
	.await?;
	let product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Strict")
			.await?;
	let study_id =
		create_study_presave(&app, &admin_cookie, product_id, "STUDY-STRICT")
			.await?;
	let other_product_id =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand Other")
			.await?;

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("SR-STRICT-SCOPE-{}", Uuid::new_v4()),
		Some("PROD-STRICT"),
	)
	.await?;
	create_message_header(&app, &admin_cookie, case_id, "SEND-STRICT").await?;
	create_sender_information(&app, &admin_cookie, case_id, "Sender Strict Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_id, "Brand Strict").await?;
	create_study(&app, &admin_cookie, case_id, "STUDY-STRICT").await?;
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

	// Sender matches; product/study left unset. Unset scope means "allow all",
	// so the case is visible even though it carries product/study values.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({ "access_sender_ids": [sender_id.to_string()] }),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// A product scope that is set but does NOT match the case must still block:
	// filtering only kicks in when the user has an explicit scope value.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [other_product_id.to_string()]
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
	assert_eq!(status, StatusCode::FORBIDDEN);

	// Matching scope on every dimension is allowed.
	update_user_scope(
		&app,
		&admin_cookie,
		seed.viewer.id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [product_id.to_string()],
			"access_study_ids": [study_id.to_string()]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"GET",
		&viewer_cookie,
		format!("/api/cases/{case_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_active_sender_selection_does_not_filter_case_list() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_a =
		create_sender_presave(&app, &admin_cookie, "Sender Org A", "SEND-ACTIVE-A")
			.await?;
	let sender_b =
		create_sender_presave(&app, &admin_cookie, "Sender Org B", "SEND-ACTIVE-B")
			.await?;

	let case_a = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ACTIVE-A-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	let case_b = create_case(
		&app,
		&admin_cookie,
		&format!("SR-ACTIVE-B-{}", Uuid::new_v4()),
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
		json!({
			"access_sender_ids": [sender_a.to_string(), sender_b.to_string()]
		}),
	)
	.await?;

	let (status, value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({ "data": { "sender_id": sender_a.to_string() } })),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) =
		request_json(&app, "GET", &viewer_cookie, "/api/cases".to_string(), None)
			.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let cases = value["data"].as_array().ok_or("missing cases array")?;
	assert!(cases.iter().any(|row| row["id"] == case_a.to_string()));
	assert!(cases.iter().any(|row| row["id"] == case_b.to_string()));

	let (status, _value) = request_json(
		&app,
		"PUT",
		&viewer_cookie,
		"/api/users/me/routing".to_string(),
		Some(json!({
			"data": { "active_sender_identifier": Uuid::new_v4().to_string() }
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	Ok(())
}
