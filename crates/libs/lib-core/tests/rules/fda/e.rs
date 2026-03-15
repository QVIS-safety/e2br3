use crate::common::Result;
use crate::support::{
	assert_has_xml_rule, assert_lacks_xml_rule, read_base_xml_fixture,
	validate_business_xml,
};

fn replace_required_intervention_value(xml: &str, replacement: &str) -> String {
	let code_marker =
		"<code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\" displayName=\"requiredIntervention\"/>";
	let value_marker = "<value xsi:type=\"BL\" nullFlavor=\"NI\"/>";
	let Some(code_idx) = xml.find(code_marker) else {
		return xml.to_string();
	};
	let Some(rel_value_idx) = xml[code_idx..].find(value_marker) else {
		return xml.to_string();
	};
	let value_idx = code_idx + rel_value_idx;
	let mut out =
		String::with_capacity(xml.len() - value_marker.len() + replacement.len());
	out.push_str(&xml[..value_idx]);
	out.push_str(replacement);
	out.push_str(&xml[value_idx + value_marker.len()..]);
	out
}

fn fda_xml_base() -> Result<String> {
	Ok(read_base_xml_fixture()?
		.replacen(
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		))
}

#[test]
fn fda_e_i_3_2h_required_false() -> Result<()> {
	let xml = fda_xml_base()?;
	let broken =
		replace_required_intervention_value(&xml, "<value xsi:type=\"BL\"/>");

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "FDA.E.i.3.2h.REQUIRED");
	Ok(())
}

#[test]
fn fda_e_i_3_2h_required_true() -> Result<()> {
	let xml = fda_xml_base()?;
	let fixed = replace_required_intervention_value(
		&xml,
		"<value xsi:type=\"BL\" value=\"true\"/>",
	);

	let report = validate_business_xml(&fixed)?;

	assert_lacks_xml_rule(&report, "FDA.E.i.3.2h.REQUIRED");
	Ok(())
}
