use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::ModelManager;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

const SETTINGS_KEY: &str = "system";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSettingsPayload {
	pub timezone: Option<String>,
	pub meddra_language: Option<String>,
	pub appendices: Option<Vec<String>>,
	pub case_number_prefix: Option<String>,
	pub case_number_padding: Option<i32>,
	pub workflow_enabled: Option<bool>,
	pub idle_session_minutes: Option<i32>,
	pub session_warning_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AdminSettingsUpdateBody {
	pub timezone: Option<String>,
	pub meddra_language: Option<String>,
	pub appendices: Option<Vec<String>>,
	pub case_number_prefix: Option<String>,
	pub case_number_padding: Option<i32>,
	pub workflow_enabled: Option<bool>,
	pub idle_session_minutes: Option<i32>,
	pub session_warning_minutes: Option<i32>,
}

fn require_admin_role(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_admin() {
		return Err(WebError::AccessDenied {
			required_role: "admin".to_string(),
		});
	}
	Ok(())
}

fn default_settings() -> AdminSettingsPayload {
	AdminSettingsPayload {
		timezone: Some("Asia/Seoul".to_string()),
		meddra_language: Some("en".to_string()),
		appendices: Some(vec!["FDA".to_string(), "MFDS".to_string()]),
		case_number_prefix: Some("ICSR".to_string()),
		case_number_padding: Some(6),
		workflow_enabled: Some(true),
		idle_session_minutes: Some(60),
		session_warning_minutes: Some(5),
	}
}

fn payload_to_value(payload: &AdminSettingsUpdateBody) -> Value {
	json!({
		"timezone": payload.timezone.clone().unwrap_or_else(|| "Asia/Seoul".to_string()),
		"meddra_language": payload.meddra_language.clone().unwrap_or_else(|| "en".to_string()),
		"appendices": payload.appendices.clone().unwrap_or_else(|| vec!["FDA".to_string(), "MFDS".to_string()]),
		"case_number_prefix": payload.case_number_prefix.clone().unwrap_or_else(|| "ICSR".to_string()),
		"case_number_padding": payload.case_number_padding.unwrap_or(6),
		"workflow_enabled": payload.workflow_enabled.unwrap_or(true),
		"idle_session_minutes": payload.idle_session_minutes.unwrap_or(60),
		"session_warning_minutes": payload.session_warning_minutes.unwrap_or(5),
	})
}

/// GET /api/admin/settings
pub async fn get_admin_settings(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<AdminSettingsPayload>)> {
	let _ctx = ctx_w.0;

	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Value,)>(
				"SELECT value FROM app_settings WHERE key = $1",
			)
			.bind(SETTINGS_KEY),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	if let Some((value,)) = row {
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
	require_admin_role(&ctx)?;

	let value = payload_to_value(&payload.data);
	let updated_by: Option<Uuid> = Some(ctx.user_id());

	mm.dbx()
		.execute(
			sqlx::query(
				r#"
				INSERT INTO app_settings (key, value, updated_by)
				VALUES ($1, $2, $3)
				ON CONFLICT (key)
				DO UPDATE SET
					value = EXCLUDED.value,
					updated_at = now(),
					updated_by = EXCLUDED.updated_by
				"#,
			)
			.bind(SETTINGS_KEY)
			.bind(&value)
			.bind(updated_by),
		)
		.await
		.map_err(|err| {
			WebError::Model(lib_core::model::Error::Store(err.to_string()))
		})?;

	let response = serde_json::from_value::<AdminSettingsPayload>(value)
		.unwrap_or_else(|_| default_settings());
	Ok((StatusCode::OK, Json(response)))
}
