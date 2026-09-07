use time::{Duration, OffsetDateTime};

pub use time::format_description::well_known::Rfc3339;

pub fn now_utc() -> OffsetDateTime {
	OffsetDateTime::now_utc()
}

pub fn format_time(time: OffsetDateTime) -> String {
	time.format(&Rfc3339).unwrap() // TODO: need to check if safe.
}

/// Formats the supplied date/time components as an E2B timestamp, without an offset.
pub fn format_e2b_timestamp(value: OffsetDateTime) -> String {
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		value.year(),
		value.month() as u8,
		value.day(),
		value.hour(),
		value.minute(),
		value.second()
	)
}

pub fn now_utc_plus_sec_str(sec: f64) -> String {
	let new_time = now_utc() + Duration::seconds_f64(sec);
	format_time(new_time)
}

pub fn parse_utc(moment: &str) -> Result<OffsetDateTime> {
	OffsetDateTime::parse(moment, &Rfc3339)
		.map_err(|_| Error::FailToDateParse(moment.to_string()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn e2b_timestamp_preserves_components_and_zero_padding() {
		for month in 1..=12 {
			let value = OffsetDateTime::parse(
				&format!("2024-{month:02}-01T02:03:04+09:00"),
				&Rfc3339,
			)
			.unwrap();
			assert_eq!(
				format_e2b_timestamp(value),
				format!("2024{month:02}01020304")
			);
		}
		let leap_day =
			OffsetDateTime::parse("2024-02-29T23:59:59Z", &Rfc3339).unwrap();
		assert_eq!(format_e2b_timestamp(leap_day), "20240229235959");
	}
}

// region:    --- Error

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	FailToDateParse(String),
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
	fn fmt(
		&self,
		fmt: &mut core::fmt::Formatter,
	) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}
// endregion: --- Error Boilerplate

// endregion: --- Error
