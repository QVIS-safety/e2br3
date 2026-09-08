// region:    --- Modules

pub mod authorization;
mod error;
pub mod rest_params;
pub mod rest_result;
mod utils;

pub use self::error::{ConstraintViolation, Error, Result};
pub use authorization::{
	denied as authorization_denied, notice_read_allowed,
	rls_ctx_for_authorized_mutation, rls_ctx_for_authorized_read,
	rls_ctx_for_authorized_subject, with_authorized_audit_log_collection,
	with_authorized_case_audit_read, with_authorized_case_child_mutation,
	with_authorized_case_child_read, with_authorized_case_collection,
	with_authorized_case_create, with_authorized_case_export,
	with_authorized_case_mutation, with_authorized_case_read,
	with_authorized_export_history_collection, with_authorized_export_history_read,
	with_authorized_import_history_collection, with_authorized_import_history_read,
	with_authorized_notice_update, with_authorized_presave_atomic_create,
	with_authorized_presave_atomic_update, with_authorized_presave_collection,
	with_authorized_presave_collection_action, with_authorized_presave_create,
	with_authorized_presave_read, with_authorized_presave_update,
	with_authorized_settings_read, with_authorized_settings_update,
	with_authorized_subject_action, with_authorized_submission_collection,
	with_authorized_submission_collection_action,
	with_authorized_submission_mutation, with_authorized_submission_read,
	with_authorized_terminology_mutation, with_authorized_terminology_read,
	with_authorized_user_mutation, with_authorized_xml_import,
};
pub use rest_params::*;
pub use rest_result::*;

use lib_core::model::store::set_full_context_dbx;

/// Run a database fetch that requires RLS context, wrapped in a transaction.
///
/// Handles `begin_txn → set_full_context_dbx → f(dbx) → commit_txn`, rolling
/// back on any failure. Replaces the repeated 10-line boilerplate across
/// read-only endpoints that need RLS.
pub async fn with_rls_read<T, F>(mm: &ModelManager, ctx: &Ctx, f: F) -> Result<T>
where
	F: for<'a> FnOnce(
		&'a lib_core::model::store::dbx::Dbx,
	) -> std::pin::Pin<
		Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>,
	>,
{
	let dbx = mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(Error::from)?;
	let result = f(dbx).await;
	match result {
		Ok(data) => {
			dbx.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Ok(data)
		}
		Err(err) => {
			let _ = dbx.rollback_txn().await;
			Err(err)
		}
	}
}

use lib_core::ctx::{
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_USER,
};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::Case;
use lib_core::model::user::UserBmc;
use lib_core::model::ModelManager;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::FromRow;
use std::collections::HashSet;
use uuid::Uuid;

/// Returns true when a model error represents a unique-constraint violation.
/// Use this in singleton-create handlers to implement idempotent upsert logic.
pub fn is_unique_violation(err: &lib_core::model::Error) -> bool {
	use std::borrow::Cow;
	matches!(err, lib_core::model::Error::UniqueViolation { .. })
		|| matches!(
			err.as_database_error().and_then(|db| db.code()),
			Some(Cow::Borrowed("23505"))
		) || {
		let text = format!("{err:?}").to_ascii_lowercase();
		text.contains("duplicate") || text.contains("unique")
	}
}

