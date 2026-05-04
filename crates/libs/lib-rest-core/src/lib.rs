// region:    --- Modules

mod error;
pub mod rest_params;
pub mod rest_result;
mod utils;

pub use self::error::{Error, Result};
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
	canonical_role, Ctx, ROLE_HEAD_PV, ROLE_MANAGER, ROLE_PVM, ROLE_PVS,
	ROLE_SPONSOR, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER,
	ROLE_VIEWER,
};
use lib_core::model::acs::{has_permission, Permission};
use lib_core::model::case::{Case, CaseBmc};
use lib_core::model::user::UserBmc;
use lib_core::model::ModelManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// Require that the caller has the safety-db admin role (`can_admin_safety_db`).
/// Used by organization, user, role, and settings management endpoints.
pub fn require_admin_role(ctx: &Ctx) -> Result<()> {
	if !ctx.can_admin_safety_db() {
		return Err(Error::AccessDenied {
			required_role: "safety_db_admin".to_string(),
		});
	}
	Ok(())
}

pub async fn is_safety_db_admin(ctx: &Ctx, mm: &ModelManager) -> Result<bool> {
	if ctx.is_sponsor_admin() {
		return Ok(true);
	}
	if ctx.is_system_admin() {
		return Ok(false);
	}
	let role = canonical_role(ctx.role());
	if role.is_empty() {
		return Ok(false);
	}
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT sponsor_admin_capable
				FROM app_roles
				WHERE role_name = $1
				  AND active = true
				"#,
			)
			.bind(&role),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(matches!(row, Some((true,))))
}

pub async fn require_safety_db_admin_role(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<()> {
	if !is_safety_db_admin(ctx, mm).await? {
		return Err(Error::AccessDenied {
			required_role: "safety_db_admin".to_string(),
		});
	}
	Ok(())
}

pub async fn safety_db_admin_db_ctx(ctx: &Ctx, mm: &ModelManager) -> Result<Ctx> {
	require_safety_db_admin_role(ctx, mm).await?;
	if ctx.is_sponsor_admin() {
		return Ok(ctx.clone());
	}
	let elevated = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::AccessDenied {
		required_role: "safety_db_admin".to_string(),
	})?
	.with_compliance(
		ctx.change_reason().map(ToString::to_string),
		ctx.e_signature_id(),
	);
	Ok(elevated)
}

pub fn sponsor_admin_provisioning_db_ctx(ctx: &Ctx) -> Result<Ctx> {
	Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::AccessDenied {
		required_role: "sponsor_admin_provisioning".to_string(),
	})
}

pub fn require_permission(ctx: &Ctx, permission: Permission) -> Result<()> {
	if !has_permission(ctx.role(), permission) {
		return Err(Error::PermissionDenied {
			required_permission: format!("{permission}"),
		});
	}
	Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WorkflowStatusConfigDoc {
	name: String,
	editable: bool,
	description: Option<String>,
	allowed_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WorkflowConfigDoc {
	statuses: Option<Vec<WorkflowStatusConfigDoc>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
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
}

#[derive(Debug, Clone)]
pub struct WorkflowRuntimeSettings {
	pub enabled: bool,
	pub statuses: Vec<WorkflowStatusRule>,
}

impl WorkflowRuntimeSettings {
	fn default_disabled() -> Self {
		Self {
			enabled: false,
			statuses: vec![WorkflowStatusRule {
				name: "Saved".to_string(),
				editable: true,
				description: Some("Default authoring state".to_string()),
				allowed_roles: Vec::new(),
			}],
		}
	}

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
		ROLE_SPONSOR_ADMIN_CRO
			| ROLE_SPONSOR_ADMIN_COMPANY
			| ROLE_MANAGER
			| ROLE_PVM
			| ROLE_HEAD_PV
			| ROLE_USER
			| ROLE_PVS
			| ROLE_VIEWER
			| ROLE_SPONSOR
	)
}

pub async fn workflow_role_exists_and_is_active(
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
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT active
				FROM app_roles
				WHERE role_name = $1
				"#,
			)
			.bind(&role),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(matches!(row, Some((true,))))
}

async fn workflow_admin_override_allowed(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<bool> {
	if ctx.is_system_admin() {
		return Ok(false);
	}
	if ctx.is_sponsor_admin() {
		return Ok(true);
	}
	let role = canonical_role(ctx.role());
	if role.is_empty() {
		return Ok(false);
	}
	is_safety_db_admin(ctx, mm).await
}

pub async fn workflow_ownership_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
	rule: &WorkflowStatusRule,
) -> Result<WorkflowOwnershipDecision> {
	let role_match = current_user_matches_workflow_role(ctx, rule);
	let user_match = current_user_matches_workflow_assignment(ctx, case);
	let admin_override_allowed = if role_match && user_match {
		false
	} else {
		workflow_admin_override_allowed(ctx, mm).await?
	};
	Ok(WorkflowOwnershipDecision {
		role_match,
		user_match,
		admin_override_allowed,
	})
}

