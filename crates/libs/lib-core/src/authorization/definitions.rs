use super::{ActionId, EntitlementId, FactId, GrantId};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
	Implemented,
	Reserved,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RoleClass {
	PlatformBuiltIn,
	SponsorCroBuiltIn,
	SponsorCompanyBuiltIn,
	OperationalBuiltIn,
	ServiceBuiltIn,
	Custom,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInIdentityKind {
	PlatformAdministrator,
	SponsorCroAdministrator,
	SponsorCompanyAdministrator,
	OperationalUser,
	InternalServicePrincipal,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
	Application,
	Case,
	CaseChild,
	CaseAuditTrail,
	Presave,
	ImportHistory,
	Submission,
	User,
	Role,
	Organization,
	Settings,
	Notice,
	AuditLog,
	Terminology,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
	CaseCreate,
	PresaveCreate,
	XmlImportBatch,
	UserCreate,
	RoleCreate,
	OrganizationCreate,
	TerminologyImport,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
	Collection(ResourceKind),
	Proposed(ProposalKind),
	Existing(ResourceKind),
	Parent {
		parent: ResourceKind,
		child: ResourceKind,
	},
	ResourceSet(ResourceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStage {
	SubjectOnly,
	ContextRequired(ContextKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementRule {
	AllOf,
	AnyOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeCondition {
	ActiveMembership,
	AccessWindow,
	Organization,
	SenderProductStudy,
	BlindData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCondition {
	SameOrganization,
	WithinPrincipalScope,
	CompatibleLifecycle,
	ParentAuthorized,
	EveryTargetAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditClassification {
	Read,
	Mutation,
	PrivilegedRead,
	PrivilegedMutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationDomain {
	Organization,
	Principal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrantDefinition {
	pub id: GrantId,
	pub pdf_menu: String,
	pub pdf_type: String,
	pub pdf_privilege: String,
	pub availability: Availability,
	pub implied_grants: Vec<GrantId>,
	pub entitlements: Vec<EntitlementId>,
	pub assignable_role_classes: Vec<RoleClass>,
}

#[derive(Debug, Clone)]
pub struct GrantDefinitionInput {
	pub id: String,
	pub pdf_menu: String,
	pub pdf_type: String,
	pub pdf_privilege: String,
	pub availability: Availability,
	pub implied_grants: Vec<String>,
	pub entitlements: Vec<String>,
	pub assignable_role_classes: Vec<RoleClass>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyGrantAlias {
	pub legacy_id: String,
	pub grant_id: GrantId,
}

#[derive(Debug, Clone)]
pub struct LegacyGrantAliasInput {
	pub legacy_id: String,
	pub grant_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntitlementDefinition {
	pub id: EntitlementId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionPolicy {
	pub id: ActionId,
	pub decision_stage: DecisionStage,
	pub entitlement_rule: EntitlementRule,
	pub entitlements: Vec<EntitlementId>,
	pub allowed_identities: Vec<BuiltInIdentityKind>,
	pub scope_conditions: Vec<ScopeCondition>,
	pub context_conditions: Vec<ContextCondition>,
	pub audit_classification: AuditClassification,
}

#[derive(Debug, Clone)]
pub struct ActionPolicyInput {
	pub id: String,
	pub decision_stage: DecisionStage,
	pub entitlement_rule: EntitlementRule,
	pub entitlements: Vec<String>,
	pub allowed_identities: Vec<BuiltInIdentityKind>,
	pub scope_conditions: Vec<ScopeCondition>,
	pub context_conditions: Vec<ContextCondition>,
	pub audit_classification: AuditClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuiltInIdentityDefinition {
	pub kind: BuiltInIdentityKind,
	pub id: Uuid,
	pub stable_key: String,
	pub role_class: RoleClass,
	pub grants: Vec<GrantId>,
}

#[derive(Debug, Clone)]
pub struct BuiltInIdentityInput {
	pub kind: BuiltInIdentityKind,
	pub id: String,
	pub stable_key: String,
	pub role_class: RoleClass,
	pub grants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorizationFactDefinition {
	pub id: FactId,
	pub table: String,
	pub columns: Vec<String>,
	pub invalidation_domain: InvalidationDomain,
}

#[derive(Debug, Clone)]
pub struct AuthorizationFactInput {
	pub id: String,
	pub table: String,
	pub columns: Vec<String>,
	pub invalidation_domain: InvalidationDomain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectActionId(ActionId);

impl SubjectActionId {
	pub(crate) fn new(id: ActionId) -> Self {
		Self(id)
	}

	pub fn as_action_id(&self) -> &ActionId {
		&self.0
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextActionId<C> {
	id: ActionId,
	marker: PhantomData<fn() -> C>,
}

impl<C> ContextActionId<C> {
	pub fn as_action_id(&self) -> &ActionId {
		&self.id
	}
}
