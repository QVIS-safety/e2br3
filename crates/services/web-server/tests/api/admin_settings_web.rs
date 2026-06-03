use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn request_json(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: &str,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice(&bytes)
		.unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
	Ok((status, value))
}

#[serial]
#[tokio::test]
async fn test_idle_session_settings_are_org_scoped_and_validated() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other_seed =
		seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let other_viewer_token =
		generate_web_token(&other_seed.viewer.email, other_seed.viewer.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let other_viewer_cookie = cookie_header(&other_viewer_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 10
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/admin/settings",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 30);
	assert_eq!(value["session_warning_minutes"], 10);

	let (status, value) = request_json(
		&app,
		&other_viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["idle_session_minutes"], 60);
	assert_eq!(value["session_warning_minutes"], 5);

	let (status, _value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 4,
				"session_warning_minutes": 1
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	let (status, _value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"idle_session_minutes": 30,
				"session_warning_minutes": 30
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_settings_appendices_are_supported_and_never_empty() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"appendices": ["FDA", "MFDS", "ICH"]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["appendices"], json!(["ICH", "FDA", "MFDS"]));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"appendices": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["appendices"], json!(["ICH"]));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_settings_audit_trail_records_changed_field() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"timezone": "Asia/Seoul",
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let create_audit = dbx
		.fetch_optional(
			sqlx::query_as::<
				_,
				(
					Uuid,
					String,
					serde_json::Value,
					Option<serde_json::Value>,
					serde_json::Value,
				),
			>(
				r#"
				SELECT user_id, action, changed_fields, old_values, new_values
				FROM audit_logs
				WHERE table_name = 'app_settings'
				  AND organization_id = $1
				  AND record_id = $1
				  AND action = 'CREATE'
				  AND changed_fields ? 'timezone'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(seed.org_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	let (user_id, action, changed_fields, old_values, new_values) =
		create_audit.ok_or("missing app_settings create audit row")?;
	assert_eq!(user_id, seed.admin.id);
	assert_eq!(action, "CREATE");
	assert_eq!(changed_fields["timezone"]["old"], serde_json::Value::Null);
	assert_eq!(changed_fields["timezone"]["new"], json!("Asia/Seoul"));
	assert!(old_values.is_none());
	assert_eq!(new_values["timezone"], json!("Asia/Seoul"));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"timezone": "UTC",
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let audit = dbx
		.fetch_optional(
			sqlx::query_as::<
				_,
				(
					Uuid,
					String,
					serde_json::Value,
					serde_json::Value,
					serde_json::Value,
					String,
				),
			>(
				r#"
				SELECT user_id, action, changed_fields, old_values, new_values, created_at::TEXT
				FROM audit_logs
				WHERE table_name = 'app_settings'
				  AND organization_id = $1
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'timezone'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(seed.org_id),
		)
		.await?;
	dbx.rollback_txn().await?;

	let (user_id, action, changed_fields, old_values, new_values, created_at) =
		audit.ok_or("missing app_settings audit row")?;
	assert_eq!(user_id, seed.admin.id);
	assert_eq!(action, "UPDATE");
	assert_eq!(changed_fields["timezone"]["old"], json!("Asia/Seoul"));
	assert_eq!(changed_fields["timezone"]["new"], json!("UTC"));
	assert_eq!(old_values["timezone"], json!("Asia/Seoul"));
	assert_eq!(new_values["timezone"], json!("UTC"));
	assert!(!created_at.is_empty());

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		"/api/admin/settings",
		Some(json!({
			"data": {
				"timezone": "UTC",
				"session_warning_minutes": 5
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let (count,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				r#"
				SELECT COUNT(*)
				FROM audit_logs
				WHERE table_name = 'app_settings'
				  AND organization_id = $1
				  AND record_id = $1
				  AND action = 'UPDATE'
				  AND changed_fields ? 'timezone'
				"#,
			)
			.bind(seed.org_id),
		)
		.await?;
	dbx.rollback_txn().await?;
	assert_eq!(count, 1);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_dashboard_notices_are_org_scoped_and_audited() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other_seed =
		seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	let sponsor_admin_token =
		generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let other_viewer_token =
		generate_web_token(&other_seed.viewer.email, other_seed.viewer.token_salt)?;
	let sponsor_admin_cookie = cookie_header(&sponsor_admin_token.to_string());
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let other_viewer_cookie = cookie_header(&other_viewer_token.to_string());
	let app = web_server::app(mm.clone());

	let (status, value) = request_json(
		&app,
		&sponsor_admin_cookie,
		Method::PUT,
		"/api/admin/notices",
		Some(json!({
			"data": {
				"notices": [{
					"id": "notice-1",
					"title": "System maintenance",
					"body": "System maintenance at 18:00.",
					"effective_date": "2026-05-14",
					"expire_date": "2026-05-20"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"][0]["title"], "System maintenance");
	assert_eq!(value["notices"][0]["writer"], seed.admin.email);

	let (status, value) = request_json(
		&app,
		&sponsor_admin_cookie,
		Method::PUT,
		"/api/admin/notices",
		Some(json!({
			"data": {
				"notices": [{
					"id": "notice-1",
					"title": "Updated maintenance",
					"body": "System maintenance at 18:00.",
					"effective_date": "2026-05-14",
					"expire_date": "2026-05-20"
				}]
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"][0]["title"], "Updated maintenance");

	let (status, value) = request_json(
		&app,
		&viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"][0]["title"], "Updated maintenance");
	assert_eq!(value["notices"][0]["writer"], seed.admin.email);

	let (status, value) = request_json(
		&app,
		&other_viewer_cookie,
		Method::GET,
		"/api/settings/runtime",
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	assert_eq!(value["notices"].as_array().unwrap().len(), 0);

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	dbx.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let (create_count,) = dbx
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				r#"
				SELECT COUNT(*)
				FROM audit_logs
				WHERE table_name = 'dashboard_notices'
				  AND organization_id = $1
				  AND user_id = $2
				  AND action = 'CREATE'
				"#,
			)
			.bind(seed.org_id)
			.bind(seed.admin.id),
		)
		.await?;
	let update_audit = dbx
		.fetch_optional(
			sqlx::query_as::<
				_,
				(serde_json::Value, serde_json::Value, serde_json::Value),
			>(
				r#"
				SELECT changed_fields, old_values, new_values
				FROM audit_logs
				WHERE table_name = 'dashboard_notices'
				  AND organization_id = $1
				  AND user_id = $2
				  AND action = 'UPDATE'
				  AND changed_fields ? 'title'
				ORDER BY id DESC
				LIMIT 1
				"#,
			)
			.bind(seed.org_id)
			.bind(seed.admin.id),
		)
		.await?;
	dbx.rollback_txn().await?;
	assert_eq!(create_count, 1);
	let (changed_fields, old_values, new_values) =
		update_audit.ok_or("missing dashboard_notices update audit row")?;
	assert_eq!(changed_fields["title"]["old"], json!("System maintenance"));
	assert_eq!(changed_fields["title"]["new"], json!("Updated maintenance"));
	assert_eq!(old_values["title"], json!("System maintenance"));
	assert_eq!(new_values["title"], json!("Updated maintenance"));

	Ok(())
}
