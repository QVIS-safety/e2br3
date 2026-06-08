#![allow(unused_imports)]

use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

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
