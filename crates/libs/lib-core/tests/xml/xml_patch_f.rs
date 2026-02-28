use lib_core::model::test_result::TestResult;
use lib_core::xml::raw::patch::patch_f_test_results;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;

#[test]
fn patch_f_test_updates_raw_xml() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let test = TestResult {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		test_date: None,
		test_name: "ALT".to_string(),
		test_meddra_version: None,
		test_meddra_code: None,
		test_result_code: None,
		test_result_value: Some("25".to_string()),
		test_result_unit: Some("U/L".to_string()),
		result_unstructured: None,
		normal_low_value: None,
		normal_high_value: None,
		comments: None,
		more_info_available: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_f_test_results(&xml, &[test]).expect("patch");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let name = xpath
		.findvalue("//hl7:observation/hl7:code/hl7:originalText", None)
		.unwrap();
	assert!(name.contains("ALT"));
}
