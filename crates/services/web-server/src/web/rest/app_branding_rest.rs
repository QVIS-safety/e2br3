use axum::Json;
use lib_rest_core::rest_result::DataRestResult;
use serde::Serialize;

const DEFAULT_APP_NAME: &str = "QVIS Safety";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBranding {
	pub app_name: String,
	pub app_short_name: String,
}

fn env_or_default(name: &str, default: &str) -> String {
	std::env::var(name)
		.ok()
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| default.to_string())
}

pub fn app_branding() -> AppBranding {
	let app_name = env_or_default("E2BR3_APP_NAME", DEFAULT_APP_NAME);
	let app_short_name = env_or_default("E2BR3_APP_SHORT_NAME", app_name.as_str());
	AppBranding {
		app_name,
		app_short_name,
	}
}

/// GET /api/app/branding
pub async fn get_app_branding() -> Json<DataRestResult<AppBranding>> {
	Json(DataRestResult {
		data: app_branding(),
	})
}
