// region:    --- Modules

mod dev_db;

use crate::ctx::Ctx;
use crate::model::user::UserBmc;
use crate::model::ModelManager;
use lib_auth::pwd::{self, ContentToHash};
use tokio::sync::OnceCell;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

// endregion: --- Modules

/// Initialize environment for local development.
/// (for early development, will be called from main()).
/// Skip if SKIP_DEV_INIT=1 (e.g., when using Docker PostgreSQL).
pub async fn init_dev() {
	// Skip if using Docker PostgreSQL (already initialized)
	if std::env::var("SKIP_DEV_INIT").unwrap_or_default() == "1" {
		info!(
			"{:<12} - init_dev() SKIPPED (SKIP_DEV_INIT=1)",
			"FOR-DEV-ONLY"
		);
		maybe_set_demo_pwd().await;
		return;
	}

	static INIT: OnceCell<()> = OnceCell::const_new();

	INIT.get_or_init(|| async {
		info!("{:<12} - init_dev_all()", "FOR-DEV-ONLY");

		dev_db::init_dev_db().await.unwrap();
		maybe_set_demo_pwd().await;
	})
	.await;
}

async fn maybe_set_demo_pwd() {
	let email = std::env::var("DEMO_USER_EMAIL")
		.unwrap_or_else(|_| "demo.user@example.com".to_string());
	let pwd = match std::env::var("DEMO_USER_PWD") {
		Ok(value) if !value.trim().is_empty() => value,
		_ => {
			warn!(
				"FOR-DEV-ONLY - DEMO_USER_PWD not set; falling back to default demo password."
			);
			"welcome".to_string()
		}
	};

	let mm = match ModelManager::new().await {
		Ok(mm) => mm,
		Err(err) => {
			warn!("FOR-DEV-ONLY - demo pwd skipped; db init failed: {err}");
			return;
		}
	};

	let ctx = Ctx::root_ctx();
	// Use auth_email-based lookup to bypass RLS when no org context is set.
	// On cold starts, schema may still be initializing; retry briefly on missing
	// relation errors so we don't require a manual restart for demo login.
	let mut user = None;
	for attempt in 1..=10 {
		match UserBmc::auth_login_by_email(&mm, &email).await {
			Ok(value) => {
				user = value;
				break;
			}
			Err(err) if should_retry_demo_pwd_lookup(&err) && attempt < 10 => {
				warn!(
					"FOR-DEV-ONLY - demo pwd lookup retry {attempt}/10 (transient): {err}"
				);
				sleep(Duration::from_secs(1)).await;
			}
			Err(err) => {
				warn!("FOR-DEV-ONLY - demo pwd lookup failed: {err}");
				return;
			}
		}
	}

	let Some(user) = user else {
		warn!("FOR-DEV-ONLY - demo pwd skipped; user not found: {email}");
		return;
	};

	let force_sync = env_truthy("DEMO_USER_FORCE_SYNC");
	if let Some(existing_pwd_hash) = user.pwd.clone() {
		let is_valid = pwd::validate_pwd(
			ContentToHash {
				salt: user.pwd_salt,
				content: pwd.clone(),
			},
			existing_pwd_hash,
		)
		.await
		.is_ok();
		if is_valid {
			info!("FOR-DEV-ONLY - demo pwd already in sync for {email}");
			return;
		}
		if !force_sync {
			panic!(
				"FOR-DEV-ONLY - demo pwd mismatch for {email}. Refusing to overwrite existing password on boot. Set DEMO_USER_FORCE_SYNC=1 once to repair, then remove it."
			);
		}
		warn!(
			"FOR-DEV-ONLY - forcing demo pwd resync for {email} (DEMO_USER_FORCE_SYNC=1)"
		);
	}

	if let Err(err) = UserBmc::update_pwd(&ctx, &mm, user.id, &pwd).await {
		panic!("FOR-DEV-ONLY - demo pwd update failed for {email}: {err}");
	}

	info!("FOR-DEV-ONLY - demo pwd synced for {email}");
}

fn should_retry_demo_pwd_lookup(err: &crate::model::Error) -> bool {
	let msg = err.to_string();
	msg.contains("42P01") || msg.contains("relation \"users\" does not exist")
}

fn env_truthy(name: &str) -> bool {
	matches!(
		std::env::var(name),
		Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
	)
}

/// Initialize test environment.
pub async fn init_test() -> ModelManager {
	static INIT: OnceCell<ModelManager> = OnceCell::const_new();

	let mm = INIT
		.get_or_init(|| async {
			init_dev().await;
			// NOTE: Rare occasion where unwrap is kind of ok.
			ModelManager::new().await.unwrap()
		})
		.await;

	mm.clone()
}

// NOTE: Test seed/clean helpers for user/agent/conv have been removed
// as those models were replaced with the E2B(R3) SafetyDB models.
// Add new test helpers here as needed for User, Organization, Case, etc.

pub fn fx_org_id() -> uuid::Uuid {
	// NOTE: This org_id is created via sql/dev_initial/00-recreate-db.sql
	uuid::uuid!("00000000-0000-0000-0000-000000000001")
}
