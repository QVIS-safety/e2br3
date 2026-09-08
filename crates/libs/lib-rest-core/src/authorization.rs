use crate::{case_write_block_reason_for_case, Error, Result};
use lib_core::authorization::{
	authorize_contextual_mutation, authorize_contextual_read, authorize_subject,
	existing_notice_context, policy_registry, AuditLogResource,
	AuthorizationContext, AuthorizationDenial, AuthorizedMutation, AuthorizedRead,
	AuthorizedSubject, CaseAuditTrailResource, CaseChildResource,
	CaseCreateProposal, CaseResource, Collection, EnforcedScopeFilter, Existing,
	ImportHistoryResource, NoticeResource, Parent, PolicySnapshotVersion,
	PresaveCreateProposal, PresaveResource, Proposed, RequestAuthorizationSnapshot,
	ResourceSet, SettingsResource, SubmissionResource, TerminologyImportProposal,
	TerminologyResource, UserResource, XmlImportBatchProposal,
};
use lib_core::ctx::{Ctx, ROLE_SYSTEM_ADMIN};
use lib_core::model::authorization::{
	AuthorizationFactLoadError, AuthorizationFactLoader, CaseMutationKind,
	PresaveAuthorizationKind,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::ModelManager;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub fn denied(denial: AuthorizationDenial) -> Error {
	Error::PermissionDenied {
		required_permission: format!(
			"{} ({:?})",
			denial.action_id(),
			denial.reason()
		),
	}
}

pub fn notice_read_allowed(snapshot: &RequestAuthorizationSnapshot) -> bool {
	let Some(action) =
		policy_registry().context_action::<Existing<NoticeResource>>("notice.read")
	else {
		return false;
	};
	authorize_contextual_read(
		action,
		snapshot,
		existing_notice_context(snapshot.organization_id()),
	)
	.is_ok()
}

trait RequestBoundPermit {
	fn principal_id(&self) -> Uuid;
	fn organization_id(&self) -> Uuid;
	fn snapshot_version(&self) -> &PolicySnapshotVersion;
}

trait PermitEvidence: RequestBoundPermit {
	fn target_organization_id(&self) -> Option<Uuid>;
}

impl RequestBoundPermit for AuthorizedSubject {
	fn principal_id(&self) -> Uuid {
		self.principal_id()
	}
	fn organization_id(&self) -> Uuid {
		self.organization_id()
	}
	fn snapshot_version(&self) -> &PolicySnapshotVersion {
		self.snapshot_version()
	}
}

impl<C: AuthorizationContext> RequestBoundPermit for AuthorizedRead<'_, C> {
	fn principal_id(&self) -> Uuid {
		self.principal_id()
	}
	fn organization_id(&self) -> Uuid {
		self.organization_id()
	}
	fn snapshot_version(&self) -> &PolicySnapshotVersion {
		self.snapshot_version()
	}
}

impl<C: AuthorizationContext> PermitEvidence for AuthorizedRead<'_, C> {
	fn target_organization_id(&self) -> Option<Uuid> {
		self.target_organization_id()
	}
}

impl<C: AuthorizationContext> RequestBoundPermit for AuthorizedMutation<'_, C> {
	fn principal_id(&self) -> Uuid {
		self.principal_id()
	}
	fn organization_id(&self) -> Uuid {
		self.organization_id()
	}
	fn snapshot_version(&self) -> &PolicySnapshotVersion {
		self.snapshot_version()
	}
}

impl<C: AuthorizationContext> PermitEvidence for AuthorizedMutation<'_, C> {
	fn target_organization_id(&self) -> Option<Uuid> {
		self.target_organization_id()
	}
}

pub fn rls_ctx_for_authorized_subject(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &AuthorizedSubject,
) -> Result<Ctx> {
	validate_request_binding(request_ctx, snapshot, permit)?;
	Ok(request_ctx.clone())
}

async fn begin_fact_transaction<'a>(
	request_ctx: &Ctx,
	mm: &'a ModelManager,
) -> Result<&'a lib_core::model::store::dbx::Dbx> {
	let dbx = mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	if let Err(error) = set_full_context_from_ctx_dbx(dbx, request_ctx).await {
		let _ = dbx.rollback_txn().await;
		return Err(error.into());
	}
	Ok(dbx)
}

