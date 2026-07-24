use super::{ContextKind, ProposalKind, ResourceKind};
use std::marker::PhantomData;
use uuid::Uuid;

mod sealed {
	pub trait Context {}
	pub trait Resource {}
	pub trait Proposal {}
}

pub trait AuthorizationContext: sealed::Context {
	fn kind() -> ContextKind;
}

pub trait ResourceMarker: sealed::Resource {
	const KIND: ResourceKind;
}

pub trait ProposalMarker: sealed::Proposal {
	const KIND: ProposalKind;
}

macro_rules! resource_markers {
	($(($name:ident, $kind:ident)),+ $(,)?) => {$ (
		#[derive(Debug, Clone, Copy)]
		pub struct $name;
		impl sealed::Resource for $name {}
		impl ResourceMarker for $name {
			const KIND: ResourceKind = ResourceKind::$kind;
		}
	)+ };
}

resource_markers!(
	(ApplicationResource, Application),
	(CaseResource, Case),
	(CaseChildResource, CaseChild),
	(CaseAuditTrailResource, CaseAuditTrail),
	(PresaveResource, Presave),
	(ImportHistoryResource, ImportHistory),
	(SubmissionResource, Submission),
	(UserResource, User),
	(RoleResource, Role),
	(OrganizationResource, Organization),
	(SettingsResource, Settings),
	(NoticeResource, Notice),
	(AuditLogResource, AuditLog),
	(TerminologyResource, Terminology),
);

macro_rules! proposal_markers {
	($(($name:ident, $kind:ident)),+ $(,)?) => {$ (
		#[derive(Debug, Clone, Copy)]
		pub struct $name;
		impl sealed::Proposal for $name {}
		impl ProposalMarker for $name {
			const KIND: ProposalKind = ProposalKind::$kind;
		}
	)+ };
}

proposal_markers!(
	(CaseCreateProposal, CaseCreate),
	(PresaveCreateProposal, PresaveCreate),
	(XmlImportBatchProposal, XmlImportBatch),
	(UserCreateProposal, UserCreate),
	(RoleCreateProposal, RoleCreate),
	(OrganizationCreateProposal, OrganizationCreate),
	(TerminologyImportProposal, TerminologyImport),
);

#[derive(Debug, Clone, Copy)]
pub struct Existing<R>(PhantomData<fn() -> R>);
#[derive(Debug, Clone, Copy)]
pub struct Collection<R>(PhantomData<fn() -> R>);
#[derive(Debug, Clone, Copy)]
pub struct Proposed<P>(PhantomData<fn() -> P>);
#[derive(Debug, Clone, Copy)]
pub struct Parent<P, C>(PhantomData<fn() -> (P, C)>);
#[derive(Debug, Clone, Copy)]
pub struct ResourceSet<R>(PhantomData<fn() -> R>);

