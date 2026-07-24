use crate::authorization::{
	ActionId, DenialReason, PolicySnapshotVersion, RequestAuthorizationSnapshot,
};
use crate::model::store::{
	set_authorization_isolation_context, DatabaseIsolationContext,
};
use crate::model::Error;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditDecision {
	Allowed,
	Denied(DenialReason),
}

#[derive(Debug, Clone)]
pub struct AuthorizationAuditEvent {
	principal_id: Uuid,
	organization_id: Uuid,
	role_id: Uuid,
	action_id: ActionId,
	decision: AuditDecision,
	version: PolicySnapshotVersion,
	target_identifier: Option<String>,
	request_id: Uuid,
	isolation: DatabaseIsolationContext,
}

impl AuthorizationAuditEvent {
	pub fn allowed(
		snapshot: &RequestAuthorizationSnapshot,
		action_id: ActionId,
		request_id: Uuid,
		target_identifier: Option<String>,
	) -> Self {
		Self::new(
			snapshot,
			action_id,
			request_id,
			target_identifier,
			AuditDecision::Allowed,
		)
	}

	pub fn denied(
		snapshot: &RequestAuthorizationSnapshot,
		action_id: ActionId,
		request_id: Uuid,
		target_identifier: Option<String>,
		reason: DenialReason,
	) -> Self {
		Self::new(
			snapshot,
			action_id,
			request_id,
			target_identifier,
			AuditDecision::Denied(reason),
		)
	}

	fn new(
		snapshot: &RequestAuthorizationSnapshot,
		action_id: ActionId,
		request_id: Uuid,
		target_identifier: Option<String>,
		decision: AuditDecision,
	) -> Self {
		Self {
			principal_id: snapshot.principal_id(),
			organization_id: snapshot.organization_id(),
			role_id: snapshot.role_id(),
			action_id,
			decision,
			version: snapshot.version().clone(),
			target_identifier,
			request_id,
			isolation: DatabaseIsolationContext::from_snapshot(snapshot),
		}
	}
}

pub struct AuthorizationAuditRepository;

impl AuthorizationAuditRepository {
	/// Append an allowed decision inside the protected operation's transaction.
	pub async fn append_allowed(
		transaction: &mut Transaction<'_, Postgres>,
		event: &AuthorizationAuditEvent,
	) -> Result<(), Error> {
		if event.decision != AuditDecision::Allowed {
			return Err(Error::Store(
				"Denied authorization audit must be appended after rollback".into(),
			));
		}
		Self::append_record(transaction, event).await
	}

	async fn append_record(
		transaction: &mut Transaction<'_, Postgres>,
		event: &AuthorizationAuditEvent,
	) -> Result<(), Error> {
		let (decision, denial_reason) = decision_fields(event.decision);
		sqlx::query(
			"INSERT INTO authorization_audit_events (principal_id, organization_id, role_id, action_id, decision, denial_reason, catalog_hash, organization_revision, principal_revision, target_identifier, request_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
		)
		.bind(event.principal_id)
		.bind(event.organization_id)
		.bind(event.role_id)
		.bind(event.action_id.as_str())
		.bind(decision)
		.bind(denial_reason)
		.bind(event.version.catalog_hash())
		.bind(event.version.organization_revision())
		.bind(event.version.principal_revision())
		.bind(&event.target_identifier)
		.bind(event.request_id)
		.execute(&mut **transaction)
		.await
		.map_err(|error| {
			Error::Store(format!("Failed to append authorization audit: {error}"))
		})?;
		Ok(())
	}

	/// Append a denial only after the protected transaction has rolled back.
	pub async fn append_denial_after_rollback(
		pool: &Pool<Postgres>,
		event: &AuthorizationAuditEvent,
	) -> Result<(), Error> {
		if !matches!(event.decision, AuditDecision::Denied(_)) {
			return Err(Error::Store(
				"Allowed authorization audit must share the protected transaction"
					.into(),
			));
		}
		let mut transaction = pool.begin().await.map_err(|error| {
			Error::Store(format!("Failed to begin authorization audit: {error}"))
		})?;
		set_authorization_isolation_context(&mut transaction, &event.isolation)
			.await?;
		Self::append_record(&mut transaction, event).await?;
		transaction.commit().await.map_err(|error| {
			Error::Store(format!("Failed to commit authorization audit: {error}"))
		})?;
		Ok(())
	}
}

fn decision_fields(decision: AuditDecision) -> (&'static str, Option<&'static str>) {
	match decision {
		AuditDecision::Allowed => ("allowed", None),
		AuditDecision::Denied(reason) => ("denied", Some(denial_reason(reason))),
	}
}

fn denial_reason(reason: DenialReason) -> &'static str {
	match reason {
		DenialReason::UnknownAction => "unknown_action",
		DenialReason::WrongDecisionStage => "wrong_decision_stage",
		DenialReason::WrongOperationClass => "wrong_operation_class",
		DenialReason::MissingGrant => "missing_grant",
		DenialReason::IncompatibleIdentity => "incompatible_identity",
		DenialReason::SameOrganizationRequired => "same_organization_required",
		DenialReason::OutsidePrincipalScope => "outside_principal_scope",
		DenialReason::IncompatibleLifecycle => "incompatible_lifecycle",
		DenialReason::ParentNotAuthorized => "parent_not_authorized",
		DenialReason::TargetSetNotAuthorized => "target_set_not_authorized",
	}
}
