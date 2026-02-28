use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn create_case(
	app: &axum::Router,
	cookie: &str,
	org_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create case status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn create_drug(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Test Drug"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/drugs"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create drug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

async fn create_patient(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let body = json!({
		"data": {
			"case_id": case_id,
			"patient_initials": "AB",
			"sex": "1"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/patient"))
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create patient status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}
	let value: serde_json::Value = serde_json::from_slice(&body)?;
	let id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing data.id")?;
	Ok(Uuid::parse_str(id)?)
}

#[serial]
#[tokio::test]
async fn test_admin_can_create_drug_active_substance() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let cookie = cookie_header(&token.to_string());

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let body = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"substance_name": "Substance A"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances"
		))
		.header("cookie", &cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::CREATED);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_create_drug_active_substance() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &admin_cookie, case_id).await?;

	let body = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"substance_name": "Substance B"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances"
		))
		.header("cookie", &viewer_cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_create_medical_history() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());

	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let patient_id = create_patient(&app, &admin_cookie, case_id).await?;

	let body = json!({
		"data": {
			"patient_id": patient_id,
			"sequence_number": 1,
			"meddra_code": "10000001"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri(format!("/api/cases/{case_id}/patient/medical-history"))
		.header("cookie", &viewer_cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}
