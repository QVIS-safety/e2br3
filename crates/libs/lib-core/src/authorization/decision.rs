use super::ActionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialReason {
	UnknownAction,
	WrongDecisionStage,
	WrongOperationClass,
	MissingGrant,
	IncompatibleIdentity,
	SameOrganizationRequired,
	OutsidePrincipalScope,
	IncompatibleLifecycle,
	ParentNotAuthorized,
	TargetSetNotAuthorized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationDenial {
	action_id: ActionId,
	reason: DenialReason,
}

impl AuthorizationDenial {
	pub(crate) fn new(action_id: ActionId, reason: DenialReason) -> Self {
		Self { action_id, reason }
	}

	pub fn action_id(&self) -> &ActionId {
		&self.action_id
	}

	pub fn reason(&self) -> DenialReason {
		self.reason
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityDecision {
	Eligible,
	Denied(AuthorizationDenial),
}

impl EligibilityDecision {
	pub fn is_eligible(&self) -> bool {
		matches!(self, Self::Eligible)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationDecision {
	Allowed,
	Denied(AuthorizationDenial),
}

impl AuthorizationDecision {
	pub fn is_allowed(&self) -> bool {
		matches!(self, Self::Allowed)
	}
}
