use crate::common::{date, fixture};
use lib_core::xml::import_sections::c_safety_report::parse_c_safety_report;
use lib_core::xml::import_sections::c_safety_report::CSafetyReportImport;
use lib_core::xml::{
	apply_c_safety_report_import_settings, apply_default_values_to_imported_r2_case,
	CImportSettings,
};

#[test]
fn import_c_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");

	assert_eq!(report.transmission_date, "20140614000000");
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

#[test]
fn import_settings_update_only_enabled_c1_dates_to_import_date() {
	let xml = fixture("FAERS2022Scenario6.xml");
	let mut report = parse_c_safety_report(&xml)
		.expect("parse")
		.expect("section C should exist");
	let import_date = date(2026, 5, 17);

	apply_c_safety_report_import_settings(
		&mut report,
		&CImportSettings {
			update_date_of_creation: true,
			update_most_recent_info_date: false,
			update_report_first_received_date: true,
			..CImportSettings::default()
		},
		import_date,
	);

	assert_eq!(report.transmission_date, "20260517000000");
	assert_eq!(report.date_first_received_from_source, import_date);
	assert_eq!(report.date_of_most_recent_information, import_date);
}

#[test]
fn import_settings_apply_r2_defaults_only_when_enabled() {
	let mut report = CSafetyReportImport {
		transmission_date: "20260517000000".to_string(),
		report_type: "1".to_string(),
		date_first_received_from_source: date(2026, 5, 17),
		date_of_most_recent_information: date(2026, 5, 17),
		fulfil_expedited_criteria: false,
		additional_documents_available: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		nullification_code: None,
		nullification_reason: None,
	};

	apply_default_values_to_imported_r2_case(&mut report, false);
	assert_eq!(report.additional_documents_available, None);
	assert_eq!(report.first_sender_type, None);

	apply_default_values_to_imported_r2_case(&mut report, true);
	assert_eq!(report.additional_documents_available, Some(false));
	assert_eq!(report.first_sender_type.as_deref(), Some("1"));
}
