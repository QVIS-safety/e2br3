use crate::model::reaction::Reaction;
use crate::xml::error::Error;
use crate::xml::validate::{
	normalize_outcome_code, outcome_display_name,
	should_emit_required_intervention_null_flavor_ni,
};
use crate::xml::Result;
use sqlx::types::time::Date;

pub fn export_e_reactions_xml(reactions: &[Reaction]) -> Result<String> {
	let mut reactions_xml = String::new();
	for reaction in reactions {
		reactions_xml.push_str(&reaction_fragment(reaction)?);
	}
	let xml = base_e_reaction_skeleton().replace("{REACTIONS}", &reactions_xml);
	Ok(xml)
}

pub(crate) fn reaction_fragment(reaction: &Reaction) -> Result<String> {
	let mut out = String::new();
	out.push_str("<subjectOf2 typeCode=\"SBJ\"><observation classCode=\"OBS\" moodCode=\"EVN\">");
	out.push_str("<id root=\"");
	out.push_str(&xml_escape(&reaction.id.to_string()));
	out.push_str("\"/>");
	out.push_str(
		"<code code=\"29\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>",
	);

	if reaction.start_date.is_some()
		|| reaction.end_date.is_some()
		|| reaction.duration_value.is_some()
	{
		let has_duration = reaction.duration_value.is_some();
		if has_duration {
			out.push_str("<effectiveTime xsi:type=\"SXPR_TS\">");
		} else {
			out.push_str("<effectiveTime xsi:type=\"IVL_TS\">");
		}
		if let Some(start) = reaction.start_date {
			if has_duration {
				out.push_str(
					"<comp xsi:type=\"IVL_TS\" operator=\"A\"><low value=\"",
				);
				out.push_str(&fmt_date(start));
				out.push_str("\"/></comp>");
			} else {
				out.push_str("<low value=\"");
				out.push_str(&fmt_date(start));
				out.push_str("\"/>");
			}
		}
		if let Some(end) = reaction.end_date {
			if has_duration {
				out.push_str(
					"<comp xsi:type=\"IVL_TS\" operator=\"A\"><high value=\"",
				);
				out.push_str(&fmt_date(end));
				out.push_str("\"/></comp>");
			} else {
				out.push_str("<high value=\"");
				out.push_str(&fmt_date(end));
				out.push_str("\"/>");
			}
		}
		if let Some(width) = reaction.duration_value.as_ref() {
			out.push_str("<comp xsi:type=\"IVL_TS\" operator=\"A\"><width value=\"");
			out.push_str(&xml_escape(&width.to_string()));
			out.push_str("\"");
			if let Some(unit) = reaction.duration_unit.as_deref() {
				out.push_str(" unit=\"");
				out.push_str(&xml_escape(unit));
				out.push_str("\"");
			}
			out.push_str("/></comp>");
		}
		out.push_str("</effectiveTime>");
	}

	let meddracode = reaction
		.reaction_meddra_code
		.as_deref()
		.unwrap_or("")
		.trim();
	if !meddracode.is_empty() {
		out.push_str("<value xsi:type=\"CE\" code=\"");
		out.push_str(&xml_escape(meddracode));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(version) = reaction.reaction_meddra_version.as_deref() {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(version));
			out.push_str("\"");
		}
		out.push_str("><originalText");
		if let Some(lang) = reaction.reaction_language.as_deref() {
			out.push_str(" language=\"");
			out.push_str(&xml_escape(lang));
			out.push_str("\"");
		}
		out.push_str(">");
		out.push_str(&xml_escape(&reaction.primary_source_reaction));
		out.push_str("</originalText></value>");
	} else {
		out.push_str("<value xsi:type=\"CE\"><originalText");
		if let Some(lang) = reaction.reaction_language.as_deref() {
			out.push_str(" language=\"");
			out.push_str(&xml_escape(lang));
			out.push_str("\"");
		}
		out.push_str(">");
		out.push_str(&xml_escape(&reaction.primary_source_reaction));
		out.push_str("</originalText></value>");
	}
	out.push_str(&observation_rel_translation(reaction));
	if let Some(term_code) =
		term_highlight_code(reaction.term_highlighted, reaction.serious)
	{
		out.push_str(&observation_rel_code("37", &term_code));
	}

	out.push_str(&observation_rel_bool_or_ni("34", reaction.criteria_death));
	out.push_str(&observation_rel_bool_or_ni(
		"21",
		reaction.criteria_life_threatening,
	));
	out.push_str(&observation_rel_bool_or_ni(
		"33",
		reaction.criteria_hospitalization,
	));
	out.push_str(&observation_rel_bool_or_ni(
		"35",
		reaction.criteria_disabling,
	));
	out.push_str(&observation_rel_bool_or_ni(
		"12",
		reaction.criteria_congenital_anomaly,
	));
	out.push_str(&observation_rel_bool_or_ni(
		"26",
		reaction.criteria_other_medically_important,
	));

	out.push_str(&observation_rel_required_intervention(
		reaction.required_intervention.as_deref(),
	));

	out.push_str(&observation_rel_outcome(
		reaction.outcome.as_deref(),
		reaction.sequence_number,
	)?);
	if let Some(value) = reaction.medical_confirmation {
		out.push_str(&observation_rel_bool("24", value));
	}

	out.push_str("</observation></subjectOf2>");
	Ok(out)
}

