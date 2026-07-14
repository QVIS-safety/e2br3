use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
async fn test_presave_rest_rejects_deleting_referenced_parent() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm);

	let sender_id =
		create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
	let product_id = create_named_product_presave_for_sender_via_api(
		&app,
		&admin_cookie,
		sender_id,
		format!("REST Referenced Product {}", Uuid::new_v4()),
		"Referenced Product",
	)
	.await?;
	let _study_id = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_id,
		"fda",
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::DELETE,
		format!("/api/presaves/senders/{sender_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(
		value.to_string().contains("sender presave is in use"),
		"unexpected sender delete conflict body: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/senders/{sender_id}"),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(value.to_string().contains("sender presave is in use"));

	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::DELETE,
		format!("/api/presaves/products/{product_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(
		value.to_string().contains("product presave is in use"),
		"unexpected product delete conflict body: {value:?}"
	);
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PATCH,
		format!("/api/presaves/products/{product_id}"),
		Some(json!({ "data": { "deleted": true } })),
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(value.to_string().contains("product presave is in use"));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_canonical_product_parent_soft_delete_allows_editor_with_unset_scope(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());
	let (editor_id, editor_cookie) =
		create_info_editor(&app, &mm, &admin_cookie, seed.org_id).await?;
	let (status, value) = request_json(
		&app,
		&admin_cookie,
		Method::PUT,
		format!("/api/users/{editor_id}"),
		Some(json!({
			"data": {
				"access_product_ids": []
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let patch_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PATCH,
		format!("/api/presaves/products/{patch_id}"),
		Some(json!({
			"data": {
				"deleted": true
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	let details_id =
		create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
	let (status, value) = request_json(
		&app,
		&editor_cookie,
		Method::PUT,
		format!("/api/presaves/products/{details_id}/details"),
		Some(json!({
			"data": {
				"parent": {
					"deleted": true
				}
			}
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}
