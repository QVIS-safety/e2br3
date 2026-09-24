use serde::de::{self, Deserializer};
use serde::Deserialize;
use sqlx::types::time::Date;
use time::Month;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexDateInput {
	// Most common representations.
	Str(String),
	// `time::Date` can serialize as a 2-tuple [year, ordinal] depending on serde config.
	YearOrdinal(i32, u16),
	// Some clients may send [year, month, day].
	YearMonthDay(i32, u8, u8),
}

fn parse_yyyymmdd_digits(digits: &str) -> Option<Date> {
	if digits.len() < 8 {
		return None;
	}
	let y: i32 = digits.get(0..4)?.parse().ok()?;
	let m: u8 = digits.get(4..6)?.parse().ok()?;
	let d: u8 = digits.get(6..8)?.parse().ok()?;
	let month = Month::try_from(m).ok()?;
	Date::from_calendar_date(y, month, d).ok()
}

fn parse_flexible_date_str(s: &str) -> Option<Date> {
	let trimmed = s.trim();
	if trimmed.is_empty() {
		return None;
	}

	// Accept `YYYY-MM-DD` (strip non-digits).
	let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
	parse_yyyymmdd_digits(&digits)
}

fn parse_flexible_date_str_with_partial_precision(
	s: &str,
) -> Result<Option<Date>, ()> {
	let trimmed = s.trim();
	if trimmed.is_empty() {
		return Err(());
	}
	let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
	match digits.len() {
		4 => {
			if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
				return Err(());
			}
			let year: i32 = digits.parse().map_err(|_| ())?;
			Date::from_calendar_date(year, Month::January, 1)
				.map(|_| None)
				.map_err(|_| ())
		}
		6 => {
			if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
				return Err(());
			}
			let year: i32 = digits.get(0..4).ok_or(())?.parse().map_err(|_| ())?;
			let month: u8 = digits.get(4..6).ok_or(())?.parse().map_err(|_| ())?;
			Date::from_calendar_date(
				year,
				Month::try_from(month).map_err(|_| ())?,
				1,
			)
			.map(|_| None)
			.map_err(|_| ())
		}
		_ => parse_yyyymmdd_digits(&digits).map(Some).ok_or(()),
	}
}

fn format_e2b_datetime(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}000000",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn normalize_e2b_datetime_str(s: &str) -> Option<String> {
	let trimmed = s.trim();
	if trimmed.is_empty() {
		return None;
	}

	let offset = trimmed
		.get(trimmed.len().saturating_sub(5)..)
		.filter(|value| {
			let bytes = value.as_bytes();
			bytes.len() == 5
				&& (bytes[0] == b'+' || bytes[0] == b'-')
				&& bytes[1..].iter().all(u8::is_ascii_digit)
		});
	let without_offset = offset
		.map(|_| &trimmed[..trimmed.len().saturating_sub(5)])
		.unwrap_or(trimmed);
	let digits: String = without_offset
		.chars()
		.filter(|c| c.is_ascii_digit())
		.collect();
	let date = parse_yyyymmdd_digits(&digits)?;
	let mut normalized = if digits.len() >= 14 {
		digits[..14].to_string()
	} else {
		format_e2b_datetime(date)
	};
	if let Some(offset) = offset {
		normalized.push_str(offset);
	}
	Some(normalized)
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
	D: Deserializer<'de>,
{
	let v = FlexDateInput::deserialize(deserializer)?;
	match v {
		FlexDateInput::Str(s) => parse_flexible_date_str(&s).ok_or_else(|| {
			de::Error::custom(
				"invalid date: expected YYYY-MM-DD or YYYYMMDD (or YYYYMMDDhhmmss)",
			)
		}),
		FlexDateInput::YearOrdinal(year, ordinal) => {
			Date::from_ordinal_date(year, u16::max(1, ordinal)).map_err(|_| {
				de::Error::custom("invalid date: expected [year, ordinal]")
			})
		}
		FlexDateInput::YearMonthDay(year, month, day) => {
			let month = Month::try_from(month).map_err(|_| {
				de::Error::custom("invalid date: expected [year, month, day]")
			})?;
			Date::from_calendar_date(year, month, day).map_err(|_| {
				de::Error::custom("invalid date: expected [year, month, day]")
			})
		}
	}
}

