use super::{init_test_mm, Result};
use lib_core::authorization::policy_registry;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER};
use lib_core::model::authorization::AuthorizationMigrationService;
use lib_core::model::organization::{
	OrganizationBmc, OrganizationForCreate, ORG_TYPE_CRO,
};
use lib_core::model::user::{User, UserBmc, UserForCreate, UserForUpdate};
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_expired_user_is_soft_deactivated() -> Result<()> {
	let mm = init_test_mm().await?;
	let migration_pool = PgPoolOptions::new()
		.max_connections(1)
		.connect(&std::env::var("SERVICE_MIGRATION_DB_URL")?)
		.await?;
	let database: String = sqlx::query_scalar("SELECT current_database()")
		.fetch_one(&migration_pool)
		.await?;
	assert_eq!(database, std::env::var("E2BR3_TEST_DATABASE_NAME")?);
	AuthorizationMigrationService::reconcile_registry_storage(
		&migration_pool,
		policy_registry(),
	)
	.await?;
	migration_pool.close().await;

	let root_ctx = Ctx::root_ctx();
	let suffix = Uuid::new_v4();
	let org_id = OrganizationBmc::create(
		&root_ctx,
		&mm,
		OrganizationForCreate {
			name: format!("User expiration {suffix}"),
			org_type: Some(ORG_TYPE_CRO.to_string()),
			address: None,
			contact_email: None,
		},
	)
	.await?;
	let admin_id = UserBmc::create(
		&root_ctx,
		&mm,
		UserForCreate {
			organization_id: org_id,
			email: format!("expiration-admin-{suffix}@example.test"),
			username: Some(format!("expiration_admin_{suffix}")),
			pwd_clear: "expiration-admin-password".to_string(),
			role: Some(ROLE_SPONSOR_ADMIN_CRO.to_string()),
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
		},
	)
	.await?;
	let admin_ctx = Ctx::new(admin_id, org_id, ROLE_SPONSOR_ADMIN_CRO.to_string())?;
	let user_id = UserBmc::create(
		&admin_ctx,
		&mm,
		UserForCreate {
			organization_id: org_id,
			email: format!("expired-user-{suffix}@example.test"),
			username: Some(format!("expired_user_{suffix}")),
			pwd_clear: "expired-user-password".to_string(),
			role: Some(ROLE_USER.to_string()),
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
		},
	)
	.await?;

	UserBmc::update(
		&admin_ctx,
		&mm,
		user_id,
		UserForUpdate {
			organization_id: None,
			email: None,
			username: None,
			role: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: Some(OffsetDateTime::now_utc() - Duration::seconds(1)),
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
			active: Some(true),
			last_login_at: None,
		},
	)
	.await?;

	assert!(UserBmc::deactivate_expired(&root_ctx, &mm).await? >= 1);
	let user: User = UserBmc::get(&admin_ctx, &mm, user_id).await?;
	assert!(!user.active);
	Ok(())
}
