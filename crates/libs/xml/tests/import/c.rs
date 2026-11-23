use crate::common::{date, fixture};
use xml::import_sections::c_safety_report::parse_c_safety_report;
use xml::{apply_c_safety_report_import_settings, CImportSettings};

fn conformant_scenario6() -> Vec<u8> {
	String::from_utf8(fixture("FAERS2022Scenario6.xml"))
		.expect("utf-8 fixture")
		.replacen(
			"<effectiveTime value=\"20140614151617-0500\"/>",
			"<effectiveTime value=\"20220614151617-0500\"/>",
			1,
		)
		.into_bytes()
}

#[test]
fn import_c_section_all_fields_from_scenario6() {
	let xml = conformant_scenario6();

	let report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");

	assert_eq!(
		report.transmission_date.as_deref(),
		Some("20220614151617-0500")
	);
	assert_eq!(report.report_type.as_deref(), Some("1"));
	assert_eq!(
		report.date_first_received_from_source.as_deref(),
		Some("20220614101010-0500")
	);
	assert_eq!(
		report.date_of_most_recent_information.as_deref(),
		Some("20220614101010-0500")
	);
	assert_eq!(report.fulfil_expedited_criteria, Some(true));
	assert_eq!(report.additional_documents_available, Some(true));
	assert_eq!(report.local_criteria_report_type.as_deref(), Some("1"));
	assert_eq!(
		report.combination_product_report_indicator.as_deref(),
		Some("true")
	);
	assert_eq!(
		report.worldwide_unique_id.as_deref(),
		Some("US-APHARMA-8744554B")
	);
	assert_eq!(report.first_sender_type.as_deref(), Some("1"));
	assert_eq!(report.nullification_code, None);
	assert_eq!(report.nullification_reason, None);
}

#[test]
fn import_settings_update_only_enabled_c1_dates_to_import_date() {
	let xml = conformant_scenario6();
	let mut report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");
	let import_date = date(2022, 6, 14);

	apply_c_safety_report_import_settings(
		&mut report,
		&CImportSettings {
			update_date_of_creation: true,
			update_most_recent_info_date: false,
			update_report_first_received_date: true,
			..CImportSettings::default()
		},
		import_date,
	)
	.expect("import date settings should keep required dates valid");

	assert_eq!(report.transmission_date.as_deref(), Some("20220614000000"));
	assert_eq!(
		report.date_first_received_from_source.as_deref(),
		Some("20220614")
	);
	assert_eq!(
		report.date_of_most_recent_information.as_deref(),
		Some("20220614101010-0500")
	);
}

#[test]
fn import_settings_preserve_official_inbound_dates() {
	let xml = fixture("FAERS2022Scenario5-1.xml");
	let mut report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");

	apply_c_safety_report_import_settings(
		&mut report,
		&CImportSettings::default(),
		date(2026, 8, 7),
	)
	.expect("inbound source dates must not block import");

	assert_eq!(
		report.transmission_date.as_deref(),
		Some("20140714151617-0500")
	);
	assert_eq!(
		report.date_first_received_from_source.as_deref(),
		Some("20220614101010-0500")
	);
	assert_eq!(
		report.date_of_most_recent_information.as_deref(),
		Some("20220614101010-0500")
	);
}
