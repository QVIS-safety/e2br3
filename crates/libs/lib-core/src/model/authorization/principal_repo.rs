use sqlx::types::time::OffsetDateTime;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct PrincipalSnapshotRow {
	pub role_id: Uuid,
	pub role_organization_id: Option<Uuid>,
	pub role_active: bool,
	pub role_deleted_at: Option<OffsetDateTime>,
	pub user_active: bool,
	pub membership_active: bool,
	pub assignment_active: bool,
	pub organization_active: bool,
	pub organization_type: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
	pub access_sender_ids: Option<String>,
	pub access_product_ids: Option<String>,
	pub access_study_ids: Option<String>,
	pub access_blind_allowed: Option<bool>,
	pub active_sender_identifier: Option<String>,
	pub organization_revision: Option<i64>,
	pub principal_revision: Option<i64>,
	pub stored_catalog_hash: Option<String>,
}

pub(crate) struct PrincipalRepository;

impl PrincipalRepository {
	pub async fn load(
		transaction: &mut Transaction<'_, Postgres>,
		user_id: Uuid,
		organization_id: Uuid,
	) -> Result<Option<PrincipalSnapshotRow>, sqlx::Error> {
		sqlx::query_as::<_, PrincipalSnapshotRow>(include_str!(
			"../sql/authorization/load_principal_snapshot.sql"
		))
		.bind(user_id)
		.bind(organization_id)
		.fetch_optional(&mut **transaction)
		.await
	}

	pub async fn grant_ids(
		transaction: &mut Transaction<'_, Postgres>,
		role_id: Uuid,
	) -> Result<Vec<String>, sqlx::Error> {
		sqlx::query_scalar(
			"SELECT grant_id FROM role_grants WHERE role_id = $1 ORDER BY grant_id",
		)
		.bind(role_id)
		.fetch_all(&mut **transaction)
		.await
	}
}
