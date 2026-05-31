use axum::extract::{Path, State};
use axum::Json;
use lib_core::ctx::{canonical_role, Ctx};
use lib_core::model::acs::{CASE_READ, CASE_UPDATE};
use lib_core::model::case::{
	Case, CaseBmc, CaseWorkflowEventBmc, CaseWorkflowEventRow, WorkflowAssignRecord,
	WorkflowTransitionRecord,
};
use lib_core::model::ModelManager;
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_rest_core::{
	load_workflow_runtime_settings, workflow_ownership_for_case,
	workflow_role_exists_and_is_active, WorkflowStatusRule,
};
use lib_utils::time::parse_utc;
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use time::Duration;
use uuid::Uuid;

use crate::web::rest::case_rest::CaseReadResult;

// -- Types

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStatusRuntimeDoc {
	pub name: String,
	pub editable: bool,
	pub description: Option<String>,
	pub allowed_roles: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfigRuntimeDoc {
	pub workflow_enabled: bool,
	pub statuses: Vec<WorkflowStatusRuntimeDoc>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowTransitionInput {
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub due_at: Option<String>,
	pub override_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowAssignInput {
	pub target_role: String,
	pub target_user_id: Option<Uuid>,
	pub comment: Option<String>,
	pub due_at: Option<String>,
	pub override_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEventReadResult {
	pub id: Uuid,
	pub case_id: Uuid,
	pub from_status: String,
	pub from_role: Option<String>,
	pub from_user_id: Option<Uuid>,
	pub from_user_display: Option<String>,
	pub to_status: String,
	pub target_role: Option<String>,
	pub target_user_id: Option<Uuid>,
	pub target_user_display: Option<String>,
	pub comment: Option<String>,
	pub date_of_most_recent: Option<String>,
	pub due_at: Option<OffsetDateTime>,
	pub delay: String,
	pub acted_by: Uuid,
	pub acted_by_display: Option<String>,
	pub actor_role_id: String,
	pub used_admin_override: bool,
	pub override_reason: Option<String>,
	pub created_at: OffsetDateTime,
}

// -- Helpers

fn parse_workflow_due_at(value: Option<&str>) -> Result<Option<OffsetDateTime>> {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(None);
	};
	parse_utc(value).map(Some).map_err(|_| Error::BadRequest {
		message: "workflow due_at must be a valid RFC3339 timestamp".to_string(),
	})
}

async fn require_current_workflow_step_owner(
	ctx: &Ctx,
	mm: &ModelManager,
	current_status_name: &str,
	case: &Case,
	current_status: &WorkflowStatusRule,
) -> Result<bool> {
	let ownership =
		workflow_ownership_for_case(ctx, mm, case, current_status).await?;
	if !ownership.role_match && !ownership.admin_override_allowed {
		return Err(Error::BadRequest {
			message: format!(
				"workflow status '{current_status_name}' is assigned to a different role"
			),
		});
	}
	if !ownership.user_match && !ownership.admin_override_allowed {
		return Err(Error::BadRequest {
			message: format!(
				"workflow status '{current_status_name}' is assigned to a different user"
			),
		});
	}
	Ok(ownership.used_admin_override())
}

fn normalize_override_reason(
	value: Option<&str>,
	used_admin_override: bool,
) -> Option<String> {
	let provided = value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());
	if used_admin_override {
		return Some(provided.unwrap_or_else(|| {
			"workflow ownership override by audited admin policy".to_string()
		}));
	}
	provided
}

fn workflow_delay(
	due_at: Option<OffsetDateTime>,
	created_at: OffsetDateTime,
) -> String {
	let Some(due_at) = due_at else {
		return "N/A".to_string();
	};
	if created_at <= due_at {
		return "N/A".to_string();
	}
	let elapsed: Duration = created_at - due_at;
	format!("{}d", elapsed.whole_days().max(1))
}

async fn user_display(
	mm: &ModelManager,
	user_id: Option<Uuid>,
) -> Result<Option<String>> {
	let Some(user_id) = user_id else {
		return Ok(None);
	};
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Option<String>,)>(
				r#"
				SELECT audit_user_display($1)
				"#,
			)
			.bind(user_id),
		)
		.await
		.map_err(|err| Error::Model(err.into()))?;
	Ok(row.and_then(|(display,)| display))
}

