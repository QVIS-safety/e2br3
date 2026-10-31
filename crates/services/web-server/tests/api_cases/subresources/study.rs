use super::support::*;

#[serial]
#[tokio::test]
async fn test_study_create_accepts_study_type_reaction() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"case_id": case_id,
			"study_name": "Study",
			"sponsor_study_number": "S-1",
			"study_type_reaction": "3"
		}
	});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/studies"),
		body,
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"study create status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["study_type_reaction"].as_str(), Some("3"));

	Ok(())
}
