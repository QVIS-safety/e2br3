use super::*;
use lib_core::model::narrative::{
	CaseSummaryInformation, NarrativeInformation, SenderDiagnosis,
};

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let Some(narrative) =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?
	else {
		return std::str::from_utf8(raw_xml)
			.map(str::to_owned)
			.map_err(|err| Error::InvalidXml {
				message: format!("XML not valid UTF-8: {err}"),
				line: None,
				column: None,
			});
	};
	patch_h_narrative(raw_xml, &narrative)
}

pub(crate) fn apply_h_5_case_summaries(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	summaries: &[CaseSummaryInformation],
) -> Result<()> {
	let node_path = "//hl7:investigationEvent/hl7:component[hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]]";
	remove_nodes(xpath, node_path);
	for (text, language_code) in summaries.iter().filter_map(|summary| {
		summary.summary_text.as_deref().and_then(|text| {
			(!text.trim().is_empty())
				.then_some((text, summary.language_code.as_deref()))
		})
	}) {
		let text = xml_escape(text);
		let language = if let Some(value) = language_code {
			format!(" language=\"{}\"", xml_escape(value))
		} else {
			String::new()
		};
		let fragment = format!("<component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"summaryAndComment\"/><value xsi:type=\"ED\"{language}>{text}</value><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"reporter\"/></assignedEntity></author></observationEvent></component>");
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			&fragment,
		)?;
	}
	reorder_investigation_event_children(xpath);
	Ok(())
}

pub(crate) fn apply_h_3_sender_diagnoses(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	diagnoses: &[SenderDiagnosis],
) -> Result<()> {
	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]",
	);

	for diagnosis in diagnoses {
		let mut attrs = String::from("xsi:type=\"CE\"");
		if let Some(code) = diagnosis.diagnosis_meddra_code.as_deref() {
			write_h_3_r_1b(&mut attrs, code);
		}
		attrs.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(version) = diagnosis.diagnosis_meddra_version.as_deref() {
			write_h_3_r_1a(&mut attrs, version);
		}
		let fragment = format!(
			"<component1 typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"15\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"diagnosis\"/><value {attrs}/><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"sender\"/></assignedEntity></author></observationEvent></component1>"
		);
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment",
			&fragment,
		)?;
	}

	Ok(())
}

/// e2b:H.3.r.1a
fn write_h_3_r_1a(attrs: &mut String, meddra_version: &str) {
	attrs.push_str(&format!(
		" codeSystemVersion=\"{}\"",
		xml_escape(meddra_version)
	));
}

/// e2b:H.3.r.1b
fn write_h_3_r_1b(attrs: &mut String, meddra_code: &str) {
	attrs.push_str(&format!(" code=\"{}\"", xml_escape(meddra_code)));
}

/// e2b:H.5.r.1a
fn write_h_5_r_1a(xpath: &mut Context, summary_text: &str) {
	set_text_first(
		xpath,
		"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
		summary_text,
	);
}

/// e2b:H.5.r.1b
fn write_h_5_r_1b(xpath: &mut Context, language_code: &str) {
	set_attr_first(
		xpath,
		"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
		"language",
		language_code,
	);
}

fn reorder_investigation_event_children(xpath: &mut Context) {
	if let Ok(outbound_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:outboundRelationship", None)
	{
		for mut node in outbound_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject1_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf1", None)
	{
		for mut node in subject1_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
	if let Ok(subject2_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf2", None)
	{
		for mut node in subject2_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}

pub fn export_h_narrative_xml(narrative: &NarrativeInformation) -> Result<String> {
	let mut components = String::new();
	if let Some(comments) = narrative.reporter_comments.as_deref() {
		components.push_str(&write_h_2_or_h_4(comments, "3"));
	}
	if let Some(comments) = narrative.sender_comments.as_deref() {
		components.push_str(&write_h_2_or_h_4(comments, "1"));
	}
	let xml = base_h_narrative_skeleton()
		.replace("{CASE_NARRATIVE}", &xml_escape(&narrative.case_narrative))
		.replace("{COMMENTS}", &components);
	Ok(xml)
}

pub(crate) fn write_h_2_or_h_4(text: &str, author_code: &str) -> String {
	format!(
		"<component1 typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"10\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\">{}</value><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\"/></assignedEntity></author></observationEvent></component1>",
		xml_escape(text),
		author_code
	)
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

#[cfg(test)]
mod tests {
	use super::*;
	use libxml::parser::Parser;
	use libxml::xpath::Context;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;
	use std::collections::BTreeSet;

	#[test]
	fn h_5_exports_all_nonempty_summaries_in_source_order() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string(crate::export::base_export_skeleton())
			.expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("HL7 namespace");
		let summary =
			|sequence_number, text: Option<&str>, language: Option<&str>| {
				CaseSummaryInformation {
					id: Uuid::new_v4(),
					narrative_id: Uuid::new_v4(),
					sequence_number,
					deleted: false,
					language_code: language.map(str::to_string),
					summary_text: text.map(str::to_string),
					created_at: OffsetDateTime::UNIX_EPOCH,
					updated_at: OffsetDateTime::UNIX_EPOCH,
					created_by: Uuid::nil(),
					updated_by: None,
				}
			};
		let summaries = [
			summary(1, Some("  "), Some("jpn")),
			summary(2, Some("first & <summary>"), Some("eng")),
			summary(3, Some("두번째 요약"), Some("kor")),
		];
		apply_h_5_case_summaries(&mut doc, &parser, &mut xpath, &summaries)
			.expect("apply H.5 summaries");

		let values = "//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value";
		assert_eq!(xpath.findnodes(values, None).expect("H.5 values").len(), 2);
		assert_eq!(
			xpath.findvalue(&format!("({values})[1]"), None).unwrap(),
			"first & <summary>"
		);
		assert_eq!(
			xpath
				.findvalue(&format!("({values})[1]/@language"), None)
				.unwrap(),
			"eng"
		);
		assert_eq!(
			xpath.findvalue(&format!("({values})[2]"), None).unwrap(),
			"두번째 요약"
		);
		assert_eq!(
			xpath
				.findvalue(&format!("({values})[2]/@language"), None)
				.unwrap(),
			"kor"
		);
	}

	#[test]
	fn section_h_writers_cover_exported_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/h-narrative.json"
		))
		.expect("section H registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["e2br3_code"] != "H.additionalInformation")
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let source = format!(
			"{}\n{}",
			include_str!("h.rs"),
			include_str!("../roundtrip/h_narrative.rs")
		);
		let implemented = source
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}
