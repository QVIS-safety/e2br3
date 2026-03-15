use crate::common::{date, fixture};
use lib_core::xml::import_sections::f_test_result::parse_f_test_results;

#[test]
fn import_f_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let tests = parse_f_test_results(&xml).expect("parse");
	assert_eq!(tests.len(), 7);

	let first = &tests[0];
	assert_eq!(first.test_date, Some(date(2009, 1, 1)));
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
	assert_eq!(first.more_info_available, None);

	let sixth = &tests[5];
	assert_eq!(sixth.test_date, Some(date(2009, 1, 1)));
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
	assert_eq!(first.more_info_available, None);
}

fn scenario6_with_first_test_date_null_flavor() -> String {
	let xml = String::from_utf8(fixture("FAERS2022Scenario6.xml")).expect("utf-8");
	let original = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime value=\"20090101\"/>";
	let replacement = "<originalText>Calcium Level</originalText>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<!--  F.r.2.1 Test Name (free text) #1 -->\n\t\t\t\t\t\t\t\t\t\t\t\t\t</code>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<effectiveTime nullFlavor=\"UNK\"/>";
	let updated = xml.replacen(original, replacement, 1);
	assert_ne!(updated, xml, "fixture patch should replace one F.r.1 date");
	updated
}
