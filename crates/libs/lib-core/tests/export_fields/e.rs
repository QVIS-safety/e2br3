use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};

#[tokio::test]
async fn export_e_rebuilds_reactions_in_sequence_order_and_exports_fields(
) -> Result<()> {
	std::env::set_var("XML_V2_PATCH_E", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let second_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 2,
			primary_source_reaction: "Fever".to_string(),
		},
	)
	.await?;
	ReactionBmc::update(
		&ctx,
		&mm,
		second_id,
		ReactionForUpdate {
			primary_source_reaction: None,
			primary_source_reaction_translation: Some("Pyrexia".to_string()),
			reaction_language: Some("en".to_string()),
			reaction_meddra_code: Some("10016256".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			term_highlighted: Some(false),
			serious: Some(false),
			criteria_death: Some(false),
			criteria_death_null_flavor: Some("MSK".to_string()),
			criteria_life_threatening: Some(false),
			criteria_life_threatening_null_flavor: Some("NI".to_string()),
			criteria_hospitalization: Some(false),
			criteria_hospitalization_null_flavor: Some("UNK".to_string()),
			criteria_disabling: Some(false),
			criteria_disabling_null_flavor: Some("ASKU".to_string()),
			criteria_congenital_anomaly: Some(false),
			criteria_congenital_anomaly_null_flavor: Some("NI".to_string()),
			criteria_other_medically_important: Some(false),
			criteria_other_medically_important_null_flavor: Some("MSK".to_string()),
			required_intervention: Some("true".to_string()),
			start_date: None,
			start_date_null_flavor: Some("UNK".to_string()),
			end_date: None,
			end_date_null_flavor: Some("MSK".to_string()),
			duration_value: None,
			duration_unit: None,
			outcome: Some("1".to_string()),
			medical_confirmation: Some(false),
			country_code: Some("US".to_string()),
		},
	)
	.await?;

	let first_id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	ReactionBmc::update(
		&ctx,
		&mm,
		first_id,
		ReactionForUpdate {
			primary_source_reaction: None,
			primary_source_reaction_translation: Some("Head pain".to_string()),
			reaction_language: Some("en".to_string()),
			reaction_meddra_code: Some("10019211".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			term_highlighted: Some(true),
			serious: Some(true),
			criteria_death: Some(true),
			criteria_death_null_flavor: None,
			criteria_life_threatening: Some(false),
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: Some(false),
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: Some(false),
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: Some(false),
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: Some(true),
			criteria_other_medically_important_null_flavor: None,
			required_intervention: Some("false".to_string()),
			start_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 2)
					.unwrap(),
			),
			start_date_null_flavor: None,
			end_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 4)
					.unwrap(),
			),
			end_date_null_flavor: None,
			duration_value: Some(rust_decimal::Decimal::new(2, 0)),
			duration_unit: Some("d".to_string()),
			outcome: Some("2".to_string()),
			medical_confirmation: Some(true),
			country_code: Some("KR".to_string()),
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, false, true, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath.findvalue("count(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[1]/hl7:value/hl7:originalText", None).unwrap(),
		"Headache"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[2]/hl7:value/hl7:originalText", None).unwrap(),
		"Fever"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[1]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='30']]/hl7:value", None).unwrap(),
		"Head pain"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[1]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='7']]/hl7:value/@value", None).unwrap(),
		"false"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[1]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='27']]/hl7:value/@code", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[1]/hl7:location/hl7:locatedEntity/hl7:locatedPlace/hl7:code/@code", None).unwrap(),
		"KR"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[2]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='34']]/hl7:value/@nullFlavor", None).unwrap(),
		"MSK"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[2]/hl7:effectiveTime/hl7:low/@nullFlavor", None).unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='29']])[2]/hl7:effectiveTime/hl7:high/@nullFlavor", None).unwrap(),
		"MSK"
	);
	Ok(())
}
