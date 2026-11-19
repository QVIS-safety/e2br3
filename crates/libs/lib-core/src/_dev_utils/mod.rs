// region:    --- Modules

// endregion: --- Modules

/// Database setup is explicit: use the shared migration runner before startup.
pub async fn init_dev() -> std::io::Result<()> {
	match std::env::var("SKIP_DEV_INIT").as_deref() {
		Ok("1") => Ok(()),
		_ => Err(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"Automatic database initialization is disabled. Run deploy/ec2/migrate-rds.sh for an existing database, or --initialize for an empty database, then set SKIP_DEV_INIT=1.",
		)),
	}
}

// NOTE: Test seed/clean helpers for user/agent/conv have been removed
// as those models were replaced with the E2B(R3) SafetyDB models.
// Add new test helpers here as needed for User, Organization, Case, etc.

pub fn fx_org_id() -> uuid::Uuid {
	// NOTE: This org_id is created via db/admin/00-recreate-db.sql
	uuid::uuid!("00000000-0000-0000-0000-000000000001")
}
