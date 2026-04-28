use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, init_test_mm,
	set_current_user, unique_suffix, Result,
};
use lib_core::ctx::{Ctx, ROLE_ADMIN, ROLE_USER, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use lib_core::model::case::CaseBmc;
use lib_core::model::organization::{OrganizationBmc, OrganizationForCreate};
use lib_core::model::store::{set_full_context_dbx, set_org_context_dbx};
use lib_core::model::user::{UserBmc, UserForCreate};
use lib_core::model::Error as ModelError;
use serial_test::serial;
use sqlx::types::Uuid;

async fn enable_rls(mm: &lib_core::model::ModelManager) -> Result<()> {
	let query = sqlx::query("SET ROLE e2br3_app_role");
	mm.dbx().execute(query).await?;
	let query = sqlx::query("SET row_security = on");
	mm.dbx().execute(query).await?;
	Ok(())
}

fn system_user_id() -> Uuid {
	Uuid::parse_str(SYSTEM_USER_ID).expect("system user id")
}

fn system_org_id() -> Uuid {
	Uuid::parse_str(SYSTEM_ORG_ID).expect("system org id")
}

async fn create_org(mm: &lib_core::model::ModelManager, ctx: &Ctx) -> Result<Uuid> {
	let suffix = unique_suffix();
	let org_c = OrganizationForCreate {
		name: format!("RLS Org {suffix}"),
		org_type: Some("internal".to_string()),
		address: Some("123 RLS St".to_string()),
		contact_email: Some(format!("rls-org-{suffix}@example.com")),
	};
	let org_id = OrganizationBmc::create(ctx, mm, org_c).await?;
	Ok(org_id)
}

async fn create_user(
	mm: &lib_core::model::ModelManager,
	ctx: &Ctx,
	org_id: Uuid,
	role: &str,
) -> Result<Uuid> {
	let suffix = unique_suffix();
	let user_c = UserForCreate {
		organization_id: org_id,
		email: format!("rls-user-{suffix}@example.com"),
		username: Some(format!("rls_user_{suffix}")),
		pwd_clear: "pwd123".to_string(),
		role: Some(role.to_string()),
		first_name: None,
		last_name: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		access_blind_allowed: None,
	};
	let user_id = UserBmc::create(ctx, mm, user_c).await?;
	Ok(user_id)
}

#[serial]
#[tokio::test]
async fn test_rls_case_org_isolation() -> Result<()> {
	let mm = init_test_mm().await;
	let admin_ctx =
		Ctx::new(system_user_id(), system_org_id(), ROLE_ADMIN.to_string())?;

	let org1_id = create_org(&mm, &admin_ctx).await?;
	let user1_id = create_user(&mm, &admin_ctx, org1_id, ROLE_USER).await?;

	let org2_id = create_org(&mm, &admin_ctx).await?;
	let user2_id = create_user(&mm, &admin_ctx, org2_id, ROLE_USER).await?;

	set_current_user(&mm, user1_id).await?;
	let ctx = Ctx::new(user1_id, org1_id, ROLE_USER.to_string())?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_org1 = create_case_fixture(&mm, org1_id, user1_id).await?;
	commit_test_ctx(&mm).await?;
	set_current_user(&mm, user2_id).await?;
	let ctx2 = Ctx::new(user2_id, org2_id, ROLE_USER.to_string())?;
	begin_test_ctx(&mm, &ctx2).await?;
	let case_org2 = create_case_fixture(&mm, org2_id, user2_id).await?;
	commit_test_ctx(&mm).await?;

	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	enable_rls(&mm).await?;
	set_org_context_dbx(dbx, org1_id, ROLE_USER).await?;
	let user_ctx = Ctx::new(user1_id, org1_id, ROLE_USER.to_string())?;

	let cases = CaseBmc::list(&user_ctx, &mm, None, None).await?;
	assert!(cases.iter().any(|c| c.id == case_org1));
	assert!(!cases.iter().any(|c| c.id == case_org2));

	let err = CaseBmc::get(&user_ctx, &mm, case_org2).await.unwrap_err();
	assert!(matches!(err, ModelError::EntityUuidNotFound { .. }));

	dbx.rollback_txn().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_rls_user_org_isolation() -> Result<()> {
	let mm = init_test_mm().await;
	let admin_ctx =
		Ctx::new(system_user_id(), system_org_id(), ROLE_ADMIN.to_string())?;

	let org1_id = create_org(&mm, &admin_ctx).await?;
	let user1_id = create_user(&mm, &admin_ctx, org1_id, ROLE_USER).await?;

	let org2_id = create_org(&mm, &admin_ctx).await?;
	let user2_id = create_user(&mm, &admin_ctx, org2_id, ROLE_USER).await?;

	commit_test_ctx(&mm).await?;
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	enable_rls(&mm).await?;
	set_full_context_dbx(dbx, user1_id, org1_id, ROLE_USER).await?;
	let user_ctx = Ctx::new(user1_id, org1_id, ROLE_USER.to_string())?;

	let users = UserBmc::list(&user_ctx, &mm, None, None).await?;
	assert!(!users.iter().any(|u| u.id == user2_id));

	let err = UserBmc::get::<lib_core::model::user::User>(&user_ctx, &mm, user2_id)
		.await
		.unwrap_err();
	assert!(matches!(err, ModelError::EntityUuidNotFound { .. }));
	dbx.rollback_txn().await?;
	Ok(())
}
