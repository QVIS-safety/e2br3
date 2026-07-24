use super::{
	policy_registry, ActionId, AuditClassification, AuthorizationContext,
	AuthorizationDecision, AuthorizationDenial, AuthorizedMutation, AuthorizedRead,
	ContextActionId, ContextCondition, ContextSnapshot, DecisionStage, DenialReason,
	EligibilityDecision, EvaluatedContext, LockedMutationContext,
	RequestAuthorizationSnapshot, SubjectActionId,
};
use crate::model::acs::{has_permission, Permission};

/// Temporary compatibility entry point for routes that have not yet adopted a
/// typed canonical action. Legacy permissions are generated one-way from the
/// canonical grants; web code must not evaluate them independently.
pub fn legacy_permission_allowed(
	permission_subject: &str,
	permission: Permission,
) -> bool {
	has_permission(permission_subject, permission)
}

pub fn check_eligibility(
	action_id: &ActionId,
	snapshot: &RequestAuthorizationSnapshot,
) -> EligibilityDecision {
	let Some(policy) = policy_registry().action(action_id.as_str()) else {
		return EligibilityDecision::Denied(AuthorizationDenial::new(
			action_id.clone(),
			DenialReason::UnknownAction,
		));
	};
	if !policy.allowed_identities.is_empty()
		&& !snapshot
			.identity()
			.built_in_kind()
			.is_some_and(|identity| policy.allowed_identities.contains(&identity))
	{
		return EligibilityDecision::Denied(AuthorizationDenial::new(
			action_id.clone(),
			DenialReason::IncompatibleIdentity,
		));
	}
	if !policy
		.required_grants
		.iter()
		.all(|grant| snapshot.grants().contains(grant))
	{
		return EligibilityDecision::Denied(AuthorizationDenial::new(
			action_id.clone(),
			DenialReason::MissingGrant,
		));
	}
	EligibilityDecision::Eligible
}

pub fn authorize_subject(
	action: SubjectActionId,
	snapshot: &RequestAuthorizationSnapshot,
) -> AuthorizationDecision {
	match check_eligibility(action.as_action_id(), snapshot) {
		EligibilityDecision::Eligible => AuthorizationDecision::Allowed,
		EligibilityDecision::Denied(denial) => AuthorizationDecision::Denied(denial),
	}
}

pub fn authorize_contextual_read<'tx, C: AuthorizationContext>(
	action: ContextActionId<C>,
	snapshot: &RequestAuthorizationSnapshot,
	context: ContextSnapshot<'tx, C>,
) -> Result<AuthorizedRead<'tx, C>, AuthorizationDenial> {
	let action_id = action.as_action_id().clone();
	let policy = contextual_policy::<C>(&action_id)?;
	if !matches!(
		policy.audit_classification,
		AuditClassification::Read | AuditClassification::PrivilegedRead
	) {
		return Err(AuthorizationDenial::new(
			action_id,
			DenialReason::WrongOperationClass,
		));
	}
	check_context(&action_id, snapshot, &context.evaluated)?;
	Ok(AuthorizedRead::new(
		action_id,
		snapshot.principal_id(),
		snapshot.organization_id(),
		context.evaluated.organization_id,
		context.evaluated.target_fingerprint,
		snapshot.version().clone(),
		snapshot.evaluated_at(),
		context.evaluated.enforced_scope_filter,
	))
}

pub fn authorize_contextual_mutation<'tx, C: AuthorizationContext>(
	action: ContextActionId<C>,
	snapshot: &RequestAuthorizationSnapshot,
	context: LockedMutationContext<'tx, C>,
) -> Result<AuthorizedMutation<'tx, C>, AuthorizationDenial> {
	let action_id = action.as_action_id().clone();
	let policy = contextual_policy::<C>(&action_id)?;
	if !matches!(
		policy.audit_classification,
		AuditClassification::Mutation | AuditClassification::PrivilegedMutation
	) {
		return Err(AuthorizationDenial::new(
			action_id,
			DenialReason::WrongOperationClass,
		));
	}
	check_context(&action_id, snapshot, &context.evaluated)?;
	Ok(AuthorizedMutation::new(
		action_id,
		snapshot.principal_id(),
		snapshot.organization_id(),
		context.evaluated.organization_id,
		context.evaluated.target_fingerprint,
		snapshot.version().clone(),
		snapshot.evaluated_at(),
	))
}

