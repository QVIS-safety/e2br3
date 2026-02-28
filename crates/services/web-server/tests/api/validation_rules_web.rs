use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::Value;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_admin_can_list_validation_rules() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/validation/rules")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let etag = res
		.headers()
		.get(header::ETAG)
		.and_then(|v| v.to_str().ok())
		.ok_or("missing ETag header")?
		.to_string();
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"validation rules status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let value: Value = serde_json::from_slice(&body)?;
	let rules = value
		.get("data")
		.and_then(Value::as_array)
		.ok_or("missing data array")?;
	assert!(!rules.is_empty(), "expected non-empty validation rule list");

	let has_known = rules.iter().any(|rule| {
		rule.get("code").and_then(Value::as_str) == Some("FDA.C.1.7.1.REQUIRED")
	});
	assert!(has_known, "expected FDA.C.1.7.1.REQUIRED in catalog");

	let required_codes = [
		// Frontend required markers used in section components.
		"ICH.C.1.1.REQUIRED",
		"ICH.C.1.2.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.4.REQUIRED",
		"ICH.C.1.5.REQUIRED",
		"ICH.C.1.7.REQUIRED",
		"FDA.C.1.7.1.REQUIRED",
		"ICH.C.2.r.4.REQUIRED",
		"FDA.C.2.r.2.EMAIL.REQUIRED",
		"ICH.C.3.1.REQUIRED",
		"ICH.C.3.2.REQUIRED",
		"ICH.D.1.REQUIRED",
		"ICH.F.r.2.REQUIRED",
		"ICH.E.i.1.1a.REQUIRED",
		"FDA.E.i.3.2h.REQUIRED",
		"ICH.E.i.7.REQUIRED",
		"ICH.G.k.1.REQUIRED",
		"ICH.G.k.2.2.REQUIRED",
		"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
		"MFDS.G.k.9.i.2.r.1.REQUIRED",
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
		"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
		"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
		"ICH.H.1.REQUIRED",
	];
	for code in required_codes {
		let present = rules
			.iter()
			.any(|rule| rule.get("code").and_then(Value::as_str) == Some(code));
		assert!(present, "expected {code} in validation rules catalog");
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/validation/rules?profile=fda")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::OK {
		return Err(format!(
			"validation rules fda status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: Value = serde_json::from_slice(&body)?;
	let rules = value
		.get("data")
		.and_then(Value::as_array)
		.ok_or("missing data array for profile query")?;
	assert!(!rules.is_empty(), "expected non-empty filtered rule list");
	let contains_mfds = rules
		.iter()
		.any(|rule| rule.get("profile").and_then(Value::as_str) == Some("mfds"));
	assert!(!contains_mfds, "profile=fda should not include mfds rules");

	let fda_contains_create_gate_codes = [
		"ICH.C.1.1.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.4.REQUIRED",
		"ICH.C.1.5.REQUIRED",
	];
	for code in fda_contains_create_gate_codes {
		let present = rules
			.iter()
			.any(|rule| rule.get("code").and_then(Value::as_str) == Some(code));
		assert!(
			present,
			"profile=fda must include inherited ICH required code {code}"
		);
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/validation/rules")
		.header("cookie", cookie_header(&token.to_string()))
		.header(header::IF_NONE_MATCH, etag)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::NOT_MODIFIED);

	Ok(())
}
