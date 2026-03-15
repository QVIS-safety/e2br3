use rust_decimal::Decimal;
use sqlx::types::time::Date;
use time::Month;

pub fn fixture(name: &str) -> Vec<u8> {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	std::fs::read(root.join("docs/refs/instances").join(name)).expect("read fixture")
}

pub fn date(year: i32, month: u8, day: u8) -> Date {
	Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
		.expect("valid date")
}

pub fn decimal(value: &str) -> Decimal {
	value.parse::<Decimal>().expect("valid decimal")
}
