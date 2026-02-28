use lib_core::model::test_result::TestResult;
use lib_core::xml::export_sections::f_test_result::export_f_test_results_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;
use time::OffsetDateTime;

#[test]
fn export_f_test_basic() {
	let test = TestResult {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		test_date: Some(Date::from_calendar_date(2024, Month::January, 3).unwrap()),
		test_name: "ALT".to_string(),
		test_meddra_version: Some("24.1".to_string()),
		test_meddra_code: Some("10001552".to_string()),
		test_result_code: Some("N".to_string()),
		test_result_value: Some("25".to_string()),
		test_result_unit: Some("U/L".to_string()),
		result_unstructured: None,
		normal_low_value: Some("0".to_string()),
		normal_high_value: Some("40".to_string()),
		comments: Some("All normal".to_string()),
		more_info_available: Some(false),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let xml = export_f_test_results_xml(&[test]).expect("export xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&xml).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let name = xpath
		.findvalue("//hl7:observation/hl7:code/hl7:originalText", None)
		.unwrap();
	assert_eq!(name, "ALT");
}
