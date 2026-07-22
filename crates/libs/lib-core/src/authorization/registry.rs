use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
	InvalidIdentifier {
		kind: &'static str,
		value: String,
	},
	DuplicateIdentifier {
		kind: &'static str,
		value: String,
	},
	InvalidPdfOrder {
		grant: String,
		order: u16,
	},
	DuplicatePdfOrder {
		order: u16,
	},
	DuplicateUiBinding {
		menu_key: String,
		field: GrantUiField,
	},
	UnknownEntitlement {
		owner: String,
		entitlement: String,
	},
	UnknownGrant {
		owner: String,
		grant: String,
	},
	GrantImplicationCycle {
		grants: Vec<String>,
	},
	ReservedGrantHasEntitlements {
		grant: String,
	},
	ReservedGrantNotAssignable {
		grant: String,
	},
	EmptyRoleClasses {
		grant: String,
	},
	InvalidBuiltInUuid {
		stable_key: String,
		value: String,
	},
	DuplicateBuiltInUuid {
		value: Uuid,
	},
	DuplicateBuiltInKind {
		kind: BuiltInIdentityKind,
	},
	UnknownAliasTarget {
		legacy_id: String,
		target: String,
	},
	DuplicateAlias {
		legacy_id: String,
	},
}

impl Display for RegistryError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for RegistryError {}

#[derive(Debug)]
pub struct PolicyRegistry {
	grants: BTreeMap<GrantId, GrantDefinition>,
	entitlements: BTreeMap<EntitlementId, EntitlementDefinition>,
	actions: BTreeMap<ActionId, ActionPolicy>,
	built_in_identities: Vec<BuiltInIdentityDefinition>,
	facts: BTreeMap<FactId, AuthorizationFactDefinition>,
	legacy_aliases: BTreeMap<String, LegacyGrantAlias>,
}

impl PolicyRegistry {
	pub fn grant(&self, id: &str) -> Option<&GrantDefinition> {
		self.grants.get(id)
	}

	pub fn grants(&self) -> impl ExactSizeIterator<Item = &GrantDefinition> {
		self.grants.values()
	}

	pub fn entitlement(&self, id: &str) -> Option<&EntitlementDefinition> {
		self.entitlements.get(id)
	}

	pub fn entitlements(
		&self,
	) -> impl ExactSizeIterator<Item = &EntitlementDefinition> {
		self.entitlements.values()
	}

	pub fn action(&self, id: &str) -> Option<&ActionPolicy> {
		self.actions.get(id)
	}

	pub fn actions(&self) -> impl ExactSizeIterator<Item = &ActionPolicy> {
		self.actions.values()
	}

	pub fn built_in_identities(&self) -> &[BuiltInIdentityDefinition] {
		&self.built_in_identities
	}

	pub fn built_in_identity(
		&self,
		kind: BuiltInIdentityKind,
	) -> Option<&BuiltInIdentityDefinition> {
		self.built_in_identities
			.iter()
			.find(|definition| definition.kind == kind)
	}

	pub fn facts(
		&self,
	) -> impl ExactSizeIterator<Item = &AuthorizationFactDefinition> {
		self.facts.values()
	}

	pub fn legacy_alias(&self, id: &str) -> Option<&LegacyGrantAlias> {
		self.legacy_aliases.get(id)
	}

	pub fn legacy_aliases(
		&self,
	) -> impl ExactSizeIterator<Item = &LegacyGrantAlias> {
		self.legacy_aliases.values()
	}

	pub fn validate_assignable_grants<'a>(
		&self,
		role_class: RoleClass,
		grant_ids: impl IntoIterator<Item = &'a str>,
	) -> Result<Vec<GrantId>, RegistryError> {
		let mut grants = Vec::new();
		for raw_id in grant_ids {
			let grant =
				self.grant(raw_id)
					.ok_or_else(|| RegistryError::UnknownGrant {
						owner: "role assignment".to_string(),
						grant: raw_id.to_string(),
					})?;
			if grant.availability == Availability::Reserved {
				return Err(RegistryError::ReservedGrantNotAssignable {
					grant: raw_id.to_string(),
				});
			}
			if !grant.assignable_role_classes.contains(&role_class) {
				return Err(RegistryError::UnknownGrant {
					owner: format!("role class {role_class:?}"),
					grant: raw_id.to_string(),
				});
			}
			grants.push(grant.id.clone());
		}
		grants.sort_unstable();
		grants.dedup();
		Ok(grants)
	}

	pub fn effective_entitlements<'a>(
		&self,
		grant_ids: impl IntoIterator<Item = &'a str>,
	) -> Result<Vec<EntitlementId>, RegistryError> {
		fn collect(
			registry: &PolicyRegistry,
			grant: &GrantDefinition,
			visited: &mut BTreeSet<GrantId>,
			entitlements: &mut BTreeSet<EntitlementId>,
		) {
			if !visited.insert(grant.id.clone()) {
				return;
			}
			entitlements.extend(grant.entitlements.iter().cloned());
			for implied in &grant.implied_grants {
				collect(registry, &registry.grants[implied], visited, entitlements);
			}
		}

		let mut visited = BTreeSet::new();
		let mut entitlements = BTreeSet::new();
		for grant_id in grant_ids {
			let grant =
				self.grant(grant_id)
					.ok_or_else(|| RegistryError::UnknownGrant {
						owner: "entitlement compilation".to_string(),
						grant: grant_id.to_string(),
					})?;
			collect(self, grant, &mut visited, &mut entitlements);
		}
		Ok(entitlements.into_iter().collect())
	}

	pub fn subject_action(&self, id: &str) -> Option<SubjectActionId> {
		let action = self.action(id)?;
		(action.decision_stage == DecisionStage::SubjectOnly)
			.then(|| SubjectActionId::new(action.id.clone()))
	}
}

