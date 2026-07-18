//! Reference privilege model: Review/Lock exist only as CASE menu rows
//! (CASE|Review|Edit, CASE|Lock|Edit) plus reserved E-mail subscription rows.
//! These tests pin (1) normalization of review/lock flags to the case key,
//! (2) the dedicated CASE_APPROVE / CASE_LOCK grants, and (3) enforcement of
//! reviewed/validated/locked case-status transitions.

use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::model::acs::{
	has_permission, CASE_APPROVE, CASE_LOCK, CASE_UPDATE, SETTINGS_UPDATE,
	TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT, USER_CREATE, USER_DELETE, USER_UPDATE,
};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

async fn update_case_status(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
	status: &str,
) -> Result<(StatusCode, serde_json::Value)> {
	request_json(
		app,
		"PUT",
		cookie,
		format!("/api/cases/{case_id}"),
		Some(json!({ "data": { "status": status } })),
	)
	.await
}

#[serial]
#[tokio::test]
async fn test_review_lock_flags_are_normalized_to_case_menu_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_review_lock_norm_{}", Uuid::new_v4().simple()),
	)
	.await?;

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": true
			},
			{
				"menu_key": "case_workflow",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": true
			},
			{
				"menu_key": "users",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": true
			},
			{
				"menu_key": "settings",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": true
			},
			{
				"menu_key": "data",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": true
			}
		]),
	)
	.await?;

	let privileges = value["privileges"]
		.as_array()
		.ok_or("privileges should be an array")?;
	for row in privileges {
		let menu_key = row["menu_key"].as_str().unwrap_or_default();
		let expected_review_lock = menu_key == "case";
		assert_eq!(
			row["can_review"].as_bool(),
			Some(expected_review_lock),
			"{menu_key} can_review"
		);
		assert_eq!(
			row["can_lock"].as_bool(),
			Some(expected_review_lock),
			"{menu_key} can_lock"
		);
	}

	// The normalized-away flags must not leak edit-grade permissions.
	assert!(has_permission(&profile_id, CASE_APPROVE));
	assert!(has_permission(&profile_id, CASE_LOCK));
	assert!(!has_permission(&profile_id, CASE_UPDATE));
	assert!(!has_permission(&profile_id, USER_CREATE));
	assert!(!has_permission(&profile_id, USER_UPDATE));
	assert!(!has_permission(&profile_id, USER_DELETE));
	assert!(!has_permission(&profile_id, SETTINGS_UPDATE));
	assert!(!has_permission(&profile_id, TERMINOLOGY_IMPORT));
	assert!(!has_permission(&profile_id, TERMINOLOGY_APPROVE));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_email_subscription_menu_keys_persist_and_grant_nothing() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_email_rows_{}", Uuid::new_v4().simple()),
	)
	.await?;

	let value = update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "email_report_due",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			},
			{
				"menu_key": "email_review",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			},
			{
				"menu_key": "email_lock",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;

	let privileges = value["privileges"]
		.as_array()
		.ok_or("privileges should be an array")?;
	for menu_key in ["email_report_due", "email_review", "email_lock"] {
		let row = privileges
			.iter()
			.find(|row| row["menu_key"] == menu_key)
			.ok_or_else(|| format!("missing persisted e-mail row {menu_key}"))?;
		assert_eq!(row["can_read"].as_bool(), Some(true), "{menu_key}");
	}

	// Subscription rows are reserved: they grant no operational permissions.
	assert!(!has_permission(&profile_id, CASE_UPDATE));
	assert!(!has_permission(&profile_id, CASE_APPROVE));
	assert!(!has_permission(&profile_id, CASE_LOCK));
	assert!(!has_permission(&profile_id, USER_CREATE));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_review_and_lock_profile_permissions_are_distinct() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	let profile_id = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_review_caps_{}", Uuid::new_v4().simple()),
	)
	.await?;
	let (_custom_user_id, custom_cookie) =
		custom_role_user(&mm, seed.org_id, &profile_id).await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": false
			}
		]),
	)
	.await?;
	assert_profile_permissions(
		&app,
		&custom_cookie,
		&["Case.Read", "Case.Approve"],
		&["Case.Update", "Case.Lock"],
	)
	.await?;

	update_role_privileges(
		&app,
		&admin_cookie,
		&profile_id,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": true
			}
		]),
	)
	.await?;
	assert_profile_permissions(
		&app,
		&custom_cookie,
		&["Case.Read", "Case.Lock"],
		&["Case.Update", "Case.Approve"],
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reviewed_and_locked_status_transitions_require_dedicated_privileges(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	// Editor: case read+edit, no review/lock.
	let editor_profile = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_case_editor_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&editor_profile,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": true,
				"can_review": false,
				"can_lock": false
			}
		]),
	)
	.await?;
	let (_editor_id, editor_cookie) =
		custom_role_user(&mm, seed.org_id, &editor_profile).await?;

	// Reviewer: case read+review, no edit/lock.
	let reviewer_profile = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_case_reviewer_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&reviewer_profile,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": true,
				"can_lock": false
			}
		]),
	)
	.await?;
	let (_reviewer_id, reviewer_cookie) =
		custom_role_user(&mm, seed.org_id, &reviewer_profile).await?;

	// Locker: case read+lock, no edit/review.
	let locker_profile = create_empty_custom_role(
		&app,
		&admin_cookie,
		&format!("qa_case_locker_{}", Uuid::new_v4().simple()),
	)
	.await?;
	update_role_privileges(
		&app,
		&admin_cookie,
		&locker_profile,
		json!([
			{
				"menu_key": "case",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": true
			}
		]),
	)
	.await?;
	let (_locker_id, locker_cookie) =
		custom_role_user(&mm, seed.org_id, &locker_profile).await?;

	let case_id = create_case(
		&app,
		&admin_cookie,
		&format!("QA-REVLOCK-{}", Uuid::new_v4().simple()),
		None,
	)
	.await?;

	// Edit-only cannot enter reviewed; reviewer can.
	let (status, value) =
		update_case_status(&app, &editor_cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let (status, value) =
		update_case_status(&app, &reviewer_cookie, case_id, "reviewed").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// Reviewer can also move to validated (review-grade QC step).
	let (status, value) =
		update_case_status(&app, &reviewer_cookie, case_id, "validated").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// Neither editor nor reviewer can lock; locker can.
	let (status, value) =
		update_case_status(&app, &editor_cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let (status, value) =
		update_case_status(&app, &reviewer_cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let (status, value) =
		update_case_status(&app, &locker_cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// Unlock (locked -> validated) is a lock-grade action too.
	let (status, value) =
		update_case_status(&app, &reviewer_cookie, case_id, "validated").await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
	let (status, value) =
		update_case_status(&app, &locker_cookie, case_id, "validated").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	// Reviewer without edit cannot touch non-status fields.
	let (status, value) = request_json(
		&app,
		"PUT",
		&reviewer_cookie,
		format!("/api/cases/{case_id}"),
		Some(json!({ "data": { "dg_prd_key": "PRD-REVLOCK" } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");

	// Sponsor admin keeps full review/lock control.
	let (status, value) =
		update_case_status(&app, &admin_cookie, case_id, "locked").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");
	let (status, value) =
		update_case_status(&app, &admin_cookie, case_id, "validated").await?;
	assert_eq!(status, StatusCode::OK, "{value:?}");

	Ok(())
}
