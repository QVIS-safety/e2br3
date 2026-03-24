use lib_core::model::parent_history::{ParentMedicalHistory, ParentPastDrugHistory};
use lib_core::model::patient::{
	AutopsyCauseOfDeath, MedicalHistoryEpisode, ParentInformation, PastDrugHistory,
	PatientDeathInformation, PatientIdentifier, PatientInformation,
	ReportedCauseOfDeath,
};
use serial_test::serial;

use crate::common::{
	date, decimal, fetch_one_by_uuid, fetch_optional_by_uuid, import_fixture,
	list_by_uuid,
};

#[serial]
#[tokio::test]
async fn imports_d_persisted_models() {
	let imported_c6 = import_fixture("FAERS2022Scenario6.xml").await;
	let patient: PatientInformation = fetch_one_by_uuid(
		&imported_c6,
		"SELECT * FROM patient_information WHERE case_id = $1 LIMIT 1",
		imported_c6.case_id,
	)
	.await;
	let identifiers: Vec<PatientIdentifier> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM patient_identifiers WHERE patient_id = $1 ORDER BY sequence_number",
		patient.id,
	)
	.await;
	let episodes: Vec<MedicalHistoryEpisode> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM medical_history_episodes WHERE patient_id = $1 ORDER BY sequence_number",
		patient.id,
	)
	.await;
	let death: Option<PatientDeathInformation> = fetch_optional_by_uuid(
		&imported_c6,
		"SELECT * FROM patient_death_information WHERE patient_id = $1 LIMIT 1",
		patient.id,
	)
	.await;
	let parents: Vec<ParentInformation> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM parent_information WHERE patient_id = $1 ORDER BY created_at",
		patient.id,
	)
	.await;

	assert_patient_core(&patient);
	assert_eq!(patient.case_id, imported_c6.case_id);

	assert_eq!(identifiers.len(), 4);
	assert_eq!(identifiers[0].patient_id, patient.id);
	assert_eq!(identifiers[0].sequence_number, 1);
	assert_eq!(identifiers[0].identifier_type_code, "1");
	assert_eq!(identifiers[0].identifier_value, "83444908");
	assert_eq!(identifiers[1].patient_id, patient.id);
	assert_eq!(identifiers[1].sequence_number, 2);
	assert_eq!(identifiers[1].identifier_type_code, "2");
	assert_eq!(identifiers[1].identifier_value, "8999-B");
	assert_eq!(identifiers[2].patient_id, patient.id);
	assert_eq!(identifiers[2].sequence_number, 3);
	assert_eq!(identifiers[2].identifier_type_code, "3");
	assert_eq!(identifiers[2].identifier_value, "3333444A32");
	assert_eq!(identifiers[3].patient_id, patient.id);
	assert_eq!(identifiers[3].sequence_number, 4);
	assert_eq!(identifiers[3].identifier_type_code, "4");
	assert_eq!(identifiers[3].identifier_value, "09233347580934");

	assert_eq!(episodes.len(), 6);
	assert_eq!(episodes[0].patient_id, patient.id);
	assert_eq!(episodes[0].sequence_number, 1);
	assert_eq!(episodes[0].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(episodes[0].meddra_code.as_deref(), Some("10000621"));
	assert_eq!(episodes[0].start_date, Some(date(2001, 9, 26)));
	assert_eq!(episodes[0].start_date_null_flavor, None);
	assert_eq!(episodes[0].continuing, Some(false));
	assert_eq!(episodes[0].end_date, Some(date(2001, 10, 31)));
	assert_eq!(episodes[0].end_date_null_flavor, None);
	assert_eq!(
		episodes[0].comments.as_deref(),
		Some("This is what we thought")
	);
	assert_eq!(episodes[0].family_history, None);

	assert_eq!(episodes[4].patient_id, patient.id);
	assert_eq!(episodes[4].sequence_number, 5);
	assert_eq!(episodes[4].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(episodes[4].meddra_code.as_deref(), Some("10061122"));
	assert_eq!(episodes[4].start_date, Some(date(2009, 1, 1)));
	assert_eq!(episodes[4].continuing, Some(true));
	assert_eq!(episodes[4].end_date, Some(date(2009, 1, 1)));
	assert_eq!(episodes[4].comments.as_deref(), Some("Comment"));
	assert_eq!(episodes[4].family_history, Some(true));

	assert_eq!(episodes[5].patient_id, patient.id);
	assert_eq!(episodes[5].sequence_number, 6);
	assert_eq!(episodes[5].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(episodes[5].meddra_code.as_deref(), Some("10012431"));
	assert_eq!(episodes[5].start_date, Some(date(2002, 1, 1)));
	assert_eq!(episodes[5].continuing, Some(true));
	assert_eq!(episodes[5].end_date, Some(date(2006, 1, 1)));
	assert_eq!(
		episodes[5].comments.as_deref(),
		Some("The condition was accentuated by poor diet.")
	);
	assert_eq!(episodes[5].family_history, Some(true));

	let death = death.expect("expected patient_death_information row");
	assert_eq!(death.patient_id, patient.id);
	assert_eq!(death.date_of_death, Some(date(2009, 1, 1)));
	assert_eq!(death.date_of_death_null_flavor, None);
	assert_eq!(death.autopsy_performed, Some(true));
	let reported: Vec<ReportedCauseOfDeath> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM reported_causes_of_death WHERE death_info_id IN (SELECT id FROM patient_death_information WHERE patient_id = $1) ORDER BY sequence_number",
		patient.id,
	)
	.await;
	let autopsy: Vec<AutopsyCauseOfDeath> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM autopsy_causes_of_death WHERE death_info_id IN (SELECT id FROM patient_death_information WHERE patient_id = $1) ORDER BY sequence_number",
		patient.id,
	)
	.await;
	assert_eq!(reported.len(), 2);
	assert_eq!(reported[0].death_info_id, death.id);
	assert_eq!(reported[0].sequence_number, 1);
	assert_eq!(reported[0].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(reported[0].meddra_code.as_deref(), Some("10036807"));
	assert_eq!(
		reported[0].comments.as_deref(),
		Some("Progressive multifocal leukoencephalopathy")
	);
	assert_eq!(reported[1].death_info_id, death.id);
	assert_eq!(reported[1].sequence_number, 2);
	assert_eq!(reported[1].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(reported[1].meddra_code.as_deref(), Some("10036805"));
	assert_eq!(reported[1].comments.as_deref(), Some("Excessive Fibrosis"));

	assert_eq!(autopsy.len(), 2);
	assert_eq!(autopsy[0].death_info_id, death.id);
	assert_eq!(autopsy[0].sequence_number, 1);
	assert_eq!(autopsy[0].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(autopsy[0].meddra_code.as_deref(), Some("10067063"));
	assert_eq!(
		autopsy[0].comments.as_deref(),
		Some("What we learned during the autopsy")
	);
	assert_eq!(autopsy[1].death_info_id, death.id);
	assert_eq!(autopsy[1].sequence_number, 2);
	assert_eq!(autopsy[1].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(autopsy[1].meddra_code.as_deref(), Some("10061227"));
	assert_eq!(
		autopsy[1].comments.as_deref(),
		Some("Metabolic Abnormality")
	);

	assert_eq!(parents.len(), 1);
	assert_eq!(parents[0].patient_id, patient.id);
	assert_eq!(
		parents[0].parent_identification.as_deref(),
		Some("Mr. John Doe Jr.")
	);
	assert_eq!(parents[0].parent_birth_date, Some(date(2014, 10, 1)));
	assert_eq!(parents[0].parent_birth_date_null_flavor, None);
	assert_eq!(parents[0].parent_age, Some(decimal("33")));
	assert_eq!(parents[0].parent_age_null_flavor, None);
	assert_eq!(parents[0].parent_age_unit.as_deref(), Some("a"));
	assert_eq!(
		parents[0].last_menstrual_period_date,
		Some(date(2009, 1, 1))
	);
	assert_eq!(parents[0].last_menstrual_period_date_null_flavor, None);
	assert_eq!(parents[0].weight_kg, Some(decimal("50")));
	assert_eq!(parents[0].height_cm, Some(decimal("160")));
	assert_eq!(parents[0].sex.as_deref(), Some("2"));
	assert_eq!(
		parents[0].medical_history_text.as_deref(),
		Some("Systems Review.")
	);
	let parent_medical: Vec<ParentMedicalHistory> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM parent_medical_history WHERE parent_id IN (SELECT id FROM parent_information WHERE patient_id = $1) ORDER BY sequence_number",
		patient.id,
	)
	.await;
	let parent_past: Vec<ParentPastDrugHistory> = list_by_uuid(
		&imported_c6,
		"SELECT * FROM parent_past_drug_history WHERE parent_id IN (SELECT id FROM parent_information WHERE patient_id = $1) ORDER BY sequence_number",
		patient.id,
	)
	.await;
	assert_eq!(parent_medical.len(), 4);
	assert_eq!(parent_medical[0].parent_id, parents[0].id);
	assert_eq!(parent_medical[0].sequence_number, 1);
	assert_eq!(parent_medical[0].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(parent_medical[0].meddra_code.as_deref(), Some("10000621"));
	assert_eq!(parent_medical[0].start_date, Some(date(2001, 9, 26)));
	assert_eq!(parent_medical[0].start_date_null_flavor, None);
	assert_eq!(parent_medical[0].continuing, Some(false));
	assert_eq!(parent_medical[0].end_date, Some(date(2001, 10, 31)));
	assert_eq!(parent_medical[0].end_date_null_flavor, None);
	assert_eq!(
		parent_medical[0].comments.as_deref(),
		Some("This is what we thought")
	);

	assert_eq!(parent_medical[1].parent_id, parents[0].id);
	assert_eq!(parent_medical[1].sequence_number, 2);
	assert_eq!(parent_medical[1].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(parent_medical[1].meddra_code.as_deref(), Some("10010538"));
	assert_eq!(parent_medical[1].start_date, Some(date(2009, 1, 1)));
	assert_eq!(parent_medical[1].start_date_null_flavor, None);
	assert_eq!(parent_medical[1].continuing, Some(true));
	assert_eq!(parent_medical[1].end_date, Some(date(2009, 1, 1)));
	assert_eq!(parent_medical[1].end_date_null_flavor, None);
	assert_eq!(
		parent_medical[1].comments.as_deref(),
		Some("Further relevant comments")
	);

	assert_eq!(parent_medical[2].parent_id, parents[0].id);
	assert_eq!(parent_medical[2].sequence_number, 3);
	assert_eq!(parent_medical[2].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(parent_medical[2].meddra_code.as_deref(), Some("10060747"));
	assert_eq!(parent_medical[2].start_date, None);
	assert_eq!(parent_medical[2].start_date_null_flavor, None);
	assert_eq!(parent_medical[2].continuing, None);
	assert_eq!(parent_medical[2].end_date, None);
	assert_eq!(parent_medical[2].end_date_null_flavor, None);
	assert_eq!(parent_medical[2].comments, None);

	assert_eq!(parent_medical[3].parent_id, parents[0].id);
	assert_eq!(parent_medical[3].sequence_number, 4);
	assert_eq!(parent_medical[3].meddra_version.as_deref(), Some("12.0"));
	assert_eq!(parent_medical[3].meddra_code.as_deref(), Some("10023637"));
	assert_eq!(parent_medical[3].start_date, None);
	assert_eq!(parent_medical[3].start_date_null_flavor, None);
	assert_eq!(parent_medical[3].continuing, None);
	assert_eq!(parent_medical[3].end_date, None);
	assert_eq!(parent_medical[3].end_date_null_flavor, None);
	assert_eq!(parent_medical[3].comments, None);

	assert_eq!(parent_past.len(), 4);
	assert_eq!(parent_past[0].parent_id, parents[0].id);
	assert_eq!(parent_past[0].sequence_number, 1);
	assert_eq!(
		parent_past[0].drug_name.as_deref(),
		Some("Molds, Rusts and Smuts, Penicillium notatum")
	);
	assert_eq!(parent_past[0].drug_name_null_flavor, None);
	assert_eq!(parent_past[0].mpid.as_deref(), Some("65044-5208"));
	assert_eq!(parent_past[0].mpid_version.as_deref(), Some("12.0"));
	assert_eq!(parent_past[0].phpid, None);
	assert_eq!(parent_past[0].phpid_version, None);
	assert_eq!(parent_past[0].start_date, Some(date(2009, 1, 1)));
	assert_eq!(parent_past[0].start_date_null_flavor, None);
	assert_eq!(parent_past[0].end_date, Some(date(2009, 1, 2)));
	assert_eq!(parent_past[0].end_date_null_flavor, None);
	assert_eq!(
		parent_past[0].indication_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[0].indication_meddra_code.as_deref(),
		Some("10054929")
	);
	assert_eq!(
		parent_past[0].reaction_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[0].reaction_meddra_code.as_deref(),
		Some("10060051")
	);

	assert_eq!(parent_past[1].parent_id, parents[0].id);
	assert_eq!(parent_past[1].sequence_number, 2);
	assert_eq!(
		parent_past[1].drug_name.as_deref(),
		Some("Guaifenesin, Dextromethorphan")
	);
	assert_eq!(parent_past[1].drug_name_null_flavor, None);
	assert_eq!(parent_past[1].mpid.as_deref(), Some("14505-488"));
	assert_eq!(parent_past[1].mpid_version.as_deref(), Some("12.0"));
	assert_eq!(parent_past[1].phpid, None);
	assert_eq!(parent_past[1].phpid_version, None);
	assert_eq!(parent_past[1].start_date, None);
	assert_eq!(parent_past[1].start_date_null_flavor, None);
	assert_eq!(parent_past[1].end_date, None);
	assert_eq!(parent_past[1].end_date_null_flavor, None);
	assert_eq!(
		parent_past[1].indication_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[1].indication_meddra_code.as_deref(),
		Some("10061372")
	);
	assert_eq!(
		parent_past[1].reaction_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[1].reaction_meddra_code.as_deref(),
		Some("10036960")
	);

	assert_eq!(parent_past[2].parent_id, parents[0].id);
	assert_eq!(parent_past[2].sequence_number, 3);
	assert_eq!(
		parent_past[2].drug_name.as_deref(),
		Some("Guaifenesin, Dextromethorphan")
	);
	assert_eq!(parent_past[2].drug_name_null_flavor, None);
	assert_eq!(parent_past[2].mpid.as_deref(), Some("14505-488"));
	assert_eq!(parent_past[2].mpid_version.as_deref(), Some("12.0"));
	assert_eq!(parent_past[2].phpid, None);
	assert_eq!(parent_past[2].phpid_version, None);
	assert_eq!(parent_past[2].start_date, Some(date(2009, 1, 1)));
	assert_eq!(parent_past[2].start_date_null_flavor, None);
	assert_eq!(parent_past[2].end_date, Some(date(2009, 1, 1)));
	assert_eq!(parent_past[2].end_date_null_flavor, None);
	assert_eq!(parent_past[2].indication_meddra_version, None);
	assert_eq!(parent_past[2].indication_meddra_code, None);
	assert_eq!(
		parent_past[2].reaction_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[2].reaction_meddra_code.as_deref(),
		Some("10036960")
	);

	assert_eq!(parent_past[3].parent_id, parents[0].id);
	assert_eq!(parent_past[3].sequence_number, 4);
	assert_eq!(
		parent_past[3].drug_name.as_deref(),
		Some("Guaifenesin, Dextromethorphan")
	);
	assert_eq!(parent_past[3].drug_name_null_flavor, None);
	assert_eq!(parent_past[3].mpid, None);
	assert_eq!(parent_past[3].mpid_version, None);
	assert_eq!(parent_past[3].phpid, None);
	assert_eq!(parent_past[3].phpid_version, None);
	assert_eq!(parent_past[3].start_date, None);
	assert_eq!(parent_past[3].start_date_null_flavor, None);
	assert_eq!(parent_past[3].end_date, None);
	assert_eq!(parent_past[3].end_date_null_flavor, None);
	assert_eq!(
		parent_past[3].indication_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		parent_past[3].indication_meddra_code.as_deref(),
		Some("10061372")
	);
	assert_eq!(parent_past[3].reaction_meddra_version, None);
	assert_eq!(parent_past[3].reaction_meddra_code, None);

	let imported_c1 = import_fixture("FAERS2022Scenario1.xml").await;
	let patient_c1: PatientInformation = fetch_one_by_uuid(
		&imported_c1,
		"SELECT * FROM patient_information WHERE case_id = $1 LIMIT 1",
		imported_c1.case_id,
	)
	.await;
	let past_drugs: Vec<PastDrugHistory> = list_by_uuid(
		&imported_c1,
		"SELECT * FROM past_drug_history WHERE patient_id = $1 ORDER BY sequence_number",
		patient_c1.id,
	)
	.await;
	assert_eq!(patient_c1.case_id, imported_c1.case_id);
	assert_eq!(past_drugs.len(), 1);
	assert_eq!(past_drugs[0].patient_id, patient_c1.id);
	assert_eq!(past_drugs[0].sequence_number, 1);
	assert_eq!(past_drugs[0].drug_name.as_deref(), Some("CureAll"));
	assert_eq!(past_drugs[0].drug_name_null_flavor, None);
	assert_eq!(past_drugs[0].mpid.as_deref(), Some("59762-2858"));
	assert_eq!(past_drugs[0].mpid_version.as_deref(), Some("2014110112"));
	assert_eq!(past_drugs[0].phpid, None);
	assert_eq!(past_drugs[0].phpid_version, None);
	assert_eq!(past_drugs[0].start_date, None);
	assert_eq!(past_drugs[0].start_date_null_flavor, None);
	assert_eq!(past_drugs[0].end_date, None);
	assert_eq!(past_drugs[0].end_date_null_flavor, None);
	assert_eq!(past_drugs[0].indication_meddra_version, None);
	assert_eq!(past_drugs[0].indication_meddra_code, None);
	assert_eq!(past_drugs[0].reaction_meddra_version, None);
	assert_eq!(past_drugs[0].reaction_meddra_code, None);
}

fn assert_patient_core(patient: &PatientInformation) {
	assert_eq!(patient.patient_initials.as_deref(), Some("SM"));
	assert_eq!(patient.patient_given_name, None);
	assert_eq!(patient.patient_family_name, None);
	assert_eq!(patient.birth_date, Some(date(2014, 10, 1)));
	assert_eq!(patient.age_at_time_of_onset, Some(decimal("33")));
	assert_eq!(patient.age_unit.as_deref(), Some("a"));
	assert_eq!(patient.gestation_period, Some(decimal("10")));
	assert_eq!(patient.gestation_period_unit.as_deref(), Some("wk"));
	assert_eq!(patient.age_group, None);
	assert_eq!(patient.weight_kg, Some(decimal("50")));
	assert_eq!(patient.height_cm, Some(decimal("160")));
	assert_eq!(patient.sex.as_deref(), Some("1"));
	assert_eq!(patient.patient_initials_null_flavor, None);
	assert_eq!(patient.birth_date_null_flavor, None);
	assert_eq!(patient.age_at_time_of_onset_null_flavor, None);
	assert_eq!(patient.sex_null_flavor, None);
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
