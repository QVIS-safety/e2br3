//! Shared primitives used by explicit field validators.

use crate::context::VocabularyContext;
use crate::ValidationIssue;
use base64::engine::{general_purpose, Engine};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Decimal;

pub(crate) enum DateValues {
	One(Option<Date>),
	Two(Option<Date>, Option<Date>),
}

impl DateValues {
	fn any_future(self) -> bool {
		match self {
			Self::One(value) => is_future_date(value),
			Self::Two(left, right) => is_future_date(left) || is_future_date(right),
		}
	}
}

fn is_future_date(value: Option<Date>) -> bool {
	value.is_some_and(|value| value > OffsetDateTime::now_utc().date())
}

pub(crate) fn reject_when(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	violated: bool,
) {
	if violated {
		crate::push_field_issue(issues, code, path, section, message);
	}
}

pub(crate) fn require(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	present: bool,
) {
	reject_when(issues, code, path, section, message, !present);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn max_length(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	value: Option<&str>,
	max: usize,
) {
	reject_when(
		issues,
		code,
		path,
		section,
		message,
		value.is_some_and(|value| value.chars().count() > max),
	);
}

pub(crate) fn reject_future_date(
	issues: &mut Vec<ValidationIssue>,
	code: &str,
	path: &str,
	section: &str,
	message: &str,
	dates: DateValues,
) {
	reject_when(issues, code, path, section, message, dates.any_future());
}

pub(crate) fn e2b_datetime_date(value: Option<&str>) -> Option<Date> {
	value.and_then(lib_core::serde::flex_date::e2b_datetime_date)
}

pub(crate) fn e2b_ts_date(value: Option<&str>) -> Option<Date> {
	let value = value?.trim();
	let local = value
		.char_indices()
		.skip(4)
		.find(|(_, char)| matches!(char, '+' | '-'))
		.map(|(index, _)| &value[..index])
		.unwrap_or(value);
	let local = local.split('.').next()?;
	let digits: String = local.chars().filter(|c| c.is_ascii_digit()).collect();
	let year = digits.get(0..4)?.parse().ok()?;
	match digits.len() {
		4 => Date::from_calendar_date(year, time::Month::January, 1).ok(),
		6 => {
			let month: u8 = digits.get(4..6)?.parse().ok()?;
			Date::from_calendar_date(year, time::Month::try_from(month).ok()?, 1)
				.ok()
		}
		_ if digits.len() >= 8 => {
			let month: u8 = digits.get(4..6)?.parse().ok()?;
			let day: u8 = digits.get(6..8)?.parse().ok()?;
			Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day)
				.ok()
		}
		_ => None,
	}
}

pub(crate) fn valid_decimal(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| value.parse::<Decimal>().is_ok())
}

pub(crate) fn valid_code(value: Option<&str>, allowed: &[&str]) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| allowed.contains(&value))
}

pub(crate) fn valid_identifier(value: Option<&str>, max_length: usize) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| {
			value.len() <= max_length && !value.chars().any(char::is_control)
		})
}

pub(crate) fn valid_base64(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| general_purpose::STANDARD.decode(value).is_ok())
}

pub(crate) fn valid_e2b_datetime(value: &str) -> bool {
	let (local, offset) = match value
		.char_indices()
		.skip(4)
		.find(|(_, char)| matches!(char, '+' | '-'))
	{
		Some((index, _)) => (&value[..index], Some(&value[index..])),
		None => (value, None),
	};
	if let Some(offset) = offset {
		let bytes = offset.as_bytes();
		if bytes.len() != 5
			|| !matches!(bytes[0], b'+' | b'-')
			|| !bytes[1..].iter().all(u8::is_ascii_digit)
		{
			return false;
		}
		let hour = offset[1..3].parse::<u8>().ok();
		let minute = offset[3..5].parse::<u8>().ok();
		if !matches!((hour, minute), (Some(0..=14), Some(0..=59))) {
			return false;
		}
	}

	let (digits, fraction) = match local.split_once('.') {
		Some((digits, fraction)) => (digits, Some(fraction)),
		None => (local, None),
	};
	if !matches!(digits.len(), 4 | 6 | 8 | 10 | 12 | 14)
		|| !digits.bytes().all(|byte| byte.is_ascii_digit())
	{
		return false;
	}
	if let Some(fraction) = fraction {
		if digits.len() != 14
			|| fraction.is_empty()
			|| fraction.len() > 4
			|| !fraction.bytes().all(|byte| byte.is_ascii_digit())
		{
			return false;
		}
	}

	let number = |range: std::ops::Range<usize>| {
		digits.get(range).and_then(|value| value.parse::<u8>().ok())
	};
	if digits.len() >= 6 && !matches!(number(4..6), Some(1..=12)) {
		return false;
	}
	if digits.len() >= 8
		&& lib_core::serde::flex_date::e2b_datetime_date(digits).is_none()
	{
		return false;
	}
	if digits.len() >= 10 && !matches!(number(8..10), Some(0..=23)) {
		return false;
	}
	if digits.len() >= 12 && !matches!(number(10..12), Some(0..=59)) {
		return false;
	}
	if digits.len() >= 14 && !matches!(number(12..14), Some(0..=59)) {
		return false;
	}
	true
}

pub(crate) fn valid_ucum(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| octofhir_ucum::validate(value).is_ok())
}