#[cfg(test)]
#[test]
fn unique_violation_preserves_typed_and_text_fallbacks() {
	use lib_core::model::Error as ModelError;
	assert!(is_unique_violation(&ModelError::UniqueViolation {
		table: "message_headers".into(),
		constraint: "message_headers_case_id_key".into(),
	}));
	for message in ["DUPLICATE key", "UNIQUE constraint"] {
		assert!(is_unique_violation(&ModelError::Store(message.into())));
	}
	assert!(!is_unique_violation(&ModelError::Store(
		"connection closed".into()
	)));
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowStatusConfigDoc {
	name: String,
	editable: bool,
	description: Option<String>,
	allowed_roles: Option<Vec<String>>,
	due_days: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowConfigDoc {
	statuses: Option<Vec<WorkflowStatusConfigDoc>>,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowSettingsDoc {
	workflow_enabled: Option<bool>,
	workflow: Option<WorkflowConfigDoc>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStatusRule {
	pub name: String,
	pub editable: bool,
	pub description: Option<String>,
	pub allowed_roles: Vec<String>,
	pub due_days: i32,
}

#[derive(Debug, Clone)]
pub struct WorkflowRuntimeSettings {
	pub enabled: bool,
	pub statuses: Vec<WorkflowStatusRule>,
}

impl WorkflowRuntimeSettings {
	pub fn find_status(&self, value: &str) -> Option<&WorkflowStatusRule> {
		self.statuses
			.iter()
			.find(|status| status.name.eq_ignore_ascii_case(value))
	}
}

#[derive(Debug, Clone)]
pub struct WorkflowBlockReason {
	pub code: &'static str,
	pub message: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowActionability {
	pub can_act_on_workflow: bool,
	pub workflow_block_reason: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct WorkflowOwnershipDecision {
	pub role_match: bool,
	pub user_match: bool,
	pub admin_override_allowed: bool,
}

impl WorkflowOwnershipDecision {
	pub fn used_admin_override(&self) -> bool {
		self.admin_override_allowed && (!self.role_match || !self.user_match)
	}
}

fn current_user_matches_workflow_role(ctx: &Ctx, rule: &WorkflowStatusRule) -> bool {
	if rule.allowed_roles.is_empty() {
		return true;
	}
	let role = canonical_role(ctx.role());
	rule.allowed_roles
		.iter()
		.any(|allowed| allowed.eq_ignore_ascii_case(&role))
}

fn current_user_matches_workflow_assignment(ctx: &Ctx, case: &Case) -> bool {
	match case.workflow_assigned_user_id {
		Some(user_id) => user_id == ctx.user_id(),
		None => true,
	}
}

fn is_built_in_workflow_role(role: &str) -> bool {
	matches!(
		role,
		ROLE_SPONSOR_ADMIN_CRO | ROLE_SPONSOR_ADMIN_COMPANY | ROLE_USER
	)
}

pub async fn workflow_role_exists_and_is_active(
	ctx: &Ctx,
	mm: &ModelManager,
	role: &str,
) -> Result<bool> {
	let role = canonical_role(role);
	if role.is_empty() {
		return Ok(false);
	}
	if is_built_in_workflow_role(&role) {
		return Ok(true);
	}
	let roles = AdminSettingsBmc::known_workflow_roles(ctx, mm)
		.await
		.map_err(Error::Model)?;
	Ok(roles.contains(&role))
}

fn workflow_ownership_decision(
	ctx: &Ctx,
	case: &Case,
	rule: &WorkflowStatusRule,
) -> WorkflowOwnershipDecision {
	let role_match = current_user_matches_workflow_role(ctx, rule);
	let user_match = current_user_matches_workflow_assignment(ctx, case);
	let admin_override_allowed = if role_match && user_match {
		false
	} else {
		ctx.is_sponsor_admin() && !ctx.is_system_admin()
	};
	WorkflowOwnershipDecision {
		role_match,
		user_match,
		admin_override_allowed,
	}
}

pub async fn workflow_ownership_for_case(
	ctx: &Ctx,
	_mm: &ModelManager,
	case: &Case,
	rule: &WorkflowStatusRule,
) -> Result<WorkflowOwnershipDecision> {
	Ok(workflow_ownership_decision(ctx, case, rule))
}

pub fn qc_state_for_case_status(
	status: &str,
	status_before_lock: Option<&str>,
) -> &'static str {
	let status = if status.trim().eq_ignore_ascii_case("locked") {
		status_before_lock.unwrap_or(status)
	} else {
		status
	};
	match status.trim().to_ascii_lowercase().as_str() {
		"reviewed" | "validated" => "QCed",
		_ => "Pending",
	}
}

pub async fn load_workflow_runtime_settings(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<WorkflowRuntimeSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, "system")
		.await
		.map_err(Error::Model)?;
	let value = value.ok_or_else(|| Error::BadRequest {
		message: "admin settings record is missing".to_string(),
	})?;
	let parsed =
		serde_json::from_value::<WorkflowSettingsDoc>(value).map_err(|err| {
			Error::BadRequest {
				message: format!("stored workflow settings are invalid: {err}"),
			}
		})?;
	let enabled = parsed.workflow_enabled.ok_or_else(|| Error::BadRequest {
		message: "workflow_enabled is required".to_string(),
	})?;
	let statuses = parsed
		.workflow
		.ok_or_else(|| Error::BadRequest {
			message: "workflow configuration is required".to_string(),
		})?
		.statuses
		.ok_or_else(|| Error::BadRequest {
			message: "workflow statuses are required".to_string(),
		})?;
	if statuses.is_empty() {
		return Err(Error::BadRequest {
			message: "workflow must define at least one status".to_string(),
		});
	}
	let statuses = statuses
		.into_iter()
		.map(|status| {
			let name = status.name.trim().to_string();
			if name.is_empty() {
				return Err(Error::BadRequest {
					message: "workflow status name is required".to_string(),
				});
			}
			let due_days = status.due_days.ok_or_else(|| Error::BadRequest {
				message: format!("workflow status '{name}' due_days is required"),
			})?;
			if due_days < 0 {
				return Err(Error::BadRequest {
					message: format!(
						"workflow status '{name}' due_days must be zero or greater"
					),
				});
			}
			let allowed_roles =
				status.allowed_roles.ok_or_else(|| Error::BadRequest {
					message: format!(
						"workflow status '{name}' allowed_roles is required"
					),
				})?;
			let allowed_roles = allowed_roles
				.into_iter()
				.map(|role| canonical_role(role.trim()))
				.collect::<Vec<_>>();
			if allowed_roles.iter().any(String::is_empty) {
				return Err(Error::BadRequest {
					message: format!(
						"workflow status '{name}' contains an empty role"
					),
				});
			}
			Ok(WorkflowStatusRule {
				name,
				editable: status.editable,
				description: status
					.description
					.map(|value| value.trim().to_string()),
				allowed_roles,
				due_days,
			})
		})
		.collect::<Result<Vec<_>>>()?;

	Ok(WorkflowRuntimeSettings { enabled, statuses })
}

#[derive(Debug, FromRow)]
struct CaseScopeRow {
	case_id: Uuid,
	sender_identifiers: Vec<String>,
	product_identifiers: Vec<String>,
	study_identifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSenderOption {
	pub sender_identifier: String,
	pub sender_organization: Option<String>,
	pub case_count: i64,
	#[serde(skip)]
	scope_identifiers: Vec<String>,
	#[serde(skip)]
	product_identifiers: Vec<String>,
	#[serde(skip)]
	study_identifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveScopeSummary {
	pub assigned_sender_ids: Vec<String>,
	pub assigned_product_ids: Vec<String>,
	pub assigned_study_ids: Vec<String>,
	pub access_blind_allowed: bool,
	pub active_sender_identifier: Option<String>,
	pub effective_sender_filter: Option<String>,
	pub access_start_at: Option<OffsetDateTime>,
	pub access_end_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingProfile {
	pub built_in_role_id: String,
	pub operational: bool,
	pub sender_selection_required: bool,
	pub active_sender_identifier: Option<String>,
	pub available_senders: Vec<RoutingSenderOption>,
	pub effective_scope: EffectiveScopeSummary,
}

#[derive(Debug, FromRow)]
struct SenderOptionRow {
	sender_identifier: String,
	sender_organization: Option<String>,
	scope_identifiers: Vec<String>,
	product_identifiers: Vec<String>,
	study_identifiers: Vec<String>,
	case_count: i64,
}

#[derive(Debug, Default)]
struct ParsedScope {
	values: HashSet<String>,
	invalid: bool,
}

fn parse_scope_values(raw: Option<&str>) -> ParsedScope {
	let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
		return ParsedScope::default();
	};
	let Ok(values) = serde_json::from_str::<Vec<String>>(raw) else {
		return ParsedScope {
			invalid: true,
			..ParsedScope::default()
		};
	};
	let mut parsed = ParsedScope::default();
	for value in values {
		match Uuid::parse_str(value.trim()) {
			Ok(value) => {
				parsed.values.insert(value.to_string());
			}
			Err(_) => parsed.invalid = true,
		}
	}
	parsed
}

pub fn scope_values_from_raw(
	raw: Option<&str>,
) -> std::result::Result<Vec<String>, String> {
	let parsed = parse_scope_values(raw);
	if parsed.invalid {
		return Err("scope contains non-UUID values".to_string());
	}
	let mut values = parsed.values.into_iter().collect::<Vec<_>>();
	values.sort();
	Ok(values)
}

fn normalize_values(values: &[String]) -> HashSet<String> {
	values
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

/// Case-list scope gate. Filters a case out only when the user has an explicit
/// scope for the dimension AND the case carries a value for it that does not
/// match. An unset user scope means "allow all"; a case with no value for the
/// dimension is always allowed. Applied uniformly to sender/product/study.
fn scope_allows(assigned: &ParsedScope, available: &[String]) -> bool {
	if assigned.invalid {
		return false;
	}
	if assigned.values.is_empty() || available.is_empty() {
		return true;
	}
	let available = normalize_values(available);
	available
		.iter()
		.any(|value| assigned.values.contains(value))
}

async fn load_sender_options_for_org(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
) -> Result<Vec<RoutingSenderOption>> {
	let rows = with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(
				sqlx::query_as::<_, SenderOptionRow>(
					r#"
			SELECT s.id::text AS sender_identifier,
			       NULLIF(BTRIM(s.organization_name), '') AS sender_organization,
			       ARRAY[s.id::text] AS scope_identifiers,
			       COALESCE((
				       SELECT array_agg(DISTINCT p.id::text)
				       FROM product_presaves p
				       WHERE p.sender_presave_id = s.id
				         AND p.deleted = FALSE
				   ), ARRAY[]::text[]) AS product_identifiers,
			       COALESCE((
				       SELECT array_agg(DISTINCT study.id::text)
				       FROM study_presaves study
				       JOIN product_presaves p ON p.id = study.product_presave_id
				       WHERE p.sender_presave_id = s.id
				         AND p.deleted = FALSE
				         AND study.deleted = FALSE
				   ), ARRAY[]::text[]) AS study_identifiers,
			       COUNT(DISTINCT sender.case_id)::bigint AS case_count
			FROM sender_presaves s
			LEFT JOIN sender_information sender
			  ON sender.source_sender_presave_id = s.id
			WHERE s.organization_id = $1
			  AND s.deleted = FALSE
			GROUP BY s.id, s.organization_name
			ORDER BY s.organization_name ASC NULLS LAST, s.id ASC
				"#,
				)
				.bind(organization_id),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))
		})
	})
	.await?;

	Ok(rows
		.into_iter()
		.map(|row| RoutingSenderOption {
			sender_organization: row.sender_organization,
			sender_identifier: row.sender_identifier,
			case_count: row.case_count,
			scope_identifiers: row.scope_identifiers,
			product_identifiers: row.product_identifiers,
			study_identifiers: row.study_identifiers,
		})
		.collect())
}

pub async fn routing_profile_for_user(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<RoutingProfile> {
	let built_in_role_id = canonical_role(ctx.role());
	let user: lib_core::model::user::User =
		match UserBmc::get(ctx, mm, ctx.user_id()).await {
			Ok(user) => user,
			Err(original @ lib_core::model::Error::EntityUuidNotFound { .. }) => {
				let organizations =
					UserBmc::list_member_organizations(ctx, mm, ctx.user_id())
						.await?;
				let mut user = None;
				for organization in organizations {
					let candidate_ctx = Ctx::new(
						ctx.user_id(),
						organization.id,
						ctx.role().to_string(),
					)
					.map_err(|err| Error::BadRequest {
						message: err.to_string(),
					})?;
					if let Ok(candidate) =
						UserBmc::get(&candidate_ctx, mm, ctx.user_id()).await
					{
						user = Some(candidate);
						break;
					}
				}
				user.ok_or(Error::Model(original))?
			}
			Err(err) => return Err(err.into()),
		};
	let sender_scope = parse_scope_values(user.access_sender_ids.as_deref());
	let product_scope = parse_scope_values(user.access_product_ids.as_deref());
	let study_scope = parse_scope_values(user.access_study_ids.as_deref());
	if !ctx.is_sponsor_admin()
		&& [
			sender_scope.invalid,
			product_scope.invalid,
			study_scope.invalid,
		]
		.into_iter()
		.any(|invalid| invalid)
	{
		return Err(Error::Model(lib_core::model::Error::Store(
			"invalid user access scope".to_string(),
		)));
	}
	let assigned_sender_ids =
		sender_scope.values.iter().cloned().collect::<Vec<_>>();
	let assigned_product_ids =
		product_scope.values.iter().cloned().collect::<Vec<_>>();
	let assigned_study_ids = study_scope.values.iter().cloned().collect::<Vec<_>>();
	let active_sender_identifier = user
		.active_sender_identifier
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());

	let all_senders = if ctx.is_system_admin() {
		Vec::new()
	} else {
		load_sender_options_for_org(ctx, mm, ctx.organization_id()).await?
	};

	let available_senders = if ctx.is_sponsor_admin()
		|| (sender_scope.values.is_empty()
			&& product_scope.values.is_empty()
			&& study_scope.values.is_empty())
	{
		all_senders
	} else {
		all_senders
			.into_iter()
			.filter(|row| {
				(scope_allows_strict(&sender_scope, &row.scope_identifiers))
					&& scope_allows_strict(&product_scope, &row.product_identifiers)
					&& scope_allows_strict(&study_scope, &row.study_identifiers)
			})
			.collect()
	};

	let operational = !ctx.is_system_admin();
	let sender_selection_required = operational && available_senders.len() > 1;

	Ok(RoutingProfile {
		built_in_role_id,
		operational,
		sender_selection_required,
		active_sender_identifier: active_sender_identifier.clone(),
		effective_scope: EffectiveScopeSummary {
			assigned_sender_ids,
			assigned_product_ids,
			assigned_study_ids,
			access_blind_allowed: user.access_blind_allowed == Some(true),
			active_sender_identifier: active_sender_identifier.clone(),
			effective_sender_filter: active_sender_identifier,
			access_start_at: user.access_start_at,
			access_end_at: user.access_end_at,
		},
		available_senders,
	})
}

fn scope_allows_strict(assigned: &ParsedScope, available: &[String]) -> bool {
	if assigned.invalid {
		return false;
	}
	assigned.values.is_empty()
		|| available.iter().any(|value| {
			assigned
				.values
				.iter()
				.any(|assigned| assigned.eq_ignore_ascii_case(value))
		})
}

pub async fn validate_active_sender_selection(
	ctx: &Ctx,
	mm: &ModelManager,
	active_sender_identifier: Option<&str>,
) -> Result<Option<String>> {
	let next = active_sender_identifier
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());
	if next.is_none() {
		return Ok(None);
	}
	let profile = routing_profile_for_user(ctx, mm).await?;
	let requested = next.clone().expect("checked is_some");
	let requested = Uuid::parse_str(&requested)
		.map_err(|_| Error::BadRequest {
			message: "active_sender_identifier accepts a UUID value only"
				.to_string(),
		})?
		.to_string();
	let allowed = profile
		.available_senders
		.iter()
		.any(|sender| sender.sender_identifier.eq_ignore_ascii_case(&requested));
	if !allowed {
		return Err(Error::PermissionDenied {
			required_permission: "Routing.SenderSelection".to_string(),
		});
	}
	Ok(Some(requested))
}

async fn load_case_scopes(
	ctx: &Ctx,
	mm: &ModelManager,
	case_ids: &[Uuid],
) -> Result<Vec<CaseScopeRow>> {
	if case_ids.is_empty() {
		return Ok(Vec::new());
	}
	with_rls_read(mm, ctx, |dbx| {
		let case_ids = case_ids.to_vec();
		Box::pin(async move {
			dbx.fetch_all(
				sqlx::query_as::<_, CaseScopeRow>(
					r#"
			SELECT c.id AS case_id,
				COALESCE(
					(
						SELECT array_agg(identifier)
						FROM case_scope_identifiers(c.id)
						WHERE scope_kind = 'sender'
					),
					ARRAY[]::text[]
				) AS sender_identifiers,
				COALESCE(
					(
						SELECT array_agg(identifier)
						FROM case_scope_identifiers(c.id)
						WHERE scope_kind = 'product'
					),
					ARRAY[]::text[]
				) AS product_identifiers,
				COALESCE(
					(
						SELECT array_agg(identifier)
						FROM case_scope_identifiers(c.id)
						WHERE scope_kind = 'study'
					),
					ARRAY[]::text[]
				) AS study_identifiers
			FROM cases c
			WHERE c.id = ANY($1)
			"#,
				)
				.bind(case_ids),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))
		})
	})
	.await
}

