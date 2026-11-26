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

pub(crate) fn env_non_empty(name: &str) -> core::result::Result<bool, String> {
	match std::env::var(name) {
		Ok(value) if !value.trim().is_empty() => Ok(true),
		Ok(_) => Err(format!("{name} must not be empty")),
		Err(std::env::VarError::NotPresent) => Ok(false),
		Err(error) => Err(format!("failed to read {name}: {error}")),
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
	];
	let esg = [
		"FDA_ESG_BASE_URL",
		"FDA_ESG_SUBMIT_PATH",
		"FDA_ESG_TIMEOUT_SECS",
	];
	let required = |key: &str| -> core::result::Result<String, String> {
		match read(key)? {
			Some(value) if !value.trim().is_empty() => Ok(value),
			_ => Err(format!("configured submission transport requires {key}")),
		}
	};
	let mut configured = 0;
	for keys in [&as2[..], &esg[..]] {
		let mut present = false;
		for key in keys {
			present |= read(key)?.is_some();
		}
		if keys == &esg[..] {
			present |= read("FDA_ESG_BEARER_TOKEN")?.is_some();
			present |= read("FDA_ESG_API_KEY")?.is_some();
		}
		if !present {
			continue;
		}
		configured += 1;
		for key in keys {
			let value = required(key)?;
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
	if configured > 1 {
		return Err("ambiguous transport config: set only one of AS2_SUBMITTER_URL or FDA_ESG_BASE_URL".into());
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
		]);
		let esg = HashMap::from([
			("FDA_ESG_BASE_URL", "https://gateway.example"),
			("FDA_ESG_SUBMIT_PATH", "submit"),
			("FDA_ESG_TIMEOUT_SECS", "20"),
		]);
		for values in [&as2, &esg] {
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
		let mut both = as2.clone();
		both.extend(esg);
		assert!(check(&both).expect_err("ambiguous").contains("ambiguous"));
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
		assert!(
			check(&HashMap::from([("FDA_ESG_API_KEY", "orphan-token")])).is_err()
		);
	}
}