pub fn deserialize_option_date<'de, D>(
	deserializer: D,
) -> Result<Option<Date>, D::Error>
where
	D: Deserializer<'de>,
{
	let opt = Option::<FlexDateInput>::deserialize(deserializer)?;
	let Some(v) = opt else { return Ok(None) };
	match v {
		FlexDateInput::Str(s) => parse_flexible_date_str(&s)
			.map(Some)
			.ok_or_else(|| de::Error::custom("invalid optional date")),
		FlexDateInput::YearOrdinal(year, ordinal) => {
			Date::from_ordinal_date(year, ordinal)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid optional date"))
		}
		FlexDateInput::YearMonthDay(year, month, day) => {
			let month = Month::try_from(month)
				.map_err(|_| de::Error::custom("invalid optional date"))?;
			Date::from_calendar_date(year, month, day)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid optional date"))
		}
	}
}

pub fn deserialize_option_date_with_partial_precision<'de, D>(
	deserializer: D,
) -> Result<Option<Date>, D::Error>
where
	D: Deserializer<'de>,
{
	let opt = Option::<FlexDateInput>::deserialize(deserializer)?;
	let Some(v) = opt else { return Ok(None) };
	match v {
		FlexDateInput::Str(s) => parse_flexible_date_str_with_partial_precision(&s)
			.map_err(|_| de::Error::custom("invalid optional date")),
		FlexDateInput::YearOrdinal(year, ordinal) => {
			Date::from_ordinal_date(year, ordinal)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid optional date"))
		}
		FlexDateInput::YearMonthDay(year, month, day) => {
			let month = Month::try_from(month)
				.map_err(|_| de::Error::custom("invalid optional date"))?;
			Date::from_calendar_date(year, month, day)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid optional date"))
		}
	}
}

pub fn deserialize_optional_partial_ts_raw<'de, D>(
	deserializer: D,
) -> Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let value = Option::<String>::deserialize(deserializer)?;
	Ok(value.map(|value| {
		let trimmed = value.trim();
		let digits: String =
			trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
		if matches!(digits.len(), 4 | 6) {
			trimmed.to_string()
		} else {
			// A present full-precision/empty value clears an older partial raw
			// value through the update model's non-NULL field handling.
			String::new()
		}
	}))
}

pub fn deserialize_option_e2b_datetime<'de, D>(
	deserializer: D,
) -> Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let opt = Option::<FlexDateInput>::deserialize(deserializer)?;
	let Some(v) = opt else { return Ok(None) };
	Ok(match v {
		FlexDateInput::Str(s) => normalize_e2b_datetime_str(&s),
		FlexDateInput::YearOrdinal(year, ordinal) => {
			Date::from_ordinal_date(year, u16::max(1, ordinal))
				.ok()
				.map(format_e2b_datetime)
		}
		FlexDateInput::YearMonthDay(year, month, day) => {
			let month = Month::try_from(month).ok();
			month
				.and_then(|m| Date::from_calendar_date(year, m, day).ok())
				.map(format_e2b_datetime)
		}
	})
}

pub fn deserialize_option_f_r_1_ts<'de, D>(
	deserializer: D,
) -> Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let value = Option::<FlexDateInput>::deserialize(deserializer)?;
	let Some(value) = value else { return Ok(None) };
	let format_date = |date: Date| {
		format!(
			"{:04}{:02}{:02}",
			date.year(),
			u8::from(date.month()),
			date.day()
		)
	};
	match value {
		FlexDateInput::Str(value) => {
			let value = value.trim();
			if value.is_empty() {
				return Ok(None);
			}
			let issues = input_contracts::generated::f::f_r_1(
				input_contracts::FieldInput::new(
					input_contracts::InputValue::String(value),
					None,
				),
			);
			if issues.is_empty() {
				return Ok(Some(value.to_string()));
			}
			let date = (value.len() == 10
				&& value.as_bytes().get(4) == Some(&b'-')
				&& value.as_bytes().get(7) == Some(&b'-'))
			.then(|| {
				let year = value.get(0..4)?.parse().ok()?;
				let month = value.get(5..7)?.parse::<u8>().ok()?;
				let day = value.get(8..10)?.parse().ok()?;
				Date::from_calendar_date(year, Month::try_from(month).ok()?, day)
					.ok()
			})
			.flatten();
			date.map(format_date)
				.map(Some)
				.ok_or_else(|| de::Error::custom("invalid F.r.1 test date"))
		}
		FlexDateInput::YearOrdinal(year, ordinal) => {
			Date::from_ordinal_date(year, ordinal)
				.map(format_date)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid F.r.1 test date"))
		}
		FlexDateInput::YearMonthDay(year, month, day) => {
			let month = Month::try_from(month)
				.map_err(|_| de::Error::custom("invalid F.r.1 test date"))?;
			Date::from_calendar_date(year, month, day)
				.map(format_date)
				.map(Some)
				.map_err(|_| de::Error::custom("invalid F.r.1 test date"))
		}
	}
}

