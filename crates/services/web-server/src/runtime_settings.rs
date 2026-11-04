use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Tz;
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::{Error, Result};
use serde_json::Value;
use time::{Date, Month};

pub const SETTINGS_KEY: &str = "system";
pub const DATA_ORDERING_BASIC: &str = "Basic";
pub const DATA_ORDERING_PRIMARY: &str = "Primary data will appear first";
pub const DATA_ORDERING_LATEST: &str = "Latest data will appear first";

/// Resolve stored/UI aliases to the values consumed by runtime and export code.
/// Missing or unsupported values are configuration errors.
pub fn normalize_data_ordering(value: Option<&str>) -> Result<String> {
	let value = value.ok_or_else(|| Error::BadRequest {
		message: "data_ordering is required".to_string(),
	})?;
	let value = value.trim();
	if value.is_empty() {
		return Err(Error::BadRequest {
			message: "data_ordering must not be empty".to_string(),
		});
	}
	let compact = value
		.chars()
		.filter(|character| character.is_ascii_alphanumeric())
		.collect::<String>()
		.to_ascii_lowercase();

	match compact.as_str() {
		"basic" | "basicdata" => Ok(DATA_ORDERING_BASIC.to_string()),
		"primary"
		| "primarydata"
		| "primarydatafirst"
		| "primarydatawillappearfirst" => Ok(DATA_ORDERING_PRIMARY.to_string()),
		"latest"
		| "latestdata"
		| "latestdatafirst"
		| "latestdatawillappearfirst" => Ok(DATA_ORDERING_LATEST.to_string()),
		_ => Err(Error::BadRequest {
			message: format!("unsupported data_ordering '{value}'"),
		}),
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportDateSettings {
	pub update_date_of_creation: bool,
	pub update_most_recent_info_date: bool,
	pub update_report_first_received_date: bool,
}

pub(crate) fn import_date_update_is_supported(
	date_of_creation: bool,
	most_recent_info_date: bool,
	report_first_received_date: bool,
) -> bool {
	matches!(
		(
			date_of_creation,
			most_recent_info_date,
			report_first_received_date,
		),
		(false, false, false)
			| (true, false, false)
			| (true, true, false)
			| (true, true, true)
	)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSettings {
	pub timezone: String,
	pub appendices: Vec<RegulatoryAuthority>,
	pub notation: bool,
	pub import_dates: ImportDateSettings,
	pub apply_sender_info_to_imported_cases: bool,
	pub orientation: String,
	pub data_ordering: String,
}

impl RuntimeSettings {
	pub(crate) fn from_value(value: Option<&Value>) -> Result<Self> {
		let value = value.ok_or_else(|| Error::Configuration {
			key: "admin_settings",
			message: "admin settings record is missing".to_string(),
		})?;
		let required_string = |key: &'static str| -> Result<String> {
			let value = value
				.get(key)
				.and_then(Value::as_str)
				.map(str::trim)
				.filter(|value| !value.is_empty())
				.ok_or_else(|| Error::Configuration {
					key,
					message: format!("admin settings field '{key}' is required"),
				})?;
			Ok(value.to_string())
		};
		let required_bool = |key: &'static str| -> Result<bool> {
			value.get(key).and_then(Value::as_bool).ok_or_else(|| {
				Error::Configuration {
					key,
					message: format!("admin settings field '{key}' must be boolean"),
				}
			})
		};
		let timezone = required_string("timezone")?;
		let timezone =
			validate_timezone(&timezone).ok_or_else(|| Error::Configuration {
				key: "timezone",
				message: "stored timezone must be a valid IANA timezone".to_string(),
			})?;
		let orientation = required_string("orientation")?;
		if !matches!(orientation.as_str(), "Portrait" | "Landscape") {
			return Err(Error::Configuration {
				key: "orientation",
				message: "stored orientation must be Portrait or Landscape"
					.to_string(),
			});
		}
		let import_dates = value
			.get("import_date_update")
			.and_then(Value::as_object)
			.ok_or_else(|| Error::Configuration {
				key: "import_date_update",
				message: "import_date_update is required".to_string(),
			})?;
		let import_bool = |key: &'static str| -> Result<bool> {
			import_dates
				.get(key)
				.and_then(Value::as_bool)
				.ok_or_else(|| Error::Configuration {
					key: "import_date_update",
					message: format!("import_date_update.{key} must be boolean"),
				})
		};
		let import_dates = ImportDateSettings {
			update_date_of_creation: import_bool("date_of_creation")?,
			update_most_recent_info_date: import_bool("most_recent_info_date")?,
			update_report_first_received_date: import_bool(
				"report_first_received_date",
			)?,
		};
		if !import_date_update_is_supported(
			import_dates.update_date_of_creation,
			import_dates.update_most_recent_info_date,
			import_dates.update_report_first_received_date,
		) {
			return Err(Error::Configuration {
				key: "import_date_update",
				message:
					"import_date_update must be one of the four supported states"
						.to_string(),
			});
		}
		Ok(Self {
			timezone,
			appendices: parse_appendices(value.get("appendices")).map_err(
				|error| Error::Configuration {
					key: "appendices",
					message: error.to_string(),
				},
			)?,
			notation: required_bool("notation")?,
			import_dates,
			apply_sender_info_to_imported_cases: required_bool(
				"apply_sender_info_to_imported_cases",
			)?,
			orientation,
			data_ordering: normalize_data_ordering(
				value.get("data_ordering").and_then(Value::as_str),
			)
			.map_err(|error| Error::Configuration {
				key: "data_ordering",
				message: error.to_string(),
			})?,
		})
	}

	pub fn resolve_notation(&self, requested: Option<bool>) -> bool {
		requested.unwrap_or(self.notation)
	}

	pub fn import_date(&self) -> Date {
		let timezone = self
			.timezone
			.parse::<Tz>()
			.expect("runtime settings timezone is validated when loaded");
		let now = Utc::now();
		let local = DateTime::<Utc>::from_timestamp(
			now.timestamp(),
			now.timestamp_subsec_nanos(),
		)
		.expect("current UTC timestamp is valid")
		.with_timezone(&timezone)
		.date_naive();
		Date::from_calendar_date(
			local.year(),
			Month::try_from(local.month() as u8).expect("chrono month is valid"),
			local.day() as u8,
		)
		.expect("current local date is valid")
	}
}

