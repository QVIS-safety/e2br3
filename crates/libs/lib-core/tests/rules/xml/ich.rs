use crate::common::Result;
use crate::support::{
	assert_has_xml_rule, assert_lacks_xml_rule, read_base_xml_fixture,
	validate_business_xml,
};

fn with_test_result_value(xml: &str, value_xml: &str) -> String {
	xml.replacen(
		"</investigationEvent>",
		&format!(
			"<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/><component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\">{value_xml}</observation></component></organizer></subjectOf2></investigationEvent>"
		),
		1,
	)
}

fn replace_first_reaction_effective_time(xml: &str, replacement: &str) -> String {
	let reaction_id = "<id root=\"154eb889-958b-45f2-a02f-42d4d6f4657f\"/>";
	let Some(reaction_idx) = xml.find(reaction_id) else {
		return xml.to_string();
	};
	let Some(rel_eff_start) = xml[reaction_idx..].find("<effectiveTime") else {
		return xml.to_string();
	};
	let eff_start = reaction_idx + rel_eff_start;
	let Some(rel_eff_end) = xml[eff_start..].find("</effectiveTime>") else {
		return xml.to_string();
	};
	let eff_end = eff_start + rel_eff_end + "</effectiveTime>".len();
	let mut out =
		String::with_capacity(xml.len() - (eff_end - eff_start) + replacement.len());
	out.push_str(&xml[..eff_start]);
	out.push_str(replacement);
	out.push_str(&xml[eff_end..]);
	out
}

fn replace_first_inv_char_bl(xml: &str, replacement: &str) -> String {
	let marker =
		"<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.23\" displayName=\"otherCaseIds\"/>";
	let value = "<value xsi:type=\"BL\" nullFlavor=\"NI\"/>";
	let Some(marker_idx) = xml.find(marker) else {
		return xml.to_string();
	};
	let Some(rel_value_idx) = xml[marker_idx..].find(value) else {
		return xml.to_string();
	};
	let value_idx = marker_idx + rel_value_idx;
	let mut out = String::with_capacity(xml.len() - value.len() + replacement.len());
	out.push_str(&xml[..value_idx]);
	out.push_str(replacement);
	out.push_str(&xml[value_idx + value.len()..]);
	out
}

#[test]
fn ich_xml_bl_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"BL\" value=\"true\"/>",
		"<value xsi:type=\"BL\" value=\"true\" nullFlavor=\"NI\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.BL.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_bl_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.BL.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_bl_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"BL\" nullFlavor=\"NI\"/>",
		"<value xsi:type=\"BL\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.BL.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_bl_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.BL.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_code_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>",
		"<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\" nullFlavor=\"NI\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.CODE.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_code_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.CODE.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_code_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>",
		"<code/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.CODE.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_code_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.CODE.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_country_code_format_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/>",
		"<code code=\"USA\" codeSystem=\"1.0.3166.1.2.2\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_country_code_format_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_dose_quantity_value_unit_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\"><doseQuantity value=\"1\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_dose_quantity_value_unit_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = xml.replacen(
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\">",
		"<routeCode code=\"001\" displayName=\"Oral\" codeSystem=\"0.4.0.127.0.16.1.1.2.6\" codeSystemVersion=\"2014.10.30\"><doseQuantity value=\"1\" unit=\"mg\"/>",
		1,
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_effectivetime_width_requires_bound_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime><width value=\"1\" unit=\"d\"/></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.EFFECTIVETIME.WIDTH.REQUIRES_BOUND");
	Ok(())
}

#[test]
fn ich_xml_effectivetime_width_requires_bound_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime><low value=\"20141010\"/><width value=\"1\" unit=\"d\"/></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.EFFECTIVETIME.WIDTH.REQUIRES_BOUND");
	Ok(())
}