pub async fn case_matches_user_scope(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<bool> {
	Ok(case_ids_matching_user_scope(ctx, mm, &[case_id])
		.await?
		.contains(&case_id))
}

pub async fn case_ids_matching_user_scope(
	ctx: &Ctx,
	mm: &ModelManager,
	case_ids: &[Uuid],
) -> Result<HashSet<Uuid>> {
	if case_ids.is_empty() {
		return Ok(HashSet::new());
	}
	if ctx.is_system_admin() || ctx.is_sponsor_admin() {
		return Ok(case_ids.iter().copied().collect());
	}

	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let now = OffsetDateTime::now_utc();
	if let Some(start_at) = user.access_start_at {
		if now < start_at {
			return Ok(HashSet::new());
		}
	}
	if let Some(end_at) = user.access_end_at {
		if now > end_at {
			return Ok(HashSet::new());
		}
	}

	let sender_scope = parse_scope_values(user.access_sender_ids.as_deref());
	let product_scope = parse_scope_values(user.access_product_ids.as_deref());
	let study_scope = parse_scope_values(user.access_study_ids.as_deref());
	let scopes = load_case_scopes(ctx, mm, case_ids).await?;
	Ok(scopes
		.into_iter()
		.filter(|scope| {
			scope_allows(&sender_scope, &scope.sender_identifiers)
				&& scope_allows(&product_scope, &scope.product_identifiers)
				&& scope_allows(&study_scope, &scope.study_identifiers)
		})
		.map(|scope| scope.case_id)
		.collect())
}

pub async fn case_write_block_reason_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
) -> Result<Option<WorkflowBlockReason>> {
	if let Some(reason) = legacy_case_write_block_reason(case) {
		return Ok(Some(reason));
	}
	let workflow = load_workflow_runtime_settings(ctx, mm).await?;
	Ok(case_write_block_reason_with_workflow(ctx, case, &workflow))
}

