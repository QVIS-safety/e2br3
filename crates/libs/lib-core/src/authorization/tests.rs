use super::*;
use std::collections::BTreeSet;

#[test]
fn pdf_sensitive_grants_are_explicit() {
	let registry = policy_registry();
	assert_eq!(
		registry.grant("case.review").unwrap().availability,
		Availability::Implemented
	);
	assert_eq!(
		registry.grant("case.lock").unwrap().availability,
		Availability::Implemented
	);
	assert_eq!(
		registry
			.grant("email.report_due.read")
			.unwrap()
			.availability,
		Availability::Reserved
	);
	assert_eq!(
		registry
			.grant("email.report_due.send")
			.unwrap()
			.availability,
		Availability::Reserved
	);
	assert!(registry.grant("settings.read").is_none());
}

#[test]
fn case_review_and_lock_are_independent() {
	let registry = policy_registry();
	let review = registry.grant("case.review").unwrap();
	let lock = registry.grant("case.lock").unwrap();
	assert_eq!(
		review.entitlements,
		[EntitlementId::parse("case.review").unwrap()]
	);
	assert_eq!(
		lock.entitlements,
		[EntitlementId::parse("case.lock").unwrap()]
	);
	assert_ne!(review.entitlements, lock.entitlements);
}

#[test]
fn built_in_identity_uuids_are_fixed_and_unique() {
	let identities = policy_registry().built_in_identities();
	assert_eq!(identities.len(), 5);
	let unique = identities
		.iter()
		.map(|role| role.id)
		.collect::<BTreeSet<_>>();
	assert_eq!(unique.len(), identities.len());
	assert_eq!(
		policy_registry()
			.built_in_identity(BuiltInIdentityKind::PlatformAdministrator)
			.unwrap()
			.id
			.to_string(),
		"00000000-0000-0000-0000-000000000101"
	);
}

#[test]
fn built_in_grants_are_registry_owned_and_pdf_sensitive() {
	let registry = policy_registry();
	let platform = registry
		.built_in_identity(BuiltInIdentityKind::PlatformAdministrator)
		.unwrap();
	assert!(platform
		.grants
		.iter()
		.any(|grant| grant.as_str() == "admin.edit"));
	assert!(platform
		.grants
		.iter()
		.all(|grant| registry.grant(grant.as_str()).is_some()));

	let sponsor = registry
		.built_in_identity(BuiltInIdentityKind::SponsorCroAdministrator)
		.unwrap();
	assert!(sponsor
		.grants
		.iter()
		.any(|grant| grant.as_str() == "case.review"));
	assert!(sponsor
		.grants
		.iter()
		.any(|grant| grant.as_str() == "case.lock"));
	assert!(sponsor.grants.iter().all(|grant| {
		registry.grant(grant.as_str()).unwrap().availability
			== Availability::Implemented
	}));
}

#[test]
fn reserved_grants_are_visible_but_never_assignable() {
	let error = policy_registry()
		.validate_assignable_grants(RoleClass::Custom, ["email.report_due.read"])
		.unwrap_err();
	assert!(matches!(
		error,
		RegistryError::ReservedGrantNotAssignable { .. }
	));
}

#[test]
fn contextual_action_stages_preserve_review_lock_and_audit_boundaries() {
	let registry = policy_registry();
	assert_eq!(
		registry
			.action("case.review.toggle")
			.unwrap()
			.decision_stage,
		DecisionStage::ContextRequired(ContextKind::Existing(ResourceKind::Case))
	);
	assert_eq!(
		registry.action("case.lock.toggle").unwrap().decision_stage,
		DecisionStage::ContextRequired(ContextKind::Existing(ResourceKind::Case))
	);
	assert_eq!(
		registry.action("audit_log.list").unwrap().decision_stage,
		DecisionStage::ContextRequired(ContextKind::Collection(
			ResourceKind::AuditLog
		))
	);
}

#[test]
fn authenticated_profile_action_is_subject_only_without_a_role_grant() {
	let registry = policy_registry();
	let action = registry.action("user.profile.read").unwrap();
	assert_eq!(action.decision_stage, DecisionStage::SubjectOnly);
	assert!(action.entitlements.is_empty());
	assert!(registry.subject_action("user.profile.read").is_some());
}

#[test]
fn every_registered_entitlement_is_reachable_from_a_pdf_grant() {
	let registry = policy_registry();
	let reachable = registry
		.grants()
		.flat_map(|grant| grant.entitlements.iter().cloned())
		.collect::<BTreeSet<_>>();
	let registered = registry
		.entitlements()
		.map(|definition| definition.id.clone())
		.collect::<BTreeSet<_>>();
	assert_eq!(reachable, registered);
}

#[test]
fn implied_grants_expand_in_the_registry_not_in_callers() {
	let effective = policy_registry()
		.effective_entitlements(["admin.edit"])
		.unwrap();
	assert!(effective.iter().any(|id| id.as_str() == "role.manage"));
	assert!(effective.iter().any(|id| id.as_str() == "role.read"));
}

#[test]
fn identifiers_reject_alias_like_or_noncanonical_values() {
	for invalid in [
		"",
		"Case.Read",
		"case/read",
		"case..read",
		"case-read",
		" case.read",
	] {
		assert!(GrantId::parse(invalid).is_err(), "accepted {invalid:?}");
	}
	assert_eq!(GrantId::parse("case.read").unwrap().as_str(), "case.read");
}

#[test]
fn registry_rejects_unknown_entitlements_and_implication_cycles() {
	let unknown = PolicyRegistryBuilder::new()
		.entitlement("known.read")
		.grant(test_grant("known.read", &["missing.read"], &[]))
		.build()
		.unwrap_err();
	assert!(matches!(unknown, RegistryError::UnknownEntitlement { .. }));

	let mut cycle_tail = test_grant("b.read", &[], &["a.read"]);
	cycle_tail.pdf_order = 2;
	let cycle = PolicyRegistryBuilder::new()
		.grant(test_grant("a.read", &[], &["b.read"]))
		.grant(cycle_tail)
		.build()
		.unwrap_err();
	assert!(matches!(cycle, RegistryError::GrantImplicationCycle { .. }));
}

#[test]
fn registry_rejects_missing_or_duplicate_pdf_order() {
	let mut missing = test_grant("missing.read", &[], &[]);
	missing.pdf_order = 0;
	assert!(matches!(
		PolicyRegistryBuilder::new().grant(missing).build(),
		Err(RegistryError::InvalidPdfOrder { .. })
	));

	let duplicate = PolicyRegistryBuilder::new()
		.grant(test_grant("first.read", &[], &[]))
		.grant(test_grant("second.read", &[], &[]))
		.build();
	assert!(matches!(
		duplicate,
		Err(RegistryError::DuplicatePdfOrder { order: 1 })
	));
}

fn test_grant(
	id: &str,
	entitlements: &[&str],
	implied_grants: &[&str],
) -> GrantDefinitionInput {
	GrantDefinitionInput {
		id: id.to_string(),
		pdf_order: 1,
		pdf_menu: "TEST".to_string(),
		pdf_type: "Test".to_string(),
		pdf_privilege: "Read".to_string(),
		availability: Availability::Implemented,
		implied_grants: implied_grants
			.iter()
			.map(|value| value.to_string())
			.collect(),
		entitlements: entitlements.iter().map(|value| value.to_string()).collect(),
		assignable_role_classes: vec![RoleClass::Custom],
	}
}