pub fn qc_state_for_case_status(status: &str) -> &'static str {
	match status.trim().to_ascii_lowercase().as_str() {
		"reviewed" | "validated" => "QCed",
		_ => "Pending",
	}
}

pub async fn load_workflow_runtime_settings(
	mm: &ModelManager,
) -> Result<WorkflowRuntimeSettings> {
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Value,)>(
				"SELECT value FROM app_settings WHERE key = $1",
			)
			.bind("system"),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let Some((value,)) = row else {
		return Ok(WorkflowRuntimeSettings::default_disabled());
	};

	let parsed =
		serde_json::from_value::<WorkflowSettingsDoc>(value).unwrap_or_default();

	let mut statuses = parsed
		.workflow
		.and_then(|workflow| workflow.statuses)
		.unwrap_or_default()
		.into_iter()
		.filter_map(|status| {
			let name = status.name.trim().to_string();
			if name.is_empty() {
				None
			} else {
				Some(WorkflowStatusRule {
					name,
					editable: status.editable,
					description: status
						.description
						.map(|value| value.trim().to_string()),
					allowed_roles: status
						.allowed_roles
						.unwrap_or_default()
						.into_iter()
						.map(|role| canonical_role(role.trim()))
						.filter(|role| !role.is_empty())
						.collect(),
				})
			}
		})
		.collect::<Vec<_>>();

	if statuses.is_empty() {
		statuses.push(WorkflowStatusRule {
			name: "Saved".to_string(),
			editable: true,
			description: Some("Default authoring state".to_string()),
			allowed_roles: Vec::new(),
		});
	}

	Ok(WorkflowRuntimeSettings {
		enabled: parsed.workflow_enabled.unwrap_or(false),
		statuses,
	})
}