fn legacy_case_write_block_reason(case: &Case) -> Option<WorkflowBlockReason> {
	let legacy_status = case.status.trim();
	if legacy_status.eq_ignore_ascii_case("deleted") {
		return Some(WorkflowBlockReason {
			code: "case_deleted",
			message: "deleted cases are read-only".to_string(),
		});
	}
	if legacy_status.eq_ignore_ascii_case("locked") {
		return Some(WorkflowBlockReason {
			code: "case_locked",
			message: "locked cases are read-only".to_string(),
		});
	}
	if legacy_status.eq_ignore_ascii_case("reviewed")
		|| legacy_status.eq_ignore_ascii_case("validated")
	{
		return Some(WorkflowBlockReason {
			code: "case_qced",
			message: "QCed cases are read-only".to_string(),
		});
	}
	None
}

fn case_write_block_reason_with_workflow(
	ctx: &Ctx,
	case: &Case,
	workflow: &WorkflowRuntimeSettings,
) -> Option<WorkflowBlockReason> {
	if let Some(reason) = legacy_case_write_block_reason(case) {
		return Some(reason);
	}
	if workflow.enabled {
		let Some(rule) = workflow.find_status(&case.workflow_status) else {
			return Some(WorkflowBlockReason {
				code: "workflow_status_not_configured",
				message: format!(
					"workflow status '{}' is not configured",
					case.workflow_status
				),
			});
		};
		let ownership = workflow_ownership_decision(ctx, case, rule);
		if !ownership.role_match && !ownership.admin_override_allowed {
			return Some(WorkflowBlockReason {
				code: "workflow_role_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different role",
					rule.name
				),
			});
		}
		if !ownership.user_match && !ownership.admin_override_allowed {
			return Some(WorkflowBlockReason {
				code: "workflow_user_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different user",
					rule.name
				),
			});
		}
		if !rule.editable {
			return Some(WorkflowBlockReason {
				code: "workflow_status_read_only",
				message: format!("workflow status '{}' is read-only", rule.name),
			});
		}
	}
	None
}

