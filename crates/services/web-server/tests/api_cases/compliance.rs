use crate::case_support::*;

#[serial]
#[tokio::test]
async fn test_nullification_code_marks_case_nullified() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "1",
				"nullification_reason": "Duplicate report"
			},
			"reason_for_change": "nullify duplicate case",
			"e_signature": {
				"meaning": "nullify case",
				"password": "adminpwd"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (case_status,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (String,)>("SELECT status FROM cases WHERE id = $1")
				.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert_eq!(case_status.as_str(), "nullified");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_amendment_code_does_not_require_nullification_compliance() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "2",
				"nullification_reason": "Follow-up correction"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let (status, body) = get_case(&app, &cookie, case_id).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["status"].as_str(), Some("draft"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_nullification_code_requires_compliance_payload() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let (status, body) = update_safety_report(
		&app,
		&cookie,
		case_id,
		json!({
			"data": {
				"nullification_code": "1",
				"nullification_reason": "Duplicate report"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().contains("reason_for_change is required"),
		"{body:?}"
	);
	Ok(())
}
