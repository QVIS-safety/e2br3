use crate::common::fixture;
use lib_core::xml::import_sections::h_narrative::{
	parse_h_case_summaries, parse_h_narrative, parse_h_sender_diagnoses,
};

#[test]
fn import_h_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let narrative = parse_h_narrative(&xml)
		.expect("parse")
		.expect("section H should exist");
	let diagnoses = parse_h_sender_diagnoses(&xml).expect("parse sender diagnoses");
	let summaries = parse_h_case_summaries(&xml).expect("parse case summaries");

	assert_eq!(
		narrative.case_narrative,
		"Case Narrative Including Clinical Course, Therapeutic Measures,\n\t\t\t\t\t"
	);
	assert_eq!(
		narrative.reporter_comments.as_deref(),
		Some(
			"It appears very likely that the primary drug is responsible for all the reactions."
		)
	);
	assert_eq!(
		narrative.sender_comments.as_deref(),
		Some(
			"The condition came on suddenly, and was a complete surprise to the responsible clinicians."
		)
	);

	assert_eq!(diagnoses.len(), 2);
	assert_eq!(diagnoses[0].sequence_number, 1);
	assert_eq!(
		diagnoses[0].diagnosis_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		diagnoses[0].diagnosis_meddra_code.as_deref(),
		Some("10047319")
	);
	assert_eq!(diagnoses[1].sequence_number, 2);
	assert_eq!(
		diagnoses[1].diagnosis_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		diagnoses[1].diagnosis_meddra_code.as_deref(),
		Some("10047334")
	);

	assert_eq!(summaries.len(), 1);
	assert_eq!(summaries[0].sequence_number, 1);
	assert_eq!(summaries[0].summary_type, None);
	assert_eq!(summaries[0].language_code.as_deref(), Some("en"));
	assert_eq!(summaries[0].summary_text.as_deref(), Some("H.5.r.1a 1"));
}
