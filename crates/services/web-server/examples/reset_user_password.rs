use lib_core::ctx::Ctx;
use lib_core::model::user::UserBmc;
use lib_core::model::ModelManager;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
	let email = std::env::var("E2BR3_RESET_EMAIL")
		.unwrap_or_else(|_| "demo.user@example.com".to_string());
	let password = std::env::var("E2BR3_RESET_PASSWORD")
		.map_err(|_| "E2BR3_RESET_PASSWORD is required".to_string())?;

	let mm = ModelManager::new().await?;
	let user = UserBmc::auth_login_by_email(&mm, &email)
		.await?
		.ok_or_else(|| format!("user not found for email: {email}"))?;

	let ctx = Ctx::root_ctx();
	UserBmc::update_pwd_and_clear_must_change(&ctx, &mm, user.id, &password).await?;

	println!("password reset for {}", user.email);
	Ok(())
}
