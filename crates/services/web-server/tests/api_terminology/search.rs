use super::support::*;

#[serial]
#[tokio::test]
async fn test_admin_can_access_terminology_endpoints() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for uri in [
		"/api/terminology/meddra?q=test&limit=5",
		"/api/terminology/whodrug?q=test&limit=5",
		"/api/terminology/mfds-products?q=9000000001&limit=5",
		"/api/terminology/mfds-products/9000000001/substances",
		"/api/terminology/countries",
		"/api/terminology/code-lists?list_name=report_type",
	] {
		let req = Request::builder()
			.method("GET")
			.uri(uri)
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		if res.status() != StatusCode::OK {
			let status = res.status();
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			return Err(format!(
				"terminology GET {uri} status {status} body {}",
				String::from_utf8_lossy(&body)
			)
			.into());
		}
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_mfds_product_substances_returns_active_explicit_link() -> Result<()> {
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/mfds-products/9000000001/substances")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let payload: serde_json::Value =
		serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await?)?;
	assert_eq!(payload["data"][0]["substance_code"], "M900001");
	assert_eq!(payload["data"][0]["quantity"], "10");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_receives_active_sender_type_code_list() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/code-lists?list_name=sender_type")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let codes = payload["data"]
		.as_array()
		.expect("sender_type response data must be an array")
		.iter()
		.map(|row| row["code"].as_str().expect("code must be a string"))
		.collect::<Vec<_>>();
	assert_eq!(codes, vec!["1", "2", "3", "4", "5", "6", "7"]);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_searches_only_active_mfds_products_by_code_or_name(
) -> Result<()> {
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for (query, expected_count) in [
		("9000000001", 1),
		("Active%20MFDS", 1),
		("Inactive%20MFDS", 0),
	] {
		let req = Request::builder()
			.method("GET")
			.uri(format!("/api/terminology/mfds-products?q={query}&limit=5"))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let payload: serde_json::Value = serde_json::from_slice(&body)?;
		assert_eq!(
			payload["data"].as_array().map(Vec::len),
			Some(expected_count)
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_can_search_active_meddra_and_whodrug_terms() -> Result<()>
{
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=Headache%20test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["term"], "Headache test term");

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/whodrug?q=Example%20Drug&limit=5")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["drug_name"], "Example Drug test term");

	Ok(())
}
