use crate::web::rest::case_rest::CaseReadResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::time::OffsetDateTime;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorShellDto {
	pub id: Uuid,
	pub status: String,
	pub organization_id: Uuid,
	pub safety_report_identification: CaseEditorShellSafetyReportDto,
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
	pub case_write_block_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorShellSafetyReportDto {
	pub safety_report_id: String,
}

impl CaseEditorShellDto {
	pub fn from_case_read_result(
		value: CaseReadResult,
		safety_report_id: String,
	) -> Self {
		Self {
			id: value.case.id,
			status: value.case.status,
			organization_id: value.case.organization_id,
			safety_report_identification: CaseEditorShellSafetyReportDto {
				safety_report_id,
			},
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
			case_write_block_reason: value.case_write_block_reason,
		}
	}
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorListResponse<T> {
	pub case_id: Uuid,
	pub rows: Vec<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorRowDetailResponse {
	pub case_id: Uuid,
	pub row_id: Uuid,
	pub data: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorDirectSectionResponse {
	pub case_id: Uuid,
	pub data: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorPageProjectionResponse {
	pub case_id: Uuid,
	pub page_id: &'static str,
	pub authorities: Vec<String>,
	pub saved: bool,
	pub required_count: usize,
	pub fields: BTreeMap<String, CaseEditorFieldEnvelope>,
	pub rows: BTreeMap<String, Value>,
	pub section_summaries: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiCaseDto {
	pub report_year: Option<String>,
	pub fda_report_type: Option<String>,
	pub mfds_report_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiRowsDto {
	pub case: CaseEditorCiCaseDto,
	pub safety_report_identification: Option<CaseEditorCiSafetyReportDto>,
	pub other_case_identifiers: Vec<CaseEditorCiOtherIdentifierDto>,
	pub linked_reports: Vec<CaseEditorCiLinkedReportDto>,
	pub documents_held_by_sender: Vec<CaseEditorCiDocumentDto>,
	pub source_documents: Vec<CaseEditorCiSourceDocumentDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiSafetyReportDto {
	pub id: Uuid,
	pub safety_report_id: Option<String>,
	pub safety_report_version: i32,
	pub transmission_date: Option<String>,
	pub report_type: Option<String>,
	pub date_first_received_from_source: Option<String>,
	pub date_of_most_recent_information: Option<String>,
	pub fulfil_expedited_criteria: Option<bool>,
	pub fulfil_expedited_criteria_null_flavor: Option<String>,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub combination_product_report_indicator_null_flavor: Option<String>,
	pub worldwide_unique_id: Option<String>,
	pub first_sender_type: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub other_case_identifiers_exist: Option<bool>,
	pub other_case_identifiers_exist_null_flavor: Option<String>,
	pub nullification_amendment_code: Option<String>,
	pub nullification_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiDocumentDto {
	pub id: Uuid,
	pub document_description: Option<String>,
	pub included_document: Option<String>,
	pub file_name: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub sequence_number: i32,
	pub deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiOtherIdentifierDto {
	pub id: Uuid,
	pub source: String,
	pub case_identifier: String,
	pub sequence_number: i32,
	pub deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiLinkedReportDto {
	pub id: Uuid,
	pub linked_report_number: String,
	pub sequence_number: i32,
	pub deleted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorCiSourceDocumentDto {
	pub id: Uuid,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
	pub sequence_number: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorFieldEnvelope {
	pub field_id: &'static str,
	pub path: &'static str,
	pub label: &'static str,
	pub value: Value,
	pub display: Option<String>,
	pub null_flavor: Option<String>,
	pub notation: Option<String>,
	pub origin_value: Value,
	pub origin_null_flavor: Option<String>,
	pub visible: bool,
	pub editable: bool,
	pub empty: bool,
	pub required_empty: bool,
	pub issues: Vec<CaseEditorFieldIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorFieldIssue {
	pub code: String,
	pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseEditorPagePatchRequest {
	pub authorities: Option<Vec<String>>,
	#[serde(default)]
	pub rows: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseFollowUpRowPatchRequest {
	pub client_row_id: Option<String>,
	pub request: CaseEditorPagePatchRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseFollowUpPagePatchRequest {
	pub ci: CaseEditorPagePatchRequest,
	pub rp: CaseEditorPagePatchRequest,
	pub sd: CaseEditorPagePatchRequest,
	pub si: CaseEditorPagePatchRequest,
	pub dm: CaseEditorPagePatchRequest,
	pub nr: CaseEditorPagePatchRequest,
	#[serde(default)]
	pub lr: Vec<CaseFollowUpRowPatchRequest>,
	#[serde(default)]
	pub dh: Vec<CaseFollowUpRowPatchRequest>,
	#[serde(default)]
	pub ae: Vec<CaseFollowUpRowPatchRequest>,
	#[serde(default)]
	pub lb: Vec<CaseFollowUpRowPatchRequest>,
	#[serde(default)]
	pub dg: Vec<CaseFollowUpRowPatchRequest>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorAeListRowDto {
	pub id: Uuid,
	pub sequence_number: i32,
	pub deleted: bool,
	pub reaction_primary_source_native: Option<String>,
	pub reaction_primary_source_translation: Option<String>,
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub seriousness: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorLbListRowDto {
	pub id: Uuid,
	pub sequence_number: i32,
	pub deleted: bool,
	pub test_name: String,
	pub test_date: Option<String>,
	pub result_value: Option<String>,
	pub result_unit: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorDgListRowDto {
	pub id: Uuid,
	pub sequence_number: i32,
	pub deleted: bool,
	pub drug_role: String,
	pub dg_prd_key: Option<String>,
	pub medicinal_product: String,
	pub action_taken: Option<String>,
	pub warning_count: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorDhListRowDto {
	pub id: Uuid,
	pub sequence_number: i32,
	pub drug_name: Option<String>,
	pub indication: Option<String>,
	pub start_date: Option<String>,
	pub end_date: Option<String>,
}
