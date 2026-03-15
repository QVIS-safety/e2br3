use crate::common::Result;
use crate::support::{
	assert_has_xml_rule, assert_lacks_xml_rule, read_base_xml_fixture,
	validate_business_xml,
};

#[test]
fn fda_n_1_4_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<id extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
		"<id root=\"2.16.840.1.113883.3.989.2.1.3.14\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.N.1.4.REQUIRED");
	Ok(())
}

#[test]
fn fda_n_1_4_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "FDA.N.1.4.REQUIRED");
	Ok(())
}
