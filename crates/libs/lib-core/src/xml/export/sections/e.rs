use super::*;

pub(crate) async fn export_patch(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let reactions = fetch_reactions(mm, case_id).await?;
	patch_e_reactions(raw_xml, &reactions)
}

pub(crate) async fn export_build(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let reactions = fetch_reactions(mm, case_id).await?;
	export_e_reactions_xml(&reactions)
}

async fn fetch_reactions(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<Vec<Reaction>> {
	mm.dbx()
		.fetch_all(
			sqlx::query_as::<_, Reaction>(
				"SELECT * FROM reactions WHERE case_id = $1 AND deleted = false ORDER BY sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

use crate::xml::export::policy::{
	normalize_outcome_code, outcome_display_name,
	should_emit_required_intervention_null_flavor_ni,
};
use sqlx::types::time::Date;

pub fn export_e_reactions_xml(reactions: &[Reaction]) -> Result<String> {
	let mut ordered: Vec<&Reaction> = reactions.iter().collect();
	ordered.sort_by_key(|reaction| reaction.sequence_number);

	let mut reactions_xml = String::new();
	for reaction in ordered {
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
		|| reaction.start_date_null_flavor.is_some()
		|| reaction.end_date.is_some()
		|| reaction.end_date_null_flavor.is_some()
		|| reaction.duration_value.is_some()
	{
		let has_duration = reaction.duration_value.is_some();
		if has_duration {
			out.push_str("<effectiveTime xsi:type=\"SXPR_TS\">");
		} else {
			out.push_str("<effectiveTime xsi:type=\"IVL_TS\">");
		}
		append_time_boundary_fragment(
			&mut out,
			"low",
			reaction.start_date,
			reaction.start_date_null_flavor.as_deref(),
			has_duration,
		);
		append_time_boundary_fragment(
			&mut out,
			"high",
			reaction.end_date,
			reaction.end_date_null_flavor.as_deref(),
			has_duration,
		);
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
	out.push_str(&observation_rel_bool_or_null_flavor(
		"34",
		reaction.criteria_death,
		reaction.criteria_death_null_flavor.as_deref(),
	));
	out.push_str(&observation_rel_bool_or_null_flavor(
		"21",
		reaction.criteria_life_threatening,
		reaction.criteria_life_threatening_null_flavor.as_deref(),
	));
	out.push_str(&observation_rel_bool_or_null_flavor(
		"33",
		reaction.criteria_hospitalization,
		reaction.criteria_hospitalization_null_flavor.as_deref(),
	));
	out.push_str(&observation_rel_bool_or_null_flavor(
		"35",
		reaction.criteria_disabling,
		reaction.criteria_disabling_null_flavor.as_deref(),
	));
	out.push_str(&observation_rel_bool_or_null_flavor(
		"12",
		reaction.criteria_congenital_anomaly,
		reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
	));
	out.push_str(&observation_rel_bool_or_null_flavor(
		"26",
		reaction.criteria_other_medically_important,
		reaction
			.criteria_other_medically_important_null_flavor
			.as_deref(),
	));
	out.push_str(&observation_rel_required_intervention(
		reaction.required_intervention.as_deref(),
	));
	append_extension_code(
		&mut out,
		"AE_EXPECTEDNESS",
		reaction.expectedness.as_deref(),
	);
	append_extension_code(&mut out, "AE_SEVERITY", reaction.severity.as_deref());
	append_extension_code(
		&mut out,
		"KR_DVC_AECL",
		reaction.mfds_device_ae_classification.as_deref(),
	);
	append_extension_code(
		&mut out,
		"KR_DVC_AEOUT",
		reaction.mfds_device_ae_outcome.as_deref(),
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_CC_MD",
		reaction.mfds_device_cause_medical_device,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_CC_PI",
		reaction.mfds_device_cause_procedure_issue,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_CC_PC",
		reaction.mfds_device_cause_patient_condition,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_CC_UA",
		reaction.mfds_device_cause_unable_to_assess,
	);
	append_extension_text(
		&mut out,
		"KR_DVC_CC_OTH",
		reaction.mfds_device_cause_other.as_deref(),
	);
	append_extension_text(
		&mut out,
		"KR_DVC_ACT_RSN",
		reaction.mfds_device_action_reason.as_deref(),
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_RC",
		reaction.mfds_device_action_recall,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_RP",
		reaction.mfds_device_action_repair,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_INSP",
		reaction.mfds_device_action_inspection,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_REPL",
		reaction.mfds_device_action_replacement,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_IMP",
		reaction.mfds_device_action_improvement,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_MON",
		reaction.mfds_device_action_monitoring,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_NTF",
		reaction.mfds_device_action_notification,
	);
	append_extension_bool(
		&mut out,
		"KR_DVC_ACT_CAS",
		reaction.mfds_device_action_label_change,
	);
	append_extension_text(
		&mut out,
		"KR_DVC_ACT_OTH",
		reaction.mfds_device_action_other.as_deref(),
	);
	out.push_str(&observation_rel_outcome(
		reaction.outcome.as_deref(),
		reaction.sequence_number,
	)?);
	if let Some(value) = reaction.medical_confirmation {
		out.push_str(&observation_rel_bool("24", value));
	}
	if let Some(country) = reaction.country_code.as_deref() {
		let country = country.trim();
		if !country.is_empty() {
			out.push_str("<location><locatedEntity><locatedPlace><code code=\"");
			out.push_str(&xml_escape(country));
			out.push_str("\"/></locatedPlace></locatedEntity></location>");
		}
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

fn append_extension_bool(out: &mut String, code: &str, value: Option<bool>) {
	if let Some(value) = value {
		let v = if value { "true" } else { "false" };
		out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"");
		out.push_str(&xml_escape(code));
		out.push_str("\"/><value xsi:type=\"BL\" value=\"");
		out.push_str(v);
		out.push_str("\"/></observation></outboundRelationship2>");
	}
}

fn append_extension_code(out: &mut String, code: &str, value: Option<&str>) {
	let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
		return;
	};
	out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"");
	out.push_str(&xml_escape(code));
	out.push_str("\"/><value xsi:type=\"CE\" code=\"");
	out.push_str(&xml_escape(value));
	out.push_str("\"/></observation></outboundRelationship2>");
}

fn append_extension_text(out: &mut String, code: &str, value: Option<&str>) {
	let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
		return;
	};
	out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"");
	out.push_str(&xml_escape(code));
	out.push_str("\"/><value xsi:type=\"ED\">");
	out.push_str(&xml_escape(value));
	out.push_str("</value></observation></outboundRelationship2>");
}

fn observation_rel_code(code: &str, value: &str) -> String {
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"{}\"/></observation></outboundRelationship2>",
		xml_escape(value)
	)
}

fn observation_rel_bool_or_null_flavor(
	code: &str,
	value: bool,
	null_flavor: Option<&str>,
) -> String {
	if value {
		observation_rel_bool(code, true)
	} else {
		format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"{}\"/></observation></outboundRelationship2>",
			xml_escape(null_flavor.unwrap_or("NI"))
		)
	}
}

fn append_time_boundary_fragment(
	out: &mut String,
	tag: &str,
	date: Option<Date>,
	null_flavor: Option<&str>,
	has_duration: bool,
) {
	match (date, null_flavor) {
		(Some(value), _) => {
			if has_duration {
				out.push_str("<comp xsi:type=\"IVL_TS\" operator=\"A\"><");
				out.push_str(tag);
				out.push_str(" value=\"");
				out.push_str(&fmt_date(value));
				out.push_str("\"/></comp>");
			} else {
				out.push('<');
				out.push_str(tag);
				out.push_str(" value=\"");
				out.push_str(&fmt_date(value));
				out.push_str("\"/>");
			}
		}
		(None, Some(null_flavor)) => {
			if has_duration {
				out.push_str("<comp xsi:type=\"IVL_TS\" operator=\"A\"><");
				out.push_str(tag);
				out.push_str(" nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push_str("\"/></comp>");
			} else {
				out.push('<');
				out.push_str(tag);
				out.push_str(" nullFlavor=\"");
				out.push_str(&xml_escape(null_flavor));
				out.push_str("\"/>");
			}
		}
		(None, None) => {}
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