#[derive(Debug, Default)]
pub struct PolicyRegistryBuilder {
	entitlements: Vec<String>,
	grants: Vec<GrantDefinitionInput>,
	actions: Vec<ActionPolicyInput>,
	identities: Vec<BuiltInIdentityInput>,
	facts: Vec<AuthorizationFactInput>,
	aliases: Vec<LegacyGrantAliasInput>,
}

impl PolicyRegistryBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn entitlement(mut self, id: impl Into<String>) -> Self {
		self.entitlements.push(id.into());
		self
	}

	pub fn grant(mut self, definition: GrantDefinitionInput) -> Self {
		self.grants.push(definition);
		self
	}

	pub fn action(mut self, definition: ActionPolicyInput) -> Self {
		self.actions.push(definition);
		self
	}

	pub fn identity(mut self, definition: BuiltInIdentityInput) -> Self {
		self.identities.push(definition);
		self
	}

	pub fn fact(mut self, definition: AuthorizationFactInput) -> Self {
		self.facts.push(definition);
		self
	}

	pub fn legacy_alias(mut self, definition: LegacyGrantAliasInput) -> Self {
		self.aliases.push(definition);
		self
	}

	pub fn build(self) -> Result<PolicyRegistry, RegistryError> {
		let mut entitlements = BTreeMap::new();
		for raw_id in self.entitlements {
			let id = parse_id::<EntitlementId>("entitlement", raw_id)?;
			if entitlements
				.insert(id.clone(), EntitlementDefinition { id: id.clone() })
				.is_some()
			{
				return Err(duplicate("entitlement", id.as_str()));
			}
		}

		let mut grants = BTreeMap::new();
		let mut pdf_orders = BTreeSet::new();
		let mut ui_bindings = BTreeSet::new();
		for input in self.grants {
			let id = parse_id::<GrantId>("grant", input.id)?;
			if input.pdf_order == 0 {
				return Err(RegistryError::InvalidPdfOrder {
					grant: id.to_string(),
					order: input.pdf_order,
				});
			}
			if !pdf_orders.insert(input.pdf_order) {
				return Err(RegistryError::DuplicatePdfOrder {
					order: input.pdf_order,
				});
			}
			if !ui_bindings.insert(input.ui_binding.clone()) {
				return Err(RegistryError::DuplicateUiBinding {
					menu_key: input.ui_binding.menu_key,
					field: input.ui_binding.field,
				});
			}
			if input.assignable_role_classes.is_empty() {
				return Err(RegistryError::EmptyRoleClasses {
					grant: id.to_string(),
				});
			}
			let grant_entitlements = input
				.entitlements
				.into_iter()
				.map(|value| parse_id::<EntitlementId>("entitlement", value))
				.collect::<Result<Vec<_>, _>>()?;
			if input.availability == Availability::Reserved
				&& !grant_entitlements.is_empty()
			{
				return Err(RegistryError::ReservedGrantHasEntitlements {
					grant: id.to_string(),
				});
			}
			for entitlement in &grant_entitlements {
				if !entitlements.contains_key(entitlement) {
					return Err(RegistryError::UnknownEntitlement {
						owner: id.to_string(),
						entitlement: entitlement.to_string(),
					});
				}
			}
			let implied_grants = input
				.implied_grants
				.into_iter()
				.map(|value| parse_id::<GrantId>("grant", value))
				.collect::<Result<Vec<_>, _>>()?;
			let definition = GrantDefinition {
				id: id.clone(),
				pdf_order: input.pdf_order,
				pdf_menu: input.pdf_menu,
				pdf_type: input.pdf_type,
				pdf_privilege: input.pdf_privilege,
				availability: input.availability,
				ui_binding: input.ui_binding,
				implied_grants,
				entitlements: grant_entitlements,
				assignable_role_classes: input.assignable_role_classes,
			};
			if grants.insert(id.clone(), definition).is_some() {
				return Err(duplicate("grant", id.as_str()));
			}
		}

		for grant in grants.values() {
			for implied in &grant.implied_grants {
				if !grants.contains_key(implied) {
					return Err(RegistryError::UnknownGrant {
						owner: grant.id.to_string(),
						grant: implied.to_string(),
					});
				}
			}
		}
		validate_no_grant_cycles(&grants)?;

		let mut actions = BTreeMap::new();
		for input in self.actions {
			let id = parse_id::<ActionId>("action", input.id)?;
			let action_entitlements = input
				.entitlements
				.into_iter()
				.map(|value| parse_id::<EntitlementId>("entitlement", value))
				.collect::<Result<Vec<_>, _>>()?;
			for entitlement in &action_entitlements {
				if !entitlements.contains_key(entitlement) {
					return Err(RegistryError::UnknownEntitlement {
						owner: id.to_string(),
						entitlement: entitlement.to_string(),
					});
				}
			}
			let action = ActionPolicy {
				id: id.clone(),
				decision_stage: input.decision_stage,
				entitlement_rule: input.entitlement_rule,
				entitlements: action_entitlements,
				allowed_identities: input.allowed_identities,
				scope_conditions: input.scope_conditions,
				context_conditions: input.context_conditions,
				audit_classification: input.audit_classification,
			};
			if actions.insert(id.clone(), action).is_some() {
				return Err(duplicate("action", id.as_str()));
			}
		}

		let mut identities = Vec::new();
		let mut identity_kinds = BTreeSet::new();
		let mut identity_uuids = BTreeSet::new();
		for input in self.identities {
			let id = Uuid::parse_str(&input.id).map_err(|_| {
				RegistryError::InvalidBuiltInUuid {
					stable_key: input.stable_key.clone(),
					value: input.id,
				}
			})?;
			if !identity_kinds.insert(input.kind) {
				return Err(RegistryError::DuplicateBuiltInKind {
					kind: input.kind,
				});
			}
			if !identity_uuids.insert(id) {
				return Err(RegistryError::DuplicateBuiltInUuid { value: id });
			}
			let identity_grants = input
				.grants
				.into_iter()
				.map(|value| parse_id::<GrantId>("grant", value))
				.collect::<Result<Vec<_>, _>>()?;
			for grant_id in &identity_grants {
				let grant = grants.get(grant_id).ok_or_else(|| {
					RegistryError::UnknownGrant {
						owner: input.stable_key.clone(),
						grant: grant_id.to_string(),
					}
				})?;
				if grant.availability == Availability::Reserved {
					return Err(RegistryError::ReservedGrantNotAssignable {
						grant: grant_id.to_string(),
					});
				}
				if !grant.assignable_role_classes.contains(&input.role_class) {
					return Err(RegistryError::UnknownGrant {
						owner: format!("built-in role class {:?}", input.role_class),
						grant: grant_id.to_string(),
					});
				}
			}
			identities.push(BuiltInIdentityDefinition {
				kind: input.kind,
				id,
				stable_key: input.stable_key,
				role_class: input.role_class,
				grants: identity_grants,
			});
		}
		identities.sort_unstable_by_key(|definition| definition.kind);

		let mut facts = BTreeMap::new();
		for input in self.facts {
			let id = parse_id::<FactId>("fact", input.id)?;
			let fact = AuthorizationFactDefinition {
				id: id.clone(),
				table: input.table,
				columns: input.columns,
				invalidation_domain: input.invalidation_domain,
			};
			if facts.insert(id.clone(), fact).is_some() {
				return Err(duplicate("fact", id.as_str()));
			}
		}

		let mut legacy_aliases = BTreeMap::new();
		for input in self.aliases {
			let target = parse_id::<GrantId>("grant", input.grant_id.clone())?;
			if !grants.contains_key(&target) {
				return Err(RegistryError::UnknownAliasTarget {
					legacy_id: input.legacy_id,
					target: input.grant_id,
				});
			}
			let alias = LegacyGrantAlias {
				legacy_id: input.legacy_id.clone(),
				grant_id: target,
			};
			if legacy_aliases
				.insert(input.legacy_id.clone(), alias)
				.is_some()
			{
				return Err(RegistryError::DuplicateAlias {
					legacy_id: input.legacy_id,
				});
			}
		}

		Ok(PolicyRegistry {
			grants,
			entitlements,
			actions,
			built_in_identities: identities,
			facts,
			legacy_aliases,
		})
	}
}

