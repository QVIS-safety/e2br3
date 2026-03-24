use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PastDrugHistoryBmc, PastDrugHistoryForCreate, PatientDeathInformationBmc,
	PatientDeathInformationForCreate, PatientDeathInformationForUpdate,
	PatientIdentifierBmc, PatientIdentifierForCreate, PatientInformationBmc,
	PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn export_d_exports_positive_fields_in_canonical_order() -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("2".to_string()),
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	PatientInformationBmc::update(
		&ctx,
		&mm,
		patient_id,
		PatientInformationForUpdate {
			patient_initials: None,
			patient_given_name: None,
			patient_family_name: None,
			patient_initials_null_flavor: None,
			birth_date: Some(
				time::Date::from_calendar_date(1990, time::Month::January, 2)
					.unwrap(),
			),
			birth_date_null_flavor: None,
			age_at_time_of_onset: Some(rust_decimal::Decimal::new(33, 0)),
			age_at_time_of_onset_null_flavor: None,
			age_unit: Some("a".to_string()),
			gestation_period: Some(rust_decimal::Decimal::new(10, 0)),
			gestation_period_unit: Some("wk".to_string()),
			age_group: Some("4".to_string()),
			weight_kg: Some(rust_decimal::Decimal::new(72, 0)),
			height_cm: Some(rust_decimal::Decimal::new(168, 0)),
			sex: None,
			sex_null_flavor: None,
			race_code: Some("C41260".to_string()),
			ethnicity_code: Some("C41222".to_string()),
			last_menstrual_period_date: Some(
				time::Date::from_calendar_date(2023, time::Month::December, 15)
					.unwrap(),
			),
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: Some("History".to_string()),
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	PatientIdentifierBmc::create(
		&ctx,
		&mm,
		PatientIdentifierForCreate {
			patient_id,
			sequence_number: 1,
			identifier_type_code: "1".to_string(),
			identifier_value: "PID-1".to_string(),
		},
	)
	.await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: Some("1".to_string()),
			medical_history_text: Some("Parent history".to_string()),
		},
	)
	.await?;
	ParentInformationBmc::update(
		&ctx,
		&mm,
		parent_id,
		ParentInformationForUpdate {
			parent_identification: Some("Mother".to_string()),
			parent_birth_date: Some(
				time::Date::from_calendar_date(1980, time::Month::January, 2)
					.unwrap(),
			),
			parent_birth_date_null_flavor: None,
			parent_age: Some(rust_decimal::Decimal::new(44, 0)),
			parent_age_null_flavor: None,
			parent_age_unit: Some("801".to_string()),
			last_menstrual_period_date: Some(
				time::Date::from_calendar_date(2023, time::Month::December, 1)
					.unwrap(),
			),
			last_menstrual_period_date_null_flavor: None,
			weight_kg: Some(rust_decimal::Decimal::new(70, 0)),
			height_cm: Some(rust_decimal::Decimal::new(165, 0)),
			sex: Some("1".to_string()),
			medical_history_text: Some("Parent history".to_string()),
		},
	)
	.await?;
	let episode_id = MedicalHistoryEpisodeBmc::create(
		&ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id,
			sequence_number: 1,
			meddra_code: Some("10016256".to_string()),
			start_date_null_flavor: None,
			end_date_null_flavor: None,
		},
	)
	.await?;
	MedicalHistoryEpisodeBmc::update(
		&ctx,
		&mm,
		episode_id,
		MedicalHistoryEpisodeForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: None,
			start_date: Some(
				time::Date::from_calendar_date(2020, time::Month::January, 1)
					.unwrap(),
			),
			start_date_null_flavor: None,
			continuing: Some(true),
			end_date: Some(
				time::Date::from_calendar_date(2020, time::Month::February, 1)
					.unwrap(),
			),
			end_date_null_flavor: None,
			comments: Some("Recovered".to_string()),
			family_history: Some(true),
		},
	)
	.await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: Some(
				time::Date::from_calendar_date(2024, time::Month::March, 5).unwrap(),
			),
			date_of_death_null_flavor: None,
			autopsy_performed: Some(true),
		},
	)
	.await?;
	let reported_cause_id = ReportedCauseOfDeathBmc::create(
		&ctx,
		&mm,
		ReportedCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: Some("10036807".to_string()),
		},
	)
	.await?;
	ReportedCauseOfDeathBmc::update(
		&ctx,
		&mm,
		reported_cause_id,
		ReportedCauseOfDeathForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: None,
			comments: Some("Reported cause comment".to_string()),
		},
	)
	.await?;
	let autopsy_cause_id = AutopsyCauseOfDeathBmc::create(
		&ctx,
		&mm,
		AutopsyCauseOfDeathForCreate {
			death_info_id: death_id,
			sequence_number: 1,
			meddra_code: Some("10067063".to_string()),
		},
	)
	.await?;
	AutopsyCauseOfDeathBmc::update(
		&ctx,
		&mm,
		autopsy_cause_id,
		AutopsyCauseOfDeathForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: None,
			comments: Some("Autopsy cause comment".to_string()),
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, true, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	// D.1
	assert_eq!(
		xpath
			.findvalue("//hl7:primaryRole/hl7:player1/hl7:name", None)
			.unwrap(),
		"JD"
	);
	assert_eq!(
		xpath
			.findvalue("//hl7:primaryRole/hl7:player1/hl7:birthTime/@value", None)
			.unwrap(),
		"19900102"
	);
	// D.2
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@value", None).unwrap(),
		"33.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@unit", None).unwrap(),
		"a"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value/@value", None).unwrap(),
		"10.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value/@unit", None).unwrap(),
		"wk"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='4']]/hl7:value/@code", None).unwrap(),
		"4"
	);
	// D.3-D.5
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value/@value", None).unwrap(),
		"72.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value/@value", None).unwrap(),
		"168.00"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode/@code",
				None
			)
			.unwrap(),
		"2"
	);
	// FDA.D.11 / FDA.D.12
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049']]/hl7:value/@code", None).unwrap(),
		"C41260"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564']]/hl7:value/@code", None).unwrap(),
		"C41222"
	);
	// D.6
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value/@value", None).unwrap(),
		"20231215"
	);
	// D.7.1 / D.7.2 / D.7.3
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:code/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:effectiveTime/hl7:low/@value", None).unwrap(),
		"20200101"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:effectiveTime/hl7:high/@value", None).unwrap(),
		"20200201"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:inboundRelationship/hl7:observation[hl7:code[@code='13']]/hl7:value/@value", None).unwrap(),
		"true"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='10']]/hl7:value", None).unwrap(),
		"Recovered"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value", None).unwrap(),
		"History"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='11']]/hl7:value/@value", None).unwrap(),
		"true"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256' and @codeSystem='2.16.840.1.113883.6.163']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='38']]/hl7:value/@value", None).unwrap(),
		"true"
	);
	// D.1.1
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='1']]/hl7:code/@code", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:asIdentifiedEntity[hl7:code[@code='1']]/hl7:id/@extension", None).unwrap(),
		"PID-1"
	);
	// D.9
	assert_eq!(
		xpath
			.findvalue("//hl7:primaryRole/hl7:deceasedTime/@value", None)
			.unwrap(),
		"20240305"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='32']])[1]/hl7:observation/hl7:value/@code", None).unwrap(),
		"10036807"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='32']])[1]/hl7:observation/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='32']])[1]/hl7:observation/hl7:value/hl7:originalText", None).unwrap(),
		"Reported cause comment"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='5']])[1]/hl7:observation/hl7:value/@value", None).unwrap(),
		"true"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='5']])[1]/hl7:observation/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='8']]/hl7:value/@code", None).unwrap(),
		"10067063"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='5']])[1]/hl7:observation/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='8']]/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:observation/hl7:code[@code='5']])[1]/hl7:observation/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='8']]/hl7:value/hl7:originalText", None).unwrap(),
		"Autopsy cause comment"
	);
	// D.10
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:name", None).unwrap(),
		"Mother"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime/@value", None).unwrap(),
		"19800102"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@value", None).unwrap(),
		"44.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@unit", None).unwrap(),
		"801"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:administrativeGenderCode/@code", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='7']]/hl7:value/@value", None).unwrap(),
		"70.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='17']]/hl7:value/@value", None).unwrap(),
		"165.00"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value/@value", None).unwrap(),
		"20231201"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value", None).unwrap(),
		"Parent history"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_d_rebuilds_all_past_drug_history_rows_in_order() -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("2".to_string()),
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: Some("Alpha".to_string()),
			drug_name_null_flavor: None,
			mpid: Some("MPID-1".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: None,
			phpid_version: None,
			start_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 1)
					.unwrap(),
			),
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("10022095".to_string()),
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;
	let _row2 = PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 2,
			drug_name: Some("Beta".to_string()),
			drug_name_null_flavor: None,
			mpid: None,
			mpid_version: None,
			phpid: Some("PHPID-2".to_string()),
			phpid_version: Some("2".to_string()),
			start_date: None,
			start_date_null_flavor: None,
			end_date: Some(
				time::Date::from_calendar_date(2024, time::Month::February, 1)
					.unwrap(),
			),
			end_date_null_flavor: None,
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: Some("27.0".to_string()),
			reaction_meddra_code: Some("10034484".to_string()),
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, true, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath.findvalue("count(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:name", None).unwrap(),
		"Alpha"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:name", None).unwrap(),
		"Beta"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@code", None).unwrap(),
		"MPID"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension", None).unwrap(),
		"MPID-1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]/hl7:id/@extension", None).unwrap(),
		"PHPID-2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]/hl7:code/@codeSystemVersion", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:effectiveTime/hl7:low/@value", None).unwrap(),
		"20240101"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:effectiveTime/hl7:high/@value", None).unwrap(),
		"20240201"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='19']]/hl7:observation/hl7:value/@code", None).unwrap(),
		"10022095"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='19']]/hl7:observation/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='29']]/hl7:observation/hl7:value/@code", None).unwrap(),
		"10034484"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[2]//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='29']]/hl7:observation/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_d_exports_patient_parent_and_history_nullflavors() -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: None,
			sex: None,
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	PatientInformationBmc::update(
		&ctx,
		&mm,
		patient_id,
		PatientInformationForUpdate {
			patient_initials: None,
			patient_given_name: None,
			patient_family_name: None,
			patient_initials_null_flavor: Some("UNK".to_string()),
			birth_date: None,
			birth_date_null_flavor: Some("ASKU".to_string()),
			age_at_time_of_onset: None,
			age_at_time_of_onset_null_flavor: Some("NI".to_string()),
			age_unit: None,
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			height_cm: None,
			sex: None,
			sex_null_flavor: Some("UNK".to_string()),
			race_code: None,
			ethnicity_code: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: Some("MSK".to_string()),
			medical_history_text: None,
			concomitant_therapy: Some(false),
		},
	)
	.await?;
	let death_id = PatientDeathInformationBmc::create(
		&ctx,
		&mm,
		PatientDeathInformationForCreate {
			patient_id,
			date_of_death: None,
			date_of_death_null_flavor: Some("UNK".to_string()),
			autopsy_performed: Some(false),
		},
	)
	.await?;
	PatientDeathInformationBmc::update(
		&ctx,
		&mm,
		death_id,
		PatientDeathInformationForUpdate {
			date_of_death: None,
			date_of_death_null_flavor: Some("UNK".to_string()),
			autopsy_performed: None,
		},
	)
	.await?;
	let parent_id = ParentInformationBmc::create(
		&ctx,
		&mm,
		ParentInformationForCreate {
			patient_id,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	ParentInformationBmc::update(
		&ctx,
		&mm,
		parent_id,
		ParentInformationForUpdate {
			parent_identification: Some("Parent".to_string()),
			parent_birth_date: None,
			parent_birth_date_null_flavor: Some("NI".to_string()),
			parent_age: None,
			parent_age_null_flavor: Some("ASKU".to_string()),
			parent_age_unit: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: Some("MSK".to_string()),
			weight_kg: None,
			height_cm: None,
			sex: None,
			medical_history_text: None,
		},
	)
	.await?;
	let episode_id = MedicalHistoryEpisodeBmc::create(
		&ctx,
		&mm,
		MedicalHistoryEpisodeForCreate {
			patient_id,
			sequence_number: 1,
			meddra_code: Some("10016256".to_string()),
			start_date_null_flavor: Some("UNK".to_string()),
			end_date_null_flavor: Some("MSK".to_string()),
		},
	)
	.await?;
	MedicalHistoryEpisodeBmc::update(
		&ctx,
		&mm,
		episode_id,
		MedicalHistoryEpisodeForUpdate {
			meddra_version: Some("27.0".to_string()),
			meddra_code: None,
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			continuing: None,
			end_date: None,
			end_date_null_flavor: Some("MSK".to_string()),
			comments: None,
			family_history: None,
		},
	)
	.await?;
	PastDrugHistoryBmc::create(
		&ctx,
		&mm,
		PastDrugHistoryForCreate {
			patient_id,
			sequence_number: 1,
			drug_name: None,
			drug_name_null_flavor: Some("NI".to_string()),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date: None,
			end_date_null_flavor: Some("MSK".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: None,
			reaction_meddra_version: None,
			reaction_meddra_code: None,
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, true, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath
			.findvalue("//hl7:primaryRole/hl7:player1/hl7:name/@nullFlavor", None)
			.unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:player1/hl7:birthTime/@nullFlavor",
				None
			)
			.unwrap(),
		"ASKU"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@nullFlavor", None).unwrap(),
		"NI"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode/@nullFlavor", None).unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value/@nullFlavor", None).unwrap(),
		"MSK"
	);
	assert_eq!(
		xpath
			.findvalue("//hl7:primaryRole/hl7:deceasedTime/@nullFlavor", None)
			.unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:associatedPerson/hl7:birthTime/@nullFlavor", None).unwrap(),
		"NI"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='3']]/hl7:value/@nullFlavor", None).unwrap(),
		"ASKU"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:player1/hl7:role[hl7:code[@code='PRN']]/hl7:subjectOf2/hl7:observation[hl7:code[@code='22']]/hl7:value/@nullFlavor", None).unwrap(),
		"MSK"
	);
	assert_eq!(
		xpath.findvalue("//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='10016256']]/hl7:effectiveTime/hl7:low/@nullFlavor", None).unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:name/@nullFlavor", None).unwrap(),
		"NI"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2[hl7:organizer/hl7:code[@code='2']])[1]//hl7:effectiveTime/hl7:high/@nullFlavor", None).unwrap(),
		"MSK"
	);
	Ok(())
}
