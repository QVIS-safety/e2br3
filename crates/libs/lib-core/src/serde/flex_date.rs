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
			Date::from_ordinal_date(year, u16::max(1, ordinal) as u16).map_err(
				|_| de::Error::custom("invalid date: expected [year, ordinal]"),
			)
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
			Date::from_ordinal_date(year, u16::max(1, ordinal) as u16)
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

pub fn e2b_datetime_date(value: &str) -> Option<Date> {
	let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
	parse_yyyymmdd_digits(&digits)
}
