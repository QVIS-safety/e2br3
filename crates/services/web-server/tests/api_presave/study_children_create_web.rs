use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

async fn study_child_counts(
	mm: &lib_core::model::ModelManager,
	organization_id: Uuid,
	user_id: Uuid,
	study_id: Uuid,
) -> Result<(i64, i64, i64)> {
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, user_id).await?;
	set_org_context(&mut tx, organization_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let counts = sqlx::query_as(
		"SELECT
		 (SELECT COUNT(*) FROM study_presave_products WHERE study_presave_id=$1),
		 (SELECT COUNT(*) FROM study_presave_registration_numbers WHERE study_presave_id=$1),
		 (SELECT COUNT(*) FROM study_presave_reporters WHERE study_presave_id=$1)",
	)
	.bind(study_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(counts)
}

async fn assert_insert_audit(
	mm: &lib_core::model::ModelManager,
	table: &str,
	id: Uuid,
) -> Result<()> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM audit_logs
		 WHERE table_name=$1 AND record_id=$2 AND action='CREATE'",
	)
	.bind(table)
	.bind(id)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	assert_eq!(count, 1, "missing CREATE audit for {table}/{id}");
	Ok(())
}

async fn study_child_create_audit_count(
	mm: &lib_core::model::ModelManager,
	organization_id: Uuid,
) -> Result<i64> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let count = sqlx::query_scalar::<_, i64>(
		"SELECT COUNT(*) FROM audit_logs
		 WHERE organization_id=$1 AND action='CREATE'
		 AND table_name = ANY($2)",
	)
	.bind(organization_id)
	.bind(vec![
		"study_presave_products",
		"study_presave_registration_numbers",
		"study_presave_reporters",
	])
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(count)
}

#[serial]
#[tokio::test]
async fn direct_study_child_posts_persist_parented_rows_and_reject_unauthorized_parents(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other = seed_org_with_users(&mm, "otherpwd", "otherviewpwd").await?;
	let admin_cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let viewer_cookie = cookie_header(
		&generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?.to_string(),
	);
	let other_cookie = cookie_header(
		&generate_web_token(&other.admin.email, other.admin.token_salt)?.to_string(),
	);
	let app = web_server::app(mm.clone());
	let product_id = create_product_presave(&mm, seed.org_id, seed.admin.id).await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&admin_cookie,
		product_id,
		"fda",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&admin_cookie,
		format!("Direct child reporter {}", Uuid::new_v4()),
		"Direct child reporter org",
	)
	.await?;
	assert_eq!(
		study_child_counts(&mm, seed.org_id, seed.admin.id, study_id).await?,
		(0, 0, 0)
	);

	let cases = [
		(
			"products",
			json!({"data":{"sequence_number":1,"product_presave_id":product_id,"product_name":"Direct study product"}}),
			"product_name",
			"Direct study product",
			"study_presave_products",
		),
		(
			"registration-numbers",
			json!({"data":{"sequence_number":1,"registration_number":"DIRECT-REG","country_code":"US"}}),
			"registration_number",
			"DIRECT-REG",
			"study_presave_registration_numbers",
		),
		(
			"reporters",
			json!({"data":{"sequence_number":1,"reporter_presave_id":reporter_id,"reporter_organization":"Direct child reporter org"}}),
			"reporter_organization",
			"Direct child reporter org",
			"study_presave_reporters",
		),
	];

	for (path, payload, field, expected, table) in cases {
		let uri = format!("/api/presaves/studies/{study_id}/{path}");
		let created =
			post_json_created(&app, &admin_cookie, uri.clone(), payload).await?;
		let id = data_id(&created)?;
		assert_eq!(created["data"][field], expected);
		assert_eq!(created["data"]["study_presave_id"], study_id.to_string());
		let fresh = get_json_ok(&app, &admin_cookie, format!("{uri}/{id}")).await?;
		assert_eq!(fresh["data"]["id"], id.to_string());
		assert_eq!(fresh["data"][field], expected);
		let list = get_json_ok(&app, &admin_cookie, uri).await?;
		assert_eq!(list["data"].as_array().map(Vec::len), Some(1));
		assert_insert_audit(&mm, table, id).await?;
	}
	assert_eq!(
		study_child_counts(&mm, seed.org_id, seed.admin.id, study_id).await?,
		(1, 1, 1)
	);

	let other_product =
		create_product_presave(&mm, other.org_id, other.admin.id).await?;
	let other_study = create_study_presave_for_product_via_api(
		&app,
		&other_cookie,
		other_product,
		"fda",
	)
	.await?;
	let before =
		study_child_counts(&mm, seed.org_id, seed.admin.id, study_id).await?;
	let before_other =
		study_child_counts(&mm, other.org_id, other.admin.id, other_study).await?;
	let audit_before = study_child_create_audit_count(&mm, seed.org_id).await?;
	let other_audit_before =
		study_child_create_audit_count(&mm, other.org_id).await?;
	for (path, payload) in [
		(
			"products",
			json!({"data":{"sequence_number":99,"product_name":"Must not persist"}}),
		),
		(
			"registration-numbers",
			json!({"data":{"sequence_number":99,"registration_number":"MUST-NOT-PERSIST","country_code":"US"}}),
		),
		(
			"reporters",
			json!({"data":{"sequence_number":99,"reporter_organization":"Must not persist"}}),
		),
	] {
		for (cookie, parent, expected) in [
			(&admin_cookie, Uuid::new_v4(), StatusCode::NOT_FOUND),
			(&admin_cookie, other_study, StatusCode::NOT_FOUND),
			(&viewer_cookie, study_id, StatusCode::FORBIDDEN),
		] {
			let body = expect_json_status(
				&app,
				cookie,
				Method::POST,
				format!("/api/presaves/studies/{parent}/{path}"),
				Some(payload.clone()),
				expected,
			)
			.await?;
			assert!(body["data"]["id"].is_null(), "{body:?}");
		}
	}
	assert_eq!(
		study_child_counts(&mm, seed.org_id, seed.admin.id, study_id).await?,
		before
	);
	assert_eq!(
		study_child_counts(&mm, other.org_id, other.admin.id, other_study).await?,
		before_other
	);
	assert_eq!(
		study_child_create_audit_count(&mm, seed.org_id).await?,
		audit_before
	);
	assert_eq!(
		study_child_create_audit_count(&mm, other.org_id).await?,
		other_audit_before
	);
	Ok(())
}