impl<R: ResourceMarker> sealed::Context for Existing<R> {}
impl<R: ResourceMarker> AuthorizationContext for Existing<R> {
	fn kind() -> ContextKind {
		ContextKind::Existing(R::KIND)
	}
}
impl<R: ResourceMarker> sealed::Context for Collection<R> {}
impl<R: ResourceMarker> AuthorizationContext for Collection<R> {
	fn kind() -> ContextKind {
		ContextKind::Collection(R::KIND)
	}
}
impl<P: ProposalMarker> sealed::Context for Proposed<P> {}
impl<P: ProposalMarker> AuthorizationContext for Proposed<P> {
	fn kind() -> ContextKind {
		ContextKind::Proposed(P::KIND)
	}
}
impl<P: ResourceMarker, C: ResourceMarker> sealed::Context for Parent<P, C> {}
impl<P: ResourceMarker, C: ResourceMarker> AuthorizationContext for Parent<P, C> {
	fn kind() -> ContextKind {
		ContextKind::Parent {
			parent: P::KIND,
			child: C::KIND,
		}
	}
}
impl<R: ResourceMarker> sealed::Context for ResourceSet<R> {}
impl<R: ResourceMarker> AuthorizationContext for ResourceSet<R> {
	fn kind() -> ContextKind {
		ContextKind::ResourceSet(R::KIND)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnforcedScopeFilter {
	sender_ids: Vec<String>,
	product_ids: Vec<String>,
	study_ids: Vec<String>,
	blind_allowed: bool,
}

impl EnforcedScopeFilter {
	#[allow(dead_code)] // Constructed by scoped repositories during route cutover.
	pub(crate) fn new(
		sender_ids: Vec<String>,
		product_ids: Vec<String>,
		study_ids: Vec<String>,
		blind_allowed: bool,
	) -> Self {
		Self {
			sender_ids,
			product_ids,
			study_ids,
			blind_allowed,
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
}

#[derive(Debug, Clone)]
pub(crate) struct EvaluatedContext {
	pub organization_id: Option<Uuid>,
	pub target_fingerprint: String,
	pub within_principal_scope: bool,
	pub lifecycle_compatible: bool,
	pub parent_authorized: bool,
	pub every_target_authorized: bool,
	pub enforced_scope_filter: Option<EnforcedScopeFilter>,
}

#[derive(Debug)]
pub struct ContextSnapshot<'tx, C: AuthorizationContext> {
	pub(crate) evaluated: EvaluatedContext,
	marker: PhantomData<(&'tx (), fn() -> C)>,
}

impl<'tx, C: AuthorizationContext> ContextSnapshot<'tx, C> {
	#[allow(dead_code)] // Constructed by scoped repositories during route cutover.
	pub(crate) fn new(evaluated: EvaluatedContext) -> Self {
		Self {
			evaluated,
			marker: PhantomData,
		}
	}
}

#[derive(Debug)]
pub struct LockedMutationContext<'tx, C: AuthorizationContext> {
	pub(crate) evaluated: EvaluatedContext,
	marker: PhantomData<fn(&'tx mut ()) -> (&'tx mut (), C)>,
}

impl<'tx, C: AuthorizationContext> LockedMutationContext<'tx, C> {
	#[allow(dead_code)] // Constructed by locking repositories during route cutover.
	pub(crate) fn new(evaluated: EvaluatedContext) -> Self {
		Self {
			evaluated,
			marker: PhantomData,
		}
	}
}

/// Context factories for user administration actions.
///
/// These factories deliberately expose only the facts used by the registered
/// user policies. Callers cannot mark unrelated case lifecycle or scope
/// conditions as satisfied.
pub fn user_collection_context(
	organization_id: Option<Uuid>,
) -> ContextSnapshot<'static, Collection<UserResource>> {
	ContextSnapshot::new(EvaluatedContext {
		organization_id,
		target_fingerprint: organization_id
			.map_or_else(|| "users:all".to_string(), |id| format!("users:{id}")),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn existing_user_read_context(
	user_id: Uuid,
	organization_id: Option<Uuid>,
) -> ContextSnapshot<'static, Existing<UserResource>> {
	ContextSnapshot::new(EvaluatedContext {
		organization_id,
		target_fingerprint: format!("user:{user_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn existing_user_mutation_context(
	user_id: Uuid,
	organization_id: Option<Uuid>,
) -> LockedMutationContext<'static, Existing<UserResource>> {
	LockedMutationContext::new(EvaluatedContext {
		organization_id,
		target_fingerprint: format!("user:{user_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn proposed_user_context(
	organization_id: Uuid,
) -> LockedMutationContext<'static, Proposed<UserCreateProposal>> {
	LockedMutationContext::new(EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("user:new:{organization_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn role_collection_context(
	organization_id: Uuid,
) -> ContextSnapshot<'static, Collection<RoleResource>> {
	ContextSnapshot::new(EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("roles:{organization_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn existing_role_read_context(
	role_id: &str,
	organization_id: Uuid,
) -> ContextSnapshot<'static, Existing<RoleResource>> {
	ContextSnapshot::new(EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("role:{role_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn existing_role_mutation_context(
	role_id: &str,
	organization_id: Uuid,
) -> LockedMutationContext<'static, Existing<RoleResource>> {
	LockedMutationContext::new(EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("role:{role_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}

pub fn proposed_role_context(
	organization_id: Uuid,
) -> LockedMutationContext<'static, Proposed<RoleCreateProposal>> {
	LockedMutationContext::new(EvaluatedContext {
		organization_id: Some(organization_id),
		target_fingerprint: format!("role:new:{organization_id}"),
		within_principal_scope: false,
		lifecycle_compatible: false,
		parent_authorized: false,
		every_target_authorized: false,
		enforced_scope_filter: None,
	})
}
