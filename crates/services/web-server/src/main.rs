#![allow(dead_code)]

// region:    --- Modules

mod config;
mod error;
mod submission;
mod web;

pub use self::error::{Error, Result};
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

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
	config::validate_submission_runtime_config()
		.map_err(Error::Config)?;

	let mm = ModelManager::new().await?;

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