async fn workflow_event_to_read_result(
	mm: &ModelManager,
	row: CaseWorkflowEventRow,
) -> Result<WorkflowEventReadResult> {
	let from_user_display = user_display(mm, row.from_user_id).await?;
	let target_user_display = user_display(mm, row.target_user_id).await?;
	let acted_by_display = user_display(mm, Some(row.acted_by)).await?;
	let delay = workflow_delay(row.due_at, row.created_at);
	Ok(WorkflowEventReadResult {
		id: row.id,
		case_id: row.case_id,
		from_status: row.from_status,
		from_role: row.from_role,
		from_user_id: row.from_user_id,
		from_user_display,
		to_status: row.to_status,
		target_role: row.target_role,
		target_user_id: row.target_user_id,
		target_user_display,
		comment: row.comment,
		date_of_most_recent: row.date_of_most_recent,
		due_at: row.due_at,
		delay,
		acted_by: row.acted_by,
		acted_by_display,
		actor_role_id: row.actor_role_id,
		used_admin_override: row.used_admin_override,
		override_reason: row.override_reason,
		created_at: row.created_at,
	})
}

// -- Handlers

/// POST /api/cases/{id}/workflow/transition
pub async fn transition_case_workflow(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<WorkflowTransitionInput>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let input = params.data;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	if current.status.eq_ignore_ascii_case("locked") {
		return Err(Error::BadRequest {
			message: "locked cases are read-only".to_string(),
		});
	}
	let workflow = load_workflow_runtime_settings(&ctx, &mm).await?;
	if !workflow.enabled {
		return Err(Error::BadRequest {
			message: "workflow is not enabled".to_string(),
		});
	}
	let current_status =
		workflow
			.find_status(&current.workflow_status)
			.ok_or(Error::BadRequest {
				message: format!(
					"workflow status '{}' is not configured",
					current.workflow_status
				),
			})?;
	let used_admin_override = require_current_workflow_step_owner(
		&ctx,
		&mm,
		&current_status.name,
		&current,
		current_status,
	)
	.await?;

	let to_status = input.to_status.trim();
	if to_status.is_empty() {
		return Err(Error::BadRequest {
			message: "workflow transition requires to_status".to_string(),
		});
	}

	if current.workflow_status.eq_ignore_ascii_case(to_status) {
		return Err(Error::BadRequest {
			message: "workflow transition requires a different destination status"
				.to_string(),
		});
	}

	let target_status =
		workflow.find_status(to_status).ok_or(Error::BadRequest {
			message: format!("workflow status '{to_status}' is not configured"),
		})?;

	let target_role = input
		.target_role
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(canonical_role);
	if let Some(role) = target_role.as_deref() {
		if !workflow_role_exists_and_is_active(&ctx, &mm, role).await? {
			return Err(Error::BadRequest {
				message: format!(
					"target role '{role}' is not active or does not exist"
				),
			});
		}
		if !target_status.allowed_roles.is_empty()
			&& !target_status
				.allowed_roles
				.iter()
				.any(|allowed| allowed.eq_ignore_ascii_case(role))
		{
			return Err(Error::BadRequest {
				message: format!(
					"target role '{role}' is not allowed for workflow status '{}'",
					target_status.name
				),
			});
		}
	}

	let comment = input
		.comment
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());
	let due_at = parse_workflow_due_at(input.due_at.as_deref())?;
	let workflow_description = target_status.description.clone();
	let override_reason = normalize_override_reason(
		input.override_reason.as_deref(),
		used_admin_override,
	);

	CaseWorkflowEventBmc::record_transition(
		&mm,
		ctx.user_id(),
		WorkflowTransitionRecord {
			case_id: id,
			from_status: current.workflow_status.clone(),
			from_role: current.workflow_assigned_role.clone(),
			from_user_id: current.workflow_assigned_user_id,
			to_status: target_status.name.clone(),
			target_role,
			target_user_id: input.target_user_id,
			comment,
			date_of_most_recent: None,
			due_at,
			workflow_description,
			actor_user_id: ctx.user_id(),
			actor_role: ctx.role().to_string(),
			used_admin_override,
			override_reason,
		},
	)
	.await
	.map_err(Error::Model)?;

	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity =
		crate::web::rest::case_rest::case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// POST /api/cases/{id}/workflow/assign