fn observation_rel_bool(code: &str, value: bool) -> String {
	let v = if value { "true" } else { "false" };
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{v}\"/></observation></outboundRelationship2>"
	)
}

fn observation_rel_code(code: &str, value: &str) -> String {
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"{}\"/></observation></outboundRelationship2>",
		xml_escape(value)
	)
}

fn observation_rel_bool_or_ni(code: &str, value: bool) -> String {
	if value {
		observation_rel_bool(code, true)
	} else {
		format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"NI\"/></observation></outboundRelationship2>"
		)
	}
}

fn observation_rel_outcome(
	value: Option<&str>,
	sequence_number: i32,
) -> Result<String> {
	let code = normalize_outcome_code(value).ok_or_else(|| Error::InvalidXml {
		message: format!(
			"ICH.E.i.7.REQUIRED: reaction outcome missing or invalid for reaction sequence {}",
			sequence_number
		),
		line: None,
		column: None,
	})?;
	let display_name = outcome_display_name(code);
	Ok(format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"27\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\" displayName=\"{}\"/></observation></outboundRelationship2>",
		xml_escape(code),
		xml_escape(display_name)
	))
}

fn normalize_bl(value: &str) -> Option<&'static str> {
	match value.trim().to_ascii_lowercase().as_str() {
		"true" | "1" | "yes" | "y" => Some("true"),
		"false" | "2" | "no" | "n" => Some("false"),
		_ => None,
	}
}

fn observation_rel_required_intervention(value: Option<&str>) -> String {
	if let Some(v) = value.and_then(normalize_bl) {
		return format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\" value=\"{v}\"/></observation></outboundRelationship2>"
		);
	}
	if should_emit_required_intervention_null_flavor_ni() {
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\" nullFlavor=\"NI\"/></observation></outboundRelationship2>".to_string()
	} else {
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\" value=\"true\"/></observation></outboundRelationship2>".to_string()
	}
}

fn observation_rel_translation(reaction: &Reaction) -> String {
	let text = reaction
		.primary_source_reaction_translation
		.as_deref()
		.filter(|v| !v.trim().is_empty())
		.unwrap_or_else(|| reaction.primary_source_reaction.as_str());
	if text.trim().is_empty() {
		return String::new();
	}
	let mut out = String::new();
	out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"30\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"");
	if let Some(lang) = reaction.reaction_language.as_deref() {
		out.push_str(" language=\"");
		out.push_str(&xml_escape(lang));
		out.push_str("\"");
	}
	out.push_str(">");
	out.push_str(&xml_escape(text));
	out.push_str("</value></observation></outboundRelationship2>");
	out
}

fn term_highlight_code(
	term_highlighted: Option<bool>,
	serious: Option<bool>,
) -> Option<String> {
	match (term_highlighted, serious) {
		(Some(true), Some(true)) => Some("3".to_string()),
		(Some(true), Some(false)) => Some("1".to_string()),
		(Some(false), Some(true)) => Some("4".to_string()),
		(Some(false), Some(false)) => Some("2".to_string()),
		_ => None,
	}
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

fn base_e_reaction_skeleton() -> &'static str {
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
\t\t\t\t\t\t\t\t\t{REACTIONS}\
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