trait ParsedId: Sized {
	fn parse_id(value: String) -> Result<Self, IdentifierError>;
}

macro_rules! parsed_id {
	($kind:ty) => {
		impl ParsedId for $kind {
			fn parse_id(value: String) -> Result<Self, IdentifierError> {
				Self::parse(value)
			}
		}
	};
}

parsed_id!(GrantId);
parsed_id!(EntitlementId);
parsed_id!(ActionId);
parsed_id!(FactId);

fn parse_id<T: ParsedId>(
	kind: &'static str,
	value: String,
) -> Result<T, RegistryError> {
	T::parse_id(value).map_err(|error| RegistryError::InvalidIdentifier {
		kind,
		value: error.value().to_string(),
	})
}

fn duplicate(kind: &'static str, value: &str) -> RegistryError {
	RegistryError::DuplicateIdentifier {
		kind,
		value: value.to_string(),
	}
}

fn validate_no_grant_cycles(
	grants: &BTreeMap<GrantId, GrantDefinition>,
) -> Result<(), RegistryError> {
	fn visit(
		id: &GrantId,
		grants: &BTreeMap<GrantId, GrantDefinition>,
		visiting: &mut Vec<GrantId>,
		visited: &mut BTreeSet<GrantId>,
	) -> Result<(), RegistryError> {
		if let Some(position) = visiting.iter().position(|candidate| candidate == id)
		{
			let mut cycle = visiting[position..]
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>();
			cycle.push(id.to_string());
			return Err(RegistryError::GrantImplicationCycle { grants: cycle });
		}
		if visited.contains(id) {
			return Ok(());
		}
		visiting.push(id.clone());
		for implied in &grants[id].implied_grants {
			visit(implied, grants, visiting, visited)?;
		}
		visiting.pop();
		visited.insert(id.clone());
		Ok(())
	}

	let mut visited = BTreeSet::new();
	for id in grants.keys() {
		visit(id, grants, &mut Vec::new(), &mut visited)?;
	}
	Ok(())
}

