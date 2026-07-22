use super::principal_repo::PrincipalRepository;
use crate::authorization::{
	export_contract, BuiltInIdentityKind, IdentityTraits, PolicyRegistry,
	PolicySnapshotVersion, PrincipalScope, RequestAuthorizationSnapshot,
};
use crate::ctx::ROLE_USER;
use crate::model::store::{set_org_context, set_user_context};
use sqlx::{Pool, Postgres};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug)]
pub enum SnapshotLoadError {
	Sqlx(sqlx::Error),
	Registry(String),
	MissingPrincipalAssignment,
	MissingRevisionState,
	CatalogMismatch,
	InactivePrincipal,
	InvalidScope(String),
	IncompatibleIdentity,
	IsolationBootstrap(String),
}

impl Display for SnapshotLoadError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for SnapshotLoadError {}

impl From<sqlx::Error> for SnapshotLoadError {
	fn from(error: sqlx::Error) -> Self {
		Self::Sqlx(error)
	}
}

pub struct SnapshotRepository;

impl SnapshotRepository {
	#[allow(clippy::too_many_arguments)]
	pub async fn load_repeatable_read(
		pool: &Pool<Postgres>,
		registry: &PolicyRegistry,
		user_id: Uuid,
		organization_id: Uuid,
		evaluated_at: OffsetDateTime,
		authentication_expires_at: Option<OffsetDateTime>,
	) -> Result<RequestAuthorizationSnapshot, SnapshotLoadError> {
		let mut transaction = pool.begin().await?;
		sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
			.execute(&mut *transaction)
			.await?;
		// Transitional RLS bootstrap: the authenticated principal and organization
		// are scoped with the least-privileged legacy role. Privileged identity is
		// resolved only from the normalized assignment's registry-owned UUID below.
		set_user_context(&mut transaction, user_id)
			.await
			.map_err(|error| {
				SnapshotLoadError::IsolationBootstrap(error.to_string())
			})?;
		set_org_context(&mut transaction, organization_id, ROLE_USER)
			.await
			.map_err(|error| {
				SnapshotLoadError::IsolationBootstrap(error.to_string())
			})?;
		let result = async {
			let row = PrincipalRepository::load(
				&mut transaction,
				user_id,
				organization_id,
			)
			.await?
			.ok_or(SnapshotLoadError::MissingPrincipalAssignment)?;
			let organization_revision = row
				.organization_revision
				.filter(|revision| *revision > 0)
				.ok_or(SnapshotLoadError::MissingRevisionState)?;
			let principal_revision = row
				.principal_revision
				.filter(|revision| *revision > 0)
				.ok_or(SnapshotLoadError::MissingRevisionState)?;
			let deployed_hash = export_contract(registry)
				.map_err(|error| SnapshotLoadError::Registry(error.to_string()))?
				.catalog_hash;
			if row.stored_catalog_hash.as_deref() != Some(deployed_hash.as_str()) {
				return Err(SnapshotLoadError::CatalogMismatch);
			}
			if !row.user_active
				|| !row.membership_active
				|| !row.assignment_active
				|| !row.organization_active
				|| !row.role_active
				|| row.role_deleted_at.is_some()
				|| row
					.role_organization_id
					.is_some_and(|role_org| role_org != organization_id)
				|| row
					.access_start_at
					.is_some_and(|start| evaluated_at < start)
				|| row.access_end_at.is_some_and(|end| evaluated_at >= end)
				|| authentication_expires_at
					.is_some_and(|expiry| evaluated_at >= expiry)
			{
				return Err(SnapshotLoadError::InactivePrincipal);
			}

			let built_in_kind = registry
				.built_in_identities()
				.iter()
				.find(|identity| identity.id == row.role_id)
				.map(|identity| identity.kind);
			if built_in_kind == Some(BuiltInIdentityKind::InternalServicePrincipal) {
				return Err(SnapshotLoadError::IncompatibleIdentity);
			}
			let organization_type = row.organization_type.as_deref().unwrap_or("");
			if matches!(
				built_in_kind,
				Some(BuiltInIdentityKind::SponsorCroAdministrator)
			) && organization_type != "cro"
				|| matches!(
					built_in_kind,
					Some(BuiltInIdentityKind::SponsorCompanyAdministrator)
				) && organization_type != "pharmaceutical_company"
			{
				return Err(SnapshotLoadError::IncompatibleIdentity);
			}

			let grant_ids =
				PrincipalRepository::grant_ids(&mut transaction, row.role_id)
					.await?;
			let entitlements = registry
				.effective_entitlements(grant_ids.iter().map(String::as_str))
				.map_err(|error| SnapshotLoadError::Registry(error.to_string()))?
				.into_iter()
				.collect::<BTreeSet<_>>();
			let scope = PrincipalScope::new(
				parse_scope(row.access_sender_ids.as_deref())?,
				parse_scope(row.access_product_ids.as_deref())?,
				parse_scope(row.access_study_ids.as_deref())?,
				row.access_blind_allowed.unwrap_or(false),
				trimmed(row.active_sender_identifier),
			);
			let valid_until = [row.access_end_at, authentication_expires_at]
				.into_iter()
				.flatten()
				.filter(|boundary| *boundary > evaluated_at)
				.min();
			let legacy_permission_subject = match built_in_kind {
				Some(BuiltInIdentityKind::PlatformAdministrator) => {
					"system_admin".into()
				}
				Some(BuiltInIdentityKind::SponsorCroAdministrator) => {
					"sponsor_admin_cro".into()
				}
				Some(BuiltInIdentityKind::SponsorCompanyAdministrator) => {
					"sponsor_admin_company".into()
				}
				Some(BuiltInIdentityKind::OperationalUser) => "user".into(),
				Some(BuiltInIdentityKind::InternalServicePrincipal) => {
					unreachable!()
				}
				None => row.role_id.to_string(),
			};
			Ok(RequestAuthorizationSnapshot::new(
				user_id,
				organization_id,
				row.role_id,
				IdentityTraits::new(built_in_kind),
				entitlements,
				scope,
				PolicySnapshotVersion::new(
					deployed_hash,
					organization_id,
					organization_revision,
					principal_revision,
				),
				evaluated_at,
				valid_until,
				legacy_permission_subject,
			))
		}
		.await;
		match result {
			Ok(snapshot) => {
				transaction.commit().await?;
				Ok(snapshot)
			}
			Err(error) => {
				let _ = transaction.rollback().await;
				Err(error)
			}
		}
	}
}

fn parse_scope(raw: Option<&str>) -> Result<Vec<String>, SnapshotLoadError> {
	let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(Vec::new());
	};
	let values = serde_json::from_str::<Vec<String>>(raw)
		.map_err(|_| SnapshotLoadError::InvalidScope(raw.to_string()))?;
	Ok(values
		.into_iter()
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect())
}

fn trimmed(value: Option<String>) -> Option<String> {
	value.and_then(|value| {
		let value = value.trim().to_string();
		(!value.is_empty()).then_some(value)
	})
}