pub async fn assign_case_workflow(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<WorkflowAssignInput>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let input = params.data;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	if current.status.eq_ignore_ascii_case("locked") {
		return Err(Error::BadRequest {
			message: "locked cases are read-only".to_string(),
		});
	}
	let workflow = load_workflow_runtime_settings(&ctx, &mm).await?;
	if !workflow.enabled {
		return Err(Error::BadRequest {
			message: "workflow is not enabled".to_string(),
		});
	}
	let current_status =
		workflow
			.find_status(&current.workflow_status)
			.ok_or(Error::BadRequest {
				message: format!(
					"workflow status '{}' is not configured",
					current.workflow_status
				),
			})?;
	let used_admin_override = require_current_workflow_step_owner(
		&ctx,
		&mm,
		&current_status.name,
		&current,
		current_status,
	)
	.await?;

	let target_role = canonical_role(input.target_role.trim());
	if target_role.is_empty() {
		return Err(Error::BadRequest {
			message: "workflow assignment requires target_role".to_string(),
		});
	}
	if !workflow_role_exists_and_is_active(&ctx, &mm, &target_role).await? {
		return Err(Error::BadRequest {
			message: format!(
				"target role '{target_role}' is not active or does not exist"
			),
		});
	}
	if !current_status.allowed_roles.is_empty()
		&& !current_status
			.allowed_roles
			.iter()
			.any(|allowed| allowed.eq_ignore_ascii_case(&target_role))
	{
		return Err(Error::BadRequest {
			message: format!(
				"target role '{target_role}' is not allowed for workflow status '{}'",
				current_status.name
			),
		});
	}

	let comment = input
		.comment
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_string());
	let due_at = parse_workflow_due_at(input.due_at.as_deref())?;
	let workflow_description = current_status.description.clone();
	let override_reason = normalize_override_reason(
		input.override_reason.as_deref(),
		used_admin_override,
	);

	CaseWorkflowEventBmc::record_assignment(
		&mm,
		ctx.user_id(),
		WorkflowAssignRecord {
			case_id: id,
			current_status: current.workflow_status.clone(),
			from_role: current.workflow_assigned_role.clone(),
			from_user_id: current.workflow_assigned_user_id,
			target_role,
			target_user_id: input.target_user_id,
			comment,
			date_of_most_recent: None,
			due_at,
			workflow_description,
			actor_user_id: ctx.user_id(),
			actor_role: ctx.role().to_string(),
			used_admin_override,
			override_reason,
		},
	)
	.await
	.map_err(Error::Model)?;

	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity =
		crate::web::rest::case_rest::case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// GET /api/cases/{id}/workflow/events
pub async fn list_case_workflow_events(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<Vec<WorkflowEventReadResult>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;

	let rows = CaseWorkflowEventBmc::list_by_case(&mm, id)
		.await
		.map_err(Error::Model)?;
	let mut data = Vec::with_capacity(rows.len());
	for row in rows {
		data.push(workflow_event_to_read_result(&mm, row).await?);
	}

	Ok((axum::http::StatusCode::OK, Json(DataRestResult { data })))
}

/// GET /api/cases/workflow/config
pub async fn get_workflow_config_runtime(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<WorkflowConfigRuntimeDoc>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;

	let workflow = load_workflow_runtime_settings(&ctx, &mm).await?;
	let data = WorkflowConfigRuntimeDoc {
		workflow_enabled: workflow.enabled,
		statuses: workflow
			.statuses
			.into_iter()
			.map(|status| WorkflowStatusRuntimeDoc {
				name: status.name,
				editable: status.editable,
				description: status.description,
				allowed_roles: status.allowed_roles,
			})
			.collect(),
	};

	Ok((axum::http::StatusCode::OK, Json(DataRestResult { data })))
}
