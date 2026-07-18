use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::xml::raw::patch::patch_g_drugs;
use lib_core::xml::Error as XmlError;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;

#[test]
fn patch_g_drug_updates_raw_xml() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drug_id = Uuid::new_v4();
	let drug = DrugInformation {
		id: drug_id,
		case_id: Uuid::new_v4(),
		source_product_presave_id: None,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Drug A".to_string(),
		mpid: None,
		mpid_version: None,
		mfds_mpid_version: None,
		mfds_mpid: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		action_taken: None,
		fda_additional_info_coded: None,
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
		fda_other_characterization: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_g_drugs(
		&xml,
		&[drug],
		&[] as &[DrugActiveSubstance],
		&[] as &[DosageInformation],
		&[] as &[DrugIndication],
		&[] as &[DrugDeviceCharacteristic],
		&[] as &[DrugReactionAssessment],
		&[] as &[RelatednessAssessment],
	)
	.expect("patch");

	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let name = xpath
		.findvalue("//hl7:kindOfProduct/hl7:name", None)
		.unwrap();
	assert!(!name.trim().is_empty());
}

#[test]
fn patch_g_drug_normalizes_characterization_for_causality() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drug_id = Uuid::new_v4();
	let drug = DrugInformation {
		id: drug_id,
		case_id: Uuid::new_v4(),
		source_product_presave_id: None,
		sequence_number: 1,
		drug_characterization: "".to_string(),
		medicinal_product: "Drug B".to_string(),
		mpid: None,
		mpid_version: None,
		mfds_mpid_version: None,
		mfds_mpid: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		action_taken: None,
		fda_additional_info_coded: None,
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
		fda_other_characterization: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let err = patch_g_drugs(
		&xml,
		&[drug],
		&[] as &[DrugActiveSubstance],
		&[] as &[DosageInformation],
		&[] as &[DrugIndication],
		&[] as &[DrugDeviceCharacteristic],
		&[] as &[DrugReactionAssessment],
		&[] as &[RelatednessAssessment],
	)
	.expect_err("missing drug characterization should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.G.k.1.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}

#[test]
fn patch_g_drug_emits_relatedness_assessment_values() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drug_id = Uuid::new_v4();
	let reaction_id = Uuid::new_v4();
	let assessment_id = Uuid::new_v4();

	let drug = DrugInformation {
		id: drug_id,
		case_id: Uuid::new_v4(),
		source_product_presave_id: None,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Drug A".to_string(),
		mpid: None,
		mpid_version: None,
		mfds_mpid_version: None,
		mfds_mpid: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
		action_taken: None,
		fda_additional_info_coded: None,
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
		fda_other_characterization: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};
	let assessment = DrugReactionAssessment {
		id: assessment_id,
		drug_id,
		reaction_id,
		administration_start_interval_value: None,
		administration_start_interval_unit: None,
		last_dose_interval_value: None,
		last_dose_interval_unit: None,
		recurrence_action: None,
		reaction_recurred: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};
	let relatedness = RelatednessAssessment {
		id: Uuid::new_v4(),
		drug_reaction_assessment_id: assessment_id,
		sequence_number: 1,
		source_of_assessment: Some("RTDG18".to_string()),
		method_of_assessment: Some("RTDG19".to_string()),
		result_of_assessment: Some("RTDG20".to_string()),
		result_of_assessment_kr2: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_g_drugs(
		&xml,
		&[drug],
		&[] as &[DrugActiveSubstance],
		&[] as &[DosageInformation],
		&[] as &[DrugIndication],
		&[] as &[DrugDeviceCharacteristic],
		&[assessment],
		&[relatedness],
	)
	.expect("patch");

	assert!(patched.contains("RTDG18"));
	assert!(patched.contains("RTDG19"));
	assert!(patched.contains("RTDG20"));
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let gk1_ref = xpath
		.findvalue(
			"//hl7:causalityAssessment[hl7:code[@code='20']]/hl7:subject2/hl7:productUseReference/hl7:id/@root",
			None,
		)
		.expect("query gk1 ref");
	let related_reaction_ref = xpath
		.findvalue(
			"//hl7:causalityAssessment[hl7:code[@code='39']]/hl7:subject1/hl7:adverseEffectReference/hl7:id/@root",
			None,
		)
		.expect("query relatedness reaction ref");
	let related_drug_ref = xpath
		.findvalue(
			"//hl7:causalityAssessment[hl7:code[@code='39']]/hl7:subject2/hl7:productUseReference/hl7:id/@root",
			None,
		)
		.expect("query relatedness drug ref");
	assert_eq!(gk1_ref.trim(), drug_id.to_string());
	assert_eq!(related_reaction_ref.trim(), reaction_id.to_string());
	assert_eq!(related_drug_ref.trim(), drug_id.to_string());
}
