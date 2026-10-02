use crate::common::fixture;
use xml::import_sections::f_test_result::parse_f_test_results;

#[test]
fn import_f_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let tests = parse_f_test_results(&xml).expect("parse");
	assert_eq!(tests.len(), 7);

	let first = &tests[0];
	assert_eq!(first.test_date.as_deref(), Some("20090101"));
	assert_eq!(first.test_date_null_flavor, None);
	assert_eq!(first.test_name, "Calcium Level");
	assert_eq!(first.test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(first.test_meddra_code.as_deref(), Some("10050520"));
	assert_eq!(first.test_result_code.as_deref(), Some("2"));
	assert_eq!(first.test_result_value.as_deref(), Some("10"));
	assert_eq!(first.test_result_unit.as_deref(), Some("mg/dl"));
	assert_eq!(first.result_unstructured, None);
	assert_eq!(first.normal_low_value.as_deref(), Some("40"));
	assert_eq!(first.normal_high_value.as_deref(), Some("110"));
	assert_eq!(
		first.comments.as_deref(),
		Some("These results may be skewed")
	);
	assert_eq!(first.more_info_available, Some(true));

	let sixth = &tests[5];
	assert_eq!(sixth.test_date.as_deref(), Some("20090101"));
	assert_eq!(sixth.test_date_null_flavor, None);
	assert_eq!(sixth.test_name, "");
	assert_eq!(sixth.test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(sixth.test_meddra_code.as_deref(), Some("10005362"));
	assert_eq!(sixth.test_result_code, None);
	assert_eq!(sixth.test_result_value, None);
	assert_eq!(sixth.test_result_unit, None);
	assert_eq!(
		sixth.result_unstructured.as_deref(),
		Some(" 10 mg per imaginary unit all the way up to positive infinity\n\t\t\t\t\t\t\t\t\t\t\t\t\t")
	);
	assert_eq!(sixth.normal_low_value, None);
	assert_eq!(sixth.normal_high_value, None);
	assert_eq!(sixth.comments, None);
	assert_eq!(sixth.more_info_available, None);
}

#[test]
fn import_f_section_parses_test_date_null_flavor() {
	let xml = scenario6_with_first_test_date_null_flavor();

	let tests = parse_f_test_results(xml.as_bytes()).expect("parse");
	assert_eq!(tests.len(), 7);

	let first = &tests[0];
	assert_eq!(first.test_date, None);
	assert_eq!(first.test_date_null_flavor.as_deref(), Some("UNK"));
	assert_eq!(first.test_name, "Calcium Level");
	assert_eq!(first.test_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(first.test_meddra_code.as_deref(), Some("10050520"));
	assert_eq!(first.test_result_code.as_deref(), Some("2"));
	assert_eq!(first.test_result_value.as_deref(), Some("10"));
	assert_eq!(first.test_result_unit.as_deref(), Some("mg/dl"));
	assert_eq!(first.result_unstructured, None);
	assert_eq!(first.normal_low_value.as_deref(), Some("40"));
	assert_eq!(first.normal_high_value.as_deref(), Some("110"));
	assert_eq!(
		first.comments.as_deref(),
		Some("These results may be skewed")
	);
	assert_eq!(first.more_info_available, Some(true));
}

#[test]
fn import_f_section_rejects_test_date_value_and_null_flavor_together() {
	let xml = scenario6_with_first_test_date_value_and_null_flavor();

	let err = parse_f_test_results(xml.as_bytes()).unwrap_err();
	assert!(err
		.to_string()
		.contains("value and nullFlavor cannot both be present"));
}

#[test]
fn import_f_section_preserves_test_date_precision_and_time() {
	for value in ["2009", "200901", "20090101", "20090101123045-0800"] {
		let xml = scenario6_with_first_test_date(&format!("value=\"{value}\""));
		let tests = parse_f_test_results(xml.as_bytes()).expect("parse");
		assert_eq!(tests[0].test_date.as_deref(), Some(value));
	}
}

#[test]
fn import_f_section_rejects_invalid_test_date() {
	let xml = scenario6_with_first_test_date("value=\"20230229\"");
	assert!(parse_f_test_results(xml.as_bytes()).is_err());
}

fn scenario6_with_first_test_date(attributes: &str) -> String {
	let xml = String::from_utf8(fixture("FAERS2022Scenario6.xml")).expect("utf-8");
	let original = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime value=\"20090101\"/>";
	let replacement = format!(
		"<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime {attributes}/>"
	);
	assert_eq!(
		xml.matches(original).count(),
		1,
		"fixture must identify one F.r.1 date"
	);
	xml.replacen(original, &replacement, 1)
}

fn scenario6_with_first_test_date_null_flavor() -> String {
	scenario6_with_first_test_date("nullFlavor=\"UNK\"")
}

fn scenario6_with_first_test_date_value_and_null_flavor() -> String {
	scenario6_with_first_test_date("value=\"20090101\" nullFlavor=\"UNK\"")
}