pub fn validate_timezone(value: &str) -> Option<String> {
	let value = value.trim();
	if value.is_empty() {
		return None;
	}
	value
		.parse::<Tz>()
		.ok()
		.map(|timezone| timezone.to_string())
}

pub async fn load(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
) -> Result<RuntimeSettings> {
	let value = AdminSettingsBmc::get(ctx, mm, SETTINGS_KEY)
		.await
		.map_err(Error::Model)?;
	RuntimeSettings::from_value(value.as_ref())
}

pub fn normalize_appendices(value: Option<&[String]>) -> Result<Vec<String>> {
	let values = value.ok_or_else(|| Error::BadRequest {
		message: "appendices are required".to_string(),
	})?;
	if values.is_empty() {
		return Err(Error::BadRequest {
			message: "appendices must include at least one supported authority"
				.to_string(),
		});
	}
	let mut normalized = Vec::with_capacity(values.len());
	for value in values {
		let value = value.trim().to_ascii_uppercase();
		if !matches!(value.as_str(), "ICH" | "FDA" | "MFDS") {
			return Err(Error::BadRequest {
				message: format!("unsupported appendix '{value}'"),
			});
		}
		if !normalized.contains(&value) {
			normalized.push(value);
		}
	}
	normalized.sort_by_key(|value| match value.as_str() {
		"ICH" => 0,
		"FDA" => 1,
		"MFDS" => 2,
		_ => unreachable!(),
	});
	Ok(normalized)
}

fn parse_appendices(value: Option<&Value>) -> Result<Vec<RegulatoryAuthority>> {
	let values =
		value
			.and_then(Value::as_array)
			.ok_or_else(|| Error::BadRequest {
				message: "appendices are required".to_string(),
			})?;
	if values.is_empty() {
		return Err(Error::BadRequest {
			message: "appendices must include at least one supported authority"
				.to_string(),
		});
	}
	values
		.iter()
		.map(|value| {
			let value = value.as_str().ok_or_else(|| Error::BadRequest {
				message: "appendices must contain strings".to_string(),
			})?;
			RegulatoryAuthority::parse(value).ok_or_else(|| Error::BadRequest {
				message: format!("unsupported appendix '{value}'"),
			})
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn validates_only_real_iana_timezones() {
		assert_eq!(
			validate_timezone(" Asia/Seoul "),
			Some("Asia/Seoul".to_string())
		);
		assert_eq!(validate_timezone("not-a-timezone"), None);
	}

	#[test]
	fn resolves_notation_and_appendices_from_admin_settings() {
		let value = serde_json::json!({
			"timezone": "Asia/Seoul",
			"notation": true,
			"import_date_update": {
				"date_of_creation": false,
				"most_recent_info_date": false,
				"report_first_received_date": false,
			},
			"apply_sender_info_to_imported_cases": false,
			"orientation": "Landscape",
			"data_ordering": "Basic",
			"appendices": ["FDA"],
		});
		let settings = RuntimeSettings::from_value(Some(&value)).unwrap();

		assert!(settings.resolve_notation(None));
		assert!(!settings.resolve_notation(Some(false)));
		assert_eq!(settings.appendices, vec![RegulatoryAuthority::Fda]);
	}

	#[test]
	fn does_not_invent_an_appendix_for_empty_settings() {
		let value = serde_json::json!({
			"timezone": "Asia/Seoul",
			"notation": false,
			"import_date_update": {
				"date_of_creation": false,
				"most_recent_info_date": false,
				"report_first_received_date": false,
			},
			"apply_sender_info_to_imported_cases": false,
			"orientation": "Landscape",
			"data_ordering": "Basic",
			"appendices": [],
		});
		let settings = RuntimeSettings::from_value(Some(&value));

		assert!(settings.is_err());
		assert!(normalize_appendices(Some(&[] as &[String])).is_err());
	}
}
