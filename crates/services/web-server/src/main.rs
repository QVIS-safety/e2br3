// region:    --- Modules

mod bootstrap;
mod error;

pub use self::error::{Error, Result};
use lib_core::ctx::Ctx;
use lib_core::model::user::UserBmc;
use lib_core::model::ModelManager;
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;

// endregion: --- Modules

fn main() -> Result<()> {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.map_err(|error| {
			Error::Config(format!("failed to create Tokio runtime: {error}"))
		})?
		.block_on(run())
}

async fn run() -> Result<()> {
	tracing_subscriber::fmt()
		.with_target(false)
		.with_env_filter(EnvFilter::from_default_env())
		.init();
	lib_auth::init().map_err(|error| Error::Config(error.to_string()))?;

	// Database migrations run explicitly before the application starts.
	web_server::config::validate_submission_runtime_config()
		.map_err(Error::Config)?;

	let mm = ModelManager::new().await?;
	let authorization_status = web_server::initialize_authorization_storage()
		.await
		.map_err(|err| Error::Config(err.to_string()))?;
	let web_server::AuthorizationStartupStatus::Reconciled(report) =
		authorization_status;
	info!(
		"authorization storage reconciled: {} assignments, {} custom roles",
		report.assignments, report.custom_roles
	);
	if bootstrap::admin_bootstrap_enabled(std::env::var(
		"SERVICE_BOOTSTRAP_ENABLED",
	))? {
		bootstrap::bootstrap_admin_user(&mm).await?;
	}
	start_user_expiry_worker(mm.clone());
	start_reconcile_worker(mm.clone())?;

	// -- Define Routes
	let routes_all = web_server::app(mm.clone())
		.map_err(|error| Error::Config(error.to_string()))?;

	// region:    --- Start Server
	let bind_addr = std::env::var("SERVICE_BIND_ADDR")
		.map_err(|_| Error::Config("SERVICE_BIND_ADDR is required".to_string()))?;
	let listener = TcpListener::bind(&bind_addr).await.map_err(|error| {
		Error::Config(format!("failed to bind {bind_addr}: {error}"))
	})?;
	let local_addr = listener.local_addr().map_err(|error| {
		Error::Config(format!("failed to read listener address: {error}"))
	})?;
	info!("{:<12} - {:?}\n", "LISTENING", local_addr);
	axum::serve(listener, routes_all.into_make_service())
		.await
		.map_err(|error| Error::Config(format!("web server failed: {error}")))?;
	// endregion: --- Start Server

	Ok(())
}

fn start_user_expiry_worker(mm: ModelManager) {
	tokio::spawn(async move {
		let mut ticker = interval(Duration::from_secs(60));
		loop {
			ticker.tick().await;
			match UserBmc::deactivate_expired(&Ctx::root_ctx(), &mm).await {
				Ok(count) if count > 0 => {
					info!("USER EXPIRY - deactivated={count}");
				}
				Ok(_) => {}
				Err(err) => warn!("USER EXPIRY - failed: {err}"),
			}
		}
	});
}

fn start_reconcile_worker(mm: ModelManager) -> Result<()> {
	let enabled = env_truthy("SUBMISSION_RECONCILE_ENABLED")?;
	if !enabled {
		return Ok(());
	}
	let interval_secs = std::env::var("SUBMISSION_RECONCILE_INTERVAL_SECS")
		.map_err(|_| {
			Error::Config(
				"SUBMISSION_RECONCILE_INTERVAL_SECS is required when reconciliation is enabled"
					.to_string(),
			)
		})?
		.parse::<u64>()
		.map_err(|error| {
			Error::Config(format!(
				"invalid SUBMISSION_RECONCILE_INTERVAL_SECS: {error}"
			))
		})?;
	let limit = std::env::var("SUBMISSION_RECONCILE_LIMIT")
		.map_err(|_| {
			Error::Config(
				"SUBMISSION_RECONCILE_LIMIT is required when reconciliation is enabled"
					.to_string(),
			)
		})?
		.parse::<i64>()
		.map_err(|error| {
			Error::Config(format!("invalid SUBMISSION_RECONCILE_LIMIT: {error}"))
		})?;
	if interval_secs == 0 || limit <= 0 {
		return Err(Error::Config(
			"submission reconcile interval and limit must be positive".to_string(),
		));
	}

	tokio::spawn(async move {
		let mut ticker = interval(Duration::from_secs(interval_secs));
		loop {
			ticker.tick().await;
			match web_server::submission::reconcile_due_submissions_with_runtime_status(&mm, limit).await {
				Ok(result) => {
					if result.attempted > 0 {
						info!(
							"RECONCILE - attempted={} succeeded={} failed={} skipped={}",
							result.attempted,
							result.succeeded,
							result.failed,
							result.skipped
						);
					}
				}
				Err(err) => warn!("RECONCILE - failed: {err}"),
			}
		}
	});
	Ok(())
}

fn env_truthy(name: &str) -> Result<bool> {
	match std::env::var(name) {
		Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
			"1" | "true" | "yes" | "on" => Ok(true),
			"0" | "false" | "no" | "off" => Ok(false),
			_ => Err(Error::Config(format!("{name} must be a boolean value"))),
		},
		Err(error) => Err(Error::Config(format!("failed to read {name}: {error}"))),
	}
}
