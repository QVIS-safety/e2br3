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
	canonical_role, Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO,
	ROLE_USER,
};
use lib_core::model::acs::{has_permission, Permission, USER_CREATE};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::case::{Case, CaseBmc};
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

pub async fn is_admin(ctx: &Ctx, mm: &ModelManager) -> Result<bool> {
	let _ = mm;
	Ok(ctx.is_admin())
}

pub fn can_access_admin(ctx: &Ctx) -> bool {
	ctx.is_admin() || has_permission(ctx.permission_subject(), USER_CREATE)
}

pub async fn require_admin(ctx: &Ctx, mm: &ModelManager) -> Result<()> {
	let _ = mm;
	if !can_access_admin(ctx) {
		return Err(Error::AccessDenied {
			required_role: "admin".to_string(),
		});
	}
	Ok(())
}

pub async fn admin_db_ctx(ctx: &Ctx, mm: &ModelManager) -> Result<Ctx> {
	require_admin(ctx, mm).await?;
	if ctx.is_system_admin() || ctx.is_sponsor_admin() {
		return Ok(ctx.clone());
	}
	let elevated = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::AccessDenied {
		required_role: "admin".to_string(),
	})?
	.with_compliance(
		ctx.change_reason().map(ToString::to_string),
		ctx.e_signature_id(),
	);
	Ok(elevated)
}

pub fn require_permission(ctx: &Ctx, permission: Permission) -> Result<()> {
	if !has_permission(ctx.permission_subject(), permission) {
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
	let _ = mm;
	Ok(false)
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
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<WorkflowRuntimeSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, "system")
		.await
		.map_err(Error::Model)?;

	let Some(value) = value else {
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
	routing_sender_identifiers: Vec<String>,
	product_identifiers: Vec<String>,
	study_identifiers: Vec<String>,
	has_blinded_data: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSenderOption {
	pub sender_identifier: String,
	pub sender_organization: Option<String>,
	pub case_count: i64,
	#[serde(skip)]
	scope_identifiers: Vec<String>,
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
	scope_identifiers: Vec<String>,
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

/// Case-list scope gate. Filters a case out only when the user has an explicit
/// scope for the dimension AND the case carries a value for it that does not
/// match. An unset user scope means "allow all"; a case with no value for the
/// dimension is always allowed. Applied uniformly to sender/product/study.
fn scope_allows(assigned: &HashSet<String>, available: &[String]) -> bool {
	if assigned.is_empty() {
		return true;
	}
	let available = normalize_values(available);
	if available.is_empty() {
		return true;
	}
	available.iter().any(|value| assigned.contains(value))
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
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
) -> Result<Vec<RoutingSenderOption>> {
	let rows = with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_all(
				sqlx::query_as::<_, SenderOptionRow>(
					r#"
			WITH sender_master_rows AS (
				SELECT DISTINCT
				       NULLIF(BTRIM(g.sender_identifier), '') AS sender_identifier,
				       NULLIF(BTRIM(s.organization_name), '') AS scope_identifier
				FROM sender_presaves s
				JOIN sender_presave_gateways g ON g.sender_presave_id = s.id
				WHERE s.organization_id = $1
				  AND s.deleted = FALSE
				  AND NULLIF(BTRIM(g.sender_identifier), '') IS NOT NULL
			),
			case_sender_rows AS (
				SELECT DISTINCT senders.case_id,
				       senders.sender_identifier,
				       NULLIF(BTRIM(sender.organization_name), '') AS scope_identifier
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
				LEFT JOIN sender_information sender ON sender.case_id = senders.case_id
				WHERE c.organization_id = $1
				  AND sender_identifier IS NOT NULL
			),
			case_sender_counts AS (
				SELECT sender_identifier, COUNT(DISTINCT case_id) AS case_count
				FROM case_sender_rows
				GROUP BY sender_identifier
			),
			sender_scope_rows AS (
				SELECT sender_identifier, scope_identifier FROM sender_master_rows
				UNION
				SELECT sender_identifier, scope_identifier FROM case_sender_rows
			)
			SELECT r.sender_identifier,
			       COALESCE(
				       ARRAY_AGG(DISTINCT r.scope_identifier)
				       FILTER (WHERE r.scope_identifier IS NOT NULL),
				       ARRAY[]::text[]
			       ) AS scope_identifiers,
			       COALESCE(c.case_count, 0)::bigint AS case_count
			FROM sender_scope_rows r
			LEFT JOIN case_sender_counts c ON c.sender_identifier = r.sender_identifier
			WHERE r.sender_identifier IS NOT NULL
			GROUP BY r.sender_identifier, c.case_count
			ORDER BY sender_identifier ASC
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
			sender_organization: row.scope_identifiers.first().cloned(),
			sender_identifier: row.sender_identifier,
			case_count: row.case_count,
			scope_identifiers: row.scope_identifiers,
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
		load_sender_options_for_org(ctx, mm, ctx.organization_id()).await?
	};

	let available_senders = if ctx.is_sponsor_admin() {
		all_senders
	} else {
		all_senders
			.into_iter()
			.filter(|row| {
				!assigned_sender_ids.is_empty()
					&& assigned_sender_ids.iter().any(|assigned| {
						row.scope_identifiers
							.iter()
							.any(|scope| assigned.eq_ignore_ascii_case(scope))
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

async fn load_case_scope(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<CaseScopeRow> {
	with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			dbx.fetch_one(
				sqlx::query_as::<_, CaseScopeRow>(
					r#"
			SELECT
				COALESCE(
					(
						SELECT array_agg(DISTINCT ident)
						FROM (
							SELECT NULLIF(BTRIM(sender.organization_name), '') AS ident
							FROM sender_information sender
							WHERE sender.case_id = c.id
						) senders
						WHERE ident IS NOT NULL
					),
					ARRAY[]::text[]
				) AS sender_identifiers,
				COALESCE(
					(
						SELECT array_agg(DISTINCT ident)
						FROM (
							SELECT NULLIF(BTRIM(mh.message_sender_identifier), '') AS ident
							FROM message_headers mh
							WHERE mh.case_id = c.id
							UNION ALL
							SELECT NULLIF(BTRIM(mh.batch_sender_identifier), '')
							FROM message_headers mh
							WHERE mh.case_id = c.id
						) routing_senders
						WHERE ident IS NOT NULL
					),
					ARRAY[]::text[]
				) AS routing_sender_identifiers,
				COALESCE(
					(
						SELECT array_agg(DISTINCT ident)
						FROM (
							SELECT NULLIF(BTRIM(d.brand_name), '') AS ident
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
		})
	})
	.await
}

pub async fn case_matches_user_scope(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<bool> {
	if is_admin(ctx, mm).await? {
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

	let scope = load_case_scope(ctx, mm, case_id).await?;
	if !scope_allows(
		&parse_scope_values(user.access_sender_ids.as_deref()),
		&scope.sender_identifiers,
	) {
		return Ok(false);
	}
	if !selected_sender_matches(
		user.active_sender_identifier.as_deref(),
		&scope.routing_sender_identifiers,
	) {
		return Ok(false);
	}
	if !scope_allows(
		&parse_scope_values(user.access_product_ids.as_deref()),
		&scope.product_identifiers,
	) {
		return Ok(false);
	}
	if !scope_allows(
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

	let workflow = load_workflow_runtime_settings(ctx, mm).await?;
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

	let workflow = load_workflow_runtime_settings(ctx, mm).await?;
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
