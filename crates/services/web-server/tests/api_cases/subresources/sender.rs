use super::support::*;

#[serial]
#[tokio::test]
async fn test_sender_information_supports_mfds_health_professional_type_kr1(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	create_safety_report(&app, &cookie, case_id).await?;

	let body = json!({"data": {
		"case_id": case_id,
		"sender_type": "3",
		"organization_name": "KR Sender",
		"health_professional_type_kr1": "4"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let sender_id = extract_id(&body)?;

	let body = json!({"data": {
		"sender_type": "3",
		"organization_name": "KR Sender",
		"health_professional_type_kr1": "2"
	}});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
		body,
		"update sender KR.1",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["health_professional_type_kr1"], "2");

	Ok(())
}
