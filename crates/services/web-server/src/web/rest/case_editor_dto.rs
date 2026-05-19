use crate::web::rest::case_rest::CaseReadResult;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorShellDto {
	pub id: Uuid,
	pub status: String,
	pub appendices: Vec<String>,
	pub organization_id: Uuid,
	pub safety_report_id: String,
	pub dg_prd_key: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub workflow_status: String,
	pub workflow_assigned_role: Option<String>,
	pub workflow_assigned_user_id: Option<Uuid>,
	pub workflow_due_at: Option<OffsetDateTime>,
	pub workflow_description: Option<String>,
	pub workflow_updated_at: OffsetDateTime,
	pub qc_state: &'static str,
	pub is_locked: bool,
	pub can_act_on_workflow: bool,
	pub workflow_block_reason: Option<&'static str>,
}

impl From<CaseReadResult> for CaseEditorShellDto {
	fn from(value: CaseReadResult) -> Self {
		let appendices = value
			.case
			.appendices_json
			.as_deref()
			.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
			.unwrap_or_default();

		Self {
			id: value.case.id,
			status: value.case.status,
			appendices,
			organization_id: value.case.organization_id,
			safety_report_id: value.case.safety_report_id,
			dg_prd_key: value.case.dg_prd_key,
			created_at: value.case.created_at,
			updated_at: value.case.updated_at,
			workflow_status: value.case.workflow_status,
			workflow_assigned_role: value.case.workflow_assigned_role,
			workflow_assigned_user_id: value.case.workflow_assigned_user_id,
			workflow_due_at: value.case.workflow_due_at,
			workflow_description: value.case.workflow_description,
			workflow_updated_at: value.case.workflow_updated_at,
			qc_state: value.qc_state,
			is_locked: value.is_locked,
			can_act_on_workflow: value.can_act_on_workflow,
			workflow_block_reason: value.workflow_block_reason,
		}
	}
}
