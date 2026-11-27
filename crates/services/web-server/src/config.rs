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

pub fn validate_submission_runtime_config() -> core::result::Result<(), String> {
	validate_submission_config(|key| match std::env::var(key) {
		Ok(value) => Ok(Some(value)),
		Err(std::env::VarError::NotPresent) => Ok(None),
		Err(error) => Err(format!("failed to read {key}: {error}")),
	})
}

fn validate_submission_config(
	read: impl Fn(&str) -> core::result::Result<Option<String>, String>,
) -> core::result::Result<(), String> {
	let as2 = [
		"AS2_SUBMITTER_URL",
		"AS2_ACK_CALLBACK_URL",
		"AS2_CALLBACK_TOKEN",
		"AS2_SUBMITTER_TOKEN",
		"AS2_SUBMITTER_TIMEOUT_SECS",
		"AS2_PROFILE_ENCRYPTION_KEY",
	];
	let required = |key: &str| -> core::result::Result<String, String> {
		match read(key)? {
			Some(value) if !value.trim().is_empty() => Ok(value),
			_ => Err(format!("configured submission transport requires {key}")),
		}
	};
	for keys in [&as2[..]] {
		let mut present = false;
		for key in keys {
			present |= read(key)?.is_some();
		}
		if !present {
			continue;
		}
		for key in keys {
			let value = required(key)?;
			if *key == "AS2_PROFILE_ENCRYPTION_KEY" && value.len() < 32 {
				return Err(
					"AS2_PROFILE_ENCRYPTION_KEY must contain at least 32 characters"
						.into(),
				);
			}
			if key.ends_with("_URL") {
				let url = reqwest::Url::parse(&value)
					.map_err(|_| format!("{key} must be an absolute HTTP(S) URL"))?;
				if !matches!(url.scheme(), "http" | "https")
					|| url.host_str().is_none()
					|| !url.username().is_empty()
					|| url.password().is_some()
					|| url.fragment().is_some()
				{
					return Err(format!("{key} must be an absolute HTTP(S) URL without credentials or fragment"));
				}
			}
			if key.ends_with("_TIMEOUT_SECS")
				&& !matches!(value.parse::<u64>(), Ok(1..))
			{
				return Err(format!("{key} must be a positive integer"));
			}
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	fn check(values: &HashMap<&str, &str>) -> core::result::Result<(), String> {
		validate_submission_config(|key| {
			Ok(values.get(key).map(|value| value.to_string()))
		})
	}

	#[test]
	fn configured_transports_always_require_complete_valid_configuration() {
		assert!(check(&HashMap::new()).is_ok());
		let as2 = HashMap::from([
			("AS2_SUBMITTER_URL", "http://127.0.0.1:9090"),
			(
				"AS2_ACK_CALLBACK_URL",
				"http://127.0.0.1:8080/internal/submissions/callbacks/ack",
			),
			("AS2_CALLBACK_TOKEN", "test-callback"),
			("AS2_SUBMITTER_TOKEN", "test-submit"),
			("AS2_SUBMITTER_TIMEOUT_SECS", "20"),
			(
				"AS2_PROFILE_ENCRYPTION_KEY",
				"test-only-profile-key-32-characters",
			),
		]);
		for values in [&as2] {
			assert!(check(values).is_ok());
			for key in values.keys() {
				let mut missing = values.clone();
				missing.remove(key);
				assert!(check(&missing).is_err(), "missing {key}");
				let mut blank = values.clone();
				blank.insert(key, " ");
				assert!(check(&blank).is_err(), "blank {key}");
			}
		}
		for value in ["0", "-1", "bad"] {
			let mut invalid = as2.clone();
			invalid.insert("AS2_SUBMITTER_TIMEOUT_SECS", value);
			assert!(check(&invalid).is_err());
		}
		for value in ["relative/path", "ftp://host", "http://user:pass@host"] {
			let mut invalid = as2.clone();
			invalid.insert("AS2_SUBMITTER_URL", value);
			assert!(check(&invalid).is_err());
		}
	}
}
