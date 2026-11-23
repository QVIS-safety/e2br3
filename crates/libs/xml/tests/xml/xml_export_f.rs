use lib_core::model::test_result::TestResult;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;
use xml::export::roundtrip::patch_f_test_results;

#[test]
fn export_f_test_basic() {
	let test = basic_test_result();

	let xml = patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		&[test],
	)
	.expect("export xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&xml).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let name = xpath
		.findvalue("//hl7:observation/hl7:code/hl7:originalText", None)
		.unwrap();
	assert_eq!(name, "ALT");
	for (path, expected) in [
		("//hl7:referenceRange/hl7:observationRange[hl7:interpretationCode/@code='L']/hl7:value/@value", "0"),
		("//hl7:referenceRange/hl7:observationRange[hl7:interpretationCode/@code='H']/hl7:value/@value", "40"),
	] {
		assert_eq!(xpath.findvalue(path, None).expect("bound"), expected, "{path}");
	}
	let imported =
		xml::import_sections::f_test_result::parse_f_test_results(xml.as_bytes())
			.expect("reimport");
	assert_eq!(imported.len(), 1);
	assert_eq!(imported[0].normal_low_value.as_deref(), Some("0"));
	assert_eq!(imported[0].normal_high_value.as_deref(), Some("40"));
	assert_eq!(imported[0].test_result_value.as_deref(), Some("25"));
	assert_eq!(imported[0].test_result_unit.as_deref(), Some("U/L"));
	assert_eq!(imported[0].more_info_available, Some(false));
}

#[test]
fn export_f_test_date_null_flavor() {
	let mut test = basic_test_result();
	test.test_date = None;
	test.test_date_null_flavor = Some("UNK".to_string());

	let xml = patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		&[test],
	)
	.expect("export xml");
	assert!(xml.contains("<effectiveTime nullFlavor=\"UNK\"/>"), "{xml}");
	assert!(!xml.contains("effectiveTime value="), "{xml}");
	let imported =
		xml::import_sections::f_test_result::parse_f_test_results(xml.as_bytes())
			.expect("reimport");
	assert_eq!(imported.len(), 1);
	assert_eq!(imported[0].test_date, None);
	assert_eq!(imported[0].test_date_null_flavor.as_deref(), Some("UNK"));
}

#[test]
fn export_f_missing_date_does_not_invent_a_null_flavor() {
	let mut test = basic_test_result();
	test.test_date = None;
	let xml = patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		&[test],
	)
	.expect("export");
	let imported =
		xml::import_sections::f_test_result::parse_f_test_results(xml.as_bytes())
			.expect("reimport");
	assert_eq!(imported.len(), 1);
	assert_eq!(imported[0].test_date, None);
	assert_eq!(imported[0].test_date_null_flavor, None);
}

#[test]
fn export_f_rejects_test_date_value_and_null_flavor_together() {
	let mut test = basic_test_result();
	test.test_date_null_flavor = Some("UNK".to_string());

	let err = patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		&[test],
	)
	.unwrap_err();
	assert!(
		err.to_string()
			.contains("value and nullFlavor cannot both be present"),
		"unexpected error: {err}"
	);
}

fn basic_test_result() -> TestResult {
	TestResult {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		deleted: false,
		test_date: Some("20240103".to_string()),
		test_date_null_flavor: None,
		test_name: Some("ALT".to_string()),
		test_meddra_version: Some("24.1".to_string()),
		test_meddra_code: Some("10001552".to_string()),
		test_result_code: Some("2".to_string()),
		test_result_value: Some("25".to_string()),
		test_result_qualifier: None,
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
	}
}

#[test]
fn export_f_keeps_unknown_null_flavor_and_rejects_unrepresentable_qualifier() {
	let mut test = basic_test_result();
	test.test_date = None;
	test.test_date_null_flavor = Some("CUSTOM".into());
	let xml = patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		std::slice::from_ref(&test),
	)
	.expect("raw nullFlavor");
	let doc = Parser::default().parse_string(&xml).expect("XML");
	let mut xpath = Context::new(&doc).expect("XPath");
	xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.expect("namespace");
	assert_eq!(
		xpath
			.findvalue("//hl7:effectiveTime/@nullFlavor", None)
			.expect("date NF"),
		"CUSTOM"
	);
	test.test_result_qualifier = Some("UNKNOWN".into());
	assert!(patch_f_test_results(
		include_bytes!("../../src/fixtures/base_export_skeleton.xml"),
		&[test]
	)
	.is_err());
}