#[derive(Debug, FromRow)]
struct CaseScopeRow {
	sender_identifiers: Vec<String>,
	product_identifiers: Vec<String>,
	study_identifiers: Vec<String>,
	has_blinded_data: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSenderOption {
	pub sender_identifier: String,
	pub case_count: i64,
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
	case_count: i64,
}

fn parse_scope_values(raw: Option<&str>) -> HashSet<String> {
	let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
		return HashSet::new();
	};
	if let Ok(values) = serde_json::from_str::<Vec<String>>(raw) {
		return values
			.into_iter()
			.map(|value| value.trim().to_ascii_lowercase())
			.filter(|value| !value.is_empty())
			.collect();
	}
	raw.split(',')
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

pub fn scope_values_from_raw(raw: Option<&str>) -> Vec<String> {
	let mut values = parse_scope_values(raw).into_iter().collect::<Vec<_>>();
	values.sort();
	values
}

fn normalize_values(values: &[String]) -> HashSet<String> {
	values
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

fn optional_scope_matches(assigned: &HashSet<String>, available: &[String]) -> bool {
	if assigned.is_empty() {
		return true;
	}
	let available = normalize_values(available);
	!available.is_empty() && available.iter().any(|value| assigned.contains(value))
}

fn required_scope_matches(assigned: &HashSet<String>, available: &[String]) -> bool {
	let available = normalize_values(available);
	if available.is_empty() {
		return true;
	}
	!assigned.is_empty() && available.iter().any(|value| assigned.contains(value))
}

fn selected_sender_matches(
	selected_sender: Option<&str>,
	available: &[String],
) -> bool {
	let Some(selected_sender) = selected_sender
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_ascii_lowercase())
	else {
		return true;
	};
	let available = normalize_values(available);
	available.contains(&selected_sender)
}

async fn load_sender_options_for_org(
	mm: &ModelManager,
	organization_id: Uuid,
) -> Result<Vec<RoutingSenderOption>> {
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, SenderOptionRow>(
				r#"
			WITH sender_master_options AS (
				SELECT DISTINCT
				       NULLIF(
				           BTRIM(COALESCE(
				               data->>'senderIdentifier',
				               data->>'messageSenderIdentifier',
				               data->>'batchSenderIdentifier',
				               data->>'senderOrganization'
				           )),
				           ''
				       ) AS sender_identifier,
				       0::bigint AS case_count
				FROM presave_templates
				WHERE organization_id = $1
				  AND entity_type = 'sender'
				  AND LOWER(COALESCE(data->>'senderDeleted', 'false')) NOT IN ('true', '1', 'yes')
			),
			case_sender_options AS (
				SELECT sender_identifier, COUNT(DISTINCT case_id) AS case_count
				FROM (
					SELECT mh.case_id,
					       NULLIF(BTRIM(mh.message_sender_identifier), '') AS sender_identifier
					FROM message_headers mh
					UNION ALL
					SELECT mh.case_id,
					       NULLIF(BTRIM(mh.batch_sender_identifier), '') AS sender_identifier
					FROM message_headers mh
				) senders
				JOIN cases c ON c.id = senders.case_id
				WHERE c.organization_id = $1
				  AND sender_identifier IS NOT NULL
				GROUP BY sender_identifier
			)
			SELECT sender_identifier, SUM(case_count)::bigint AS case_count
			FROM (
				SELECT sender_identifier, case_count FROM sender_master_options
				UNION ALL
				SELECT sender_identifier, case_count FROM case_sender_options
			) sender_options
			WHERE sender_identifier IS NOT NULL
			GROUP BY sender_identifier
			ORDER BY sender_identifier ASC
			"#,
			)
			.bind(organization_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	Ok(rows
		.into_iter()
		.map(|row| RoutingSenderOption {
			sender_identifier: row.sender_identifier,
			case_count: row.case_count,
		})
		.collect())
}

pub async fn routing_profile_for_user(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<RoutingProfile> {
	let built_in_role_id = canonical_role(ctx.role());
	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let assigned_sender_ids =
		scope_values_from_raw(user.access_sender_ids.as_deref());
	let assigned_product_ids =
		scope_values_from_raw(user.access_product_ids.as_deref());
	let assigned_study_ids = scope_values_from_raw(user.access_study_ids.as_deref());
	let active_sender_identifier = user
		.active_sender_identifier
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());

	let all_senders = if ctx.is_system_admin() {
		Vec::new()
	} else {
		load_sender_options_for_org(mm, ctx.organization_id()).await?
	};

	let available_senders = if ctx.is_sponsor_admin() {
		all_senders
	} else {
		all_senders
			.into_iter()
			.filter(|row| {
				!assigned_sender_ids.is_empty()
					&& assigned_sender_ids.iter().any(|assigned| {
						assigned.eq_ignore_ascii_case(&row.sender_identifier)
					})
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
	let allowed = profile
		.available_senders
		.iter()
		.any(|sender| sender.sender_identifier.eq_ignore_ascii_case(&requested));
	if !allowed {
		return Err(Error::PermissionDenied {
			required_permission: "Routing.SenderSelection".to_string(),
		});
	}
	Ok(next)
}

async fn load_case_scope(mm: &ModelManager, case_id: Uuid) -> Result<CaseScopeRow> {
	mm.dbx()
		.fetch_one(
			sqlx::query_as::<_, CaseScopeRow>(
				r#"
			SELECT
				COALESCE(
					(
						SELECT array_remove(
							ARRAY[
								NULLIF(BTRIM(mh.message_sender_identifier), ''),
								NULLIF(BTRIM(mh.batch_sender_identifier), '')
							],
							NULL
						)
						FROM message_headers mh
						WHERE mh.case_id = c.id
						LIMIT 1
					),
					ARRAY[]::text[]
				) AS sender_identifiers,
				COALESCE(
					(
						SELECT array_agg(DISTINCT ident)
						FROM (
							SELECT NULLIF(BTRIM(c.dg_prd_key), '') AS ident
							UNION ALL
							SELECT NULLIF(BTRIM(d.mpid), '')
							FROM drug_information d
							WHERE d.case_id = c.id
							UNION ALL
							SELECT NULLIF(BTRIM(d.medicinal_product), '')
							FROM drug_information d
							WHERE d.case_id = c.id
						) products
						WHERE ident IS NOT NULL
					),
					ARRAY[]::text[]
				) AS product_identifiers,
				COALESCE(
					(
						SELECT array_agg(DISTINCT ident)
						FROM (
							SELECT NULLIF(BTRIM(s.sponsor_study_number), '') AS ident
							FROM study_information s
							WHERE s.case_id = c.id
							UNION ALL
							SELECT NULLIF(BTRIM(s.study_name), '')
							FROM study_information s
							WHERE s.case_id = c.id
						) studies
						WHERE ident IS NOT NULL
					),
					ARRAY[]::text[]
				) AS study_identifiers,
				EXISTS(
					SELECT 1
					FROM drug_information d
					WHERE d.case_id = c.id
					  AND d.investigational_product_blinded = TRUE
				) AS has_blinded_data
			FROM cases c
			WHERE c.id = $1
			"#,
			)
			.bind(case_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))
}

pub async fn case_matches_user_scope(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<bool> {
	if is_safety_db_admin(ctx, mm).await? {
		return Ok(true);
	}

	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let now = OffsetDateTime::now_utc();
	if let Some(start_at) = user.access_start_at {
		if now < start_at {
			return Ok(false);
		}
	}
	if let Some(end_at) = user.access_end_at {
		if now > end_at {
			return Ok(false);
		}
	}

	let scope = load_case_scope(mm, case_id).await?;
	let assigned_sender_ids = parse_scope_values(user.access_sender_ids.as_deref());
	if assigned_sender_ids.is_empty()
		|| !optional_scope_matches(&assigned_sender_ids, &scope.sender_identifiers)
	{
		return Ok(false);
	}
	if !selected_sender_matches(
		user.active_sender_identifier.as_deref(),
		&scope.sender_identifiers,
	) {
		return Ok(false);
	}
	if !required_scope_matches(
		&parse_scope_values(user.access_product_ids.as_deref()),
		&scope.product_identifiers,
	) {
		return Ok(false);
	}
	if !required_scope_matches(
		&parse_scope_values(user.access_study_ids.as_deref()),
		&scope.study_identifiers,
	) {
		return Ok(false);
	}
	if scope.has_blinded_data && user.access_blind_allowed != Some(true) {
		return Ok(false);
	}
	Ok(true)
}

pub async fn require_case_read_allowed(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	CaseBmc::get(ctx, mm, case_id).await?;
	if case_matches_user_scope(ctx, mm, case_id).await? {
		return Ok(());
	}
	Err(Error::PermissionDenied {
		required_permission: "Case.Scope".to_string(),
	})
}

pub async fn case_write_block_reason_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
) -> Result<Option<WorkflowBlockReason>> {
	let legacy_status = case.status.trim();
	if legacy_status.eq_ignore_ascii_case("deleted") {
		return Ok(Some(WorkflowBlockReason {
			code: "case_deleted",
			message: "deleted cases are read-only".to_string(),
		}));
	}
	if legacy_status.eq_ignore_ascii_case("locked") {
		return Ok(Some(WorkflowBlockReason {
			code: "case_locked",
			message: "locked cases are read-only".to_string(),
		}));
	}
	if legacy_status.eq_ignore_ascii_case("reviewed")
		|| legacy_status.eq_ignore_ascii_case("validated")
	{
		return Ok(Some(WorkflowBlockReason {
			code: "case_qced",
			message: "QCed cases are read-only".to_string(),
		}));
	}

	let workflow = load_workflow_runtime_settings(mm).await?;
	if workflow.enabled {
		let Some(rule) = workflow.find_status(&case.workflow_status) else {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_status_not_configured",
				message: format!(
					"workflow status '{}' is not configured",
					case.workflow_status
				),
			}));
		};
		let ownership = workflow_ownership_for_case(ctx, mm, case, rule).await?;
		if !ownership.role_match && !ownership.admin_override_allowed {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_role_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different role",
					rule.name
				),
			}));
		}
		if !ownership.user_match && !ownership.admin_override_allowed {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_user_mismatch",
				message: format!(
					"workflow status '{}' is assigned to a different user",
					rule.name
				),
			}));
		}
		if !rule.editable {
			return Ok(Some(WorkflowBlockReason {
				code: "workflow_status_read_only",
				message: format!("workflow status '{}' is read-only", rule.name),
			}));
		}
		return Ok(None);
	}

	Ok(None)
}

