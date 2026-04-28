use crate::common::{
	cookie_header, init_test_env, init_test_mm, seed_org_with_users, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tower::ServiceExt;

fn workspace_root() -> std::path::PathBuf {
	std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn fixture_xml(filename: &str) -> Result<String> {
	Ok(std::fs::read_to_string(
		workspace_root().join("docs/refs/instances").join(filename),
	)?)
}

async fn import_xml_string(
	app: &axum::Router,
	cookie: &str,
	filename: &str,
	xml: &str,
) -> Result<()> {
	let boundary = "X-BOUNDARY-IMPORT-HISTORY";
	let body = format!(
		"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n"
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/import/xml")
		.header(
			"content-type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("cookie", cookie)
		.body(Body::from(body))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"import status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	Ok(())
}

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

#[serial]
#[tokio::test]
async fn test_import_history_uploaded_at_is_rfc3339() -> Result<()> {
	init_test_env().await;
	std::env::set_var("E2BR3_SKIP_XML_VALIDATE", "1");

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let xml = fixture_xml("FAERS2022Scenario1.xml")?;
	import_xml_string(&app, &cookie, "FAERS2022Scenario1.xml", &xml).await?;

	let (status, body) = get_json(&app, &cookie, "/api/import/xml/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let item = body["data"]["items"]
		.as_array()
		.and_then(|items| items.first())
		.cloned()
		.ok_or_else(|| format!("missing history items in response: {body}"))?;
	let uploaded_at = item["uploadedAt"]
		.as_str()
		.ok_or_else(|| format!("uploadedAt must be a string: {item}"))?;

	let parsed = OffsetDateTime::parse(uploaded_at, &Rfc3339).map_err(|err| {
		format!("uploadedAt was not RFC3339: {uploaded_at} ({err})")
	})?;
	assert!(parsed.unix_timestamp() > 0, "{item:?}");

	Ok(())
}
