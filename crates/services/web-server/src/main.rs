#![allow(dead_code)]

// region:    --- Modules

mod bootstrap;
mod config;
mod error;
mod submission;
mod web;

pub use self::error::{Error, Result};
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use tokio::net::TcpListener;
use tokio::time::{interval, Duration};
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use web::rest::permission_profile_rest;

// endregion: --- Modules

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.without_time() // For early local development.
		.with_target(false)
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	// -- FOR DEV ONLY (skips automatically if SKIP_DEV_INIT=1)
	_dev_utils::init_dev().await;
	config::validate_submission_runtime_config().map_err(Error::Config)?;

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
	bootstrap::bootstrap_admin_user(&mm).await?;
	permission_profile_rest::refresh_dynamic_roles(&mm)
		.await
		.map_err(|err| Error::Config(err.to_string()))?;
	start_reconcile_worker(mm.clone());

	// -- Define Routes
	let routes_all = web_server::app(mm.clone());

	// region:    --- Start Server
	// Note: For this block, ok to unwrap.
	// Use 0.0.0.0 in Docker, 127.0.0.1 for local dev
	let bind_addr = std::env::var("SERVICE_BIND_ADDR")
		.unwrap_or_else(|_| "127.0.0.1:8080".to_string());
	let listener = TcpListener::bind(&bind_addr).await.unwrap();
	info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
	axum::serve(listener, routes_all.into_make_service())
		.await
		.unwrap();
	// endregion: --- Start Server

	Ok(())
}

fn start_reconcile_worker(mm: ModelManager) {
	let enabled = env_truthy("SUBMISSION_RECONCILE_ENABLED");
	if !enabled {
		return;
	}
	let interval_secs = std::env::var("SUBMISSION_RECONCILE_INTERVAL_SECS")
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(60);
	let limit = std::env::var("SUBMISSION_RECONCILE_LIMIT")
		.ok()
		.and_then(|v| v.trim().parse::<i64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(25);

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
}

fn env_truthy(name: &str) -> bool {
	matches!(
		std::env::var(name),
		Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
	)
}