pub async fn workflow_actionability_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
) -> Result<WorkflowActionability> {
	if let Some(actionability) = locked_case_actionability(case) {
		return Ok(actionability);
	}
	let workflow = load_workflow_runtime_settings(ctx, mm).await?;
	Ok(workflow_actionability_with_workflow(ctx, case, &workflow))
}

fn locked_case_actionability(case: &Case) -> Option<WorkflowActionability> {
	let legacy_status = case.status.trim();
	if legacy_status.eq_ignore_ascii_case("locked") {
		return Some(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("case_locked"),
		});
	}
	None
}

fn workflow_actionability_with_workflow(
	ctx: &Ctx,
	case: &Case,
	workflow: &WorkflowRuntimeSettings,
) -> WorkflowActionability {
	if let Some(actionability) = locked_case_actionability(case) {
		return actionability;
	}
	if !workflow.enabled {
		return WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_not_enabled"),
		};
	}

	let Some(rule) = workflow.find_status(&case.workflow_status) else {
		return WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_status_not_configured"),
		};
	};

	let ownership = workflow_ownership_decision(ctx, case, rule);
	if ownership.used_admin_override() {
		return WorkflowActionability {
			can_act_on_workflow: true,
			workflow_block_reason: Some("workflow_admin_override_allowed"),
		};
	}

	if !ownership.role_match {
		return WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_role_mismatch"),
		};
	}

	if !ownership.user_match {
		return WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_user_mismatch"),
		};
	}

	WorkflowActionability {
		can_act_on_workflow: true,
		workflow_block_reason: None,
	}
}

