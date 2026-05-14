use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{
	canonical_role, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER,
};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::ModelManager;
use lib_rest_core::{require_admin, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

const SETTINGS_KEY: &str = "system";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusConfigPayload {
	pub name: String,
	pub editable: bool,
	pub description: Option<String>,
	pub allowed_roles: Option<Vec<String>>,
	pub due_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfigPayload {
	pub statuses: Option<Vec<WorkflowStatusConfigPayload>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDateUpdatePayload {
	pub date_of_creation: Option<bool>,
	pub most_recent_info_date: Option<bool>,
	pub report_first_received_date: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSettingsPayload {
	pub timezone: Option<String>,
	pub meddra_language: Option<String>,
	pub meddra_version: Option<String>,
	pub idf_version: Option<String>,
	pub company_logo: Option<String>,
	pub orientation: Option<String>,
	pub data_ordering: Option<String>,
	pub upload_excel_template_without_element_label: Option<bool>,
	pub notation: Option<bool>,
	pub apply_comments_on_exported_xml: Option<bool>,
	pub apply_sender_info_to_imported_cases: Option<bool>,
	pub apply_default_values_to_imported_r2_cases: Option<bool>,
	pub import_date_update: Option<ImportDateUpdatePayload>,
	pub appendices: Option<Vec<String>>,
	pub case_number_prefix: Option<String>,
	pub case_number_setting: Option<String>,
	pub case_number_identifier: Option<String>,
	pub case_number_padding: Option<i32>,
	pub case_number_sequence_condition: Option<String>,
	pub case_number_format_fields: Option<Vec<String>>,
	pub workflow_enabled: Option<bool>,
	pub workflow: Option<WorkflowConfigPayload>,
	pub idle_session_minutes: Option<i32>,
	pub session_warning_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AdminSettingsUpdateBody {
	pub timezone: Option<String>,
	pub meddra_language: Option<String>,
	pub meddra_version: Option<String>,
	pub idf_version: Option<String>,
	pub company_logo: Option<String>,
	pub orientation: Option<String>,
	pub data_ordering: Option<String>,
	pub upload_excel_template_without_element_label: Option<bool>,
	pub notation: Option<bool>,
	pub apply_comments_on_exported_xml: Option<bool>,
	pub apply_sender_info_to_imported_cases: Option<bool>,
	pub apply_default_values_to_imported_r2_cases: Option<bool>,
	pub import_date_update: Option<ImportDateUpdatePayload>,
	pub appendices: Option<Vec<String>>,
	pub case_number_prefix: Option<String>,
	pub case_number_setting: Option<String>,
	pub case_number_identifier: Option<String>,
	pub case_number_padding: Option<i32>,
	pub case_number_sequence_condition: Option<String>,
	pub case_number_format_fields: Option<Vec<String>>,
	pub workflow_enabled: Option<bool>,
	pub workflow: Option<WorkflowConfigPayload>,
	pub idle_session_minutes: Option<i32>,
	pub session_warning_minutes: Option<i32>,
}

fn default_settings() -> AdminSettingsPayload {
	AdminSettingsPayload {
		timezone: Some("Asia/Seoul".to_string()),
		meddra_language: Some("English".to_string()),
		meddra_version: Some(String::new()),
		idf_version: Some(String::new()),
		company_logo: Some(String::new()),
		orientation: Some("Landscape".to_string()),
		data_ordering: Some("Primary data will appear first".to_string()),
		upload_excel_template_without_element_label: Some(false),
		notation: Some(false),
		apply_comments_on_exported_xml: Some(false),
		apply_sender_info_to_imported_cases: Some(false),
		apply_default_values_to_imported_r2_cases: Some(false),
		import_date_update: Some(ImportDateUpdatePayload {
			date_of_creation: Some(false),
			most_recent_info_date: Some(false),
			report_first_received_date: Some(false),
		}),
		appendices: Some(vec![
			"ICH".to_string(),
			"FDA".to_string(),
			"MFDS".to_string(),
		]),
		case_number_prefix: Some("ICSR".to_string()),
		case_number_setting: Some(String::new()),
		case_number_identifier: Some(String::new()),
		case_number_padding: Some(6),
		case_number_sequence_condition: Some(String::new()),
		case_number_format_fields: Some(Vec::new()),
		workflow_enabled: Some(false),
		workflow: Some(default_workflow_config()),
		idle_session_minutes: Some(60),
		session_warning_minutes: Some(5),
	}
}

fn default_workflow_config() -> WorkflowConfigPayload {
	WorkflowConfigPayload {
		statuses: Some(vec![
			WorkflowStatusConfigPayload {
				name: "Saved".to_string(),
				editable: true,
				description: Some("Default authoring state".to_string()),
				due_days: Some(0),
				allowed_roles: Some(vec![ROLE_USER.to_string()]),
			},
			WorkflowStatusConfigPayload {
				name: "To be reviewed".to_string(),
				editable: false,
				description: Some("Pending internal review".to_string()),
				due_days: Some(0),
				allowed_roles: Some(vec![ROLE_USER.to_string()]),
			},
			WorkflowStatusConfigPayload {
				name: "Internal review completed".to_string(),
				editable: false,
				description: Some("QCed and routed onward".to_string()),
				due_days: Some(0),
				allowed_roles: Some(vec![ROLE_USER.to_string()]),
			},
			WorkflowStatusConfigPayload {
				name: "Finalized".to_string(),
				editable: false,
				description: Some("Final workflow state".to_string()),
				due_days: Some(0),
				allowed_roles: Some(vec![
					ROLE_SPONSOR_ADMIN_CRO.to_string(),
					ROLE_SPONSOR_ADMIN_COMPANY.to_string(),
				]),
			},
		]),
	}
}

async fn normalize_workflow_config(
	mm: &ModelManager,
	workflow: Option<WorkflowConfigPayload>,
) -> Result<WorkflowConfigPayload> {
	let known_roles = AdminSettingsBmc::known_workflow_roles(mm)
		.await
		.map_err(Error::Model)?;
	let mut statuses = workflow
		.and_then(|config| config.statuses)
		.unwrap_or_default()
		.into_iter()
		.filter_map(|status| {
			let name = status.name.trim().to_string();
			if name.is_empty() {
				None
			} else {
				Some(WorkflowStatusConfigPayload {
					name,
					editable: status.editable,
					description: status.description.map(|v| v.trim().to_string()),
					due_days: status.due_days,
					allowed_roles: status.allowed_roles.map(|roles| {
						roles
							.into_iter()
							.map(|role| canonical_role(role.trim()))
							.filter(|role| !role.is_empty())
							.collect()
					}),
				})
			}
		})
		.collect::<Vec<_>>();

	if statuses.is_empty() {
		return Ok(default_workflow_config());
	}

	let mut seen = HashSet::new();
	for status in &statuses {
		let key = status.name.to_ascii_lowercase();
		if !seen.insert(key) {
			return Err(Error::BadRequest {
				message: format!("duplicate workflow status '{}'", status.name),
			});
		}
	}

	if !statuses.iter().any(|status| status.editable) {
		return Err(Error::BadRequest {
			message: "workflow must define at least one editable status".to_string(),
		});
	}

	if !statuses
		.iter()
		.any(|status| status.name.eq_ignore_ascii_case("Saved"))
	{
		statuses.insert(
			0,
			WorkflowStatusConfigPayload {
				name: "Saved".to_string(),
				editable: true,
				description: Some("Default authoring state".to_string()),
				due_days: Some(0),
				allowed_roles: Some(vec![ROLE_USER.to_string()]),
			},
		);
	}

	for status in &statuses {
		if status.due_days.unwrap_or(0) < 0 {
			return Err(Error::BadRequest {
				message: format!(
					"workflow status '{}' due_days must be zero or greater",
					status.name
				),
			});
		}
		for role in status.allowed_roles.as_deref().unwrap_or(&[]) {
			if !known_roles.contains(role) {
				return Err(Error::BadRequest {
					message: format!(
						"workflow status '{}' references unknown role '{}'",
						status.name, role
					),
				});
			}
		}
	}

	Ok(WorkflowConfigPayload {
		statuses: Some(statuses),
	})
}

async fn payload_to_value(
	mm: &ModelManager,
	payload: &AdminSettingsUpdateBody,
) -> Result<serde_json::Value> {
	let workflow = normalize_workflow_config(mm, payload.workflow.clone()).await?;
	let idle_session_minutes = payload.idle_session_minutes.unwrap_or(60);
	let session_warning_minutes = payload.session_warning_minutes.unwrap_or(5);
	if idle_session_minutes < 5 {
		return Err(Error::BadRequest {
			message: "idle_session_minutes must be at least 5".to_string(),
		});
	}
	if session_warning_minutes < 1 {
		return Err(Error::BadRequest {
			message: "session_warning_minutes must be at least 1".to_string(),
		});
	}
	if session_warning_minutes >= idle_session_minutes {
		return Err(Error::BadRequest {
			message:
				"session_warning_minutes must be less than idle_session_minutes"
					.to_string(),
		});
	}
	let case_number_padding = payload.case_number_padding.unwrap_or(6);
	if case_number_padding < 0 {
		return Err(Error::BadRequest {
			message: "case_number_padding must be zero or greater".to_string(),
		});
	}
	Ok(json!({
		"timezone": payload.timezone.clone().unwrap_or_else(|| "Asia/Seoul".to_string()),
		"meddra_language": payload.meddra_language.clone().unwrap_or_else(|| "English".to_string()),
		"meddra_version": payload.meddra_version.clone().unwrap_or_default(),
		"idf_version": payload.idf_version.clone().unwrap_or_default(),
		"company_logo": payload.company_logo.clone().unwrap_or_default(),
		"orientation": payload.orientation.clone().unwrap_or_else(|| "Landscape".to_string()),
		"data_ordering": payload.data_ordering.clone().unwrap_or_else(|| "Primary data will appear first".to_string()),
		"upload_excel_template_without_element_label": payload.upload_excel_template_without_element_label.unwrap_or(false),
		"notation": payload.notation.unwrap_or(false),
		"apply_comments_on_exported_xml": payload.apply_comments_on_exported_xml.unwrap_or(false),
		"apply_sender_info_to_imported_cases": payload.apply_sender_info_to_imported_cases.unwrap_or(false),
		"apply_default_values_to_imported_r2_cases": payload.apply_default_values_to_imported_r2_cases.unwrap_or(false),
		"import_date_update": payload.import_date_update.clone().unwrap_or(ImportDateUpdatePayload {
			date_of_creation: Some(false),
			most_recent_info_date: Some(false),
			report_first_received_date: Some(false),
		}),
		"appendices": payload.appendices.clone().unwrap_or_else(|| vec!["ICH".to_string(), "FDA".to_string(), "MFDS".to_string()]),
		"case_number_prefix": payload.case_number_prefix.clone().unwrap_or_else(|| "ICSR".to_string()),
		"case_number_setting": payload.case_number_setting.clone().unwrap_or_default(),
		"case_number_identifier": payload.case_number_identifier.clone().unwrap_or_default(),
		"case_number_padding": case_number_padding,
		"case_number_sequence_condition": payload.case_number_sequence_condition.clone().unwrap_or_default(),
		"case_number_format_fields": payload.case_number_format_fields.clone().unwrap_or_default(),
		"workflow_enabled": payload.workflow_enabled.unwrap_or(false),
		"workflow": workflow,
		"idle_session_minutes": idle_session_minutes,
		"session_warning_minutes": session_warning_minutes,
	}))
}

/// GET /api/admin/settings
pub async fn get_admin_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<AdminSettingsPayload>)> {
	let _ctx = ctx_w.0;
	let value = AdminSettingsBmc::get(&mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	if let Some(value) = value {
		let payload = serde_json::from_value::<AdminSettingsPayload>(value)
			.unwrap_or_else(|_| default_settings());
		return Ok((StatusCode::OK, Json(payload)));
	}
	Ok((StatusCode::OK, Json(default_settings())))
}

/// PUT /api/admin/settings
pub async fn update_admin_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(payload): Json<
		lib_rest_core::rest_params::ParamsForUpdate<AdminSettingsUpdateBody>,
	>,
) -> Result<(StatusCode, Json<AdminSettingsPayload>)> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	let value = payload_to_value(&mm, &payload.data).await?;
	let updated_by: Option<Uuid> = Some(ctx.user_id());
	AdminSettingsBmc::upsert(&mm, SETTINGS_KEY, &value, updated_by)
		.await
		.map_err(Error::Model)?;
	let response = serde_json::from_value::<AdminSettingsPayload>(value)
		.unwrap_or_else(|_| default_settings());
	Ok((StatusCode::OK, Json(response)))
}
