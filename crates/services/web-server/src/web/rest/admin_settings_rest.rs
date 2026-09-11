use crate::runtime_settings::{self, normalize_appendices};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{NaiveDate, Utc};
use lib_core::authorization::eligible_action_ids;
use lib_core::ctx::{canonical_role, Ctx};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::terminology_import;
use lib_core::model::ModelManager;
use lib_rest_core::{
	notice_read_allowed, with_authorized_notice_update,
	with_authorized_settings_read, with_authorized_settings_update, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use uuid::Uuid;

const SETTINGS_KEY: &str = "system";
const SUPPORTED_CASE_NUMBER_SETTING: &str = "AE Row No.";
const SUPPORTED_CASE_NUMBER_SEQUENCE_CONDITION: &str = "Per sender";
const CASE_NUMBER_FORMAT_FIELDS: &[&str] = &[
	"AE Row No.",
	"Date of Most Recent Information for This Report (C.1.5)",
	"Sender's Organisation (C.3.2)",
	"Sponsor Study Number (C.5.3)",
	"Country Code",
	"Patient Name or Initials (D.1)",
	"Investigation Number (D.1.1.4)",
];

fn notice_update_allowed(
	snapshot: &lib_core::authorization::RequestAuthorizationSnapshot,
) -> bool {
	eligible_action_ids(snapshot)
		.iter()
		.any(|action| action.as_str() == "notice.update")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardNoticePayload {
	pub id: Option<String>,
	pub title: String,
	pub body: Option<String>,
	pub effective_date: Option<String>,
	pub expire_date: Option<String>,
	pub writer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminNoticesPayload {
	pub notices: Vec<DashboardNoticePayload>,
	pub revision: String,
}

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
	pub notices: Option<Vec<DashboardNoticePayload>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSettingsPayload {
	pub timezone: String,
	pub meddra_language: Option<String>,
	pub meddra_version: Option<String>,
	pub orientation: String,
	pub data_ordering: String,
	pub notation: bool,
	pub apply_sender_info_to_imported_cases: bool,
	pub import_date_update: ImportDateUpdatePayload,
	pub appendices: Vec<String>,
	pub idle_session_minutes: i32,
	pub session_warning_minutes: i32,
	pub notices: Vec<DashboardNoticePayload>,
	pub notices_revision: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminNoticesUpdateBody {
	pub notices: Vec<DashboardNoticePayload>,
	pub revision: Option<String>,
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

async fn load_notices(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<Vec<DashboardNoticePayload>> {
	let values = AdminSettingsBmc::list_dashboard_notices(ctx, mm)
		.await
		.map_err(Error::Model)?;
	let notices = values
		.into_iter()
		.map(serde_json::from_value)
		.collect::<std::result::Result<Vec<_>, _>>()?;
	Ok(notices)
}

async fn current_user_email(
	ctx: &Ctx,
	mm: &ModelManager,
	user_id: Uuid,
) -> Result<String> {
	let user: lib_core::model::user::User =
		lib_core::model::user::UserBmc::get(ctx, mm, user_id)
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("failed to resolve current user email: {err}"),
			})?;
	Ok(user.email)
}

fn normalize_notice_date(
	value: Option<String>,
	field: &str,
) -> Result<Option<String>> {
	let Some(value) = value.map(|value| value.trim().to_string()) else {
		return Ok(None);
	};
	if value.is_empty() {
		return Ok(None);
	}
	let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
		Error::BadRequest {
			message: format!("{field} must be an ISO date in YYYY-MM-DD format"),
		}
	})?;
	if parsed.format("%Y-%m-%d").to_string() != value {
		return Err(Error::BadRequest {
			message: format!("{field} must be an ISO date in YYYY-MM-DD format"),
		});
	}
	Ok(Some(value))
}

fn import_date_update_is_supported(value: &ImportDateUpdatePayload) -> bool {
	let (
		Some(date_of_creation),
		Some(most_recent_info_date),
		Some(report_first_received_date),
	) = (
		value.date_of_creation,
		value.most_recent_info_date,
		value.report_first_received_date,
	)
	else {
		return false;
	};
	runtime_settings::import_date_update_is_supported(
		date_of_creation,
		most_recent_info_date,
		report_first_received_date,
	)
}

fn validate_case_number_settings(payload: &AdminSettingsPayload) -> Result<()> {
	if payload
		.case_number_setting
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none()
	{
		return Err(Error::BadRequest {
			message: "case_number_setting is required".to_string(),
		});
	}
	if payload
		.case_number_identifier
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none()
	{
		return Err(Error::BadRequest {
			message: "case_number_identifier is required".to_string(),
		});
	}
	if payload
		.case_number_sequence_condition
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none()
	{
		return Err(Error::BadRequest {
			message: "case_number_sequence_condition is required".to_string(),
		});
	}
	if payload.case_number_setting.as_deref().map(str::trim)
		!= Some(SUPPORTED_CASE_NUMBER_SETTING)
	{
		return Err(Error::BadRequest {
			message: format!(
				"case_number_setting must be '{SUPPORTED_CASE_NUMBER_SETTING}'"
			),
		});
	}
	if payload
		.case_number_sequence_condition
		.as_deref()
		.map(str::trim)
		!= Some(SUPPORTED_CASE_NUMBER_SEQUENCE_CONDITION)
	{
		return Err(Error::BadRequest {
			message: format!(
				"case_number_sequence_condition must be '{SUPPORTED_CASE_NUMBER_SEQUENCE_CONDITION}'"
			),
		});
	}
	if !matches!(payload.case_number_padding, Some(value) if value >= 1) {
		return Err(Error::BadRequest {
			message: "case_number_padding must be a positive integer".to_string(),
		});
	}
	let fields = payload
		.case_number_format_fields
		.as_deref()
		.ok_or_else(|| Error::BadRequest {
			message: "case_number_format_fields is required".to_string(),
		})?;
	if fields.is_empty()
		|| fields.iter().any(|field| {
			let field = field.trim();
			field.is_empty() || !CASE_NUMBER_FORMAT_FIELDS.contains(&field)
		}) || fields
		.iter()
		.map(|field| field.trim())
		.collect::<HashSet<_>>()
		.len() != fields.len()
	{
		return Err(Error::BadRequest {
			message:
				"case_number_format_fields contains an invalid or duplicate field"
					.to_string(),
		});
	}
	if fields.len() != 1 || fields[0].trim() != SUPPORTED_CASE_NUMBER_SETTING {
		return Err(Error::BadRequest {
			message: format!(
				"case number format is not supported; select only '{SUPPORTED_CASE_NUMBER_SETTING}'"
			),
		});
	}
	Ok(())
}

fn normalize_notices(
	notices: Vec<DashboardNoticePayload>,
	writer: String,
) -> Result<Vec<DashboardNoticePayload>> {
	let mut seen_ids = HashSet::new();
	let mut normalized = Vec::new();
	for (index, notice) in notices.into_iter().enumerate() {
		let title = notice.title.trim().to_string();
		let body = notice.body.unwrap_or_default().trim().to_string();
		if title.is_empty() && body.is_empty() {
			continue;
		}
		if title.is_empty() {
			return Err(Error::BadRequest {
				message: format!("notice at index {index} requires a title"),
			});
		}
		let id = notice
			.id
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.ok_or_else(|| Error::BadRequest {
				message: format!("notice at index {index} requires an id"),
			})?;
		if !seen_ids.insert(id.clone()) {
			return Err(Error::BadRequest {
				message: format!("duplicate notice id '{id}'"),
			});
		}
		let effective_date =
			normalize_notice_date(notice.effective_date, "effective_date")?;
		let expire_date = normalize_notice_date(notice.expire_date, "expire_date")?;
		if let (Some(effective_date), Some(expire_date)) =
			(effective_date.as_deref(), expire_date.as_deref())
		{
			if effective_date > expire_date {
				return Err(Error::BadRequest {
					message: "effective_date must be on or before expire_date"
						.to_string(),
				});
			}
		}
		normalized.push(DashboardNoticePayload {
			id: Some(id),
			title,
			body: if body.is_empty() { None } else { Some(body) },
			effective_date,
			expire_date,
			writer: Some(writer.clone()),
		});
	}
	Ok(normalized)
}

async fn normalize_workflow_config(
	ctx: &Ctx,
	mm: &ModelManager,
	workflow: Option<WorkflowConfigPayload>,
) -> Result<WorkflowConfigPayload> {
	let known_roles = AdminSettingsBmc::known_workflow_roles(ctx, mm)
		.await
		.map_err(Error::Model)?;
	let statuses = workflow
		.ok_or_else(|| Error::BadRequest {
			message: "workflow configuration is required".to_string(),
		})?
		.statuses
		.ok_or_else(|| Error::BadRequest {
			message: "workflow statuses are required".to_string(),
		})?
		.into_iter()
		.map(|status| {
			let name = status.name.trim().to_string();
			if name.is_empty() {
				Err(Error::BadRequest {
					message: "workflow status name is required".to_string(),
				})
			} else {
				Ok(WorkflowStatusConfigPayload {
					name,
					editable: status.editable,
					description: status.description.map(|v| v.trim().to_string()),
					due_days: Some(status.due_days.ok_or_else(|| {
						Error::BadRequest {
							message: "workflow status due_days is required"
								.to_string(),
						}
					})?),
					allowed_roles: Some(
						status
							.allowed_roles
							.ok_or_else(|| Error::BadRequest {
								message: "workflow status allowed_roles is required"
									.to_string(),
							})?
							.into_iter()
							.map(|role| canonical_role(role.trim()))
							.collect(),
					),
				})
			}
		})
		.collect::<Result<Vec<_>>>()?;

	if statuses.is_empty() {
		return Err(Error::BadRequest {
			message: "workflow must define at least one status".to_string(),
		});
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
		return Err(Error::BadRequest {
			message: "workflow must define a Saved status".to_string(),
		});
	}

	for status in &statuses {
		if status.due_days.ok_or_else(|| Error::BadRequest {
			message: format!(
				"workflow status '{}' due_days is required",
				status.name
			),
		})? < 0
		{
			return Err(Error::BadRequest {
				message: format!(
					"workflow status '{}' due_days must be zero or greater",
					status.name
				),
			});
		}
		for role in
			status
				.allowed_roles
				.as_deref()
				.ok_or_else(|| Error::BadRequest {
					message: format!(
						"workflow status '{}' allowed_roles is required",
						status.name
					),
				})? {
			if role.is_empty() || !known_roles.contains(role) {
				return Err(Error::BadRequest {
					message: format!(
						"workflow status '{}' references unknown role '{}'",
						status.name, role
					),
				});
			}
		}
	}

	let configured_statuses = statuses
		.iter()
		.map(|status| status.name.to_ascii_lowercase())
		.collect::<HashSet<_>>();
	for status in AdminSettingsBmc::list_workflow_statuses_in_use(ctx, mm)
		.await
		.map_err(Error::Model)?
	{
		if !configured_statuses.contains(&status.to_ascii_lowercase()) {
			return Err(Error::BadRequest {
				message: format!(
					"cannot remove workflow status '{status}'; it is referenced by existing cases"
				),
			});
		}
	}

	Ok(WorkflowConfigPayload {
		statuses: Some(statuses),
	})
}

async fn payload_to_value(
	ctx: &Ctx,
	mm: &ModelManager,
	existing: Option<&Value>,
	payload: &AdminSettingsUpdateBody,
) -> Result<Value> {
	let mut merged = existing.cloned().ok_or_else(|| Error::BadRequest {
		message: "admin settings record is missing".to_string(),
	})?;
	let object = merged.as_object_mut().ok_or_else(|| Error::BadRequest {
		message: "stored admin settings must be a JSON object".to_string(),
	})?;
	object.remove("notices");
	let existing_timezone = object
		.get("timezone")
		.and_then(Value::as_str)
		.ok_or_else(|| Error::BadRequest {
			message: "stored timezone is required".to_string(),
		})?;
	let existing_timezone = runtime_settings::validate_timezone(existing_timezone)
		.ok_or_else(|| Error::BadRequest {
		message: "stored timezone must be a valid IANA timezone".to_string(),
	})?;
	object.insert("timezone".to_string(), json!(existing_timezone));
	let existing_data_ordering = runtime_settings::normalize_data_ordering(
		object.get("data_ordering").and_then(Value::as_str),
	)?;
	object.insert("data_ordering".to_string(), json!(existing_data_ordering));

	if let Some(timezone) = payload.timezone.as_deref() {
		let timezone =
			runtime_settings::validate_timezone(timezone).ok_or_else(|| {
				Error::BadRequest {
					message: "timezone must be a valid IANA timezone".to_string(),
				}
			})?;
		object.insert("timezone".to_string(), json!(timezone));
	}

	if let Some(orientation) = payload.orientation.as_deref() {
		let orientation = if orientation.eq_ignore_ascii_case("portrait") {
			"Portrait"
		} else if orientation.eq_ignore_ascii_case("landscape") {
			"Landscape"
		} else {
			return Err(Error::BadRequest {
				message: "orientation must be Portrait or Landscape".to_string(),
			});
		};
		object.insert("orientation".to_string(), json!(orientation));
	}

	fn set_if_present<T: Serialize>(
		object: &mut Map<String, Value>,
		key: &str,
		value: Option<&T>,
	) -> Result<()> {
		if let Some(value) = value {
			object.insert(key.to_string(), serde_json::to_value(value)?);
		}
		Ok(())
	}

	set_if_present(object, "meddra_language", payload.meddra_language.as_ref())?;
	set_if_present(object, "meddra_version", payload.meddra_version.as_ref())?;
	set_if_present(object, "idf_version", payload.idf_version.as_ref())?;
	set_if_present(object, "company_logo", payload.company_logo.as_ref())?;
	if let Some(data_ordering) = payload.data_ordering.as_deref() {
		object.insert(
			"data_ordering".to_string(),
			json!(runtime_settings::normalize_data_ordering(Some(
				data_ordering
			))?),
		);
	}
	set_if_present(
		object,
		"upload_excel_template_without_element_label",
		payload.upload_excel_template_without_element_label.as_ref(),
	)?;
	set_if_present(object, "notation", payload.notation.as_ref())?;
	set_if_present(
		object,
		"apply_comments_on_exported_xml",
		payload.apply_comments_on_exported_xml.as_ref(),
	)?;
	set_if_present(
		object,
		"apply_sender_info_to_imported_cases",
		payload.apply_sender_info_to_imported_cases.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_prefix",
		payload.case_number_prefix.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_setting",
		payload.case_number_setting.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_identifier",
		payload.case_number_identifier.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_padding",
		payload.case_number_padding.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_sequence_condition",
		payload.case_number_sequence_condition.as_ref(),
	)?;
	set_if_present(
		object,
		"case_number_format_fields",
		payload.case_number_format_fields.as_ref(),
	)?;
	set_if_present(
		object,
		"workflow_enabled",
		payload.workflow_enabled.as_ref(),
	)?;

	let existing_idle = object
		.get("idle_session_minutes")
		.and_then(Value::as_i64)
		.and_then(|value| i32::try_from(value).ok())
		.ok_or_else(|| Error::BadRequest {
			message: "stored idle_session_minutes is required".to_string(),
		})?;
	let existing_warning = object
		.get("session_warning_minutes")
		.and_then(Value::as_i64)
		.and_then(|value| i32::try_from(value).ok())
		.ok_or_else(|| Error::BadRequest {
			message: "stored session_warning_minutes is required".to_string(),
		})?;
	let idle_session_minutes = payload.idle_session_minutes.unwrap_or(existing_idle);
	let session_warning_minutes =
		payload.session_warning_minutes.unwrap_or(existing_warning);
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
	let existing_case_number_padding = object
		.get("case_number_padding")
		.and_then(Value::as_i64)
		.and_then(|value| i32::try_from(value).ok())
		.ok_or_else(|| Error::BadRequest {
			message: "stored case_number_padding is required".to_string(),
		})?;
	let case_number_padding = payload
		.case_number_padding
		.unwrap_or(existing_case_number_padding);
	if case_number_padding < 1 {
		return Err(Error::BadRequest {
			message: "case_number_padding must be a positive integer".to_string(),
		});
	}
	set_if_present(object, "idle_session_minutes", Some(&idle_session_minutes))?;
	set_if_present(
		object,
		"session_warning_minutes",
		Some(&session_warning_minutes),
	)?;

	if let Some(appendices) = payload.appendices.as_deref() {
		let appendices = normalize_appendices(Some(appendices))?;
		object.insert("appendices".to_string(), json!(appendices));
	}

	let mut import_date_update = object
		.get("import_date_update")
		.and_then(Value::as_object)
		.cloned()
		.ok_or_else(|| Error::BadRequest {
			message: "stored import_date_update is required".to_string(),
		})?;
	if let Some(import_date) = payload.import_date_update.as_ref() {
		set_if_present(
			&mut import_date_update,
			"date_of_creation",
			import_date.date_of_creation.as_ref(),
		)?;
		set_if_present(
			&mut import_date_update,
			"most_recent_info_date",
			import_date.most_recent_info_date.as_ref(),
		)?;
		set_if_present(
			&mut import_date_update,
			"report_first_received_date",
			import_date.report_first_received_date.as_ref(),
		)?;
	}
	let import_date_update = serde_json::from_value::<ImportDateUpdatePayload>(
		Value::Object(import_date_update.clone()),
	)
	.map_err(|_| Error::BadRequest {
		message: "import_date_update contains invalid boolean values".to_string(),
	})?;
	if !import_date_update_is_supported(&import_date_update) {
		return Err(Error::BadRequest {
			message: "import_date_update must be one of the four supported states"
				.to_string(),
		});
	}
	object.insert(
		"import_date_update".to_string(),
		serde_json::to_value(import_date_update)?,
	);

	if payload.workflow.is_some() {
		let workflow =
			normalize_workflow_config(ctx, mm, payload.workflow.clone()).await?;
		object.insert("workflow".to_string(), serde_json::to_value(workflow)?);
	}

	runtime_settings::RuntimeSettings::from_value(Some(&merged))?;
	let validated_payload =
		serde_json::from_value::<AdminSettingsPayload>(merged.clone())?;
	runtime_settings_payload(validated_payload.clone(), Vec::new(), String::new())?;
	let meddra_language = validated_payload
		.meddra_language
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.ok_or_else(|| Error::BadRequest {
			message: "MedDRA language and version are required".to_string(),
		})?;
	let meddra_version = validated_payload
		.meddra_version
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.ok_or_else(|| Error::BadRequest {
			message: "MedDRA language and version are required".to_string(),
		})?;
	if ["meddra_language", "meddra_version"].iter().any(|field| {
		merged.get(*field) != existing.and_then(|value| value.get(*field))
	}) {
		let language = match meddra_language.to_ascii_lowercase().as_str() {
			"english" => "en".to_string(),
			"korean" => "ko".to_string(),
			_ => meddra_language.to_ascii_lowercase(),
		};
		let releases = terminology_import::fetch_releases(mm, Some("meddra"), None)
			.await
			.map_err(|error| match error {
				terminology_import::ImportError::BadInput(message) => {
					Error::BadRequest { message }
				}
				terminology_import::ImportError::Store(message) => {
					Error::Model(lib_core::model::Error::Store(message))
				}
			})?;
		if !releases.iter().any(|release| {
			release.language.eq_ignore_ascii_case(&language)
				&& release.version == meddra_version
				&& matches!(
					release.status.as_str(),
					"active" | "approved" | "validated"
				)
		}) {
			return Err(Error::BadRequest {
				message: "Select an available MedDRA language and version."
					.to_string(),
			});
		}
	}
	if validated_payload.workflow_enabled.is_none() {
		return Err(Error::BadRequest {
			message: "workflow_enabled is required".to_string(),
		});
	}
	validate_case_number_settings(&validated_payload)?;
	normalize_workflow_config(ctx, mm, validated_payload.workflow.clone()).await?;

	Ok(merged)
}

async fn load_admin_settings_payload(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<AdminSettingsPayload> {
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	if let Some(value) = value {
		runtime_settings::RuntimeSettings::from_value(Some(&value))?;
		let mut payload = serde_json::from_value::<AdminSettingsPayload>(value)?;
		let runtime_payload =
			runtime_settings_payload(payload.clone(), Vec::new(), String::new())?;
		if payload
			.meddra_language
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.is_none()
			|| payload
				.meddra_version
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty())
				.is_none()
		{
			return Err(Error::BadRequest {
				message: "MedDRA language and version are required".to_string(),
			});
		}
		if payload.workflow_enabled.is_none() {
			return Err(Error::BadRequest {
				message: "workflow_enabled is required".to_string(),
			});
		}
		let case_number_padding =
			payload
				.case_number_padding
				.ok_or_else(|| Error::BadRequest {
					message: "case_number_padding is required".to_string(),
				})?;
		if case_number_padding < 1 {
			return Err(Error::BadRequest {
				message: "case_number_padding must be a positive integer"
					.to_string(),
			});
		}
		normalize_workflow_config(ctx, mm, payload.workflow.clone()).await?;
		payload.timezone = Some(runtime_payload.timezone);
		payload.data_ordering = Some(runtime_payload.data_ordering);
		payload.appendices = Some(runtime_payload.appendices);
		payload.notices = Some(load_notices(ctx, mm).await?);
		return Ok(payload);
	}
	Err(Error::BadRequest {
		message: "admin settings record is missing".to_string(),
	})
}

fn active_notices(
	notices: Vec<DashboardNoticePayload>,
	timezone: &str,
) -> Result<Vec<DashboardNoticePayload>> {
	let timezone =
		timezone
			.parse::<chrono_tz::Tz>()
			.map_err(|_| Error::BadRequest {
				message: "stored timezone must be a valid IANA timezone".to_string(),
			})?;
	let today = Utc::now().with_timezone(&timezone).date_naive();
	notices
		.into_iter()
		.map(|notice| {
			let effective = notice
				.effective_date
				.as_deref()
				.map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
				.transpose()
				.map_err(|_| Error::BadRequest {
					message: "stored notice effective_date is invalid".to_string(),
				})?;
			let expire = notice
				.expire_date
				.as_deref()
				.map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
				.transpose()
				.map_err(|_| Error::BadRequest {
					message: "stored notice expire_date is invalid".to_string(),
				})?;
			if let (Some(effective), Some(expire)) = (effective, expire) {
				if effective > expire {
					return Err(Error::BadRequest {
						message: "stored notice effective_date is after expire_date"
							.to_string(),
					});
				}
			}
			Ok((notice, effective, expire))
		})
		.filter_map(|result| match result {
			Ok((notice, effective, expire)) => {
				let effective_ok = match effective {
					None => true,
					Some(value) => value <= today,
				};
				let expire_ok = match expire {
					None => true,
					Some(value) => value >= today,
				};
				(effective_ok && expire_ok).then_some(Ok(notice))
			}
			Err(err) => Some(Err(err)),
		})
		.collect()
}

fn runtime_settings_payload(
	payload: AdminSettingsPayload,
	notices: Vec<DashboardNoticePayload>,
	notices_revision: String,
) -> Result<RuntimeSettingsPayload> {
	let timezone = payload.timezone.ok_or_else(|| Error::BadRequest {
		message: "timezone is required".to_string(),
	})?;
	let timezone =
		runtime_settings::validate_timezone(&timezone).ok_or_else(|| {
			Error::BadRequest {
				message: "stored timezone must be a valid IANA timezone".to_string(),
			}
		})?;
	let orientation = payload.orientation.ok_or_else(|| Error::BadRequest {
		message: "orientation is required".to_string(),
	})?;
	if !matches!(orientation.as_str(), "Portrait" | "Landscape") {
		return Err(Error::BadRequest {
			message: "stored orientation must be Portrait or Landscape".to_string(),
		});
	}
	let data_ordering =
		runtime_settings::normalize_data_ordering(payload.data_ordering.as_deref())?;
	let notation = payload.notation.ok_or_else(|| Error::BadRequest {
		message: "notation is required".to_string(),
	})?;
	let apply_sender_info_to_imported_cases = payload
		.apply_sender_info_to_imported_cases
		.ok_or_else(|| Error::BadRequest {
			message: "apply_sender_info_to_imported_cases is required".to_string(),
		})?;
	let import_date_update =
		payload
			.import_date_update
			.ok_or_else(|| Error::BadRequest {
				message: "import_date_update is required".to_string(),
			})?;
	if !import_date_update_is_supported(&import_date_update) {
		return Err(Error::BadRequest {
			message: "import_date_update must contain supported boolean values"
				.to_string(),
		});
	}
	let appendices = normalize_appendices(payload.appendices.as_deref())?;
	let idle_session_minutes =
		payload
			.idle_session_minutes
			.ok_or_else(|| Error::BadRequest {
				message: "idle_session_minutes is required".to_string(),
			})?;
	let session_warning_minutes =
		payload
			.session_warning_minutes
			.ok_or_else(|| Error::BadRequest {
				message: "session_warning_minutes is required".to_string(),
			})?;
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
	Ok(RuntimeSettingsPayload {
		timezone,
		meddra_language: payload.meddra_language,
		meddra_version: payload.meddra_version,
		orientation,
		data_ordering,
		notation,
		apply_sender_info_to_imported_cases,
		import_date_update,
		appendices,
		idle_session_minutes,
		session_warning_minutes,
		notices,
		notices_revision,
	})
}

/// GET /api/settings/runtime
pub async fn get_runtime_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<RuntimeSettingsPayload>)> {
	let ctx = ctx_w.0;
	let payload = load_admin_settings_payload(&ctx, &mm).await?;
	let notices =
		if notice_read_allowed(&snapshot) || notice_update_allowed(&snapshot) {
			active_notices(
				load_notices(&ctx, &mm).await?,
				payload
					.timezone
					.as_deref()
					.ok_or_else(|| Error::BadRequest {
						message: "timezone is required".to_string(),
					})?,
			)?
		} else {
			Vec::new()
		};
	let notices_revision = AdminSettingsBmc::dashboard_notices_revision(&ctx, &mm)
		.await
		.map_err(Error::Model)?;
	Ok((
		StatusCode::OK,
		Json(runtime_settings_payload(
			payload,
			notices,
			notices_revision,
		)?),
	))
}

/// GET /api/admin/settings
pub async fn get_admin_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<AdminSettingsPayload>)> {
	let ctx = ctx_w.0;
	let can_read_notices = notice_read_allowed(&snapshot);
	with_authorized_settings_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let mut payload = load_admin_settings_payload(ctx, mm).await?;
			if !can_read_notices {
				payload.notices = Some(Vec::new());
			}
			Ok((StatusCode::OK, Json(payload)))
		})
	})
	.await
}

