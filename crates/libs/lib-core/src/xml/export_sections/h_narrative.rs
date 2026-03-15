use crate::model::narrative::NarrativeInformation;
use crate::xml::Result;

pub fn export_h_narrative_xml(narrative: &NarrativeInformation) -> Result<String> {
	let mut components = String::new();

	// H.2 - reporter comments.
	if let Some(comments) = narrative.reporter_comments.as_deref() {
		components.push_str(&comment_fragment(comments, "3"));
	}

	// H.4 - sender comments.
	if let Some(comments) = narrative.sender_comments.as_deref() {
		components.push_str(&comment_fragment(comments, "1"));
	}

	// H.1 - case narrative.
	let xml = base_h_narrative_skeleton()
		.replace("{CASE_NARRATIVE}", &xml_escape(&narrative.case_narrative))
		.replace("{COMMENTS}", &components);
	Ok(xml)
}

pub(crate) fn comment_fragment(text: &str, author_code: &str) -> String {
	format!(
		"<component1 typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\">{}</value><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\"/></assignedEntity></author></observationEvent></component1>",
		xml_escape(text),
		author_code
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

fn base_h_narrative_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t<text>{CASE_NARRATIVE}</text>\
\t\t\t\t\t<component typeCode=\"COMP\">\
\t\t\t\t\t\t<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t\t\t{COMMENTS}\
\t\t\t\t\t\t</adverseEventAssessment>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}
