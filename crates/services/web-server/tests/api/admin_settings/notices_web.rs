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
