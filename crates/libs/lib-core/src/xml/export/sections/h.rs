use super::*;
use crate::model::narrative::NarrativeInformation;

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	patch_h_narrative(raw_xml, &narrative)
}

pub(crate) async fn export_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let narrative = NarrativeInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	export_h_narrative_xml(&narrative)
}

pub(crate) async fn apply_case_summary_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let narrative =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(v) => v,
			Err(_) => return Ok(()),
		};
	let summaries = fetch_case_summaries(ctx, mm, narrative.id).await?;
	let Some(summary) = summaries.iter().find(|s| {
		s.summary_text
			.as_deref()
			.is_some_and(|v| !v.trim().is_empty())
	}) else {
		return Ok(());
	};

	let node_path = "//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]";
	if xpath
		.findnodes(node_path, None)
		.map(|nodes| nodes.is_empty())
		.unwrap_or(true)
	{
		let fragment = "<component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"36\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" displayName=\"summaryAndComment\"/><value xsi:type=\"ED\"/><author typeCode=\"AUT\"><assignedEntity classCode=\"ASSIGNED\"><code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.21\" displayName=\"reporter\"/></assignedEntity></author></observationEvent></component>";
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			fragment,
		)?;
		reorder_investigation_event_children(xpath);
	}

	if let Some(text) = summary.summary_text.as_deref() {
		set_text_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			text,
		);
	}
	if let Some(language) = summary.language_code.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]/hl7:value",
			"language",
			language,
		);
	}
	if let Some(summary_type) = summary.summary_type.as_deref() {
		set_attr_first(
			xpath,
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:author/hl7:assignedEntity/hl7:code",
			"code",
			summary_type,
		);
	}
	Ok(())
}

pub(crate) async fn apply_sender_diagnosis_section(
	ctx: &Ctx,
	doc: &mut Document,
	parser: &Parser,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	xpath: &mut Context,
) -> Result<()> {
	let narrative =
		match NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(v) => v,
			Err(_) => return Ok(()),
		};
	let diagnoses = fetch_sender_diagnoses(ctx, mm, narrative.id).await?;

	remove_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]",
	);

	for diagnosis in diagnoses {
		let mut attrs = String::from("xsi:type=\"CE\"");
		if let Some(code) = diagnosis.diagnosis_meddra_code.as_deref() {
			attrs.push_str(&format!(" code=\"{}\"", xml_escape(code)));
		}
		attrs.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(version) = diagnosis.diagnosis_meddra_version.as_deref() {
			attrs.push_str(&format!(
				" codeSystemVersion=\"{}\"",
				xml_escape(version)
			));
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
		components.push_str(&comment_fragment(comments, "3"));
	}
	if let Some(comments) = narrative.sender_comments.as_deref() {
		components.push_str(&comment_fragment(comments, "1"));
	}
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
