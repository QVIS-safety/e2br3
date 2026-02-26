mod common;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use common::{
	cookie_header, init_test_mm, seed_org_with_users, system_org_id, Result,
};
use hmac::{Hmac, Mac};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_ADMIN};
use lib_core::model::store::set_full_context_dbx;
use lib_utils::b64::{b64u_decode, b64u_encode};
use serde_json::{json, Value};
use serial_test::serial;
use sha2::Sha512;
use sqlx::query;
use tower::ServiceExt;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_auth_login_refresh_logoff() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_email = seed.admin.email.clone();
	let admin_salt = seed.admin.token_salt;

	let app = web_server::app(mm.clone());
	let login_body = json!({ "email": admin_email, "pwd": "adminpwd" });
	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	if status != StatusCode::OK {
		let err = res
			.extensions()
			.get::<std::sync::Arc<lib_web::Error>>()
			.map(|e| format!("{e:?}"));
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"login status {} body {} err {:?}",
			status,
			String::from_utf8_lossy(&body),
			err
		)
		.into());
	}

	let token = generate_web_token(&admin_email, admin_salt)?;
	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/refresh")
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	if status != StatusCode::OK {
		let err = res
			.extensions()
			.get::<std::sync::Arc<lib_web::Error>>()
			.map(|e| format!("{e:?}"));
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"refresh status {} body {} err {:?}",
			status,
			String::from_utf8_lossy(&body),
			err
		)
		.into());
	}

	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/logoff")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(r#"{"logoff":true}"#))?;
	let res = app.oneshot(req).await?;
	let status = res.status();
	if status != StatusCode::OK {
		let err = res
			.extensions()
			.get::<std::sync::Arc<lib_web::Error>>()
			.map(|e| format!("{e:?}"));
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"logoff status {} body {} err {:?}",
			status,
			String::from_utf8_lossy(&body),
			err
		)
		.into());
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_wrong_password() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let app = web_server::app(mm);

	let login_body = json!({ "email": seed.admin.email, "pwd": "wrongpwd" });
	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_unknown_email() -> Result<()> {
	let mm = init_test_mm().await?;
	let _seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let app = web_server::app(mm);

	let login_body = json!({ "email": "missing@example.com", "pwd": "anypwd" });
	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_user_with_nil_org_fails() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm.clone());
	let suffix = Uuid::new_v4();
	let email = format!("nil-org-login-{suffix}@example.com");

	let create_body = json!({
		"data": {
			"organization_id": system_org_id(),
			"email": email,
			"username": format!("nil_org_login_{suffix}"),
			"pwd_clear": "NilOrgPwd123!",
			"role": "user"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	// Simulate a legacy malformed row where organization_id is nil UUID.
	let create_body = to_bytes(create_res.into_body(), usize::MAX).await?;
	let created: Value = serde_json::from_slice(&create_body)?;
	let created_user_id = Uuid::parse_str(
		created["data"]["id"]
			.as_str()
			.ok_or("missing created user id")?,
	)?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	let root_ctx = Ctx::root_ctx();
	set_full_context_dbx(
		dbx,
		root_ctx.user_id(),
		root_ctx.organization_id(),
		ROLE_ADMIN,
	)
	.await?;
	dbx.execute(
		query("UPDATE users SET organization_id = $1 WHERE id = $2")
			.bind(system_org_id())
			.bind(created_user_id),
	)
	.await?;
	dbx.commit_txn().await?;

	let login_body = json!({ "email": email, "pwd": "NilOrgPwd123!" });
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_created_user_email_case_insensitive() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let mixed_case_email = format!("CaseMix-{suffix}@Example.COM");
	let login_email = mixed_case_email.to_lowercase();

	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": mixed_case_email,
			"username": format!("case_mix_{suffix}"),
			"pwd_clear": "CaseMixPwd123!",
			"role": "user"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	let login_body = json!({ "email": login_email, "pwd": "welcome" });
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_created_user_without_role_uses_default() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm);
	let suffix = Uuid::new_v4();
	let email = format!("role-default-{suffix}@example.com");

	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": format!("role_default_{suffix}"),
			"pwd_clear": "RoleDefaultPwd123!"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	let login_body = json!({ "email": email, "pwd": "welcome" });
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_auth_login_upgrades_legacy_hash_for_non_admin_user() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let app = web_server::app(mm.clone());
	let suffix = Uuid::new_v4();
	let email = format!("legacy-user-{suffix}@example.com");
	let password = "hello world";

	let create_body = json!({
		"data": {
			"organization_id": seed.org_id,
			"email": email,
			"username": format!("legacy_user_{suffix}"),
			"pwd_clear": password,
			"role": "user"
		}
	});
	let create_req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(create_body.to_string()))?;
	let create_res = app.clone().oneshot(create_req).await?;
	assert_eq!(create_res.status(), StatusCode::CREATED);

	// Force legacy scheme-01 hash for this user to exercise upgrade-on-login.
	let legacy_salt = Uuid::parse_str("f05e8961-d6ad-4086-9e78-a6de065e5453")?;
	let pwd_key = std::env::var("SERVICE_PWD_KEY")?;
	let pwd_key = b64u_decode(&pwd_key)?;
	let mut hmac_sha512 = Hmac::<Sha512>::new_from_slice(&pwd_key)?;
	hmac_sha512.update(password.as_bytes());
	hmac_sha512.update(legacy_salt.as_bytes());
	let legacy_hash =
		format!("#01#{}", b64u_encode(hmac_sha512.finalize().into_bytes()));
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	let root_ctx = Ctx::root_ctx();
	set_full_context_dbx(
		dbx,
		root_ctx.user_id(),
		root_ctx.organization_id(),
		ROLE_ADMIN,
	)
	.await?;
	dbx.execute(
		query("UPDATE users SET pwd = $1, pwd_salt = $2 WHERE email = $3")
			.bind(legacy_hash)
			.bind(legacy_salt)
			.bind(&email),
	)
	.await?;
	dbx.commit_txn().await?;

	let login_body = json!({ "email": email, "pwd": password });
	let login_req = Request::builder()
		.method("POST")
		.uri("/auth/v1/login")
		.header("content-type", "application/json")
		.body(Body::from(login_body.to_string()))?;
	let login_res = app.oneshot(login_req).await?;
	assert_eq!(login_res.status(), StatusCode::OK);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_middleware_requires_auth() -> Result<()> {
	let mm = init_test_mm().await?;
	let _seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let app = web_server::app(mm);
	let req = Request::builder()
		.method("GET")
		.uri("/api/users")
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_refresh_rejects_bad_token() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_email = seed.admin.email.clone();
	let admin_salt = seed.admin.token_salt;
	let mut token = generate_web_token(&admin_email, admin_salt)?.to_string();
	token.pop();
	token.push('x');

	let app = web_server::app(mm);
	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/refresh")
		.header("cookie", cookie_header(&token))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_refresh_missing_token() -> Result<()> {
	let mm = init_test_mm().await?;
	let _seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/auth/v1/refresh")
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_permission_denied_for_viewer() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let app = web_server::app(mm);
	let req = Request::builder()
		.method("POST")
		.uri("/api/users")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(
			r#"{"data":{"organization_id":"00000000-0000-0000-0000-000000000001","email":"x@y.com","username":"x","pwd_clear":"x","role":"user"}}"#,
		))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_trail_case_crud() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_id = seed.admin.id;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;

	let app = web_server::app(mm.clone());
	let case_body = json!({
		"data": {
			"organization_id": system_org_id(),
			"safety_report_id": format!("SR-{}", Uuid::new_v4()),
			"status": "draft"
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/cases")
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(case_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::CREATED);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&body)?;
	let case_id = value
		.get("data")
		.and_then(|v| v.get("id"))
		.and_then(|v| v.as_str())
		.ok_or("missing case id")?;

	let update_body = json!({
		"data": {
			"status": "submitted"
		},
		"reason_for_change": "audit test submit transition",
		"e_signature": {
			"meaning": "submit case for audit test",
			"password": "adminpwd"
		}
	});
	let req = Request::builder()
		.method("PUT")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.header("content-type", "application/json")
		.body(Body::from(update_body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("DELETE")
		.uri(format!("/api/cases/{case_id}"))
		.header("cookie", cookie_header(&token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::NO_CONTENT);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	let query = sqlx::query("SET ROLE e2br3_auditor_role");
	dbx.execute(query).await?;
	let query = sqlx::query_as::<_, (i64,)>(
		"SELECT COUNT(*) FROM audit_logs WHERE table_name = 'cases' AND record_id = $1 AND user_id = $2",
	)
	.bind(Uuid::parse_str(case_id)?)
	.bind(admin_id);
	let (count,) = dbx.fetch_one(query).await?;
	dbx.rollback_txn().await?;
	assert!(count >= 3);

	Ok(())
}