pub(crate) fn valid_dotted_version(value: Option<&str>) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| {
			let mut parts = value.split('.');
			let valid_part = |part: &str| {
				!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
			};
			parts.next().is_some_and(valid_part)
				&& parts.next().is_some_and(valid_part)
				&& parts.next().is_none()
		})
}

pub(crate) fn valid_meddra_version(
	_vocabulary: &VocabularyContext,
	_value: Option<&str>,
) -> bool {
	true
}

pub(crate) fn valid_meddra_term(
	vocabulary: &VocabularyContext,
	version: Option<&str>,
	code: Option<&str>,
) -> bool {
	if !vocabulary.meddra_available() {
		return true;
	}
	let version = version.map(str::trim).filter(|value| !value.is_empty());
	let code = code.map(str::trim).filter(|value| !value.is_empty());
	match (version, code) {
		(Some(version), Some(_)) if !vocabulary.contains_meddra_version(version) => {
			true
		}
		(Some(version), Some(code)) => {
			vocabulary.contains_meddra_term(version, code)
		}
		_ => true,
	}
}

pub(crate) fn valid_mfds_product(
	vocabulary: &VocabularyContext,
	receiver: Option<&str>,
	version: Option<&str>,
	value: Option<&str>,
) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return true;
	};
	match receiver.map(str::trim) {
		Some(receiver) if receiver.eq_ignore_ascii_case("KR") => vocabulary
			.contains_snapshot_code(
				"MFDS_PRODUCT",
				crate::VocabularyScope::ItemSeq,
				value,
			),
		Some(receiver) if receiver.eq_ignore_ascii_case("FR") => version
			.map(str::trim)
			.filter(|version| !version.is_empty())
			.is_some_and(|version| {
				vocabulary.contains_whodrug_product(version, value)
			}),
		_ => true,
	}
}

pub(crate) fn valid_mfds_substance(
	vocabulary: &VocabularyContext,
	receiver: Option<&str>,
	version: Option<&str>,
	value: Option<&str>,
) -> bool {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return true;
	};
	match receiver.map(str::trim) {
		Some(receiver) if receiver.eq_ignore_ascii_case("KR") => vocabulary
			.contains_snapshot_code(
				"MFDS_SUBSTANCE",
				crate::VocabularyScope::All,
				value,
			),
		Some(receiver) if receiver.eq_ignore_ascii_case("FR") => version
			.map(str::trim)
			.filter(|version| !version.is_empty())
			.is_some_and(|version| vocabulary.contains_whodrug_cas(version, value)),
		_ => true,
	}
}

pub(crate) fn valid_iso639(
	vocabulary: &VocabularyContext,
	value: Option<&str>,
) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| {
			vocabulary.contains_snapshot_code(
				"ISO639-2",
				crate::VocabularyScope::All,
				value,
			)
		})
}

pub(crate) fn valid_iso3166(
	vocabulary: &VocabularyContext,
	value: Option<&str>,
) -> bool {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.is_none_or(|value| {
			vocabulary.contains_snapshot_code(
				"ISO3166",
				crate::VocabularyScope::All,
				value,
			)
		})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unavailable_meddra_release_is_not_reported_as_invalid_data() {
		let vocabulary = VocabularyContext::for_meddra(&[("28.1", "10000001")]);

		assert!(valid_meddra_version(&vocabulary, Some("12.0")));
		assert!(valid_meddra_term(
			&vocabulary,
			Some("12.0"),
			Some("10047319")
		));
		assert!(!valid_meddra_term(
			&vocabulary,
			Some("28.1"),
			Some("99999999")
		));
	}

	#[test]
	fn mfds_foreign_product_matches_the_entered_whodrug_version() {
		let vocabulary = VocabularyContext::for_whodrug(&[("2025.09", "MPID1")]);
		assert!(valid_mfds_product(
			&vocabulary,
			Some("FR"),
			Some("2025.09"),
			Some("MPID1")
		));
		assert!(!valid_mfds_product(
			&vocabulary,
			Some("FR"),
			Some("2025.08"),
			Some("MPID1")
		));
	}

	#[test]
	fn mfds_domestic_substance_matches_the_active_dictionary() {
		let vocabulary = VocabularyContext::for_active_codes(&[(
			"MFDS_SUBSTANCE",
			crate::VocabularyScope::All,
			"SUB1",
		)]);
		assert!(valid_mfds_substance(
			&vocabulary,
			Some("KR"),
			None,
			Some("SUB1")
		));
		assert!(!valid_mfds_substance(
			&vocabulary,
			Some("KR"),
			None,
			Some("missing")
		));
	}

	#[test]
	fn mfds_foreign_substance_requires_the_active_whodrug_cas() {
		let vocabulary =
			VocabularyContext::for_whodrug_cas(&[("2026.03", "0000050000")]);
		assert!(valid_mfds_substance(
			&vocabulary,
			Some("FR"),
			Some("2026.03"),
			Some("0000050000")
		));
		assert!(!valid_mfds_substance(
			&VocabularyContext::default(),
			Some("FR"),
			Some("2026.03"),
			Some("0000050000")
		));
	}

	#[test]
	fn e2b_ts_date_ignores_time_offset_and_fraction() {
		assert_eq!(
			e2b_ts_date(Some("200509211242-08")),
			Date::from_calendar_date(2005, time::Month::September, 21).ok()
		);
		assert_eq!(
			e2b_ts_date(Some("20240101120000.1234+0900")),
			Date::from_calendar_date(2024, time::Month::January, 1).ok()
		);
	}
}