fn contextual_policy<C: AuthorizationContext>(
	action_id: &ActionId,
) -> Result<&'static super::ActionPolicy, AuthorizationDenial> {
	let policy = policy_registry()
		.action(action_id.as_str())
		.ok_or_else(|| {
			AuthorizationDenial::new(action_id.clone(), DenialReason::UnknownAction)
		})?;
	if policy.decision_stage != DecisionStage::ContextRequired(C::kind()) {
		return Err(AuthorizationDenial::new(
			action_id.clone(),
			DenialReason::WrongDecisionStage,
		));
	}
	Ok(policy)
}

fn check_context(
	action_id: &ActionId,
	snapshot: &RequestAuthorizationSnapshot,
	context: &EvaluatedContext,
) -> Result<(), AuthorizationDenial> {
	if let EligibilityDecision::Denied(denial) =
		check_eligibility(action_id, snapshot)
	{
		return Err(denial);
	}
	let policy = policy_registry()
		.action(action_id.as_str())
		.expect("context action was resolved before evaluation");
	for condition in &policy.context_conditions {
		let denied = match condition {
			ContextCondition::SameOrganization
				if context.organization_id != Some(snapshot.organization_id())
					&& !snapshot.identity().is_platform_administrator() =>
			{
				Some(DenialReason::SameOrganizationRequired)
			}
			ContextCondition::WithinPrincipalScope
				if !context.within_principal_scope =>
			{
				Some(DenialReason::OutsidePrincipalScope)
			}
			ContextCondition::CompatibleLifecycle
				if !context.lifecycle_compatible =>
			{
				Some(DenialReason::IncompatibleLifecycle)
			}
			ContextCondition::ParentAuthorized if !context.parent_authorized => {
				Some(DenialReason::ParentNotAuthorized)
			}
			ContextCondition::EveryTargetAuthorized
				if !context.every_target_authorized =>
			{
				Some(DenialReason::TargetSetNotAuthorized)
			}
			_ => None,
		};
		if let Some(reason) = denied {
			return Err(AuthorizationDenial::new(action_id.clone(), reason));
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authorization::{
		BuiltInIdentityKind, CaseResource, EnforcedScopeFilter, GrantId,
		IdentityTraits, PolicySnapshotVersion, PrincipalScope,
	};
	use std::collections::BTreeSet;
	use time::OffsetDateTime;
	use uuid::Uuid;

	fn snapshot(
		grants: &[&str],
		identity: Option<BuiltInIdentityKind>,
	) -> RequestAuthorizationSnapshot {
		let organization_id = Uuid::new_v4();
		RequestAuthorizationSnapshot::new(
			Uuid::new_v4(),
			organization_id,
			Uuid::new_v4(),
			IdentityTraits::new(identity),
			grants
				.iter()
				.map(|value| GrantId::parse(*value).unwrap())
				.collect::<BTreeSet<_>>(),
			PrincipalScope::new(Vec::new(), Vec::new(), Vec::new(), false, None),
			PolicySnapshotVersion::new("a".repeat(64), organization_id, 1, 1),
			OffsetDateTime::now_utc(),
			None,
			"test".into(),
		)
	}

	fn evaluated(
		snapshot: &RequestAuthorizationSnapshot,
		lifecycle_compatible: bool,
	) -> EvaluatedContext {
		EvaluatedContext {
			organization_id: Some(snapshot.organization_id()),
			target_fingerprint: "case:42:v7".into(),
			within_principal_scope: true,
			lifecycle_compatible,
			parent_authorized: true,
			every_target_authorized: true,
			enforced_scope_filter: Some(EnforcedScopeFilter::new(
				Vec::new(),
				Vec::new(),
				Vec::new(),
				false,
			)),
		}
	}

	#[test]
	fn case_review_requires_grant_and_compatible_lifecycle() {
		type Case = crate::authorization::Existing<CaseResource>;
		let action = policy_registry()
			.context_action::<Case>("case.review.toggle")
			.unwrap();
		let missing = snapshot(&[], None);
		let denial = authorize_contextual_mutation(
			action.clone(),
			&missing,
			LockedMutationContext::new(evaluated(&missing, true)),
		)
		.unwrap_err();
		assert_eq!(denial.reason(), DenialReason::MissingGrant);

		let reviewer = snapshot(&["case.review"], None);
		let denial = authorize_contextual_mutation(
			action,
			&reviewer,
			LockedMutationContext::new(evaluated(&reviewer, false)),
		)
		.unwrap_err();
		assert_eq!(denial.reason(), DenialReason::IncompatibleLifecycle);
	}

	#[test]
	fn custom_admin_edit_can_update_users_but_cannot_assign_roles() {
		type User =
			crate::authorization::Existing<crate::authorization::UserResource>;
		let custom = snapshot(&["admin.edit"], None);
		let action = policy_registry()
			.context_action::<User>("user.update")
			.unwrap();
		assert!(authorize_contextual_mutation(
			action,
			&custom,
			LockedMutationContext::new(evaluated(&custom, true)),
		)
		.is_ok());

		let role_assignment = policy_registry()
			.context_action::<User>("user.update.role_assignment")
			.unwrap();
		let denial = authorize_contextual_mutation(
			role_assignment,
			&custom,
			LockedMutationContext::new(evaluated(&custom, true)),
		)
		.unwrap_err();
		assert_eq!(denial.reason(), DenialReason::IncompatibleIdentity);
	}

	#[test]
	fn custom_pdf_grants_are_not_restricted_to_built_in_administrators() {
		let custom = snapshot(
			&[
				"admin.read",
				"admin.edit",
				"home.notice.read",
				"home.notice.edit",
			],
			None,
		);
		for action_id in [
			"settings.read",
			"settings.update",
			"notice.update",
			"audit_log.list",
			"terminology.import",
		] {
			let action_id = ActionId::parse(action_id).unwrap();
			assert!(
				check_eligibility(&action_id, &custom).is_eligible(),
				"{action_id} rejected a custom role that owns its PDF grant"
			);
		}
	}

	#[test]
	fn only_platform_identity_can_cross_organization_boundaries() {
		type Users =
			crate::authorization::Collection<crate::authorization::UserResource>;
		let action = policy_registry()
			.context_action::<Users>("user.list")
			.unwrap();
		let mut custom = snapshot(&["admin.read"], None);
		let other_organization = Uuid::new_v4();
		let mut cross_org = evaluated(&custom, true);
		cross_org.organization_id = Some(other_organization);
		let denial = authorize_contextual_read(
			action.clone(),
			&custom,
			ContextSnapshot::new(cross_org),
		)
		.unwrap_err();
		assert_eq!(denial.reason(), DenialReason::SameOrganizationRequired);

		custom = snapshot(
			&["admin.read"],
			Some(BuiltInIdentityKind::PlatformAdministrator),
		);
		let mut cross_org = evaluated(&custom, true);
		cross_org.organization_id = Some(other_organization);
		let permit = authorize_contextual_read(
			action,
			&custom,
			ContextSnapshot::new(cross_org),
		)
		.unwrap();
		assert_eq!(permit.target_organization_id(), Some(other_organization));
	}

	#[test]
	fn read_permit_preserves_exact_context_evidence() {
		type Cases = crate::authorization::Collection<CaseResource>;
		let action = policy_registry()
			.context_action::<Cases>("case.list")
			.unwrap();
		let reader = snapshot(&["case.read"], None);
		let permit = authorize_contextual_read(
			action,
			&reader,
			ContextSnapshot::new(evaluated(&reader, true)),
		)
		.unwrap();
		assert_eq!(permit.target_fingerprint(), "case:42:v7");
		assert!(permit.enforced_scope_filter().is_some());
		assert_eq!(permit.snapshot_version(), reader.version());
	}
}
