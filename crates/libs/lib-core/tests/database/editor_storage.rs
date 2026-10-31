use super::init_test_mm;
use crate::Result;
use serial_test::serial;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EditorDbPair {
	section: &'static str,
	frontend_path: &'static str,
	table: &'static str,
	value_column: &'static str,
	null_flavor_column: &'static str,
}

macro_rules! pair {
	($section:literal, $path:literal, $table:literal, $value:literal, $null_flavor:literal) => {
		EditorDbPair {
			section: $section,
			frontend_path: $path,
			table: $table,
			value_column: $value,
			null_flavor_column: $null_flavor,
		}
	};
}

const EDITOR_DB_PAIRS: &[EditorDbPair] = &[
	pair!("AE", "reactions[].reactionEndDateNullFlavor", "reactions", "end_date", "end_date_null_flavor"),
	pair!("AE", "reactions[].reactionStartDateNullFlavor", "reactions", "start_date", "start_date_null_flavor"),
	pair!("AE", "reactions[].requiredInterventionNullFlavor", "reactions", "required_intervention", "required_intervention_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaCongenitalAnomalyNullFlavor", "reactions", "criteria_congenital_anomaly", "criteria_congenital_anomaly_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaDisablingNullFlavor", "reactions", "criteria_disabling", "criteria_disabling_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaHospitalizationNullFlavor", "reactions", "criteria_hospitalization", "criteria_hospitalization_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaLifeThreateningNullFlavor", "reactions", "criteria_life_threatening", "criteria_life_threatening_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaOtherMedicallyImportantNullFlavor", "reactions", "criteria_other_medically_important", "criteria_other_medically_important_null_flavor"),
	pair!("AE", "reactions[].seriousness.criteriaResultsInDeathNullFlavor", "reactions", "criteria_death", "criteria_death_null_flavor"),
	pair!("CI", "safetyReportIdentification.combinationProductReportIndicatorNullFlavor", "safety_report_identification", "combination_product_report_indicator", "combination_product_report_indicator_null_flavor"),
	pair!("CI", "safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor", "safety_report_identification", "fulfil_expedited_criteria", "fulfil_expedited_criteria_null_flavor"),
	pair!("CI", "safetyReportIdentification.otherCaseIdentifiersExistNullFlavor", "safety_report_identification", "other_case_identifiers_exist", "other_case_identifiers_exist_null_flavor"),
	pair!("DG", "drugs[].dosageInformation[].doseFormNullFlavor", "dosage_information", "dose_form", "dose_form_null_flavor"),
	pair!("DG", "drugs[].dosageInformation[].firstAdministrationDateNullFlavor", "dosage_information", "first_administration_date", "first_administration_date_null_flavor"),
	pair!("DG", "drugs[].dosageInformation[].lastAdministrationDateNullFlavor", "dosage_information", "last_administration_date", "last_administration_date_null_flavor"),
	pair!("DG", "drugs[].dosageInformation[].parentRouteOfAdministrationNullFlavor", "dosage_information", "parent_route", "parent_route_null_flavor"),
	pair!("DG", "drugs[].dosageInformation[].routeOfAdministrationNullFlavor", "dosage_information", "route_of_administration", "route_of_administration_null_flavor"),
	pair!("DG", "drugs[].indications[].indicationTextNullFlavor", "drug_indications", "indication_text", "indication_text_null_flavor"),
	pair!("DH", "patientInformation.pastDrugHistory[].drugNameNullFlavor", "past_drug_history", "drug_name", "drug_name_null_flavor"),
	pair!("DH", "patientInformation.pastDrugHistory[].endDateNullFlavor", "past_drug_history", "end_date", "end_date_null_flavor"),
	pair!("DH", "patientInformation.pastDrugHistory[].startDateNullFlavor", "past_drug_history", "start_date", "start_date_null_flavor"),
	pair!("DM", "patientInformation.ethnicityCodeNullFlavor", "patient_information", "ethnicity_code", "ethnicity_code_null_flavor"),
	pair!("DM", "patientInformation.gpMedicalRecordNumberNullFlavor", "patient_identifiers", "identifier_value", "identifier_value_null_flavor"),
	pair!("DM", "patientInformation.hospitalRecordNumberNullFlavor", "patient_identifiers", "identifier_value", "identifier_value_null_flavor"),
	pair!("DM", "patientInformation.investigationNumberNullFlavor", "patient_identifiers", "identifier_value", "identifier_value_null_flavor"),
	pair!("DM", "patientInformation.lastMenstrualPeriodDateNullFlavor", "patient_information", "last_menstrual_period_date", "last_menstrual_period_date_null_flavor"),
	pair!("DM", "patientInformation.medicalHistoryEpisodes[].continuingNullFlavor", "medical_history_episodes", "continuing", "continuing_null_flavor"),
	pair!("DM", "patientInformation.medicalHistoryEpisodes[].endDateNullFlavor", "medical_history_episodes", "end_date", "end_date_null_flavor"),
	pair!("DM", "patientInformation.medicalHistoryEpisodes[].startDateNullFlavor", "medical_history_episodes", "start_date", "start_date_null_flavor"),
	pair!("DM", "patientInformation.medicalHistoryTextNullFlavor", "patient_information", "medical_history_text", "medical_history_text_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.medicalHistoryEpisodes[].continuingNullFlavor", "parent_medical_history", "continuing", "continuing_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.medicalHistoryEpisodes[].endDateNullFlavor", "parent_medical_history", "end_date", "end_date_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.medicalHistoryEpisodes[].startDateNullFlavor", "parent_medical_history", "start_date", "start_date_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.parentBirthDateNullFlavor", "parent_information", "parent_birth_date", "parent_birth_date_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.parentIdentificationNullFlavor", "parent_information", "parent_identification", "parent_identification_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.parentLastMenstrualPeriodDateNullFlavor", "parent_information", "last_menstrual_period_date", "last_menstrual_period_date_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.parentSexNullFlavor", "parent_information", "sex", "sex_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.pastDrugHistory[].endDateNullFlavor", "parent_past_drug_history", "end_date", "end_date_null_flavor"),
	pair!("DM", "patientInformation.parentInformation.pastDrugHistory[].startDateNullFlavor", "parent_past_drug_history", "start_date", "start_date_null_flavor"),
	pair!("DM", "patientInformation.patientBirthDateNullFlavor", "patient_information", "birth_date", "birth_date_null_flavor"),
	pair!("DM", "patientInformation.patientDeath.autopsyPerformedNullFlavor", "patient_death_information", "autopsy_performed", "autopsy_performed_null_flavor"),
	pair!("DM", "patientInformation.patientDeath.dateOfDeathNullFlavor", "patient_death_information", "date_of_death", "date_of_death_null_flavor"),
	pair!("DM", "patientInformation.patientInitialsNullFlavor", "patient_information", "patient_initials", "patient_initials_null_flavor"),
	pair!("DM", "patientInformation.patientSexNullFlavor", "patient_information", "sex", "sex_null_flavor"),
	pair!("DM", "patientInformation.specialistRecordNumberNullFlavor", "patient_identifiers", "identifier_value", "identifier_value_null_flavor"),
	pair!("LB", "testResults[].testDateNullFlavor", "test_results", "test_date", "test_date_null_flavor"),
	pair!("LR", "literatureReferences[].referenceTextNullFlavor", "literature_references", "reference_text", "reference_text_null_flavor"),
	pair!("RP", "primarySources[].qualificationNullFlavor", "primary_sources", "qualification", "qualification_null_flavor"),
	pair!("RP", "primarySources[].reporterCityNullFlavor", "primary_sources", "city", "city_null_flavor"),
	pair!("RP", "primarySources[].reporterDepartmentNullFlavor", "primary_sources", "department", "department_null_flavor"),
	pair!("RP", "primarySources[].reporterEmailNullFlavor", "primary_sources", "email", "email_null_flavor"),
	pair!("RP", "primarySources[].reporterFamilyNameNullFlavor", "primary_sources", "reporter_family_name", "reporter_family_name_null_flavor"),
	pair!("RP", "primarySources[].reporterGivenNameNullFlavor", "primary_sources", "reporter_given_name", "reporter_given_name_null_flavor"),
	pair!("RP", "primarySources[].reporterMiddleNameNullFlavor", "primary_sources", "reporter_middle_name", "reporter_middle_name_null_flavor"),
	pair!("RP", "primarySources[].reporterOrganizationNullFlavor", "primary_sources", "organization", "organization_null_flavor"),
	pair!("RP", "primarySources[].reporterPostcodeNullFlavor", "primary_sources", "postcode", "postcode_null_flavor"),
	pair!("RP", "primarySources[].reporterStateNullFlavor", "primary_sources", "state", "state_null_flavor"),
	pair!("RP", "primarySources[].reporterStreetNullFlavor", "primary_sources", "street", "street_null_flavor"),
	pair!("RP", "primarySources[].reporterTelephoneNullFlavor", "primary_sources", "telephone", "telephone_null_flavor"),
	pair!("RP", "primarySources[].reporterTitleNullFlavor", "primary_sources", "reporter_title", "reporter_title_null_flavor"),
	pair!("SI", "studyInformation.fdaCrossReportedIndNumbers[].indNumberNullFlavor", "study_fda_cross_reported_inds", "ind_number", "ind_number_null_flavor"),
	pair!("SI", "studyInformation.sponsorStudyNumberNullFlavor", "study_information", "sponsor_study_number", "sponsor_study_number_null_flavor"),
	pair!("SI", "studyInformation.studyNameNullFlavor", "study_information", "study_name", "study_name_null_flavor"),
	pair!("SI", "studyInformation.studyRegistrationNumbers[].countryCodeNullFlavor", "study_registration_numbers", "country_code", "country_code_null_flavor"),
	pair!("SI", "studyInformation.studyRegistrationNumbers[].registrationNumberNullFlavor", "study_registration_numbers", "registration_number", "registration_number_null_flavor"),
];

