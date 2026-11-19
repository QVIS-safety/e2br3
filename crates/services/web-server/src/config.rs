use lib_utils::envs::get_env;
use std::sync::OnceLock;

pub fn web_config() -> lib_utils::envs::Result<&'static WebConfig> {
	static INSTANCE: OnceLock<lib_utils::envs::Result<WebConfig>> = OnceLock::new();

	match INSTANCE.get_or_init(WebConfig::load_from_env) {
		Ok(config) => Ok(config),
		Err(error) => Err(error.clone()),
	}
}

#[allow(non_snake_case)]
pub struct WebConfig {
	pub WEB_FOLDER: String,
}

impl WebConfig {
	fn load_from_env() -> lib_utils::envs::Result<WebConfig> {
		Ok(WebConfig {
			WEB_FOLDER: get_env("SERVICE_WEB_FOLDER")?,
		})
	}
}

pub(crate) fn env_truthy(name: &str) -> core::result::Result<bool, String> {
	match std::env::var(name) {
		Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
			"1" | "true" | "yes" | "on" => Ok(true),
			"0" | "false" | "no" | "off" => Ok(false),
			_ => Err(format!("{name} must be a boolean value")),
		},
		Err(error) => Err(format!("failed to read {name}: {error}")),
	}
}

fn env_non_empty(name: &str) -> core::result::Result<bool, String> {
	match std::env::var(name) {
		Ok(value) => Ok(!value.trim().is_empty()),
		Err(std::env::VarError::NotPresent) => Ok(false),
		Err(error) => Err(format!("failed to read {name}: {error}")),
	}
}

pub fn validate_submission_runtime_config() -> core::result::Result<(), String> {
	let env_name = lib_utils::envs::get_env("E2BR3_ENV")
		.map_err(|error| format!("{error:?}"))?
		.to_ascii_lowercase();
	let is_prod = matches!(env_name.as_str(), "prod" | "production");
	let strict = is_prod || env_truthy("E2BR3_STRICT_SUBMISSION_CONFIG")?;

	let as2_enabled = env_non_empty("AS2_SUBMITTER_URL")?;
	let esg_enabled = env_truthy("FDA_ESG_ENABLED")?;

	if strict {
		if !as2_enabled && !esg_enabled {
			return Err(
				"missing submission transport: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
			);
		}
		if !env_non_empty("SUBMISSION_RECONCILE_TOKEN")? {
			return Err(
				"production requires SUBMISSION_RECONCILE_TOKEN for reconcile endpoints"
					.to_string(),
			);
		}
	}

	if as2_enabled && esg_enabled {
		return Err(
			"ambiguous transport config: set only one of AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		);
	}

	if as2_enabled {
		if !env_non_empty("AS2_ACK_CALLBACK_URL")? {
			return Err(
				"AS2_SUBMITTER_URL requires AS2_ACK_CALLBACK_URL".to_string()
			);
		}
		if !env_non_empty("AS2_CALLBACK_TOKEN")? {
			return Err("AS2_SUBMITTER_URL requires AS2_CALLBACK_TOKEN".to_string());
		}
	}

	if esg_enabled && !env_non_empty("FDA_ESG_BASE_URL")? {
		return Err("FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string());
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn feature_flag_requires_an_explicit_boolean() {
		const KEY: &str = "E2BR3_TEST_STRICT_BOOLEAN";
		std::env::remove_var(KEY);
		assert!(env_truthy(KEY).is_err());
		std::env::set_var(KEY, "treu");
		assert!(env_truthy(KEY).is_err());
		std::env::set_var(KEY, "false");
		assert!(!env_truthy(KEY).unwrap());
		std::env::set_var(KEY, "true");
		assert!(env_truthy(KEY).unwrap());
		std::env::remove_var(KEY);
	}
}