#[test]
fn ich_xml_inv_char_bl_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_inv_char_bl(
		&xml,
		"<value xsi:type=\"BL\" value=\"true\" nullFlavor=\"NI\"/>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_inv_char_bl_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_inv_char_bl_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_inv_char_bl(&xml, "<value xsi:type=\"BL\"/>");

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_inv_char_bl_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_ivl_ts_operator_a_bound_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"IVL_TS\" operator=\"A\"/></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.IVL_TS.OPERATOR_A.BOUND_REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_ivl_ts_operator_a_bound_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"IVL_TS\" operator=\"A\"><low value=\"20141010\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.IVL_TS.OPERATOR_A.BOUND_REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_low_high_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<low value=\"20220614101010-0500\"/>",
		"<low value=\"20220614101010-0500\" nullFlavor=\"NI\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_low_high_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_low_high_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("<low value=\"20220614101010-0500\"/>", "<low/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_low_high_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_meddra_code_format_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" code=\"BAD\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_meddra_code_format_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_meddra_version_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\" codeSystemVersion=\"25.0\"/>",
		"<value xsi:type=\"CE\" code=\"10027940\" codeSystem=\"2.16.840.1.113883.6.163\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.MEDDRA.VERSION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_meddra_version_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.MEDDRA.VERSION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_placeholder_value_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<value xsi:type=\"CE\" code=\"5\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\"/>",
		"<value xsi:type=\"CE\" code=\"D.2.3\" displayName=\"Adult\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.9\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_placeholder_value_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_period_value_unit_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.PERIOD.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_period_value_unit_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\" unit=\"d\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.PERIOD.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_pivl_ts_period_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"/></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.PIVL_TS.PERIOD.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_pivl_ts_period_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\" unit=\"d\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.PIVL_TS.PERIOD.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_pivl_ts_period_value_unit_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_pivl_ts_period_value_unit_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\" unit=\"d\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_root_itsversion_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("ITSVersion=\"XML_1.0\"", "ITSVersion=\"BAD\"", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.ROOT.ITSVERSION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_root_itsversion_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.ROOT.ITSVERSION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_root_schemalocation_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = if let Some(start) = xml.find("xsi:schemaLocation=\"") {
		if let Some(end_rel) = xml[start + 20..].find('"') {
			let end = start + 20 + end_rel;
			let mut mutated = xml.clone();
			mutated.replace_range(start..=end, "");
			mutated
		} else {
			xml.clone()
		}
	} else {
		xml.clone()
	};

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.ROOT.SCHEMALOCATION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_root_schemalocation_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.ROOT.SCHEMALOCATION.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_sxpr_ts_comp_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"></effectiveTime>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.SXPR_TS.COMP.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_sxpr_ts_comp_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = replace_first_reaction_effective_time(
		&xml,
		"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"PIVL_TS\"><period value=\"1\" unit=\"d\"/></comp></effectiveTime>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.SXPR_TS.COMP.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_telecom_format_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen("tel:", "phone:", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TELECOM.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_telecom_format_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TELECOM.FORMAT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_telecom_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<telecom value=\"tel:6102227777\"/>",
		"<telecom value=\"tel:6102227777\" nullFlavor=\"NI\"/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TELECOM.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_telecom_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TELECOM.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_telecom_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken =
		xml.replacen("<telecom value=\"tel:6102227777\"/>", "<telecom/>", 1);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TELECOM.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_telecom_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TELECOM.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_ivl_pq_component_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = with_test_result_value(&xml, "<value xsi:type=\"IVL_PQ\"/>");

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_ivl_pq_component_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = with_test_result_value(
		&xml,
		"<value xsi:type=\"IVL_PQ\"><low value=\"1\" unit=\"mg\"/></value>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_ivl_pq_value_unit_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = with_test_result_value(
		&xml,
		"<value xsi:type=\"IVL_PQ\"><low value=\"1\"/></value>",
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_ivl_pq_value_unit_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = with_test_result_value(
		&xml,
		"<value xsi:type=\"IVL_PQ\"><low value=\"1\" unit=\"mg\"/></value>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_pq_value_unit_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken =
		with_test_result_value(&xml, "<value xsi:type=\"PQ\" value=\"1\"/>");

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_pq_value_unit_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = with_test_result_value(
		&xml,
		"<value xsi:type=\"PQ\" value=\"1\" unit=\"mg\"/>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_testresult_xsi_type_unsupported_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken =
		with_test_result_value(&xml, "<value xsi:type=\"TS\" value=\"20260101\"/>");

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED");
	Ok(())
}

#[test]
fn ich_xml_testresult_xsi_type_unsupported_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let valid = with_test_result_value(
		&xml,
		"<value xsi:type=\"PQ\" value=\"1\" unit=\"mg\"/>",
	);

	let report = validate_business_xml(&valid)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED");
	Ok(())
}

#[test]
fn ich_xml_text_nullflavor_forbidden_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<text>Case Narrative Including Clinical Course, Therapeutic Measures, Outcome and Additional Relevant Information </text>",
		"<text nullFlavor=\"NI\">Case Narrative Including Clinical Course, Therapeutic Measures, Outcome and Additional Relevant Information </text>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TEXT.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_text_nullflavor_forbidden_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TEXT.NULLFLAVOR.FORBIDDEN");
	Ok(())
}

#[test]
fn ich_xml_text_nullflavor_required_false() -> Result<()> {
	let xml = read_base_xml_fixture()?;
	let broken = xml.replacen(
		"<text>Case Narrative Including Clinical Course, Therapeutic Measures, Outcome and Additional Relevant Information </text>",
		"<text/>",
		1,
	);

	let report = validate_business_xml(&broken)?;

	assert!(!report.ok, "expected XML validation failure");
	assert_has_xml_rule(&report, "ICH.XML.TEXT.NULLFLAVOR.REQUIRED");
	Ok(())
}

#[test]
fn ich_xml_text_nullflavor_required_true() -> Result<()> {
	let xml = read_base_xml_fixture()?;

	let report = validate_business_xml(&xml)?;

	assert_lacks_xml_rule(&report, "ICH.XML.TEXT.NULLFLAVOR.REQUIRED");
	Ok(())
}
