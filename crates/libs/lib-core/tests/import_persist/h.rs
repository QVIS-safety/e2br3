use lib_core::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
};
use serial_test::serial;

use crate::common::{fetch_one_by_uuid, import_fixture, list_by_uuid};

#[serial]
#[tokio::test]
async fn imports_h_persisted_models() {
	let imported = import_fixture("FAERS2022Scenario6.xml").await;
	let narrative: NarrativeInformation = fetch_one_by_uuid(
		&imported,
		"SELECT * FROM narrative_information WHERE case_id = $1 LIMIT 1",
		imported.case_id,
	)
	.await;
	let sender_diagnoses: Vec<SenderDiagnosis> = list_by_uuid(
		&imported,
		"SELECT * FROM sender_diagnoses WHERE narrative_id = $1 ORDER BY sequence_number",
		narrative.id,
	)
	.await;
	let case_summaries: Vec<CaseSummaryInformation> = list_by_uuid(
		&imported,
		"SELECT * FROM case_summary_information WHERE narrative_id = $1 ORDER BY sequence_number",
		narrative.id,
	)
	.await;

	assert_h_narrative(&imported, &narrative);
	assert_eq!(sender_diagnoses.len(), 2);
	assert_eq!(sender_diagnoses[0].narrative_id, narrative.id);
	assert_eq!(sender_diagnoses[0].sequence_number, 1);
	assert_eq!(
		sender_diagnoses[0].diagnosis_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		sender_diagnoses[0].diagnosis_meddra_code.as_deref(),
		Some("10047319")
	);
	assert_eq!(sender_diagnoses[1].narrative_id, narrative.id);
	assert_eq!(sender_diagnoses[1].sequence_number, 2);
	assert_eq!(
		sender_diagnoses[1].diagnosis_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		sender_diagnoses[1].diagnosis_meddra_code.as_deref(),
		Some("10047334")
	);

	assert_eq!(case_summaries.len(), 1);
	assert_eq!(case_summaries[0].narrative_id, narrative.id);
	assert_eq!(case_summaries[0].sequence_number, 1);
	assert_eq!(case_summaries[0].summary_type, None);
	assert_eq!(case_summaries[0].language_code.as_deref(), Some("en"));
	assert_eq!(
		case_summaries[0].summary_text.as_deref(),
		Some("H.5.r.1a 1")
	);
}

fn assert_h_narrative(
	imported: &crate::common::ImportedCase,
	narrative: &NarrativeInformation,
) {
	assert_eq!(narrative.case_id, imported.case_id);
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
}