pub fn e2b_datetime_date(value: &str) -> Option<Date> {
	let value = value.trim();
	let local = value
		.char_indices()
		.find(|(index, char)| *index >= 8 && matches!(char, '+' | '-'))
		.map(|(index, _)| &value[..index])
		.unwrap_or(value)
		.split('.')
		.next()
		.unwrap_or(value);
	let digits: String = local.chars().filter(|c| c.is_ascii_digit()).collect();
	parse_yyyymmdd_digits(&digits)
}

#[cfg(test)]
mod tests {
	use super::{
		deserialize_option_date_with_partial_precision, deserialize_option_f_r_1_ts,
		deserialize_optional_partial_ts_raw,
	};
	use serde::Deserialize;
	use sqlx::types::time::Date;
	use time::Month;

	#[derive(Deserialize)]
	struct Input {
		#[serde(deserialize_with = "deserialize_optional_partial_ts_raw")]
		value: Option<String>,
	}

	#[test]
	fn partial_raw_deserializer_clears_full_or_empty_values() {
		let partial: Input =
			serde_json::from_value(serde_json::json!({"value":"200304"}))
				.expect("partial raw");
		assert_eq!(partial.value, Some("200304".to_string()));

		let full: Input =
			serde_json::from_value(serde_json::json!({"value":"20240101"}))
				.expect("full raw");
		assert_eq!(full.value, Some(String::new()));

		let empty: Input = serde_json::from_value(serde_json::json!({"value":""}))
			.expect("empty raw");
		assert_eq!(empty.value, Some(String::new()));
	}

	#[derive(Deserialize)]
	struct DateInput {
		#[serde(
			deserialize_with = "deserialize_option_date_with_partial_precision"
		)]
		value: Option<sqlx::types::time::Date>,
	}

	#[test]
	fn partial_dates_do_not_create_synthetic_days() {
		for value in ["2003", "200304"] {
			let parsed: DateInput = serde_json::from_value(serde_json::json!({
				"value": value
			}))
			.expect("legal partial date");
			assert_eq!(parsed.value, None);
		}

		let parsed: DateInput = serde_json::from_value(serde_json::json!({
			"value": "20030405"
		}))
		.expect("full date");
		assert!(parsed.value.is_some());
	}

	#[test]
	fn e2b_datetime_date_ignores_time_offset_and_fraction() {
		assert_eq!(
			super::e2b_datetime_date("20240101120000.1234+0900"),
			Date::from_calendar_date(2024, Month::January, 1).ok()
		);
	}

	#[derive(Deserialize)]
	struct TestDateInput {
		#[serde(deserialize_with = "deserialize_option_f_r_1_ts")]
		value: Option<String>,
	}

	#[test]
	fn f_r_1_deserializer_preserves_valid_precision_and_rejects_invalid_dates() {
		for value in ["2024", "202402", "20240229", "20240229123045+0900"] {
			let parsed: TestDateInput = serde_json::from_value(serde_json::json!({
				"value": value
			}))
			.expect("valid F.r.1 date");
			assert_eq!(parsed.value.as_deref(), Some(value));
		}
		for value in ["202413", "20230229"] {
			assert!(serde_json::from_value::<TestDateInput>(serde_json::json!({
				"value": value
			}))
			.is_err());
		}
		for (value, expected) in [
			(serde_json::json!("2024-02-29"), "20240229"),
			(serde_json::json!([2024, 60]), "20240229"),
			(serde_json::json!([2024, 2, 29]), "20240229"),
		] {
			let parsed: TestDateInput = serde_json::from_value(serde_json::json!({
				"value": value
			}))
			.expect("compatible full date");
			assert_eq!(parsed.value.as_deref(), Some(expected));
		}
	}
}
