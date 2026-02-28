use lib_core::xml::validate::rule_layer_contract::rule_layer_contract;
use lib_core::xml::validate_e2b_xml_business;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn fixture_xml() -> Result<String, Box<dyn Error>> {
	let path = workspace_root().join("docs/refs/instances/FAERS2022Scenario2.xml");
	Ok(fs::read_to_string(path)?)
}

fn mutate_once(xml: &str, needle: &str, replacement: &str) -> String {
	let out = xml.replacen(needle, replacement, 1);
	assert_ne!(out, xml, "mutation failed; needle not found: {needle}");
	out
}

fn has_code(report: &lib_core::xml::XmlValidationReport, code: &str) -> bool {
	let tagged = format!("[{code}]");
	report.errors.iter().any(|e| e.message.contains(&tagged))
}

fn assert_code_transition(
	broken_xml: &str,
	fixed_xml: &str,
	code: &str,
) -> Result<(), Box<dyn Error>> {
	let broken = validate_e2b_xml_business(broken_xml.as_bytes(), None)?;
	assert!(
		has_code(&broken, code),
		"expected code {code} in broken report: {:?}",
		broken.errors
	);
	let fixed = validate_e2b_xml_business(fixed_xml.as_bytes(), None)?;
	assert!(
		!has_code(&fixed, code),
		"expected code {code} to clear in fixed report: {:?}",
		fixed.errors
	);
	Ok(())
}

#[test]
fn xml_lane_case_header_fields_are_not_case_code_validated(
) -> Result<(), Box<dyn Error>> {
	let xml = fixture_xml()?;
	for code in [
		"ICH.C.1.2.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.5.REQUIRED",
	] {
		let contract = rule_layer_contract(code).expect("layer contract");
		assert!(contract.case_validator);
		assert!(!contract.xml_business);
	}

	let broken_c12 = mutate_once(
		&xml,
		"<effectiveTime value=\"20140714151617-0500\"/>",
		"<effectiveTime/>",
	);
	let report_c12 = validate_e2b_xml_business(broken_c12.as_bytes(), None)?;
	assert!(!has_code(&report_c12, "ICH.C.1.2.REQUIRED"));

	let broken_c13 = mutate_once(
		&xml,
		"<value xsi:type=\"CE\" code=\"1\" displayName=\"Spontaneous report\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"/>",
		"<value xsi:type=\"CE\" displayName=\"Spontaneous report\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"/>",
	);
	let report_c13 = validate_e2b_xml_business(broken_c13.as_bytes(), None)?;
	assert!(!has_code(&report_c13, "ICH.C.1.3.REQUIRED"));

	let broken_c15 = mutate_once(
		&xml,
		"<availabilityTime value=\"20220615\"/>",
		"<availabilityTime/>",
	);
	let report_c15 = validate_e2b_xml_business(broken_c15.as_bytes(), None)?;
	assert!(!has_code(&report_c15, "ICH.C.1.5.REQUIRED"));

	Ok(())
}

#[test]
fn xml_lane_structural_guard_catches_c14_low_missing_value(
) -> Result<(), Box<dyn Error>> {
	let xml = fixture_xml()?;
	let broken = mutate_once(&xml, "<low value=\"20220615\"/>", "<low/>");
	assert_code_transition(&broken, &xml, "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED")?;
	let report = validate_e2b_xml_business(broken.as_bytes(), None)?;
	assert!(!report.ok);
	assert!(has_code(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED"));
	let structural = rule_layer_contract("ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED")
		.expect("layer contract");
	assert!(structural.xml_business);
	assert!(!has_code(&report, "ICH.C.1.4.REQUIRED"));
	let case_code =
		rule_layer_contract("ICH.C.1.4.REQUIRED").expect("layer contract");
	assert!(case_code.case_validator);
	assert!(!case_code.xml_business);
	Ok(())
}

#[test]
fn xml_lane_bl_structural_guard_catches_c17_and_fda_fields(
) -> Result<(), Box<dyn Error>> {
	let xml = fixture_xml()?;

	let broken_c17 = mutate_once(
		&xml,
		"<value xsi:type=\"BL\" value=\"false\"/>",
		"<value xsi:type=\"BL\"/>",
	);
	assert_code_transition(&broken_c17, &xml, "ICH.XML.BL.NULLFLAVOR.REQUIRED")?;
	let report_c17 = validate_e2b_xml_business(broken_c17.as_bytes(), None)?;
	assert!(has_code(&report_c17, "ICH.XML.BL.NULLFLAVOR.REQUIRED"));
	let structural = rule_layer_contract("ICH.XML.BL.NULLFLAVOR.REQUIRED")
		.expect("layer contract");
	assert!(structural.xml_business);
	assert!(!has_code(&report_c17, "ICH.C.1.7.REQUIRED"));

	let broken_fda_c112 = mutate_once(
		&xml,
		"<value xsi:type=\"BL\" value=\"true\"/>",
		"<value xsi:type=\"BL\"/>",
	);
	assert_code_transition(
		&broken_fda_c112,
		&xml,
		"ICH.XML.BL.NULLFLAVOR.REQUIRED",
	)?;
	let report_fda_c112 =
		validate_e2b_xml_business(broken_fda_c112.as_bytes(), None)?;
	assert!(has_code(&report_fda_c112, "ICH.XML.BL.NULLFLAVOR.REQUIRED"));
	assert!(!has_code(&report_fda_c112, "FDA.C.1.12.REQUIRED"));

	let broken_fda_ei32h = mutate_once(
		&xml,
		"<value xsi:type=\"BL\" nullFlavor=\"NI\"/>",
		"<value xsi:type=\"BL\"/>",
	);
	assert_code_transition(
		&broken_fda_ei32h,
		&xml,
		"ICH.XML.BL.NULLFLAVOR.REQUIRED",
	)?;
	let report_fda_ei32h =
		validate_e2b_xml_business(broken_fda_ei32h.as_bytes(), None)?;
	assert!(has_code(
		&report_fda_ei32h,
		"ICH.XML.BL.NULLFLAVOR.REQUIRED"
	));
	assert!(!has_code(&report_fda_ei32h, "FDA.E.i.3.2h.REQUIRED"));

	Ok(())
}