static POLICY_REGISTRY: OnceLock<PolicyRegistry> = OnceLock::new();

pub fn policy_registry() -> &'static PolicyRegistry {
	POLICY_REGISTRY.get_or_init(|| {
		canonical_registry_builder()
			.build()
			.expect("valid policy registry")
	})
}

fn canonical_registry_builder() -> PolicyRegistryBuilder {
	let mut builder = PolicyRegistryBuilder::new();
	for entitlement in CANONICAL_ENTITLEMENTS {
		builder = builder.entitlement(*entitlement);
	}
	for grant in canonical_grants() {
		builder = builder.grant(grant);
	}
	for action in canonical_actions() {
		builder = builder.action(action);
	}
	for identity in canonical_identities() {
		builder = builder.identity(identity);
	}
	for fact in canonical_facts() {
		builder = builder.fact(fact);
	}
	for alias in canonical_aliases() {
		builder = builder.legacy_alias(alias);
	}
	builder
}

const CANONICAL_ENTITLEMENTS: &[&str] = &[
	"notice.read",
	"notice.update",
	"case.queue_read",
	"case.read",
	"case.create",
	"case.update",
	"case.review",
	"case.lock",
	"case.audit_read",
	"case.export",
	"case.workflow_read",
	"info.read",
	"info.update",
	"import.history_read",
	"import.execute",
	"submission.history_read",
	"submission.execute",
	"user.read",
	"user.create",
	"user.update",
	"user.delete",
	"user.role_assign",
	"role.read",
	"role.manage",
	"role.assign",
	"organization.read",
	"organization.manage",
	"settings.read",
	"settings.update",
	"audit.read",
	"terminology.read",
	"terminology.manage",
];

fn grant(
	id: &str,
	menu: &str,
	type_name: &str,
	privilege: &str,
	availability: Availability,
	entitlements: &[&str],
	implied: &[&str],
) -> GrantDefinitionInput {
	GrantDefinitionInput {
		id: id.to_string(),
		pdf_order: 0,
		pdf_menu: menu.to_string(),
		pdf_type: type_name.to_string(),
		pdf_privilege: privilege.to_string(),
		availability,
		ui_binding: canonical_ui_binding(id),
		implied_grants: implied.iter().map(|value| (*value).to_string()).collect(),
		entitlements: entitlements
			.iter()
			.map(|value| (*value).to_string())
			.collect(),
		assignable_role_classes: vec![
			RoleClass::Custom,
			RoleClass::SponsorCroBuiltIn,
			RoleClass::SponsorCompanyBuiltIn,
		],
	}
}

