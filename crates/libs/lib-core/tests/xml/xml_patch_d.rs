use lib_core::xml::raw::patch::{patch_d_patient, DPatientPatch};
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use time::Month;

#[test]
fn patch_d_section_updates_values() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let patch = DPatientPatch {
		patient_name: Some("Jane Doe"),
		sex: Some("2"),
		birth_date: Some(Date::from_calendar_date(1985, Month::May, 4).unwrap()),
		age_value: Some("38"),
		age_unit: Some("a"),
		weight_kg: Some("72"),
		height_cm: Some("168"),
	};

	let patched = patch_d_patient(&xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let name = xpath
		.findvalue("//hl7:primaryRole/hl7:player1/hl7:name", None)
		.unwrap();
	assert_eq!(name, "Jane Doe");

	let sex = xpath
		.findvalue(
			"//hl7:primaryRole/hl7:player1/hl7:administrativeGenderCode/@code",
			None,
		)
		.unwrap();
	assert_eq!(sex, "2");

	let birth = xpath
		.findvalue("//hl7:primaryRole/hl7:player1/hl7:birthTime/@value", None)
		.unwrap();
	assert!(birth.starts_with("19850504"));

	let age = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(age, "38");

	let age_unit = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@unit",
			None,
		)
		.unwrap();
	assert_eq!(age_unit, "a");

	let weight = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='7' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(weight, "72");

	let height = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='17' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(height, "168");
}