pub async fn with_authorized_subject_action<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	action_id: &'static str,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let action =
			policy_registry().subject_action(action_id).ok_or_else(|| {
				Error::AccessDenied {
					required_role: format!("registered {action_id} action"),
				}
			})?;
		let permit = authorize_subject(action, snapshot).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_subject(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_user_mutation<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	user_id: Uuid,
	ordinary_action_id: &'static str,
	protected_action_id: &'static str,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let (context, protected_administrator) =
			AuthorizationFactLoader::new(dbx, snapshot)
				.user_for_mutation(user_id)
				.await
				.map_err(map_fact_load_error)?;
		let action_id = if protected_administrator {
			protected_action_id
		} else {
			ordinary_action_id
		};
		let action = policy_registry()
			.context_action::<Existing<UserResource>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot).case_collection();
		let action = policy_registry()
			.context_action::<Collection<CaseResource>>("case.list")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.list action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		let scope =
			permit
				.enforced_scope_filter()
				.ok_or_else(|| Error::AccessDenied {
					required_role: "case scope filter".to_string(),
				})?;
		operation(&authorized_ctx, mm, scope).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_settings_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).settings_existing();
		let action = policy_registry()
			.context_action::<Existing<SettingsResource>>("settings.read")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered settings.read action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_settings_update<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.settings_for_mutation()
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<SettingsResource>>("settings.update")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered settings.update action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_notice_update<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.notice_for_mutation()
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<NoticeResource>>("notice.update")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered notice.update action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_audit_log_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).audit_log_collection();
		let action = policy_registry()
			.context_action::<Collection<AuditLogResource>>("audit_log.list")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered audit_log.list action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_terminology_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).terminology_collection();
		let action = policy_registry()
			.context_action::<Collection<TerminologyResource>>("terminology.list")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered terminology.list action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_terminology_mutation<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let authorization_result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.terminology_import_for_mutation(fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Proposed<TerminologyImportProposal>>(
				"terminology.import",
			)
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered terminology.import action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)
	}
	.await;
	let authorized_ctx = finish_fact_transaction(dbx, authorization_result).await?;
	operation(&authorized_ctx, mm).await
}

pub async fn with_authorized_presave_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	with_authorized_presave_collection_action(
		request_ctx,
		snapshot,
		mm,
		"info.list",
		operation,
	)
	.await
}

pub async fn with_authorized_presave_collection_action<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	action_id: &'static str,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).presave_collection();
		let action = policy_registry()
			.context_action::<Collection<PresaveResource>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		let scope =
			permit
				.enforced_scope_filter()
				.ok_or_else(|| Error::AccessDenied {
					required_role: "presave scope filter".to_string(),
				})?;
		operation(&authorized_ctx, mm, scope).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_presave_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	kind: PresaveAuthorizationKind,
	id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.presave_existing(kind, id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<PresaveResource>>("info.read")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered info.read action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_presave_create<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let authorization_result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.presave_create_for_mutation(fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Proposed<PresaveCreateProposal>>("info.create")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered info.create action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)
	}
	.await;
	let authorized_ctx = finish_fact_transaction(dbx, authorization_result).await?;
	operation(&authorized_ctx, mm).await
}

pub async fn with_authorized_presave_atomic_create<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.presave_create_for_mutation(fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Proposed<PresaveCreateProposal>>("info.create")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered info.create action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_presave_update<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	kind: PresaveAuthorizationKind,
	id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let authorization_result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.presave_for_mutation(kind, id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<PresaveResource>>("info.update")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered info.update action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)
	}
	.await;
	let authorized_ctx = finish_fact_transaction(dbx, authorization_result).await?;
	operation(&authorized_ctx, mm).await
}

