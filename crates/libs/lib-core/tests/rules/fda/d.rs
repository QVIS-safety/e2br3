use crate::common::Result;
use crate::support::{
	assert_has_xml_rule, assert_lacks_xml_rule, read_base_xml_fixture,
	validate_business_xml,
};
use lib_core::xml::validate::{
	is_rule_condition_satisfied, is_rule_value_valid, RuleFacts,
};

#[test]
fn fda_d_11_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.D.11.REQUIRED",
		None,
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_d_11_required_true() {
	assert!(is_rule_value_valid(
		"FDA.D.11.REQUIRED",
		Some("C41260"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_d_11_required_condition_false() {
	assert!(!is_rule_condition_satisfied(
		"FDA.D.11.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_d_11_required_condition_true() {
	assert!(is_rule_condition_satisfied(
		"FDA.D.11.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_d_11_required_xml_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fda_xml = xml
		.replacen(
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let broken = fda_xml.replacen(
		"<value xsi:type=\"CE\" code=\"C41260\" displayName=\"Asian\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		"<value xsi:type=\"CE\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.D.11.REQUIRED");
	Ok(())
}

#[test]
fn fda_d_11_required_xml_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;
	assert_lacks_xml_rule(&report, "FDA.D.11.REQUIRED");
	Ok(())
}

#[test]
fn fda_d_12_required_false() {
	assert!(!is_rule_value_valid(
		"FDA.D.12.REQUIRED",
		None,
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_d_12_required_true() {
	assert!(is_rule_value_valid(
		"FDA.D.12.REQUIRED",
		Some("C41222"),
		None,
		RuleFacts::default(),
	));
}

#[test]
fn fda_d_12_required_condition_false() {
	assert!(!is_rule_condition_satisfied(
		"FDA.D.12.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(false),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_d_12_required_condition_true() {
	assert!(is_rule_condition_satisfied(
		"FDA.D.12.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(true),
			..RuleFacts::default()
		},
	));
}

#[test]
fn fda_d_12_required_xml_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let broken = fda_xml.replacen(
		"<value xsi:type=\"CE\" code=\"C41222\" displayName=\"Not Hispanic or Latino\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>",
		"<value xsi:type=\"CE\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.D.12.REQUIRED");
	Ok(())
}

#[test]
fn fda_d_12_required_xml_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let report = validate_business_xml(&xml)?;
	assert_lacks_xml_rule(&report, "FDA.D.12.REQUIRED");
	Ok(())
}