fn canonical_ui_binding(id: &str) -> GrantUiBinding {
	use GrantUiField::{CanEdit, CanLock, CanRead, CanReview};
	let (menu_key, field) = match id {
		"home.notice.read" => ("home_notice", CanRead),
		"home.notice.edit" => ("home_notice", CanEdit),
		"home.workflow.read" => ("home_workflow", CanRead),
		"case.read" => ("case", CanRead),
		"case.edit" => ("case", CanEdit),
		"case.workflow.read" => ("case_workflow", CanRead),
		"case.review" => ("case", CanReview),
		"case.lock" => ("case", CanLock),
		"info.read" => ("info", CanRead),
		"info.edit" => ("info", CanEdit),
		"import.execute" => ("import", CanEdit),
		"import.history.read" => ("import", CanRead),
		"submission.execute" => ("export_submission", CanEdit),
		"submission.history.read" => ("export_submission", CanRead),
		"admin.read" => ("admin", CanRead),
		"admin.edit" => ("admin", CanEdit),
		"email.report_due.read" => ("email_report_due", CanRead),
		"email.report_due.send" => ("email_report_due", CanEdit),
		_ => unreachable!("canonical grant {id:?} requires a UI binding"),
	};
	GrantUiBinding::new(menu_key, field)
}

fn canonical_grants() -> Vec<GrantDefinitionInput> {
	use Availability::{Implemented, Reserved};
	let mut grants = vec![
		grant(
			"home.notice.read",
			"HOME",
			"Notice",
			"Read",
			Implemented,
			&["notice.read"],
			&[],
		),
		grant(
			"home.notice.edit",
			"HOME",
			"Notice",
			"Edit",
			Implemented,
			&["notice.update"],
			&["home.notice.read"],
		),
		grant(
			"home.workflow.read",
			"HOME",
			"Workflow",
			"Read",
			Implemented,
			&["case.queue_read"],
			&[],
		),
		grant(
			"case.read",
			"CASE",
			"Case",
			"Read",
			Implemented,
			&["case.read", "case.audit_read"],
			&[],
		),
		grant(
			"case.edit",
			"CASE",
			"Case",
			"Edit",
			Implemented,
			&["case.create", "case.update"],
			&["case.read"],
		),
		grant(
			"case.workflow.read",
			"CASE",
			"Workflow",
			"Read",
			Implemented,
			&["case.workflow_read"],
			&[],
		),
		grant(
			"case.review",
			"CASE",
			"QC",
			"Edit",
			Implemented,
			&["case.review"],
			&[],
		),
		grant(
			"case.lock",
			"CASE",
			"Lock",
			"Edit",
			Implemented,
			&["case.lock"],
			&[],
		),
		grant(
			"info.read",
			"INFO",
			"Case Info",
			"Read",
			Implemented,
			&["info.read"],
			&[],
		),
		grant(
			"info.edit",
			"INFO",
			"Case Info",
			"Edit",
			Implemented,
			&["info.update"],
			&["info.read"],
		),
		grant(
			"import.execute",
			"IMPORT",
			"Import Files",
			"Edit",
			Implemented,
			&["import.execute"],
			&[],
		),
		grant(
			"import.history.read",
			"IMPORT",
			"Import History",
			"Read",
			Implemented,
			&["import.history_read"],
			&[],
		),
		grant(
			"submission.execute",
			"EXPORT/SUBMISSION",
			"Export/Submit",
			"Edit",
			Implemented,
			&["submission.execute", "case.export"],
			&[],
		),
		grant(
			"submission.history.read",
			"EXPORT/SUBMISSION",
			"Export/Submit History",
			"Read",
			Implemented,
			&["submission.history_read"],
			&[],
		),
		grant(
			"admin.read",
			"ADMIN",
			"Admin",
			"Read",
			Implemented,
			&[
				"user.read",
				"role.read",
				"organization.read",
				"settings.read",
				"audit.read",
				"terminology.read",
			],
			&[],
		),
		grant(
			"admin.edit",
			"ADMIN",
			"Admin",
			"Edit",
			Implemented,
			&[
				"user.create",
				"user.update",
				"user.delete",
				"user.role_assign",
				"role.manage",
				"role.assign",
				"organization.manage",
				"settings.update",
				"terminology.manage",
			],
			&["admin.read"],
		),
		grant(
			"email.report_due.read",
			"E-mail",
			"Report Due Mail",
			"Read",
			Reserved,
			&[],
			&[],
		),
		grant(
			"email.report_due.send",
			"E-mail",
			"Report Due Mail",
			"Send",
			Reserved,
			&[],
			&[],
		),
	];
	for (index, grant) in grants.iter_mut().enumerate() {
		grant.pdf_order = index as u16 + 1;
		if matches!(grant.id.as_str(), "admin.read" | "admin.edit") {
			grant
				.assignable_role_classes
				.push(RoleClass::PlatformBuiltIn);
		}
	}
	grants
}

