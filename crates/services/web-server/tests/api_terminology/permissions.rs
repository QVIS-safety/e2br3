use super::support::*;

#[serial]
#[tokio::test]
async fn test_viewer_can_read_terminology_but_cannot_import() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/mfds-products?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/countries")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert!(payload["data"].as_array().is_some_and(|countries| countries
		.iter()
		.any(|country| country["code"] == "KR")));

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", cookie)
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_operational_user_can_list_and_search_active_meddra() -> Result<()> {
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_org_with_all_roles(&mm).await?;

	let token = generate_web_token(&seed.user.email, seed.user.token_salt)?;
	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/releases?dictionary=meddra")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["version"], "28.1");
	assert_eq!(payload["data"][0]["status"], "active");

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=10019211&limit=5&version=28.1&language=en")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["code"], "10019211");
	assert_eq!(payload["data"][0]["term"], "Headache");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_approve_activate_or_rollback_release() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for uri in [
		"/api/terminology/releases/whodrug/2025.09/approve?language=en",
		"/api/terminology/releases/whodrug/2025.09/activate?language=en",
		"/api/terminology/releases/whodrug/2025.08/rollback?language=en",
	] {
		let req = Request::builder()
			.method("POST")
			.uri(uri)
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::FORBIDDEN);
	}

	Ok(())
}
