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