#[serial]
#[tokio::test]
async fn database_has_every_editor_pair_and_mutual_exclusion_check() -> Result<()> {
	let mm = init_test_mm().await?;
	for pair in EDITOR_DB_PAIRS {
		let column_count: i64 = sqlx::query_scalar(
			"SELECT count(*) FROM information_schema.columns
			  WHERE table_schema = 'public' AND table_name = $1
			    AND column_name = ANY($2)",
		)
		.bind(pair.table)
		.bind([pair.value_column, pair.null_flavor_column])
		.fetch_one(mm.dbx().db())
		.await?;
		assert_eq!(column_count, 2, "missing columns for {pair:?}");

		let has_pair_check: bool = sqlx::query_scalar(
			"SELECT EXISTS (
			   SELECT 1 FROM pg_constraint c
			    WHERE c.conrelid = format('public.%I', $1)::regclass
			      AND c.contype = 'c'
			      AND regexp_replace(lower(pg_get_expr(c.conbin, c.conrelid)), '[^a-z0-9_]+', '', 'g')
			          = lower($2 || 'isnullor' || $3 || 'isnull')
			 )",
		)
		.bind(pair.table)
		.bind(pair.value_column)
		.bind(pair.null_flavor_column)
		.fetch_one(mm.dbx().db())
		.await?;
		assert!(
			has_pair_check,
			"missing mutual-exclusion CHECK for {pair:?}"
		);
	}
	let has_race_pair_check: bool = sqlx::query_scalar(
		"SELECT EXISTS (
		   SELECT 1 FROM pg_constraint
		    WHERE conrelid = 'public.patient_information'::regclass
		      AND conname = 'ck_patient_race_null_flavor_pair'
		 )",
	)
	.fetch_one(mm.dbx().db())
	.await?;
	assert!(has_race_pair_check, "missing raceCodes/NullFlavor CHECK");
	Ok(())
}
