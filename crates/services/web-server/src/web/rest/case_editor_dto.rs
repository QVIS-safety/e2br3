use crate::web::rest::case_rest::CaseReadResult;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
		Self {
			id: value.case.id,
			status: value.case.status,
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
	pub profiles: Vec<String>,
	#[serde(
		serialize_with = "serialize_focused_appendix",
		skip_serializing_if = "FocusedAppendixResponse::is_omitted"
	)]
	pub focused_appendix: FocusedAppendixResponse,
	pub saved: bool,
	pub required_count: usize,
	pub fields: BTreeMap<String, CaseEditorFieldEnvelope>,
	pub rows: BTreeMap<String, Value>,
	pub section_summaries: Vec<Value>,
}

#[derive(Debug)]
pub enum FocusedAppendixResponse {
	Legacy(Option<String>),
	Omitted,
}

impl FocusedAppendixResponse {
	pub fn legacy(value: Option<String>) -> Self {
		Self::Legacy(value)
	}

	pub fn omitted() -> Self {
		Self::Omitted
	}

	fn is_omitted(&self) -> bool {
		matches!(self, Self::Omitted)
	}
}

fn serialize_focused_appendix<S>(
	value: &FocusedAppendixResponse,
	serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match value {
		FocusedAppendixResponse::Legacy(value) => value.serialize(serializer),
		FocusedAppendixResponse::Omitted => serializer.serialize_none(),
	}
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
	pub blocking: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorPagePatchRequest {
	pub appendix: Option<String>,
	#[serde(default)]
	pub changes: BTreeMap<String, CaseEditorFieldPatch>,
	#[serde(default)]
	pub rows: BTreeMap<String, Value>,
}

#[derive(Debug)]
pub struct CaseEditorFieldPatch {
	pub value: Option<Value>,
	pub null_flavor: Option<Option<String>>,
}

impl<'de> Deserialize<'de> for CaseEditorFieldPatch {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut raw = serde_json::Map::<String, Value>::deserialize(deserializer)?;
		let value = raw.remove("value");
		let null_flavor = match raw.remove("nullFlavor") {
			None => None,
			Some(Value::Null) => Some(None),
			Some(Value::String(value)) => Some(Some(value)),
			Some(_) => {
				return Err(D::Error::custom("nullFlavor must be a string or null"))
			}
		};
		Ok(Self { value, null_flavor })
	}
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseEditorAeListRowDto {
	pub id: Uuid,
	pub sequence_number: i32,
	pub reaction_primary_source_native: String,
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
