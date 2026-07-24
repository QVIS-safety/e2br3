use super::{
	ActionId, AuthorizationContext, EnforcedScopeFilter, PolicySnapshotVersion,
};
use std::marker::PhantomData;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug)]
struct PermitEvidence<C> {
	action_id: ActionId,
	principal_id: Uuid,
	organization_id: Uuid,
	target_organization_id: Option<Uuid>,
	target_fingerprint: String,
	snapshot_version: PolicySnapshotVersion,
	decision_time: OffsetDateTime,
	enforced_scope_filter: Option<EnforcedScopeFilter>,
	marker: PhantomData<fn() -> C>,
}

#[derive(Debug)]
pub struct AuthorizedRead<'tx, C: AuthorizationContext> {
	evidence: PermitEvidence<C>,
	brand: PhantomData<&'tx ()>,
}

#[derive(Debug)]
pub struct AuthorizedMutation<'tx, C: AuthorizationContext> {
	evidence: PermitEvidence<C>,
	brand: PhantomData<fn(&'tx mut ()) -> &'tx mut ()>,
}

impl<C> PermitEvidence<C> {
	fn new(
		action_id: ActionId,
		principal_id: Uuid,
		organization_id: Uuid,
		target_organization_id: Option<Uuid>,
		target_fingerprint: String,
		snapshot_version: PolicySnapshotVersion,
		decision_time: OffsetDateTime,
		enforced_scope_filter: Option<EnforcedScopeFilter>,
	) -> Self {
		Self {
			action_id,
			principal_id,
			organization_id,
			target_organization_id,
			target_fingerprint,
			snapshot_version,
			decision_time,
			enforced_scope_filter,
			marker: PhantomData,
		}
	}
}

macro_rules! permit_accessors {
	($permit:ident) => {
		impl<'tx, C: AuthorizationContext> $permit<'tx, C> {
			pub fn action_id(&self) -> &ActionId {
				&self.evidence.action_id
			}
			pub fn principal_id(&self) -> Uuid {
				self.evidence.principal_id
			}
			pub fn organization_id(&self) -> Uuid {
				self.evidence.organization_id
			}
			pub fn target_organization_id(&self) -> Option<Uuid> {
				self.evidence.target_organization_id
			}
			pub fn target_fingerprint(&self) -> &str {
				&self.evidence.target_fingerprint
			}
			pub fn snapshot_version(&self) -> &PolicySnapshotVersion {
				&self.evidence.snapshot_version
			}
			pub fn decision_time(&self) -> OffsetDateTime {
				self.evidence.decision_time
			}
			pub fn enforced_scope_filter(&self) -> Option<&EnforcedScopeFilter> {
				self.evidence.enforced_scope_filter.as_ref()
			}
		}
	};
}

permit_accessors!(AuthorizedRead);
permit_accessors!(AuthorizedMutation);

impl<'tx, C: AuthorizationContext> AuthorizedRead<'tx, C> {
	pub(crate) fn new(
		action_id: ActionId,
		principal_id: Uuid,
		organization_id: Uuid,
		target_organization_id: Option<Uuid>,
		target_fingerprint: String,
		snapshot_version: PolicySnapshotVersion,
		decision_time: OffsetDateTime,
		enforced_scope_filter: Option<EnforcedScopeFilter>,
	) -> Self {
		Self {
			evidence: PermitEvidence::new(
				action_id,
				principal_id,
				organization_id,
				target_organization_id,
				target_fingerprint,
				snapshot_version,
				decision_time,
				enforced_scope_filter,
			),
			brand: PhantomData,
		}
	}
}

impl<'tx, C: AuthorizationContext> AuthorizedMutation<'tx, C> {
	pub(crate) fn new(
		action_id: ActionId,
		principal_id: Uuid,
		organization_id: Uuid,
		target_organization_id: Option<Uuid>,
		target_fingerprint: String,
		snapshot_version: PolicySnapshotVersion,
		decision_time: OffsetDateTime,
	) -> Self {
		Self {
			evidence: PermitEvidence::new(
				action_id,
				principal_id,
				organization_id,
				target_organization_id,
				target_fingerprint,
				snapshot_version,
				decision_time,
				None,
			),
			brand: PhantomData,
		}
	}
}
