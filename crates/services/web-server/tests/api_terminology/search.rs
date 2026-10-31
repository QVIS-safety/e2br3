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

async fn get_terminology(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, serde_json::Value)> {
	let res = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri(uri)
				.header("cookie", cookie)
				.body(Body::empty())?,
		)
		.await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

#[serial]
#[tokio::test]
async fn terminology_reference_searches_enforce_boundaries_filter_and_order(
) -> Result<()> {
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (status, body) = get_terminology(
		&app,
		&cookie,
		"/api/terminology/fda-code-search?list_name=test_device_problem&q=B&limit=10",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"].as_array().map(Vec::len), Some(0));
	let (status, body) = get_terminology(
		&app,
		&cookie,
		"/api/terminology/fda-code-search?list_name=test_device_problem&q=%20Boundary%20&limit=10",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"].as_array().ok_or("missing FDA rows")?;
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0]["ncit_concept_code"], "TEST-CODE-1");
	assert_eq!(rows[1]["ncit_concept_code"], "TEST-CODE-2");
	assert!(rows
		.iter()
		.all(|row| row["list_name"] == "test_device_problem"));

	let (status, body) = get_terminology(
		&app,
		&cookie,
		"/api/terminology/mfds-substances?q=M&limit=10",
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	for (code, present) in [("M900001", true), ("M900002", true), ("M900003", false)]
	{
		let (status, body) = get_terminology(
			&app,
			&cookie,
			&format!("/api/terminology/mfds-substances?q={code}&limit=10"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body:?}");
		let rows = body["data"].as_array().ok_or("missing MFDS rows")?;
		assert_eq!(
			rows.iter().any(|row| row["substance_code"] == code),
			present,
			"{body:?}"
		);
	}

	let (status, body) =
		get_terminology(&app, &cookie, "/api/terminology/ucum-units").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"].as_array().ok_or("missing UCUM rows")?;
	let test_codes = rows
		.iter()
		.filter(|row| row["unit_type"] == "zz-test")
		.map(|row| row["code"].as_str())
		.collect::<Vec<_>>();
	assert_eq!(test_codes, vec![Some("{test-a}"), Some("{test-b}")]);
	assert!(!body.to_string().contains("{test-hidden}"));
	Ok(())
}
