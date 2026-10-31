use crate::common::{init_test_mm, Result};
use axum::extract::FromRequestParts;
use axum::http::Request;
use lib_core::authorization::policy_registry;
use lib_core::ctx::{SYSTEM_ORG_ID, SYSTEM_USER_ID};
use lib_core::model::authorization::SnapshotRepository;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serial_test::serial;
use time::OffsetDateTime;

#[serial]
#[tokio::test]
async fn protected_snapshot_extractor_reads_the_attached_snapshot() -> Result<()> {
	let mm = init_test_mm().await?;
	let principal_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let organization_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	let snapshot = SnapshotRepository::load_repeatable_read(
		mm.dbx().db(),
		policy_registry(),
		principal_id,
		organization_id,
		OffsetDateTime::now_utc(),
		None,
	)
	.await?;
	let (mut parts, _) = Request::new(()).into_parts();
	parts
		.extensions
		.insert(AuthorizationSnapshotW::new(snapshot));
	let extracted =
		AuthorizationSnapshotW::from_request_parts(&mut parts, &()).await?;
	assert_eq!(extracted.principal_id(), principal_id);
	assert_eq!(extracted.organization_id(), organization_id);
	Ok(())
}

#[tokio::test]
async fn protected_snapshot_extractor_fails_when_middleware_did_not_attach_one() {
	let (mut parts, _) = Request::new(()).into_parts();
	let result = AuthorizationSnapshotW::from_request_parts(&mut parts, &()).await;
	assert!(result.is_err());
}
