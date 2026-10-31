type Result<T> = core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[path = "database/audit_chain.rs"]
mod audit_chain;
#[path = "database/editor_storage.rs"]
mod editor_storage;
#[path = "database/reaction_term_highlight_migration.rs"]
mod reaction_term_highlight_migration;
#[path = "database/submission_schema.rs"]
mod submission_schema;
#[path = "database/transactions.rs"]
mod transactions;

async fn init_test_mm() -> Result<lib_core::model::ModelManager> {
	let expected = std::env::var("E2BR3_TEST_DATABASE_NAME")?;
	if !expected.starts_with("e2br3_test_") {
		return Err("database tests require an isolated e2br3_test_ database".into());
	}
	let mm = lib_core::model::ModelManager::new().await?;
	let actual: String = sqlx::query_scalar("SELECT current_database()")
		.fetch_one(mm.dbx().db())
		.await?;
	if actual != expected {
		return Err("database does not match E2BR3_TEST_DATABASE_NAME".into());
	}
	lib_core::_dev_utils::init_dev().await;
	Ok(mm)
}
