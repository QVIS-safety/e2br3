// region:    --- Modules

mod dev_db;

use crate::model::ModelManager;
use tokio::sync::OnceCell;
use tracing::info;

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
		return;
	}

	static INIT: OnceCell<()> = OnceCell::const_new();

	INIT.get_or_init(|| async {
		info!("{:<12} - init_dev_all()", "FOR-DEV-ONLY");

		dev_db::init_dev_db().await.unwrap();
	})
	.await;
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

pub async fn ensure_dev_schema_compatibility(
) -> Result<(), Box<dyn std::error::Error>> {
	dev_db::ensure_dev_schema_compatibility().await
}

// NOTE: Test seed/clean helpers for user/agent/conv have been removed
// as those models were replaced with the E2B(R3) SafetyDB models.
// Add new test helpers here as needed for User, Organization, Case, etc.

pub fn fx_org_id() -> uuid::Uuid {
	// NOTE: This org_id is created via db/admin/00-recreate-db.sql
	uuid::uuid!("00000000-0000-0000-0000-000000000001")
}
