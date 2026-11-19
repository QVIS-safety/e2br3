use axum::Json;
use lib_rest_core::rest_result::DataRestResult;
use serde::Serialize;

use lib_rest_core::{Error, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBranding {
	pub app_name: String,
	pub app_short_name: String,
}

fn required_name(name: &'static str) -> Result<String> {
	let value = std::env::var(name).map_err(|error| Error::Configuration {
		key: name,
		message: error.to_string(),
	})?;
	if value.trim().is_empty() {
		return Err(Error::Configuration {
			key: name,
			message: "branding name must not be empty".into(),
		});
	}
	Ok(value.trim().to_string())
}

pub fn app_branding() -> Result<AppBranding> {
	let app_name = required_name("E2BR3_APP_NAME")?;
	let app_short_name = required_name("E2BR3_APP_SHORT_NAME")?;
	Ok(AppBranding {
		app_name,
		app_short_name,
	})
}

/// GET /api/app/branding
pub async fn get_app_branding() -> Result<Json<DataRestResult<AppBranding>>> {
	Ok(Json(DataRestResult {
		data: app_branding()?,
	}))
}
