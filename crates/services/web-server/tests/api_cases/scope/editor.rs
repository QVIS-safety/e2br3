use super::support::*;

#[serial]
#[tokio::test]
async fn test_case_editor_can_create_assigned_dg_but_denies_unassigned_product(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_scoped_dg_editor_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([{
			"menu_key": "case",
			"can_read": true,
			"can_edit": true,
			"can_review": false,
			"can_lock": false
		}]),
	)
	.await?;
	let (editor_id, editor_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	let sender_id =
		create_sender_presave(&app, &admin_cookie, "DG Sender", "DG-SCOPE").await?;
	let assigned_product =
		create_product_presave(&app, &admin_cookie, sender_id, "Assigned Drug")
			.await?;
	let unassigned_product =
		create_product_presave(&app, &admin_cookie, sender_id, "Unassigned Drug")
			.await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let assigned_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(assigned_product),
		)
		.await?
		.0;
	let unassigned_product_key = dbx
		.fetch_one(
			sqlx::query_as::<_, (String,)>(
				"SELECT product_id FROM product_presaves WHERE id = $1",
			)
			.bind(unassigned_product),
		)
		.await?
		.0;
	dbx.commit_txn().await?;

	let assigned_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-ASSIGNED-{}", Uuid::new_v4()),
		Some(&assigned_product_key),
	)
	.await?;
	let unassigned_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-UNASSIGNED-{}", Uuid::new_v4()),
		Some(&unassigned_product_key),
	)
	.await?;
	let initially_unscoped_case = create_case(
		&app,
		&admin_cookie,
		&format!("SR-DG-UNSCOPED-{}", Uuid::new_v4()),
		None,
	)
	.await?;
	update_user_scope(
		&app,
		&admin_cookie,
		editor_id,
		json!({
			"access_sender_ids": [sender_id.to_string()],
			"access_product_ids": [assigned_product.to_string()]
		}),
	)
	.await?;

	let editor_case = create_case(
		&app,
		&editor_cookie,
		&format!("SR-DG-EDITOR-{}", Uuid::new_v4()),
		Some(&assigned_product_key),
	)
	.await?;
	let (status, value) = request_json(
		&app,
		"GET",
		&editor_cookie,
		format!("/api/cases/{editor_case}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		"/api/cases".to_string(),
		Some(json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-DG-DENIED-{}", Uuid::new_v4())
				},
				"status": "draft",
				"dgPrdKey": unassigned_product_key
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let dg_row = |source_product_presave_id: Uuid, medicinal_product: &str| {
		json!({
			"authorities": ["ich"],
			"rows": { "drug": {
				"sourceProductPresaveId": source_product_presave_id,
				"sequenceNumber": 1,
				"drugCharacterization": "1",
				"medicinalProduct": medicinal_product
			} }
		})
	};
	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{assigned_case}/editor/pages/DG/rows"),
		Some(dg_row(assigned_product, "Assigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let row_id = value["rowId"].as_str().ok_or("missing drug row id")?;
	let (status, saved) = request_json(
		&app,
		"GET",
		&editor_cookie,
		format!("/api/cases/{assigned_case}/editor/pages/DG/rows/{row_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved:?}");
	assert_eq!(
		saved["data"]["drug"]["source_product_presave_id"],
		json!(assigned_product),
		"{saved:?}"
	);

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{unassigned_case}/editor/pages/DG/rows"),
		Some(dg_row(unassigned_product, "Unassigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		"POST",
		&editor_cookie,
		format!("/api/cases/{initially_unscoped_case}/editor/pages/DG/rows"),
		Some(dg_row(unassigned_product, "Unassigned Drug")),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	let stored_rows = dbx
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT count(*) FROM drug_information WHERE case_id = $1",
			)
			.bind(initially_unscoped_case),
		)
		.await?
		.0;
	dbx.commit_txn().await?;
	assert_eq!(stored_rows, 0);
	Ok(())
}