fn action(
	id: &str,
	stage: DecisionStage,
	entitlements: &[&str],
	identities: &[BuiltInIdentityKind],
	context_conditions: &[ContextCondition],
	audit: AuditClassification,
) -> ActionPolicyInput {
	ActionPolicyInput {
		id: id.to_string(),
		decision_stage: stage,
		entitlement_rule: EntitlementRule::AllOf,
		entitlements: entitlements
			.iter()
			.map(|value| (*value).to_string())
			.collect(),
		allowed_identities: identities.to_vec(),
		scope_conditions: vec![
			ScopeCondition::ActiveMembership,
			ScopeCondition::AccessWindow,
		],
		context_conditions: context_conditions.to_vec(),
		audit_classification: audit,
	}
}

fn canonical_actions() -> Vec<ActionPolicyInput> {
	use AuditClassification::{Mutation, PrivilegedMutation, PrivilegedRead, Read};
	use BuiltInIdentityKind::{
		PlatformAdministrator, SponsorCompanyAdministrator, SponsorCroAdministrator,
	};
	use ContextCondition::{
		CompatibleLifecycle, EveryTargetAuthorized, ParentAuthorized,
		SameOrganization, WithinPrincipalScope,
	};
	use ContextKind::{Collection, Existing, Parent, Proposed, ResourceSet};
	let administrators = [
		PlatformAdministrator,
		SponsorCroAdministrator,
		SponsorCompanyAdministrator,
	];
	vec![
		action(
			"application.branding.read",
			DecisionStage::SubjectOnly,
			&[],
			&[],
			&[],
			Read,
		),
		action(
			"case.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Case)),
			&["case.read"],
			&[],
			&[WithinPrincipalScope],
			Read,
		),
		action(
			"case.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.read"],
			&[],
			&[SameOrganization, WithinPrincipalScope],
			Read,
		),
		action(
			"case.create",
			DecisionStage::ContextRequired(Proposed(ProposalKind::CaseCreate)),
			&["case.create"],
			&[],
			&[WithinPrincipalScope],
			Mutation,
		),
		action(
			"case.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.update"],
			&[],
			&[SameOrganization, WithinPrincipalScope, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.delete",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.update"],
			&administrators,
			&[SameOrganization, CompatibleLifecycle],
			PrivilegedMutation,
		),
		action(
			"case.review.toggle",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.review"],
			&[],
			&[SameOrganization, WithinPrincipalScope, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.lock.toggle",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.lock"],
			&[],
			&[SameOrganization, WithinPrincipalScope, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.validate",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.review"],
			&[],
			&[SameOrganization, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.audit.list",
			DecisionStage::ContextRequired(Parent {
				parent: ResourceKind::Case,
				child: ResourceKind::CaseAuditTrail,
			}),
			&["case.audit_read"],
			&[],
			&[ParentAuthorized],
			Read,
		),
		action(
			"case.child.read",
			DecisionStage::ContextRequired(Parent {
				parent: ResourceKind::Case,
				child: ResourceKind::CaseChild,
			}),
			&["case.read"],
			&[],
			&[ParentAuthorized],
			Read,
		),
		action(
			"case.child.update",
			DecisionStage::ContextRequired(Parent {
				parent: ResourceKind::Case,
				child: ResourceKind::CaseChild,
			}),
			&["case.update"],
			&[],
			&[ParentAuthorized, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.workflow.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.workflow_read"],
			&[],
			&[SameOrganization],
			Read,
		),
		action(
			"case.workflow.transition",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["case.update"],
			&[],
			&[SameOrganization, CompatibleLifecycle],
			Mutation,
		),
		action(
			"case.export.xml_set",
			DecisionStage::ContextRequired(ResourceSet(ResourceKind::Case)),
			&["case.export"],
			&[],
			&[EveryTargetAuthorized],
			Read,
		),
		action(
			"info.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Presave)),
			&["info.read"],
			&[],
			&[WithinPrincipalScope],
			Read,
		),
		action(
			"info.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Presave)),
			&["info.read"],
			&[],
			&[SameOrganization, WithinPrincipalScope],
			Read,
		),
		action(
			"info.create",
			DecisionStage::ContextRequired(Proposed(ProposalKind::PresaveCreate)),
			&["info.update"],
			&[],
			&[WithinPrincipalScope],
			Mutation,
		),
		action(
			"info.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Presave)),
			&["info.update"],
			&[],
			&[SameOrganization, WithinPrincipalScope],
			Mutation,
		),
		action(
			"import.history.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::ImportHistory)),
			&["import.history_read"],
			&[],
			&[SameOrganization],
			Read,
		),
		action(
			"import.history.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::ImportHistory)),
			&["import.history_read"],
			&[],
			&[SameOrganization],
			Read,
		),
		action(
			"import.xml.validate",
			DecisionStage::ContextRequired(Proposed(ProposalKind::XmlImportBatch)),
			&["import.execute"],
			&[],
			&[WithinPrincipalScope],
			Mutation,
		),
		action(
			"import.xml.execute",
			DecisionStage::ContextRequired(Proposed(ProposalKind::XmlImportBatch)),
			&["import.execute"],
			&[],
			&[WithinPrincipalScope],
			Mutation,
		),
		action(
			"submission.history.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Submission)),
			&["submission.history_read"],
			&[],
			&[SameOrganization],
			Read,
		),
		action(
			"submission.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Submission)),
			&["submission.history_read"],
			&[],
			&[SameOrganization],
			Read,
		),
		action(
			"submission.execute",
			DecisionStage::ContextRequired(Existing(ResourceKind::Case)),
			&["submission.execute"],
			&[],
			&[SameOrganization, WithinPrincipalScope, CompatibleLifecycle],
			Mutation,
		),
		action(
			"user.profile.read",
			DecisionStage::SubjectOnly,
			&[],
			&[],
			&[],
			Read,
		),
		action(
			"user.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::User)),
			&["user.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"user.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::User)),
			&["user.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"user.create",
			DecisionStage::ContextRequired(Proposed(ProposalKind::UserCreate)),
			&["user.create"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"user.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::User)),
			&["user.update"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"user.update.role_assignment",
			DecisionStage::ContextRequired(Existing(ResourceKind::User)),
			&["user.role_assign"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"user.delete",
			DecisionStage::ContextRequired(Existing(ResourceKind::User)),
			&["user.delete"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"role.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Role)),
			&["role.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"role.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Role)),
			&["role.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"role.create",
			DecisionStage::ContextRequired(Proposed(ProposalKind::RoleCreate)),
			&["role.manage"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"role.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Role)),
			&["role.manage"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"role.delete",
			DecisionStage::ContextRequired(Existing(ResourceKind::Role)),
			&["role.manage"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"role.restore",
			DecisionStage::ContextRequired(Existing(ResourceKind::Role)),
			&["role.manage"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"organization.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Organization)),
			&["organization.read"],
			&[PlatformAdministrator],
			&[],
			PrivilegedRead,
		),
		action(
			"organization.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Organization)),
			&["organization.read"],
			&[PlatformAdministrator],
			&[],
			PrivilegedRead,
		),
		action(
			"organization.create",
			DecisionStage::ContextRequired(Proposed(
				ProposalKind::OrganizationCreate,
			)),
			&["organization.manage"],
			&[PlatformAdministrator],
			&[],
			PrivilegedMutation,
		),
		action(
			"organization.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Organization)),
			&["organization.manage"],
			&[PlatformAdministrator],
			&[],
			PrivilegedMutation,
		),
		action(
			"organization.delete",
			DecisionStage::ContextRequired(Existing(ResourceKind::Organization)),
			&["organization.manage"],
			&[PlatformAdministrator],
			&[],
			PrivilegedMutation,
		),
		action(
			"settings.read",
			DecisionStage::ContextRequired(Existing(ResourceKind::Settings)),
			&["settings.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"settings.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Settings)),
			&["settings.update"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"notice.update",
			DecisionStage::ContextRequired(Existing(ResourceKind::Notice)),
			&["notice.update"],
			&administrators,
			&[SameOrganization],
			PrivilegedMutation,
		),
		action(
			"audit_log.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::AuditLog)),
			&["audit.read"],
			&administrators,
			&[SameOrganization],
			PrivilegedRead,
		),
		action(
			"terminology.list",
			DecisionStage::ContextRequired(Collection(ResourceKind::Terminology)),
			&["terminology.read"],
			&[],
			&[],
			Read,
		),
		action(
			"terminology.import",
			DecisionStage::ContextRequired(Proposed(
				ProposalKind::TerminologyImport,
			)),
			&["terminology.manage"],
			&administrators,
			&[],
			PrivilegedMutation,
		),
	]
}

fn canonical_identities() -> Vec<BuiltInIdentityInput> {
	let sponsor_grants = [
		"home.notice.read",
		"home.notice.edit",
		"home.workflow.read",
		"case.read",
		"case.edit",
		"case.workflow.read",
		"case.review",
		"case.lock",
		"info.read",
		"info.edit",
		"import.execute",
		"import.history.read",
		"submission.execute",
		"submission.history.read",
		"admin.edit",
	]
	.into_iter()
	.map(str::to_string)
	.collect::<Vec<_>>();
	vec![
		BuiltInIdentityInput {
			kind: BuiltInIdentityKind::PlatformAdministrator,
			id: "00000000-0000-0000-0000-000000000101".into(),
			stable_key: "platform_administrator".into(),
			role_class: RoleClass::PlatformBuiltIn,
			grants: vec!["admin.edit".into()],
		},
		BuiltInIdentityInput {
			kind: BuiltInIdentityKind::SponsorCroAdministrator,
			id: "00000000-0000-0000-0000-000000000102".into(),
			stable_key: "sponsor_cro_administrator".into(),
			role_class: RoleClass::SponsorCroBuiltIn,
			grants: sponsor_grants.clone(),
		},
		BuiltInIdentityInput {
			kind: BuiltInIdentityKind::SponsorCompanyAdministrator,
			id: "00000000-0000-0000-0000-000000000103".into(),
			stable_key: "sponsor_company_administrator".into(),
			role_class: RoleClass::SponsorCompanyBuiltIn,
			grants: sponsor_grants,
		},
		BuiltInIdentityInput {
			kind: BuiltInIdentityKind::OperationalUser,
			id: "00000000-0000-0000-0000-000000000104".into(),
			stable_key: "operational_user".into(),
			role_class: RoleClass::OperationalBuiltIn,
			grants: Vec::new(),
		},
		BuiltInIdentityInput {
			kind: BuiltInIdentityKind::InternalServicePrincipal,
			id: "00000000-0000-0000-0000-000000000105".into(),
			stable_key: "internal_service_principal".into(),
			role_class: RoleClass::ServiceBuiltIn,
			grants: Vec::new(),
		},
	]
}

fn fact(
	id: &str,
	table: &str,
	columns: &[&str],
	domain: InvalidationDomain,
) -> AuthorizationFactInput {
	AuthorizationFactInput {
		id: id.into(),
		table: table.into(),
		columns: columns.iter().map(|value| (*value).into()).collect(),
		invalidation_domain: domain,
	}
}

fn canonical_facts() -> Vec<AuthorizationFactInput> {
	use InvalidationDomain::{Organization, Principal};
	vec![
		fact(
			"organization.definition",
			"organizations",
			&["active", "org_type"],
			Organization,
		),
		fact(
			"role.definition",
			"authorization_roles",
			&[
				"organization_id",
				"identity_kind",
				"active",
				"built_in",
				"deleted_at",
				"role_class",
				"row_version",
			],
			Organization,
		),
		fact(
			"role.grants",
			"role_grants",
			&["role_id", "grant_id"],
			Organization,
		),
		fact(
			"scope.sender_definition",
			"sender_presaves",
			&["deleted", "organization_id"],
			Organization,
		),
		fact(
			"scope.product_definition",
			"product_presaves",
			&["deleted", "organization_id"],
			Organization,
		),
		fact(
			"scope.study_definition",
			"study_presaves",
			&["deleted", "organization_id"],
			Organization,
		),
		fact(
			"principal.membership",
			"user_organization_memberships",
			&["user_id", "organization_id", "active"],
			Principal,
		),
		fact(
			"principal.role_assignment",
			"user_role_assignments",
			&[
				"user_id",
				"organization_id",
				"role_id",
				"active",
				"row_version",
			],
			Principal,
		),
		fact("principal.active", "users", &["active"], Principal),
		fact(
			"principal.access_window",
			"users",
			&["access_start_at", "access_end_at"],
			Principal,
		),
		fact(
			"principal.scope",
			"users",
			&[
				"access_sender_ids",
				"access_product_ids",
				"access_study_ids",
			],
			Principal,
		),
		fact(
			"principal.blind_access",
			"users",
			&["access_blind_allowed"],
			Principal,
		),
		fact(
			"principal.active_sender",
			"users",
			&["active_sender_identifier"],
			Principal,
		),
	]
}

fn canonical_aliases() -> Vec<LegacyGrantAliasInput> {
	[
		("home_notice.read", "home.notice.read"),
		("home_notice.edit", "home.notice.edit"),
		("home_workflow.read", "home.workflow.read"),
		("case_workflow.read", "case.workflow.read"),
		("case.qc.edit", "case.review"),
		("case.lock.edit", "case.lock"),
		("import.edit", "import.execute"),
		("import.read", "import.history.read"),
		("export_submission.edit", "submission.execute"),
		("export_submission.read", "submission.history.read"),
		("export.edit", "submission.execute"),
		("export.read", "submission.history.read"),
		("submission.edit", "submission.execute"),
		("submission.read", "submission.history.read"),
	]
	.into_iter()
	.map(|(legacy_id, grant_id)| LegacyGrantAliasInput {
		legacy_id: legacy_id.into(),
		grant_id: grant_id.into(),
	})
	.collect()
}
