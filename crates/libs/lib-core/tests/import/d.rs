use crate::common::{date, decimal, fixture};
use lib_core::xml::import_sections::d_patient::parse_d_patient;

#[test]
fn import_d_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let patient = parse_d_patient(&xml)
		.expect("parse")
		.expect("section D should exist");

	assert_eq!(patient.patient_initials.as_deref(), Some("SM"));
	assert_eq!(patient.patient_given_name, None);
	assert_eq!(patient.patient_family_name, None);
	assert_eq!(patient.patient_initials_null_flavor, None);
	assert_eq!(patient.birth_date, Some(date(2014, 10, 1)));
	assert_eq!(patient.birth_date_null_flavor, None);
	assert_eq!(patient.sex.as_deref(), Some("1"));
	assert_eq!(patient.sex_null_flavor, None);
	assert_eq!(patient.age_at_time_of_onset, Some(decimal("33")));
	assert_eq!(patient.age_at_time_of_onset_null_flavor, None);
	assert_eq!(patient.age_unit.as_deref(), Some("a"));
	assert_eq!(patient.gestation_period, Some(decimal("10")));
	assert_eq!(patient.gestation_period_unit.as_deref(), Some("wk"));
	assert_eq!(patient.age_group, None);
	assert_eq!(patient.weight_kg, Some(decimal("50")));
	assert_eq!(patient.height_cm, Some(decimal("160")));
	assert_eq!(patient.race_code.as_deref(), Some("C16352"));
	assert_eq!(patient.ethnicity_code.as_deref(), Some("C17459"));
	assert_eq!(patient.last_menstrual_period_date, Some(date(2009, 1, 1)));
	assert_eq!(patient.last_menstrual_period_date_null_flavor, None);
	assert_eq!(
		patient.medical_history_text.as_deref(),
		Some("Systems Review.")
	);
	assert_eq!(patient.concomitant_therapy, None);
}
