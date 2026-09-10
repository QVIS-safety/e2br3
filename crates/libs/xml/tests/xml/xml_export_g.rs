use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;
use time::OffsetDateTime;
use xml::export::sections::g::export_g_drugs_xml;

#[test]
fn export_g_drug_basic() {
	let case_id = Uuid::new_v4();
	let drug_id = Uuid::new_v4();
	let drug = DrugInformation {
		id: drug_id,
		case_id,
		source_product_presave_id: None,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Drug A".to_string(),
		mpid: Some("MPID123".to_string()),
		mpid_version: Some("1".to_string()),
		mpid_source_code_system: None,
		mpid_source_code_system_version: None,
		mfds_mpid_version: None,
		mfds_mpid: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: Some(false),
		obtain_drug_country: Some("US".to_string()),
		drug_authorization_number: None,
		manufacturer_name: Some("Maker".to_string()),
		manufacturer_country: Some("US".to_string()),
		batch_lot_number: Some("LOT1".to_string()),
		cumulative_dose_first_reaction_value: Some(150.into()),
		cumulative_dose_first_reaction_unit: Some("mg".to_string()),
		gestation_period_exposure_value: Some(10.into()),
		gestation_period_exposure_unit: Some("wk".to_string()),
		action_taken: Some("5".to_string()),
		fda_additional_info_coded: None,
		fda_additional_info_coded_null_flavor: Some("NA".to_string()),
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_other_characterization: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
		deleted: false,
	};

	let substance = DrugActiveSubstance {
		id: Uuid::new_v4(),
		drug_id,
		sequence_number: 1,
		substance_name: Some("Substance".to_string()),
		substance_termid: Some("S1".to_string()),
		substance_termid_version: Some("1".to_string()),
		substance_termid_code_system: Some("TBD-Substance".to_string()),
		mfds_version: None,
		mfds_id: None,
		strength_value: Some(1.into()),
		strength_unit: Some("mg".to_string()),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
		deleted: false,
	};

	let dosage = DosageInformation {
		id: Uuid::new_v4(),
		drug_id,
		sequence_number: 1,
		dose_value: Some(1.into()),
		dose_unit: Some("mg".to_string()),
		number_of_units: Some(Decimal::new(5, 1)),
		frequency_unit: Some("d".to_string()),
		first_administration_date: Some(
			Date::from_calendar_date(2024, Month::January, 1).unwrap(),
		),
		first_administration_date_raw: None,
		last_administration_date: Some(
			Date::from_calendar_date(2024, Month::January, 2).unwrap(),
		),
		last_administration_date_raw: None,
		duration_value: Some(1.into()),
		duration_unit: Some("d".to_string()),
		continuing: None,
		batch_lot_number: Some("LOT1".to_string()),
		batch_lot_number_null_flavor: None,
		dosage_text: Some("Dose text".to_string()),
		dose_form: Some("Tablet".to_string()),
		dose_form_null_flavor: None,
		dose_form_termid: Some("DF1".to_string()),
		dose_form_termid_version: Some("1".to_string()),
		route_of_administration: Some("PO".to_string()),
		route_termid: None,
		route_of_administration_null_flavor: None,
		route_termid_version: Some("1".to_string()),
		route_termid_code_system: Some("0.4.0.127.0.16.1.1.2.6".to_string()),
		parent_route: Some("oral".to_string()),
		parent_route_null_flavor: None,
		parent_route_termid: Some("001".to_string()),
		parent_route_termid_version: Some("1".to_string()),
		parent_route_termid_code_system: Some(
			"2.16.840.1.113883.3.989.2.1.1.14".to_string(),
		),
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
		deleted: false,
	};

	let indication = DrugIndication {
		id: Uuid::new_v4(),
		drug_id,
		sequence_number: 1,
		indication_text: Some("Indication".to_string()),
		indication_text_null_flavor: None,
		indication_meddra_version: Some("24.1".to_string()),
		indication_meddra_code: Some("10012345".to_string()),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
		deleted: false,
	};

	let characteristic = DrugDeviceCharacteristic {
		id: Uuid::new_v4(),
		drug_id,
		sequence_number: 1,
		code: Some("KR_DVC_SN".to_string()),
		code_system: Some("CS1".to_string()),
		code_display_name: Some("Device".to_string()),
		value_type: Some("ST".to_string()),
		value_value: Some("Val".to_string()),
		value_code: None,
		value_code_system: None,
		value_display_name: None,
		deleted: false,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let xml = export_g_drugs_xml(
		&[drug],
		&[substance],
		&[dosage],
		&[indication],
		&[characteristic],
		&[],
		&[],
	)
	.expect("export xml");
	assert!(!xml.contains("KR_DVC_SN"));
	let parser = Parser::default();
	let doc = parser.parse_string(&xml).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let name = xpath
		.findvalue("//hl7:kindOfProduct/hl7:name", None)
		.unwrap();
	assert_eq!(name, "Drug A");
	let mpid = xpath
		.findvalue(
			"//hl7:kindOfProduct/hl7:asIdentifiedEntity[hl7:code[@code='MPID']]/hl7:id/@extension",
			None,
		)
		.unwrap();
	assert_eq!(mpid, "MPID123");
	let authorization_country = xpath
		.findvalue(
			"//hl7:approval/hl7:author/hl7:territorialAuthority/hl7:territory/hl7:code/@code",
			None,
		)
		.unwrap();
	assert_eq!(authorization_country, "US");
	let drug_batch = xpath
		.findvalue(
			"//hl7:consumable/hl7:instanceOfKind/hl7:productInstanceInstance/hl7:lotNumberText",
			None,
		)
		.unwrap();
	assert_eq!(drug_batch, "LOT1");
	let sub_strength_value = xpath
		.findvalue("//hl7:ingredient/hl7:quantity/hl7:numerator/@value", None)
		.unwrap();
	assert_eq!(sub_strength_value, "1");
	let dose_text = xpath
		.findvalue(
			"//hl7:outboundRelationship2/hl7:substanceAdministration[hl7:doseQuantity]/hl7:text",
			None,
		)
		.unwrap();
	assert_eq!(dose_text, "Dose text");
	let first_admin = xpath
		.findvalue(
			"//hl7:outboundRelationship2/hl7:substanceAdministration/hl7:effectiveTime/hl7:comp/hl7:low/@value",
			None,
		)
		.unwrap();
	assert_eq!(first_admin, "20240101");
	let cumulative_value = xpath
		.findvalue(
			"//hl7:substanceAdministration/hl7:outboundRelationship2[@typeCode='SUMM']/hl7:observation[hl7:code[@code='14']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(cumulative_value, "150");
	let gestation_unit = xpath
		.findvalue(
			"//hl7:substanceAdministration/hl7:outboundRelationship2[@typeCode='PERT']/hl7:observation[hl7:code[@code='16']]/hl7:value/@unit",
			None,
		)
		.unwrap();
	assert_eq!(gestation_unit, "wk");
	let fda_additional_info_null_flavor = xpath
		.findvalue(
			"//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='9']]/hl7:value/@nullFlavor",
			None,
		)
		.unwrap();
	assert_eq!(fda_additional_info_null_flavor, "");
}
