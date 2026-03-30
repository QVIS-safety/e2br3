use std::collections::BTreeSet;

use serial_test::serial;

const SUPPORTED_NULL_FLAVOR_FIELDS: &[(&str, &str)] = &[
	("C.1.transmission_date_null_flavor", "transmission_date_null_flavor"),
	(
		"C.1.date_first_received_from_source_null_flavor",
		"date_first_received_from_source_null_flavor",
	),
	(
		"C.1.date_of_most_recent_information_null_flavor",
		"date_of_most_recent_information_null_flavor",
	),
	("D.1.2.patient_initials_null_flavor", "patient_initials_null_flavor"),
	("D.1.2.birth_date_null_flavor", "birth_date_null_flavor"),
	(
		"D.1.2.age_at_time_of_onset_null_flavor",
		"age_at_time_of_onset_null_flavor",
	),
	("D.1.2.sex_null_flavor", "sex_null_flavor"),
	(
		"D.1.2.last_menstrual_period_date_null_flavor",
		"last_menstrual_period_date_null_flavor",
	),
	("D.7.start_date_null_flavor", "start_date_null_flavor"),
	("D.7.end_date_null_flavor", "end_date_null_flavor"),
	("D.8.r.drug_name_null_flavor", "drug_name_null_flavor"),
	("D.8.r.start_date_null_flavor", "start_date_null_flavor"),
	("D.8.r.end_date_null_flavor", "end_date_null_flavor"),
	("D.9.date_of_death_null_flavor", "date_of_death_null_flavor"),
	(
		"D.10.parent_birth_date_null_flavor",
		"parent_birth_date_null_flavor",
	),
	("D.10.parent_age_null_flavor", "parent_age_null_flavor"),
	(
		"D.10.last_menstrual_period_date_null_flavor",
		"last_menstrual_period_date_null_flavor",
	),
	("D.10.6.r.start_date_null_flavor", "start_date_null_flavor"),
	("D.10.6.r.end_date_null_flavor", "end_date_null_flavor"),
	("D.10.7.r.drug_name_null_flavor", "drug_name_null_flavor"),
	("D.10.7.r.start_date_null_flavor", "start_date_null_flavor"),
	("D.10.7.r.end_date_null_flavor", "end_date_null_flavor"),
	("E.i.criteria_death_null_flavor", "criteria_death_null_flavor"),
	(
		"E.i.criteria_life_threatening_null_flavor",
		"criteria_life_threatening_null_flavor",
	),
	(
		"E.i.criteria_hospitalization_null_flavor",
		"criteria_hospitalization_null_flavor",
	),
	(
		"E.i.criteria_disabling_null_flavor",
		"criteria_disabling_null_flavor",
	),
	(
		"E.i.criteria_congenital_anomaly_null_flavor",
		"criteria_congenital_anomaly_null_flavor",
	),
	(
		"E.i.criteria_other_medically_important_null_flavor",
		"criteria_other_medically_important_null_flavor",
	),
	("E.i.start_date_null_flavor", "start_date_null_flavor"),
	("E.i.end_date_null_flavor", "end_date_null_flavor"),
	("F.r.test_date_null_flavor", "test_date_null_flavor"),
	(
		"G.k.4.r.first_administration_date_null_flavor",
		"first_administration_date_null_flavor",
	),
	(
		"G.k.4.r.last_administration_date_null_flavor",
		"last_administration_date_null_flavor",
	),
];

const SAVE_FIELDS_WEB_SOURCES: &[&str] = &[
	include_str!("c.rs"),
	include_str!("d.rs"),
	include_str!("e.rs"),
	include_str!("f.rs"),
	include_str!("g.rs"),
];

#[test]
#[serial]
fn supported_null_flavor_checklist_has_no_duplicates() {
	let mut seen = BTreeSet::new();
	for (canonical_id, _) in SUPPORTED_NULL_FLAVOR_FIELDS {
		assert!(
			seen.insert(*canonical_id),
			"duplicate supported nullFlavor checklist entry: {canonical_id}"
		);
	}
}

#[test]
#[serial]
fn supported_null_flavor_fields_are_covered_by_web_tests() {
	for (canonical_id, field_name) in SUPPORTED_NULL_FLAVOR_FIELDS {
		assert!(
			SAVE_FIELDS_WEB_SOURCES
				.iter()
				.any(|source| source.contains(&format!("\"{canonical_id}\""))),
			"save_fields_web is missing canonical coverage for {canonical_id}"
		);

		assert!(
			SAVE_FIELDS_WEB_SOURCES
				.iter()
				.any(|source| source.contains(field_name)),
			"save_fields_web is missing field coverage for {field_name} ({canonical_id})"
		);
	}
}
