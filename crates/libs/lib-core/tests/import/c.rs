use crate::common::{date, fixture};
use lib_core::xml::import_sections::c_safety_report::parse_c_safety_report;

#[test]
fn import_c_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");

	assert_eq!(report.transmission_date, date(2014, 6, 14));
	assert_eq!(report.report_type, "1");
	assert_eq!(report.date_first_received_from_source, date(2022, 6, 14));
	assert_eq!(report.date_of_most_recent_information, date(2022, 6, 14));
	assert!(report.fulfil_expedited_criteria);
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
