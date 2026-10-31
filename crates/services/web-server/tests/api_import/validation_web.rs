use super::support::{scenario6, validate_xml_bytes};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serial_test::serial;
use uuid::Uuid;

async fn persisted_counts(
	mm: &lib_core::model::ModelManager,
	organization_id: Uuid,
	user_id: Uuid,
) -> Result<(i64, i64)> {
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, user_id).await?;
	set_org_context(&mut tx, organization_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let counts = sqlx::query_as(
		"SELECT (SELECT COUNT(*) FROM cases),
		        (SELECT COUNT(*) FROM xml_import_history)",
	)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(counts)
}

#[serial]
#[tokio::test]
async fn xml_validation_reports_valid_malformed_and_xsd_invalid_without_writes(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let before = persisted_counts(&mm, seed.org_id, seed.admin.id).await?;

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()?;
	let valid = scenario6(&root)?;
	let (status, body) =
		validate_xml_bytes(&app, &cookie, "valid.xml", valid.as_bytes()).await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["ok"], true, "{body:?}");
	assert_eq!(body["data"]["root_element"], "MCCI_IN200100UV01");
	assert_eq!(body["data"]["errors"].as_array().map(Vec::len), Some(0));

	let (status, body) =
		validate_xml_bytes(&app, &cookie, "malformed.xml", b"<root>").await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
	assert!(
		body.to_string().to_ascii_lowercase().contains("xml"),
		"{body:?}"
	);

	let invalid = valid.replace("MCCI_IN200100UV01", "NOT_E2B_R3");
	let (status, body) =
		validate_xml_bytes(&app, &cookie, "xsd-invalid.xml", invalid.as_bytes())
			.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert_eq!(body["data"]["ok"], false, "{body:?}");
	assert!(
		!body["data"]["errors"]
			.as_array()
			.ok_or("missing errors")?
			.is_empty(),
		"{body:?}"
	);

	assert_eq!(
		persisted_counts(&mm, seed.org_id, seed.admin.id).await?,
		before
	);
	Ok(())
}
