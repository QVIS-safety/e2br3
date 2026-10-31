type Result<T> = core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[path = "authorization/audit.rs"]
mod authorization_audit;
#[path = "authorization/isolation.rs"]
mod authorization_isolation;
#[path = "authorization/revisions.rs"]
mod authorization_revisions;
#[path = "authorization/snapshot.rs"]
mod authorization_snapshot;
#[path = "authorization/storage.rs"]
mod authorization_storage;
#[path = "authorization/test_support.rs"]
mod authorization_test_support;
