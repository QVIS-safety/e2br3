use lib_core::model::reaction::Reaction;
use serial_test::serial;

use crate::common::{date, import_fixture, list_by_uuid};

#[serial]
#[tokio::test]
async fn imports_e_persisted_models() {
	let imported = import_fixture("FAERS2022Scenario1.xml").await;
	let reactions: Vec<Reaction> = list_by_uuid(
		&imported,
		"SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;

	assert_eq!(reactions.len(), 2);

	assert_eq!(reactions[0].case_id, imported.case_id);
	assert_eq!(reactions[0].sequence_number, 1);
	assert_eq!(
		reactions[0].primary_source_reaction,
		"THROMBOSE VEINEUSE PROFONDE"
	);
	assert_eq!(
		reactions[0].primary_source_reaction_translation.as_deref(),
		Some("THROMBOSE VEINEUSE PROFONDE")
	);
	assert_eq!(reactions[0].reaction_language, None);
	assert_eq!(
		reactions[0].reaction_meddra_version.as_deref(),
		Some("25.0")
	);
	assert_eq!(
		reactions[0].reaction_meddra_code.as_deref(),
		Some("10027940")
	);
	assert_eq!(reactions[0].term_highlighted, Some(true));
	assert_eq!(reactions[0].serious, Some(true));
	assert!(!reactions[0].criteria_death);
	assert_eq!(
		reactions[0].criteria_death_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[0].criteria_life_threatening);
	assert_eq!(
		reactions[0]
			.criteria_life_threatening_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(!reactions[0].criteria_hospitalization);
	assert_eq!(
		reactions[0].criteria_hospitalization_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(reactions[0].criteria_disabling);
	assert_eq!(reactions[0].criteria_disabling_null_flavor, None);
	assert!(!reactions[0].criteria_congenital_anomaly);
	assert_eq!(
		reactions[0]
			.criteria_congenital_anomaly_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(reactions[0].criteria_other_medically_important);
	assert_eq!(
		reactions[0].criteria_other_medically_important_null_flavor,
		None
	);
	assert_eq!(reactions[0].required_intervention, None);
	assert_eq!(reactions[0].start_date, Some(date(2014, 10, 10)));
	assert_eq!(reactions[0].start_date_null_flavor, None);
	assert_eq!(reactions[0].end_date, None);
	assert_eq!(reactions[0].end_date_null_flavor, None);
	assert_eq!(reactions[0].duration_value, None);
	assert_eq!(reactions[0].duration_unit, None);
	assert_eq!(reactions[0].outcome.as_deref(), Some("3"));
	assert_eq!(reactions[0].medical_confirmation, None);
	assert_eq!(reactions[0].country_code.as_deref(), Some("US"));

	assert_eq!(reactions[1].case_id, imported.case_id);
	assert_eq!(reactions[1].sequence_number, 2);
	assert_eq!(reactions[1].primary_source_reaction, "COLITE ISCHEMIQUE");
	assert_eq!(
		reactions[1].primary_source_reaction_translation.as_deref(),
		Some("COLITE ISCHEMIQUE")
	);
	assert_eq!(reactions[1].reaction_language, None);
	assert_eq!(
		reactions[1].reaction_meddra_version.as_deref(),
		Some("25.0")
	);
	assert_eq!(
		reactions[1].reaction_meddra_code.as_deref(),
		Some("10009896")
	);
	assert_eq!(reactions[1].term_highlighted, Some(true));
	assert_eq!(reactions[1].serious, Some(true));
	assert!(!reactions[1].criteria_death);
	assert_eq!(
		reactions[1].criteria_death_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_life_threatening);
	assert_eq!(
		reactions[1]
			.criteria_life_threatening_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_hospitalization);
	assert_eq!(
		reactions[1].criteria_hospitalization_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_disabling);
	assert_eq!(
		reactions[1].criteria_disabling_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_congenital_anomaly);
	assert_eq!(
		reactions[1]
			.criteria_congenital_anomaly_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(reactions[1].criteria_other_medically_important);
	assert_eq!(
		reactions[1].criteria_other_medically_important_null_flavor,
		None
	);
	assert_eq!(reactions[1].required_intervention, None);
	assert_eq!(reactions[1].start_date, Some(date(2014, 10, 10)));
	assert_eq!(reactions[1].start_date_null_flavor, None);
	assert_eq!(reactions[1].end_date, None);
	assert_eq!(reactions[1].end_date_null_flavor, None);
	assert_eq!(reactions[1].duration_value, None);
	assert_eq!(reactions[1].duration_unit, None);
	assert_eq!(reactions[1].outcome.as_deref(), Some("1"));
	assert_eq!(reactions[1].medical_confirmation, None);
	assert_eq!(reactions[1].country_code.as_deref(), Some("US"));
}

#[serial]
#[tokio::test]
async fn imports_e_null_flavor_persisted_models() {
	let imported = import_fixture("FAERS2022Scenario6.xml").await;
	let reactions: Vec<Reaction> = list_by_uuid(
		&imported,
		"SELECT * FROM reactions WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;

	assert_eq!(reactions.len(), 2);

	assert_eq!(reactions[0].case_id, imported.case_id);
	assert_eq!(reactions[0].sequence_number, 1);
	assert_eq!(reactions[0].primary_source_reaction, "consciência alterada");
	assert_eq!(
		reactions[0].primary_source_reaction_translation.as_deref(),
		Some("Altered Consciousness")
	);
	assert_eq!(reactions[0].reaction_language, None);
	assert_eq!(
		reactions[0].reaction_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		reactions[0].reaction_meddra_code.as_deref(),
		Some("10027940")
	);
	assert_eq!(reactions[0].term_highlighted, Some(false));
	assert_eq!(reactions[0].serious, Some(true));
	assert!(reactions[0].criteria_death);
	assert_eq!(reactions[0].criteria_death_null_flavor, None);
	assert!(!reactions[0].criteria_life_threatening);
	assert_eq!(
		reactions[0]
			.criteria_life_threatening_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(reactions[0].criteria_hospitalization);
	assert_eq!(reactions[0].criteria_hospitalization_null_flavor, None);
	assert!(!reactions[0].criteria_disabling);
	assert_eq!(
		reactions[0].criteria_disabling_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[0].criteria_congenital_anomaly);
	assert_eq!(
		reactions[0]
			.criteria_congenital_anomaly_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(reactions[0].criteria_other_medically_important);
	assert_eq!(
		reactions[0].criteria_other_medically_important_null_flavor,
		None
	);
	assert_eq!(reactions[0].required_intervention, None);
	assert_eq!(reactions[0].start_date, Some(date(2009, 1, 1)));
	assert_eq!(reactions[0].start_date_null_flavor, None);
	assert_eq!(reactions[0].end_date, Some(date(2009, 1, 2)));
	assert_eq!(reactions[0].end_date_null_flavor, None);
	assert_eq!(reactions[0].duration_value, None);
	assert_eq!(reactions[0].duration_unit, None);
	assert_eq!(reactions[0].outcome.as_deref(), Some("3"));
	assert_eq!(reactions[0].medical_confirmation, Some(true));
	assert_eq!(reactions[0].country_code.as_deref(), Some("EU"));

	assert_eq!(reactions[1].case_id, imported.case_id);
	assert_eq!(reactions[1].sequence_number, 2);
	assert_eq!(reactions[1].primary_source_reaction, "");
	assert_eq!(reactions[1].primary_source_reaction_translation, None);
	assert_eq!(reactions[1].reaction_language, None);
	assert_eq!(
		reactions[1].reaction_meddra_version.as_deref(),
		Some("12.0")
	);
	assert_eq!(
		reactions[1].reaction_meddra_code.as_deref(),
		Some("10024381")
	);
	assert_eq!(reactions[1].term_highlighted, None);
	assert_eq!(reactions[1].serious, Some(true));
	assert!(!reactions[1].criteria_death);
	assert_eq!(
		reactions[1].criteria_death_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_life_threatening);
	assert_eq!(
		reactions[1]
			.criteria_life_threatening_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(!reactions[1].criteria_hospitalization);
	assert_eq!(
		reactions[1].criteria_hospitalization_null_flavor.as_deref(),
		Some("NI")
	);
	assert!(reactions[1].criteria_disabling);
	assert_eq!(reactions[1].criteria_disabling_null_flavor, None);
	assert!(!reactions[1].criteria_congenital_anomaly);
	assert_eq!(
		reactions[1]
			.criteria_congenital_anomaly_null_flavor
			.as_deref(),
		Some("NI")
	);
	assert!(reactions[1].criteria_other_medically_important);
	assert_eq!(
		reactions[1].criteria_other_medically_important_null_flavor,
		None
	);
	assert_eq!(reactions[1].required_intervention, None);
	assert_eq!(reactions[1].start_date, None);
	assert_eq!(reactions[1].start_date_null_flavor.as_deref(), Some("NASK"));
	assert_eq!(reactions[1].end_date, None);
	assert_eq!(reactions[1].end_date_null_flavor, None);
	assert_eq!(reactions[1].duration_value, None);
	assert_eq!(reactions[1].duration_unit, None);
	assert_eq!(reactions[1].outcome.as_deref(), Some("3"));
	assert_eq!(reactions[1].medical_confirmation, None);
	assert_eq!(reactions[1].country_code, None);
}