pub async fn workflow_actionability_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case: &Case,
) -> Result<WorkflowActionability> {
	let legacy_status = case.status.trim();
	if legacy_status.eq_ignore_ascii_case("locked") {
		return Ok(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("case_locked"),
		});
	}

	let workflow = load_workflow_runtime_settings(mm).await?;
	if !workflow.enabled {
		return Ok(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_not_enabled"),
		});
	}

	let Some(rule) = workflow.find_status(&case.workflow_status) else {
		return Ok(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_status_not_configured"),
		});
	};

	let ownership = workflow_ownership_for_case(ctx, mm, case, rule).await?;
	if ownership.used_admin_override() {
		return Ok(WorkflowActionability {
			can_act_on_workflow: true,
			workflow_block_reason: Some("workflow_admin_override_allowed"),
		});
	}

	if !ownership.role_match {
		return Ok(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_role_mismatch"),
		});
	}

	if !ownership.user_match {
		return Ok(WorkflowActionability {
			can_act_on_workflow: false,
			workflow_block_reason: Some("workflow_user_mismatch"),
		});
	}

	Ok(WorkflowActionability {
		can_act_on_workflow: true,
		workflow_block_reason: None,
	})
}

pub async fn require_case_write_allowed(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	require_case_read_allowed(ctx, mm, case_id).await?;
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	if let Some(reason) = case_write_block_reason_for_case(ctx, mm, &case).await? {
		return Err(Error::BadRequest {
			message: reason.message,
		});
	}
	Ok(())
}

pub mod prelude;

// endregion: --- Modules