/// PUT /api/admin/settings
pub async fn update_admin_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(payload): Json<
		lib_rest_core::rest_params::ParamsForUpdate<AdminSettingsUpdateBody>,
	>,
) -> Result<(StatusCode, Json<AdminSettingsPayload>)> {
	let ctx = ctx_w.0;
	with_authorized_settings_update(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let existing = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
				.await
				.map_err(Error::Model)?;
			let value =
				payload_to_value(ctx, mm, existing.as_ref(), &payload.data).await?;
			let updated_by: Option<Uuid> = Some(ctx.user_id());
			AdminSettingsBmc::upsert(ctx, mm, SETTINGS_KEY, &value, updated_by)
				.await
				.map_err(Error::Model)?;
			let response = load_admin_settings_payload(ctx, mm).await?;
			Ok((StatusCode::OK, Json(response)))
		})
	})
	.await
}

/// PUT /api/admin/notices
pub async fn update_admin_notices(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(payload): Json<
		lib_rest_core::rest_params::ParamsForUpdate<AdminNoticesUpdateBody>,
	>,
) -> Result<(StatusCode, Json<AdminNoticesPayload>)> {
	let ctx = ctx_w.0;
	with_authorized_notice_update(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let expected_revision =
				payload.data.revision.as_deref().ok_or_else(|| {
					Error::BadRequest {
					message: "notice revision is required; reload notices before saving"
						.to_string(),
				}
				})?;
			let current_revision =
				AdminSettingsBmc::dashboard_notices_revision(ctx, mm)
					.await
					.map_err(Error::Model)?;
			if expected_revision != current_revision {
				return Err(Error::BadRequest {
					message: "notices changed on the server; reload before saving"
						.to_string(),
				});
			}
			let writer = current_user_email(ctx, mm, ctx.user_id()).await?;
			let notices = normalize_notices(payload.data.notices, writer)?;
			let values = notices
				.iter()
				.map(serde_json::to_value)
				.collect::<std::result::Result<Vec<_>, _>>()
				.map_err(|err| Error::BadRequest {
					message: format!("failed to serialize notices: {err}"),
				})?;
			AdminSettingsBmc::replace_dashboard_notices(
				ctx,
				mm,
				&values,
				ctx.user_id(),
			)
			.await
			.map_err(Error::Model)?;
			let notices = load_notices(ctx, mm).await?;
			let revision = AdminSettingsBmc::dashboard_notices_revision(ctx, mm)
				.await
				.map_err(Error::Model)?;
			Ok((
				StatusCode::OK,
				Json(AdminNoticesPayload { notices, revision }),
			))
		})
	})
	.await
}