pub async fn with_authorized_presave_atomic_update<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	kind: PresaveAuthorizationKind,
	id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.presave_for_mutation(kind, id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<PresaveResource>>("info.update")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered info.update action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_audit_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.case_audit_trail(case_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Parent<CaseResource, CaseAuditTrailResource>>(
				"case.audit.list",
			)
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.audit.list action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_export_history_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot).case_collection();
		let action = policy_registry()
			.context_action::<Collection<CaseResource>>("case.export.history.list")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.export.history.list action"
					.to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		let scope =
			permit
				.enforced_scope_filter()
				.ok_or_else(|| Error::AccessDenied {
					required_role: "export history scope filter".to_string(),
				})?;
		operation(&authorized_ctx, mm, scope).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_export<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_ids: &[Uuid],
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.case_resource_set(case_ids)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<ResourceSet<CaseResource>>("case.export.xml_set")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.export.xml_set action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_export_history_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	history_id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let loader = AuthorizationFactLoader::new(dbx, snapshot);
		let case_id = loader
			.export_history_parent_case_id(history_id)
			.await
			.map_err(map_fact_load_error)?;
		let context = loader
			.case_existing(case_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<CaseResource>>("case.export.history.read")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.export.history.read action"
					.to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_submission_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	with_authorized_submission_collection_action(
		request_ctx,
		snapshot,
		mm,
		"submission.history.list",
		operation,
	)
	.await
}

pub async fn with_authorized_submission_collection_action<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	action_id: &'static str,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).submission_collection();
		let action = policy_registry()
			.context_action::<Collection<SubmissionResource>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_import_history_collection<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context =
			AuthorizationFactLoader::new(dbx, snapshot).import_history_collection();
		let action = policy_registry()
			.context_action::<Collection<ImportHistoryResource>>(
				"import.history.list",
			)
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered import.history.list action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		let scope =
			permit
				.enforced_scope_filter()
				.ok_or_else(|| Error::AccessDenied {
					required_role: "import history scope filter".to_string(),
				})?;
		operation(&authorized_ctx, mm, scope).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_import_history_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	history_id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.import_history_existing(history_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<ImportHistoryResource>>("import.history.read")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered import.history.read action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_xml_import<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	action_id: &'static str,
	fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
		&'ctx EnforcedScopeFilter,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.xml_import_batch_for_mutation(fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Proposed<XmlImportBatchProposal>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		let scope =
			permit
				.enforced_scope_filter()
				.ok_or_else(|| Error::AccessDenied {
					required_role: "XML import scope filter".to_string(),
				})?;
		operation(&authorized_ctx, mm, scope).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_submission_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	submission_id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.submission_existing(submission_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<SubmissionResource>>("submission.read")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered submission.read action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_submission_mutation<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	submission_id: Uuid,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.submission_parent_case_for_mutation(submission_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<CaseResource>>("submission.execute")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered submission.execute action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_id: Uuid,
	action_id: &'static str,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.case_existing(case_id)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<CaseResource>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_create<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	product_key: Option<&str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let loader = AuthorizationFactLoader::new(dbx, snapshot);
	if let Err(error) = loader.lock_and_verify_revisions().await {
		let _ = dbx.rollback_txn().await;
		return Err(map_fact_load_error(error));
	}
	let result = async {
		let context = loader
			.case_create_for_verified_mutation(
				request_ctx.organization_id(),
				product_key,
			)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Proposed<CaseCreateProposal>>("case.create")
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.create action".to_string(),
			})?;
		let permit = authorize_contextual_mutation(action, snapshot, context)
			.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_mutation<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_id: Uuid,
	action_id: &'static str,
	mutation_kind: CaseMutationKind,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let loader = AuthorizationFactLoader::new(dbx, snapshot);
	if let Err(error) = loader.lock_and_verify_revisions().await {
		let _ = dbx.rollback_txn().await;
		return Err(map_fact_load_error(error));
	}
	let result = async {
		let context = loader
			.case_existing_for_verified_mutation(case_id, mutation_kind)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Existing<CaseResource>>(action_id)
			.ok_or_else(|| Error::AccessDenied {
				required_role: format!("registered {action_id} action"),
			})?;
		let permit =
			authorize_contextual_mutation(action.clone(), snapshot, context)
				.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		let result = operation(&authorized_ctx, mm).await?;
		let context = loader
			.case_existing_scope_postcondition(case_id)
			.await
			.map_err(map_fact_load_error)?;
		authorize_contextual_mutation(action, snapshot, context).map_err(denied)?;
		Ok(result)
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_child_read<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_id: Uuid,
	child_fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let result = async {
		let context = AuthorizationFactLoader::new(dbx, snapshot)
			.case_child(case_id, child_fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Parent<CaseResource, CaseChildResource>>(
				"case.child.read",
			)
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.child.read action".to_string(),
			})?;
		let permit =
			authorize_contextual_read(action, snapshot, context).map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_read(request_ctx, snapshot, &permit)?;
		operation(&authorized_ctx, mm).await
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

pub async fn with_authorized_case_child_mutation<T, F>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	mm: &ModelManager,
	case_id: Uuid,
	child_fingerprint: impl AsRef<str>,
	operation: F,
) -> Result<T>
where
	F: for<'ctx> FnOnce(
		&'ctx Ctx,
		&'ctx ModelManager,
	) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'ctx>>,
{
	let child_fingerprint = child_fingerprint.as_ref().to_owned();
	let dbx = begin_fact_transaction(request_ctx, mm).await?;
	let loader = AuthorizationFactLoader::new(dbx, snapshot);
	if let Err(error) = loader.lock_and_verify_revisions().await {
		let _ = dbx.rollback_txn().await;
		return Err(map_fact_load_error(error));
	}
	let result = async {
		let context = loader
			.case_child_for_verified_mutation(case_id, &child_fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		let action = policy_registry()
			.context_action::<Parent<CaseResource, CaseChildResource>>(
				"case.child.update",
			)
			.ok_or_else(|| Error::AccessDenied {
				required_role: "registered case.child.update action".to_string(),
			})?;
		let permit =
			authorize_contextual_mutation(action.clone(), snapshot, context)
				.map_err(denied)?;
		let authorized_ctx =
			rls_ctx_for_authorized_mutation(request_ctx, snapshot, &permit)?;
		let case = CaseBmc::get(&authorized_ctx, mm, case_id).await?;
		if let Some(reason) =
			case_write_block_reason_for_case(&authorized_ctx, mm, &case).await?
		{
			return Err(Error::BadRequest {
				message: reason.message,
			});
		}
		let result = operation(&authorized_ctx, mm).await?;
		let context = loader
			.case_child_scope_postcondition(case_id, child_fingerprint)
			.await
			.map_err(map_fact_load_error)?;
		authorize_contextual_mutation(action, snapshot, context).map_err(denied)?;
		CaseValidationSummaryBmc::mark_stale_for_case(&authorized_ctx, mm, case_id)
			.await?;
		Ok(result)
	}
	.await;
	finish_fact_transaction(dbx, result).await
}

async fn finish_fact_transaction<T>(
	dbx: &lib_core::model::store::dbx::Dbx,
	result: Result<T>,
) -> Result<T> {
	match result {
		Ok(ctx) => {
			dbx.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Ok(ctx)
		}
		Err(error) => {
			let _ = dbx.rollback_txn().await;
			Err(error)
		}
	}
}

fn map_fact_load_error(error: AuthorizationFactLoadError) -> Error {
	match error {
		AuthorizationFactLoadError::Database(error) => {
			Error::Model(lib_core::model::Error::Dbx(error))
		}
		AuthorizationFactLoadError::FactNotFound { fact, id } => {
			Error::Model(lib_core::model::Error::EntityUuidNotFound {
				entity: fact,
				id,
			})
		}
		AuthorizationFactLoadError::SnapshotStale { .. } => {
			Error::PermissionDenied {
				required_permission: "AUTHORIZATION_SNAPSHOT_STALE".to_string(),
			}
		}
	}
}

pub fn rls_ctx_for_authorized_read<C: AuthorizationContext>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &AuthorizedRead<'_, C>,
) -> Result<Ctx> {
	rls_ctx_from_permit(request_ctx, snapshot, permit)
}

pub fn rls_ctx_for_authorized_mutation<C: AuthorizationContext>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &AuthorizedMutation<'_, C>,
) -> Result<Ctx> {
	rls_ctx_from_permit(request_ctx, snapshot, permit)
}

fn rls_ctx_from_permit(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &impl PermitEvidence,
) -> Result<Ctx> {
	validate_request_binding(request_ctx, snapshot, permit)?;
	let Some(target_organization_id) = permit.target_organization_id() else {
		if snapshot.identity().is_platform_administrator() {
			return Ok(request_ctx.clone());
		}
		return Err(Error::AccessDenied {
			required_role: "authorization permit with target organization"
				.to_string(),
		});
	};
	if target_organization_id == request_ctx.organization_id() {
		return Ok(request_ctx.clone());
	}
	if !snapshot.identity().is_platform_administrator() {
		return Err(Error::AccessDenied {
			required_role: "platform administrator cross-organization permit"
				.to_string(),
		});
	}
	Ctx::new(
		request_ctx.user_id(),
		target_organization_id,
		ROLE_SYSTEM_ADMIN.to_string(),
	)
	.map(|ctx| {
		ctx.with_compliance(
			request_ctx.change_reason().map(ToString::to_string),
			request_ctx.e_signature_id(),
		)
		.with_change_category(request_ctx.change_category().map(ToString::to_string))
	})
	.map_err(|_| Error::BadRequest {
		message: "invalid authorized target organization context".to_string(),
	})
}

fn validate_request_binding(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &impl RequestBoundPermit,
) -> Result<()> {
	if permit.principal_id() != request_ctx.user_id()
		|| permit.principal_id() != snapshot.principal_id()
		|| permit.organization_id() != snapshot.organization_id()
		|| request_ctx.organization_id() != snapshot.organization_id()
		|| permit.snapshot_version() != snapshot.version()
	{
		return Err(Error::AccessDenied {
			required_role: "authorization permit bound to this request".to_string(),
		});
	}
	Ok(())
}
