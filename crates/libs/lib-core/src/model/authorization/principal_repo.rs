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
		sqlx::query_as::<_, PrincipalSnapshotRow>(
			r#"
			SELECT
				a.role_id,
				r.organization_id AS role_organization_id,
				r.active AS role_active,
				r.deleted_at AS role_deleted_at,
				u.active AS user_active,
				m.active AS membership_active,
				a.active AS assignment_active,
				o.active AS organization_active,
				lower(o.org_type) AS organization_type,
				u.access_start_at,
				u.access_end_at,
				u.access_sender_ids,
				u.access_product_ids,
				u.access_study_ids,
				u.access_blind_allowed,
				u.active_sender_identifier,
				os.revision AS organization_revision,
				ps.revision AS principal_revision,
				cs.catalog_hash AS stored_catalog_hash
			FROM users u
			JOIN user_organization_memberships m
			  ON m.user_id = u.id AND m.organization_id = $2
			JOIN organizations o ON o.id = m.organization_id
			JOIN user_role_assignments a
			  ON a.user_id = u.id AND a.organization_id = m.organization_id
			JOIN authorization_roles r ON r.id = a.role_id
			LEFT JOIN organization_policy_state os
			  ON os.organization_id = m.organization_id
			LEFT JOIN principal_authorization_state ps
			  ON ps.user_id = u.id AND ps.organization_id = m.organization_id
			LEFT JOIN authorization_catalog_state cs ON cs.singleton
			WHERE u.id = $1
			"#,
		)
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
