use super::{BuiltInIdentityKind, GrantId};
use std::collections::BTreeSet;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityTraits {
	built_in_kind: Option<BuiltInIdentityKind>,
}

impl IdentityTraits {
	pub(crate) fn new(built_in_kind: Option<BuiltInIdentityKind>) -> Self {
		Self { built_in_kind }
	}

	pub fn built_in_kind(&self) -> Option<BuiltInIdentityKind> {
		self.built_in_kind
	}

	pub fn is_platform_administrator(&self) -> bool {
		self.built_in_kind == Some(BuiltInIdentityKind::PlatformAdministrator)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalScope {
	sender_ids: Vec<String>,
	product_ids: Vec<String>,
	study_ids: Vec<String>,
	blind_allowed: bool,
	active_sender_identifier: Option<String>,
}

impl PrincipalScope {
	pub(crate) fn new(
		sender_ids: Vec<String>,
		product_ids: Vec<String>,
		study_ids: Vec<String>,
		blind_allowed: bool,
		active_sender_identifier: Option<String>,
	) -> Self {
		Self {
			sender_ids,
			product_ids,
			study_ids,
			blind_allowed,
			active_sender_identifier,
		}
	}

	pub fn sender_ids(&self) -> &[String] {
		&self.sender_ids
	}

	pub fn product_ids(&self) -> &[String] {
		&self.product_ids
	}

	pub fn study_ids(&self) -> &[String] {
		&self.study_ids
	}

	pub fn blind_allowed(&self) -> bool {
		self.blind_allowed
	}

	pub fn active_sender_identifier(&self) -> Option<&str> {
		self.active_sender_identifier.as_deref()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySnapshotVersion {
	catalog_hash: String,
	organization_id: Uuid,
	organization_revision: i64,
	principal_revision: i64,
}

impl PolicySnapshotVersion {
	pub(crate) fn new(
		catalog_hash: String,
		organization_id: Uuid,
		organization_revision: i64,
		principal_revision: i64,
	) -> Self {
		Self {
			catalog_hash,
			organization_id,
			organization_revision,
			principal_revision,
		}
	}

	pub fn catalog_hash(&self) -> &str {
		&self.catalog_hash
	}

	pub fn organization_id(&self) -> Uuid {
		self.organization_id
	}

	pub fn organization_revision(&self) -> i64 {
		self.organization_revision
	}

	pub fn principal_revision(&self) -> i64 {
		self.principal_revision
	}
}

#[derive(Debug, Clone)]
pub struct RequestAuthorizationSnapshot {
	principal_id: Uuid,
	organization_id: Uuid,
	role_id: Uuid,
	identity: IdentityTraits,
	grants: BTreeSet<GrantId>,
	scope: PrincipalScope,
	version: PolicySnapshotVersion,
	evaluated_at: OffsetDateTime,
	authorization_valid_until: Option<OffsetDateTime>,
	legacy_permission_subject: String,
}

impl RequestAuthorizationSnapshot {
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn new(
		principal_id: Uuid,
		organization_id: Uuid,
		role_id: Uuid,
		identity: IdentityTraits,
		grants: BTreeSet<GrantId>,
		scope: PrincipalScope,
		version: PolicySnapshotVersion,
		evaluated_at: OffsetDateTime,
		authorization_valid_until: Option<OffsetDateTime>,
		legacy_permission_subject: String,
	) -> Self {
		Self {
			principal_id,
			organization_id,
			role_id,
			identity,
			grants,
			scope,
			version,
			evaluated_at,
			authorization_valid_until,
			legacy_permission_subject,
		}
	}

	pub fn principal_id(&self) -> Uuid {
		self.principal_id
	}

	pub fn organization_id(&self) -> Uuid {
		self.organization_id
	}

	pub fn role_id(&self) -> Uuid {
		self.role_id
	}

	pub fn identity(&self) -> &IdentityTraits {
		&self.identity
	}

	pub fn grants(&self) -> &BTreeSet<GrantId> {
		&self.grants
	}

	pub fn scope(&self) -> &PrincipalScope {
		&self.scope
	}

	pub fn version(&self) -> &PolicySnapshotVersion {
		&self.version
	}

	pub fn evaluated_at(&self) -> OffsetDateTime {
		self.evaluated_at
	}

	pub fn authorization_valid_until(&self) -> Option<OffsetDateTime> {
		self.authorization_valid_until
	}

	pub fn legacy_permission_subject(&self) -> &str {
		&self.legacy_permission_subject
	}
}
