use lib_utils::envs::get_env;
use std::sync::OnceLock;

pub fn web_config() -> &'static WebConfig {
	static INSTANCE: OnceLock<WebConfig> = OnceLock::new();

	INSTANCE.get_or_init(|| {
		WebConfig::load_from_env().unwrap_or_else(|ex| {
			panic!("FATAL - WHILE LOADING CONF - Cause: {ex:?}")
		})
	})
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

fn env_truthy(name: &str) -> bool {
	matches!(
		std::env::var(name),
		Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
	)
}

fn env_non_empty(name: &str) -> bool {
	std::env::var(name)
		.ok()
		.map(|v| !v.trim().is_empty())
		.unwrap_or(false)
}

pub fn validate_submission_runtime_config() -> core::result::Result<(), String> {
	let env_name = std::env::var("E2BR3_ENV")
		.or_else(|_| std::env::var("SERVICE_ENV"))
		.unwrap_or_else(|_| "dev".to_string())
		.to_ascii_lowercase();
	let is_prod = matches!(env_name.as_str(), "prod" | "production");
	let strict = is_prod || env_truthy("E2BR3_STRICT_SUBMISSION_CONFIG");

	let as2_enabled = env_non_empty("AS2_SUBMITTER_URL");
	let esg_enabled = env_truthy("FDA_ESG_ENABLED");
	let allow_mock = env_truthy("E2BR3_ALLOW_MOCK_SUBMISSION");

	if strict {
		if allow_mock {
			return Err(
				"E2BR3_ALLOW_MOCK_SUBMISSION must be disabled in strict/production mode".to_string(),
			);
		}
		if !as2_enabled && !esg_enabled {
			return Err(
				"missing submission transport: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
			);
		}
	}

	if as2_enabled && esg_enabled {
		return Err(
			"ambiguous transport config: set only one of AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		);
	}

	if as2_enabled {
		if !env_non_empty("AS2_ACK_CALLBACK_URL") {
			return Err(
				"AS2_SUBMITTER_URL requires AS2_ACK_CALLBACK_URL".to_string()
			);
		}
		if !env_non_empty("AS2_CALLBACK_TOKEN") {
			return Err("AS2_SUBMITTER_URL requires AS2_CALLBACK_TOKEN".to_string());
		}
	}

	if esg_enabled && !env_non_empty("FDA_ESG_BASE_URL") {
		return Err("FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string());
	}

	Ok(())
}
