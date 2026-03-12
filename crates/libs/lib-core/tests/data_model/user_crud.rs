use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_org_id, demo_user_id,
	init_test_mm, rollback_test_ctx, set_current_user, unique_suffix, Result,
};
use lib_auth::pwd::{self, ContentToHash};
use lib_core::model::user::{
	User, UserBmc, UserForCreate, UserForLogin, UserForUpdate,
};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use serial_test::serial;

async fn begin_user_test(mm: &ModelManager) -> Result<lib_core::ctx::Ctx> {
	let ctx = demo_ctx();
	set_current_user(mm, demo_user_id()).await?;
	begin_test_ctx(mm, &ctx).await?;
	Ok(ctx)
}

#[serial]
#[tokio::test]
async fn test_user_create_ok() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = begin_user_test(&mm).await?;
	let suffix = unique_suffix();
	let fx_username = format!("test_create_ok-user-01-{suffix}");
	let fx_pwd_clear = "test_create_ok pwd 01";
	let user_c = UserForCreate {
		organization_id: demo_org_id(),
		email: format!("{fx_username}@example.com"),
		username: Some(fx_username.to_string()),
		pwd_clear: fx_pwd_clear.to_string(),
		role: Some("user".to_string()),
		first_name: Some("Test".to_string()),
		last_name: Some("User".to_string()),
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
	};

	let user_id = UserBmc::create(&ctx, &mm, user_c).await?;

	let user: UserForLogin = UserBmc::get(&ctx, &mm, user_id).await?;

	assert_eq!(user.email, format!("{fx_username}@example.com"));
	assert_eq!(user.username, fx_username);
	assert!(user.pwd.is_some());
	let fetched: UserForLogin =
		UserBmc::first_by_email(&ctx, &mm, &format!("{fx_username}@example.com"))
			.await?
			.ok_or("expected user by email after create")?;
	assert_eq!(fetched.id, user_id);

	UserBmc::delete(&ctx, &mm, user_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_create_duplicate_email() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = begin_user_test(&mm).await?;
	let suffix = unique_suffix();
	let fx_username_1 = format!("test_create_dup_email-user-01-{suffix}");
	let fx_username_2 = format!("test_create_dup_email-user-02-{suffix}");
	let fx_email = format!("{fx_username_1}@example.com");
	let user_c_1 = UserForCreate {
		organization_id: demo_org_id(),
		email: fx_email.clone(),
		username: Some(fx_username_1.to_string()),
		pwd_clear: "test_create_dup_email pwd 01".to_string(),
		role: Some("user".to_string()),
		first_name: Some("Test".to_string()),
		last_name: Some("User".to_string()),
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
	};
	let user_c_2 = UserForCreate {
		organization_id: demo_org_id(),
		email: fx_email.clone(),
		username: Some(fx_username_2.to_string()),
		pwd_clear: "test_create_dup_email pwd 02".to_string(),
		role: Some("user".to_string()),
		first_name: Some("Test".to_string()),
		last_name: Some("User".to_string()),
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
	};

	let user_id_1 = UserBmc::create(&ctx, &mm, user_c_1).await?;
	commit_test_ctx(&mm).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let user_id_2 = UserBmc::create(&ctx, &mm, user_c_2).await;

	match user_id_2 {
		Err(ModelError::UserAlreadyExists { email }) => {
			assert_eq!(email, fx_email);
		}
		Err(other) => return Err(format!("unexpected error: {other:?}").into()),
		Ok(_) => return Err("expected duplicate email error".into()),
	}

	rollback_test_ctx(&mm).await?;
	begin_test_ctx(&mm, &ctx).await?;
	UserBmc::delete(&ctx, &mm, user_id_1).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_update_pwd_ok() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = begin_user_test(&mm).await?;
	let suffix = unique_suffix();
	let fx_username = format!("test_update_pwd-user-01-{suffix}");
	let fx_pwd_clear_1 = "test_update_pwd pwd 01";
	let fx_pwd_clear_2 = "test_update_pwd pwd 02";
	let user_c = UserForCreate {
		organization_id: demo_org_id(),
		email: format!("{fx_username}@example.com"),
		username: Some(fx_username.to_string()),
		pwd_clear: fx_pwd_clear_1.to_string(),
		role: Some("user".to_string()),
		first_name: Some("Test".to_string()),
		last_name: Some("User".to_string()),
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
	};
	let user_id = UserBmc::create(&ctx, &mm, user_c).await?;

	UserBmc::update_pwd(&ctx, &mm, user_id, fx_pwd_clear_2).await?;
	let user: UserForLogin = UserBmc::get(&ctx, &mm, user_id).await?;
	let pwd_ref = user.pwd.as_ref().expect("pwd should be set");
	let status = pwd::validate_pwd(
		ContentToHash {
			content: fx_pwd_clear_2.to_string(),
			salt: user.pwd_salt,
		},
		pwd_ref.clone(),
	)
	.await?;

	assert!(matches!(
		status,
		pwd::SchemeStatus::Ok | pwd::SchemeStatus::Outdated
	));

	UserBmc::delete(&ctx, &mm, user_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_first_by_email_seeded() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = begin_user_test(&mm).await?;
	let fx_email = "demo.user@example.com";
	let user: UserForLogin = UserBmc::first_by_email(&ctx, &mm, fx_email)
		.await?
		.ok_or("expected user by email")?;

	assert_eq!(user.id, demo_user_id());
	assert_eq!(user.email, fx_email);
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_update_ok() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = begin_user_test(&mm).await?;
	let suffix = unique_suffix();
	let fx_username = format!("test_update-user-01-{suffix}");
	let fx_email = format!("{fx_username}@example.com");
	let user_c = UserForCreate {
		organization_id: demo_org_id(),
		email: fx_email.clone(),
		username: Some(fx_username.to_string()),
		pwd_clear: "test_update pwd 01".to_string(),
		role: Some("user".to_string()),
		first_name: Some("Test".to_string()),
		last_name: Some("User".to_string()),
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
	};

	let user_id = UserBmc::create(&ctx, &mm, user_c).await?;
	let user_u = UserForUpdate {
		email: None,
		username: None,
		role: Some("admin".to_string()),
		first_name: Some("Updated".to_string()),
		last_name: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		active: Some(false),
		last_login_at: None,
	};

	UserBmc::update(&ctx, &mm, user_id, user_u).await?;
	let user: User = UserBmc::get(&ctx, &mm, user_id).await?;

	assert_eq!(user.role, "admin");
	assert_eq!(user.first_name.as_deref(), Some("Updated"));
	assert!(!user.active);

	UserBmc::delete(&ctx, &mm, user_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
