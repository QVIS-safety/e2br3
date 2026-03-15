use crate::model::test_result::TestResult;
use crate::xml::Result;
use sqlx::types::time::Date;

pub fn export_f_test_results_xml(results: &[TestResult]) -> Result<String> {
	let mut ordered: Vec<&TestResult> = results.iter().collect();
	ordered.sort_by_key(|result| result.sequence_number);

	let mut items_xml = String::new();
	for result in ordered {
		items_xml.push_str(&test_result_fragment(result));
	}
	let xml = base_f_test_result_skeleton().replace("{TESTS}", &items_xml);
	Ok(xml)
}

pub(crate) fn test_result_fragment(result: &TestResult) -> String {
	let mut out = String::new();
	out.push_str("<subjectOf2 typeCode=\"SBJ\"><organizer classCode=\"CATEGORY\" moodCode=\"EVN\">");
	out.push_str(
		"<code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.20\"/>",
	);
	out.push_str("<component typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\">");

	// F.r.3.1 / F.r.3.2 - test identity.
	out.push_str("<code");
	if let Some(code) = result.test_meddra_code.as_deref() {
		out.push_str(" code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"");
	}
	if let Some(version) = result.test_meddra_version.as_deref() {
		out.push_str(" codeSystemVersion=\"");
		out.push_str(&xml_escape(version));
		out.push_str("\"");
	}
	out.push_str(" displayName=\"");
	out.push_str(&xml_escape(&result.test_name));
	out.push_str("\">");
	out.push_str("<originalText>");
	out.push_str(&xml_escape(&result.test_name));
	out.push_str("</originalText>");
	out.push_str("</code>");

	// F.r.2.1 - test date.
	if let Some(date) = result.test_date {
		out.push_str("<effectiveTime value=\"");
		out.push_str(&fmt_date(date));
		out.push_str("\"/>");
	} else if let Some(null_flavor) = result.test_date_null_flavor.as_deref() {
		out.push_str("<effectiveTime nullFlavor=\"");
		out.push_str(&xml_escape(null_flavor));
		out.push_str("\"/>");
	}

	// F.r.3.3 - coded test result.
	if let Some(code) = result.test_result_code.as_deref() {
		out.push_str("<interpretationCode code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"/>");
	}

	// F.r.3.4 - result value and unit.
	if result.test_result_value.is_some() || result.result_unstructured.is_some() {
		out.push_str("<value");
		if let Some(val) = result.test_result_value.as_deref() {
			out.push_str(" value=\"");
			out.push_str(&xml_escape(val));
			out.push_str("\"");
		}
		if let Some(unit) = result.test_result_unit.as_deref() {
			out.push_str(" unit=\"");
			out.push_str(&xml_escape(unit));
			out.push_str("\"");
		}
		out.push_str(">");
		if let Some(text) = result.result_unstructured.as_deref() {
			out.push_str(&xml_escape(text));
		}
		out.push_str("</value>");
	}

	// F.r.3.4 supplemental ranges.
	if result.normal_low_value.is_some() || result.normal_high_value.is_some() {
		out.push_str("<referenceRange>");
		if let Some(low) = result.normal_low_value.as_deref() {
			out.push_str(
				"<observationRange><interpretationCode code=\"L\"/><value value=\"",
			);
			out.push_str(&xml_escape(low));
			out.push_str("\"/></observationRange>");
		}
		if let Some(high) = result.normal_high_value.as_deref() {
			out.push_str(
				"<observationRange><interpretationCode code=\"H\"/><value value=\"",
			);
			out.push_str(&xml_escape(high));
			out.push_str("\"/></observationRange>");
		}
		out.push_str("</referenceRange>");
	}

	// F.r.1 / F.r.2.2a - comments and more info.
	if let Some(comments) = result.comments.as_deref() {
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value>");
		out.push_str(&xml_escape(comments));
		out.push_str("</value></observation></outboundRelationship2>");
	}

	if let Some(value) = result.more_info_available {
		let val = if value { "true" } else { "false" };
		out.push_str("<outboundRelationship2 typeCode=\"COMP\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"11\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"");
		out.push_str(val);
		out.push_str("\"/></observation></outboundRelationship2>");
	}

	out.push_str("</observation></component></organizer></subjectOf2>");
	out
}

fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn xml_escape(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

fn base_f_test_result_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t<component typeCode=\"COMP\">\
\t\t\t\t\t\t<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t\t\t<subject1 typeCode=\"SBJ\">\
\t\t\t\t\t\t\t\t<primaryRole classCode=\"INVSBJ\">\
\t\t\t\t\t\t\t\t\t<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\"><name/></player1>\
\t\t\t\t\t\t\t\t\t{TESTS}\
\t\t\t\t\t\t\t\t</primaryRole>\
\t\t\t\t\t\t\t</subject1>\
\t\t\t\t\t\t</adverseEventAssessment>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}
