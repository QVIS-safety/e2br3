use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugDeviceCharacteristicBmc,
	DrugDeviceCharacteristicForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	DrugReactionAssessmentForUpdate, RelatednessAssessmentBmc,
	RelatednessAssessmentForCreate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn export_g_rebuilds_drugs_in_sequence_order_and_exports_related_data(
) -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let reaction_id = ReactionBmc::create(
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
		reaction_id,
		ReactionForUpdate {
			primary_source_reaction: None,
			primary_source_reaction_translation: None,
			reaction_language: None,
			reaction_meddra_code: Some("10019211".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			term_highlighted: None,
			serious: None,
			criteria_death: None,
			criteria_death_null_flavor: None,
			criteria_life_threatening: None,
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: None,
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: None,
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: None,
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: None,
			criteria_other_medically_important_null_flavor: None,
			required_intervention: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			duration_value: None,
			duration_unit: None,
			outcome: Some("1".to_string()),
			medical_confirmation: None,
			country_code: None,
		},
	)
	.await?;

	let second_drug_id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 2,
			drug_characterization: "2".to_string(),
			medicinal_product: "Beta".to_string(),
			drug_generic_name: None,
		},
	)
	.await?;
	DrugInformationBmc::update(
		&ctx,
		&mm,
		second_drug_id,
		DrugInformationForUpdate {
			medicinal_product: Some(String::new()),
			drug_characterization: None,
			brand_name: None,
			drug_generic_name: Some("Generic Beta".to_string()),
			drug_authorization_number: None,
			manufacturer_name: None,
			manufacturer_country: None,
			batch_lot_number: None,
			cumulative_dose_first_reaction_value: None,
			cumulative_dose_first_reaction_unit: None,
			gestation_period_exposure_value: None,
			gestation_period_exposure_unit: None,
			dosage_text: None,
			action_taken: None,
			rechallenge: None,
			investigational_product_blinded: None,
			mpid: Some("MPID-B".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: None,
			phpid_version: None,
			obtain_drug_country: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			parent_dosage_text: None,
			fda_additional_info_coded: None,
			drug_additional_info_codes_json: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
		},
	)
	.await?;

	let first_drug_id = DrugInformationBmc::create(
		&ctx,
		&mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Alpha".to_string(),
			drug_generic_name: None,
		},
	)
	.await?;
	DrugInformationBmc::update(
		&ctx,
		&mm,
		first_drug_id,
		DrugInformationForUpdate {
			medicinal_product: None,
			drug_characterization: None,
			brand_name: Some("Brand A".to_string()),
			drug_generic_name: Some("Generic A".to_string()),
			drug_authorization_number: Some("AUTH-1".to_string()),
			manufacturer_name: Some("Maker".to_string()),
			manufacturer_country: Some("US".to_string()),
			batch_lot_number: Some("LOT1".to_string()),
			cumulative_dose_first_reaction_value: Some(rust_decimal::Decimal::new(
				150, 0,
			)),
			cumulative_dose_first_reaction_unit: Some("mg".to_string()),
			gestation_period_exposure_value: Some(rust_decimal::Decimal::new(10, 0)),
			gestation_period_exposure_unit: Some("wk".to_string()),
			dosage_text: Some("Take once daily".to_string()),
			action_taken: Some("5".to_string()),
			rechallenge: Some("1".to_string()),
			investigational_product_blinded: Some(false),
			mpid: Some("MPID123".to_string()),
			mpid_version: Some("1".to_string()),
			phpid: Some("PHPID123".to_string()),
			phpid_version: Some("1".to_string()),
			obtain_drug_country: Some("US".to_string()),
			parent_route: Some("oral".to_string()),
			parent_route_termid: Some("001".to_string()),
			parent_route_termid_version: Some("1".to_string()),
			parent_dosage_text: Some("Parent dose".to_string()),
			fda_additional_info_coded: Some("1".to_string()),
			drug_additional_info_codes_json: None,
			fda_specialized_product_category: None,
			fda_device_info_json: None,
		},
	)
	.await?;

	DrugActiveSubstanceBmc::create(
		&ctx,
		&mm,
		DrugActiveSubstanceForCreate {
			drug_id: first_drug_id,
			sequence_number: 1,
			substance_name: Some("Substance".to_string()),
			substance_termid: Some("S1".to_string()),
			substance_termid_version: Some("1".to_string()),
			strength_value: Some(rust_decimal::Decimal::new(1, 0)),
			strength_unit: Some("mg".to_string()),
		},
	)
	.await?;
	DosageInformationBmc::create(
		&ctx,
		&mm,
		DosageInformationForCreate {
			drug_id: first_drug_id,
			sequence_number: 1,
			dose_value: Some(rust_decimal::Decimal::new(1, 0)),
			dose_unit: Some("mg".to_string()),
			number_of_units: Some(1),
			frequency_value: Some(rust_decimal::Decimal::new(1, 0)),
			frequency_unit: Some("d".to_string()),
			first_administration_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 1)
					.unwrap(),
			),
			first_administration_time: Some(
				sqlx::types::time::Time::from_hms(8, 0, 0).unwrap(),
			),
			last_administration_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 2)
					.unwrap(),
			),
			last_administration_time: Some(
				sqlx::types::time::Time::from_hms(8, 0, 0).unwrap(),
			),
			duration_value: Some(rust_decimal::Decimal::new(1, 0)),
			duration_unit: Some("d".to_string()),
			batch_lot_number: Some("LOT1".to_string()),
			dosage_text: Some("Dose text".to_string()),
			dose_form: Some("Tablet".to_string()),
			dose_form_termid: Some("DF1".to_string()),
			dose_form_termid_version: Some("1".to_string()),
			route_of_administration: Some("PO".to_string()),
			route_termid_version: Some("1".to_string()),
			parent_route: Some("oral".to_string()),
			parent_route_termid: Some("001".to_string()),
			parent_route_termid_version: Some("1".to_string()),
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;
	DrugIndicationBmc::create(
		&ctx,
		&mm,
		DrugIndicationForCreate {
			drug_id: first_drug_id,
			sequence_number: 1,
			indication_text: Some("Indication".to_string()),
			indication_meddra_version: Some("27.0".to_string()),
			indication_meddra_code: Some("10012345".to_string()),
		},
	)
	.await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		DrugDeviceCharacteristicForCreate {
			drug_id: first_drug_id,
			sequence_number: 1,
			code: Some("C1".to_string()),
			code_system: Some("CS1".to_string()),
			code_display_name: Some("Device".to_string()),
			value_type: Some("ST".to_string()),
			value_value: Some("Val".to_string()),
			value_code: None,
			value_code_system: None,
			value_display_name: None,
		},
	)
	.await?;
	DrugDeviceCharacteristicBmc::create(
		&ctx,
		&mm,
		DrugDeviceCharacteristicForCreate {
			drug_id: first_drug_id,
			sequence_number: 2,
			code: Some("C2".to_string()),
			code_system: Some("CS2".to_string()),
			code_display_name: Some("Mode".to_string()),
			value_type: Some("CE".to_string()),
			value_value: None,
			value_code: Some("VC1".to_string()),
			value_code_system: Some("VCS1".to_string()),
			value_display_name: Some("Coded Value".to_string()),
		},
	)
	.await?;
	let assessment_id = DrugReactionAssessmentBmc::create(
		&ctx,
		&mm,
		DrugReactionAssessmentForCreate {
			drug_id: first_drug_id,
			reaction_id,
		},
	)
	.await?;
	DrugReactionAssessmentBmc::update(
		&ctx,
		&mm,
		assessment_id,
		DrugReactionAssessmentForUpdate {
			administration_start_interval_value: Some(rust_decimal::Decimal::new(
				2, 0,
			)),
			administration_start_interval_unit: Some("d".to_string()),
			last_dose_interval_value: Some(rust_decimal::Decimal::new(1, 0)),
			last_dose_interval_unit: Some("h".to_string()),
			recurrence_action: Some("3".to_string()),
			recurrence_meddra_version: Some("27.0".to_string()),
			recurrence_meddra_code: Some("10019211".to_string()),
			reaction_recurred: Some("1".to_string()),
		},
	)
	.await?;
	RelatednessAssessmentBmc::create(
		&ctx,
		&mm,
		RelatednessAssessmentForCreate {
			drug_reaction_assessment_id: assessment_id,
			sequence_number: 1,
			source_of_assessment: Some("Reporter".to_string()),
			method_of_assessment: Some("WHO".to_string()),
			result_of_assessment: Some("related".to_string()),
			result_of_assessment_kr2: None,
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, false, false, false, true, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath.findvalue("count(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])", None).unwrap(),
		"2"
	);
	// G.k.1 / G.k.2.2
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]/hl7:component/hl7:substanceAdministration/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[1]", None).unwrap(),
		"Alpha"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[2]//hl7:kindOfProduct/hl7:name[1]", None).unwrap(),
		"Generic Beta"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]/hl7:component/hl7:substanceAdministration/hl7:consumable/hl7:instanceOfKind/hl7:kindOfProduct/hl7:name[2]", None).unwrap(),
		"Brand A"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[2]//hl7:ingredientSubstance/hl7:name", None).unwrap(),
		"Generic Beta"
	);
	assert_eq!(
		xpath.findvalue("//hl7:adverseEventAssessment/hl7:component/hl7:causalityAssessment[hl7:code[@code='20']]/hl7:value/@code", None).unwrap(),
		"1"
	);
	// G.k.2.4 / G.k.2.5
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension", None).unwrap(),
		"MPID123"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:code/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]/hl7:id/@extension", None).unwrap(),
		"PHPID123"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asIdentifiedEntity[hl7:code[@code='PHPID']]/hl7:code/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:subjectOf/hl7:observation[hl7:code[@code='G.k.2.5']]/hl7:value/@value", None).unwrap(),
		"false"
	);
	// G.k.3.1 / G.k.3.2 / G.k.3.3 / G.k.3.4
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:productEvent/hl7:performer/hl7:assignedEntity/hl7:representedOrganization/hl7:addr/hl7:country", None).unwrap(),
		"US"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.4']/@extension", None).unwrap(),
		"AUTH-1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:holder/hl7:role/hl7:playingOrganization/hl7:name", None).unwrap(),
		"Maker"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:asManufacturedProduct/hl7:subjectOf/hl7:approval/hl7:author/hl7:territorialAuthority/hl7:territory/hl7:code/@code", None).unwrap(),
		"US"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:ingredientSubstance/hl7:name", None).unwrap(),
		"Substance"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:part/hl7:partProduct/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText", None).unwrap(),
		"LOT1"
	);
	// G.k.2.3.r
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:ingredientSubstance/hl7:code/@code", None).unwrap(),
		"S1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:ingredientSubstance/hl7:code/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:ingredient/hl7:quantity/hl7:numerator/@value", None).unwrap(),
		"1.00000"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:ingredient/hl7:quantity/hl7:numerator/@unit", None).unwrap(),
		"mg"
	);
	// G.k.4.r
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration[hl7:doseQuantity]/hl7:text", None).unwrap(),
		"Dose text"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:doseQuantity/@value", None).unwrap(),
		"1.00000"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:doseQuantity/@unit", None).unwrap(),
		"mg"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[1]/hl7:comp/hl7:period/@value", None).unwrap(),
		"1.00"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[1]/hl7:comp/hl7:period/@unit", None).unwrap(),
		"d"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[2]/hl7:comp/hl7:low/@value", None).unwrap(),
		"20240101080000"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[2]/hl7:comp/hl7:high/@value", None).unwrap(),
		"20240102080000"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[2]/hl7:comp/hl7:width/@value", None).unwrap(),
		"1.00"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime[2]/hl7:comp/hl7:width/@unit", None).unwrap(),
		"d"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:routeCode/@code", None).unwrap(),
		"PO"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:routeCode/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration//hl7:formCode/@code", None).unwrap(),
		"DF1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration//hl7:formCode/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration//hl7:formCode/hl7:originalText", None).unwrap(),
		"Tablet"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration//hl7:lotNumberText", None).unwrap(),
		"LOT1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@code", None).unwrap(),
		"001"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@codeSystemVersion", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/hl7:originalText", None).unwrap(),
		"oral"
	);
	// G.k.5 / G.k.6 / G.k.7
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='14']]/hl7:value/@value", None).unwrap(),
		"150.00000"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='14']]/hl7:value/@unit", None).unwrap(),
		"mg"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='16']]/hl7:value/@value", None).unwrap(),
		"10.00"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='16']]/hl7:value/@unit", None).unwrap(),
		"wk"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:inboundRelationship/hl7:act/hl7:code/@code", None).unwrap(),
		"5"
	);
	// G.k.8
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:inboundRelationship/hl7:observation[hl7:code[@code='19']]/hl7:value/@code", None).unwrap(),
		"10012345"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:inboundRelationship/hl7:observation[hl7:code[@code='19']]/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:inboundRelationship/hl7:observation[hl7:code[@code='19']]/hl7:value/hl7:originalText", None).unwrap(),
		"Indication"
	);
	// G.k.10 / FDA device/additional info
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic/hl7:value/text()", None).unwrap(),
		"Val"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic/hl7:code/@code", None).unwrap(),
		"C1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic/hl7:code/@codeSystem", None).unwrap(),
		"CS1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic/hl7:code/@displayName", None).unwrap(),
		"Device"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic/hl7:value/@xsi:type", None).unwrap(),
		"ST"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:code/@code", None).unwrap(),
		"C2"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:code/@codeSystem", None).unwrap(),
		"CS2"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:code/@displayName", None).unwrap(),
		"Mode"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:value/@xsi:type", None).unwrap(),
		"CE"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:value/@code", None).unwrap(),
		"VC1"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:value/@codeSystem", None).unwrap(),
		"VCS1"
	);
	assert_eq!(
		xpath.findvalue("((//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:characteristic)[2]/hl7:value/@displayName", None).unwrap(),
		"Coded Value"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='9']]/hl7:value/@code", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.4.r.11']]/hl7:value/@code", None).unwrap(),
		"001"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='2']]/hl7:value", None).unwrap(),
		"Parent dose"
	);
	// G.k.8 / G.k.9 recurrences
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship1[@typeCode='SAS']/hl7:pauseQuantity/@value", None).unwrap(),
		"2.00"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship1[@typeCode='SAE']/hl7:pauseQuantity/@unit", None).unwrap(),
		"h"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]/hl7:value/@code", None).unwrap(),
		"1"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.1']]/hl7:value/@code", None).unwrap(),
		"3"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value/@code", None).unwrap(),
		"10019211"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='4']])[1]//hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='31']]/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='G.k.8.r.2']]/hl7:value/@codeSystemVersion", None).unwrap(),
		"27.0"
	);
	// G.k.9 relatedness
	assert_eq!(
		xpath.findvalue("//hl7:adverseEventAssessment/hl7:component/hl7:causalityAssessment[hl7:code[@code='39']]/hl7:value", None).unwrap(),
		"related"
	);
	assert_eq!(
		xpath.findvalue("//hl7:adverseEventAssessment/hl7:component/hl7:causalityAssessment[hl7:code[@code='39']]/hl7:methodCode/hl7:originalText", None).unwrap(),
		"WHO"
	);
	assert_eq!(
		xpath.findvalue("//hl7:adverseEventAssessment/hl7:component/hl7:causalityAssessment[hl7:code[@code='39']]/hl7:author/hl7:assignedEntity/hl7:code/hl7:originalText", None).unwrap(),
		"Reporter"
	);
	Ok(())
}