pub fn case_read_decisions_with_workflow(
	ctx: &Ctx,
	case: &Case,
	workflow: &WorkflowRuntimeSettings,
) -> (WorkflowActionability, Option<WorkflowBlockReason>) {
	(
		workflow_actionability_with_workflow(ctx, case, workflow),
		case_write_block_reason_with_workflow(ctx, case, workflow),
	)
}

#[cfg(test)]
mod scope_tests {
	use super::*;

	const SENDER_ID: &str = "11111111-1111-4111-8111-111111111111";

	#[test]
	fn empty_scope_allows_all() {
		let scope = parse_scope_values(None);
		assert!(!scope.invalid);
		assert!(scope_allows(&scope, &[SENDER_ID.to_string()]));
	}

	#[test]
	fn invalid_scope_does_not_become_unrestricted() {
		let scope = parse_scope_values(Some("[\"Demo CRO Organization\"]"));
		assert!(scope.invalid);
		assert!(!scope_allows(&scope, &[SENDER_ID.to_string()]));
		assert!(scope_values_from_raw(Some("[\"Demo CRO Organization\"]")).is_err());
	}

	#[test]
	fn parent_scope_does_not_allow_unlinked_sender() {
		let scope = parse_scope_values(Some(&format!("[\"{SENDER_ID}\"]")));
		assert!(!scope_allows_strict(&scope, &[]));
		assert!(scope_allows_strict(&scope, &[SENDER_ID.to_string()]));
	}
}

pub mod prelude;

// endregion: --- Modules
