use lib_core::model::drug::{
	DosageInformation, DrugActiveSubstance, DrugDeviceCharacteristic,
	DrugIndication, DrugInformation,
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
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drug_id = Uuid::new_v4();
	let drug = DrugInformation {
		id: drug_id,
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Drug A".to_string(),
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		brand_name: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		dosage_text: None,
		action_taken: None,
		rechallenge: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
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
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let drug_id = Uuid::new_v4();
	let drug = DrugInformation {
		id: drug_id,
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		drug_characterization: "".to_string(),
		medicinal_product: "Drug B".to_string(),
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		brand_name: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		dosage_text: None,
		action_taken: None,
		rechallenge: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
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
	)
	.expect_err("missing drug characterization should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.G.k.1.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
