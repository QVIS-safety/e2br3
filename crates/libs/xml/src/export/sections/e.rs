use super::*;
use lib_core::model::reaction::ReactionBmc;

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	let reactions = ReactionBmc::list_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	crate::export::roundtrip::patch_e_reactions_for_authority(
		raw_xml, &reactions, authority,
	)
}

pub fn export_e_reactions_xml(reactions: &[Reaction]) -> Result<String> {
	export_e_reactions_xml_for_authority(
		reactions,
		lib_core::regulatory::RegulatoryAuthority::Ich,
	)
}

pub fn export_e_reactions_xml_for_authority(
	reactions: &[Reaction],
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
	let mut ordered: Vec<&Reaction> = reactions.iter().collect();
	ordered.sort_by_key(|reaction| reaction.sequence_number);

	let mut reactions_xml = String::new();
	for reaction in ordered {
		reactions_xml.push_str(&write_e_i_reaction(reaction, authority)?);
	}
	let xml = base_e_reaction_skeleton().replace("{REACTIONS}", &reactions_xml);
	Ok(xml)
}

pub(crate) fn write_e_i_reaction(
	reaction: &Reaction,
	authority: lib_core::regulatory::RegulatoryAuthority,
) -> Result<String> {
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
		let has_low = reaction.start_date.is_some()
			|| reaction.start_date_null_flavor.is_some();
		let has_high =
			reaction.end_date.is_some() || reaction.end_date_null_flavor.is_some();
		let use_sxpr = has_duration && has_low && has_high;
		if use_sxpr {
			out.push_str("<effectiveTime xsi:type=\"SXPR_TS\">");
			out.push_str("<comp xsi:type=\"IVL_TS\">");
		} else {
			out.push_str("<effectiveTime xsi:type=\"IVL_TS\">");
		}
		if has_duration && !has_low && has_high {
			write_e_i_6_width(&mut out, reaction);
		}
		write_e_i_4(
			&mut out,
			"low",
			reaction.start_date.as_deref(),
			reaction.start_date_null_flavor.as_deref(),
		);
		if !use_sxpr && has_duration && has_low {
			write_e_i_6_width(&mut out, reaction);
		}
		write_e_i_5(
			&mut out,
			"high",
			reaction.end_date.as_deref(),
			reaction.end_date_null_flavor.as_deref(),
		);
		if use_sxpr {
			out.push_str("</comp><comp xsi:type=\"IVL_TS\" operator=\"A\">");
			write_e_i_6_width(&mut out, reaction);
			out.push_str("</comp>");
		} else if has_duration && !has_low && !has_high {
			write_e_i_6_width(&mut out, reaction);
		}
		out.push_str("</effectiveTime>");
	}
	out.push_str("<value xsi:type=\"CE\"");
	if let Some(meddracode) = write_e_i_2_1b(reaction) {
		out.push_str(" code=\"");
		out.push_str(&xml_escape(meddracode));
		out.push_str("\" codeSystem=\"2.16.840.1.113883.6.163\"");
		if let Some(version) = write_e_i_2_1a(reaction) {
			out.push_str(" codeSystemVersion=\"");
			out.push_str(&xml_escape(version));
			out.push('"');
		}
	} else {
		out.push_str(" nullFlavor=\"NI\"");
	}
	if let Some(text) = write_e_i_1_1a(reaction) {
		out.push_str("><originalText");
		if let Some(lang) = write_e_i_1_1b(reaction) {
			out.push_str(" language=\"");
			out.push_str(&xml_escape(lang));
			out.push('"');
		}
		out.push('>');
		out.push_str(&xml_escape(text));
		out.push_str("</originalText></value>");
	} else {
		out.push_str("/>");
	}
	if let Some(country) = write_e_i_9(reaction) {
		let country = country.trim();
		if !country.is_empty() {
			out.push_str("<location typeCode=\"LOC\"><locatedEntity classCode=\"LOCE\"><locatedPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"");
			out.push_str(&xml_escape(country));
			out.push_str("\" codeSystem=\"1.0.3166.1.2.2\"/></locatedPlace></locatedEntity></location>");
		}
	}
	out.push_str(&write_e_i_1_2(reaction));
	out.push_str(&write_e_i_3_1(reaction));
	out.push_str(&write_e_i_3_2a(
		"34",
		reaction.criteria_death,
		reaction.criteria_death_null_flavor.as_deref(),
	));
	out.push_str(&write_e_i_3_2b(
		"21",
		reaction.criteria_life_threatening,
		reaction.criteria_life_threatening_null_flavor.as_deref(),
	));
	out.push_str(&write_e_i_3_2c(
		"33",
		reaction.criteria_hospitalization,
		reaction.criteria_hospitalization_null_flavor.as_deref(),
	));
	out.push_str(&write_e_i_3_2d(
		"35",
		reaction.criteria_disabling,
		reaction.criteria_disabling_null_flavor.as_deref(),
	));
	out.push_str(&write_e_i_3_2e(
		"12",
		reaction.criteria_congenital_anomaly,
		reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
	));
	out.push_str(&write_e_i_3_2f(
		"26",
		reaction.criteria_other_medically_important,
		reaction
			.criteria_other_medically_important_null_flavor
			.as_deref(),
	));
	if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
		out.push_str(&write_fda_e_i_3_2h(
			reaction.required_intervention,
			reaction.required_intervention_null_flavor.as_deref(),
		));
	}
	append_extension_code(
		&mut out,
		"AE_EXPECTEDNESS",
		reaction.expectedness.as_deref(),
	);
	append_extension_code(&mut out, "AE_SEVERITY", reaction.severity.as_deref());
	out.push_str(&write_e_i_7(reaction.outcome.as_deref()));
	out.push_str(&write_e_i_8(reaction));
	out.push_str("</observation></subjectOf2>");
	Ok(out)
}

/// e2b:E.i.1.1a
fn write_e_i_1_1a(value: &Reaction) -> Option<&str> {
	value
		.primary_source_reaction
		.as_deref()
		.filter(|text| !text.trim().is_empty())
}

/// e2b:E.i.1.1b
fn write_e_i_1_1b(value: &Reaction) -> Option<&str> {
	value.reaction_language.as_deref()
}

/// e2b:E.i.1.2
fn write_e_i_1_2(value: &Reaction) -> String {
	let Some(text) = value
		.primary_source_reaction_translation
		.as_deref()
		.filter(|text| !text.trim().is_empty())
	else {
		return String::new();
	};
	let mut out = String::new();
	out.push_str("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"30\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"ED\"");
	if let Some(language) = value.reaction_language.as_deref() {
		out.push_str(" language=\"");
		out.push_str(&xml_escape(language));
		out.push('"');
	}
	out.push('>');
	out.push_str(&xml_escape(text));
	out.push_str("</value></observation></outboundRelationship2>");
	out
}

/// e2b:E.i.2.1a
fn write_e_i_2_1a(value: &Reaction) -> Option<&str> {
	value.reaction_meddra_version.as_deref()
}

/// e2b:E.i.2.1b
fn write_e_i_2_1b(value: &Reaction) -> Option<&str> {
	value
		.reaction_meddra_code
		.as_deref()
		.map(str::trim)
		.filter(|code| !code.is_empty())
}

/// e2b:E.i.3.1
fn write_e_i_3_1(value: &Reaction) -> String {
	let Some(term_code) = value.term_highlighted.as_deref() else {
		return String::new();
	};
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"37\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.10\"/></observation></outboundRelationship2>",
		xml_escape(term_code)
	)
}

/// e2b:E.i.3.2a
fn write_e_i_3_2a(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2b
fn write_e_i_3_2b(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2c
fn write_e_i_3_2c(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2d
fn write_e_i_3_2d(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2e
fn write_e_i_3_2e(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2f
fn write_e_i_3_2f(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:FDA.E.i.3.2h
fn write_fda_e_i_3_2h(value: Option<bool>, null_flavor: Option<&str>) -> String {
	if let Some(value) = value {
		let value = if value { "true" } else { "false" };
		return format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\" value=\"{value}\"/></observation></outboundRelationship2>"
		);
	}
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\" nullFlavor=\"{}\"/></observation></outboundRelationship2>",
		xml_escape(null_flavor.unwrap_or("NI"))
	)
}

/// e2b:E.i.4
fn write_e_i_4(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) {
	write_e_i_4_or_5(out, tag, date, null_flavor);
}

/// e2b:E.i.5
fn write_e_i_5(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) {
	write_e_i_4_or_5(out, tag, date, null_flavor);
}

/// e2b:E.i.6a
fn write_e_i_6a(value: &Reaction) -> Option<&str> {
	value.duration_value.as_deref()
}

/// e2b:E.i.6b
fn write_e_i_6b(value: &Reaction) -> Option<&'static str> {
	let unit = value.duration_unit.as_deref()?;
	crate::mapping::fda::e_reaction::reaction_duration_unit_to_ucum(unit)
}

fn write_e_i_6_width(out: &mut String, reaction: &Reaction) {
	let Some(width) = write_e_i_6a(reaction) else {
		return;
	};
	out.push_str("<width value=\"");
	out.push_str(&xml_escape(width));
	out.push('"');
	if let Some(unit) = write_e_i_6b(reaction) {
		out.push_str(" unit=\"");
		out.push_str(&xml_escape(unit));
		out.push('"');
	}
	out.push_str("/>");
}

/// e2b:E.i.7
fn write_e_i_7(value: Option<&str>) -> String {
	let Some(code) = value.map(str::trim).filter(|code| !code.is_empty()) else {
		return String::new();
	};
	let display_name = match code {
		"0" => Some("unknown"),
		"1" => Some("recovered/resolved"),
		"2" => Some("recovering/resolving"),
		"3" => Some("not recovered/not resolved/ongoing"),
		"4" => Some("recovered/resolved with sequelae"),
		"5" => Some("fatal"),
		_ => None,
	};
	let display_name = display_name
		.map(|name| format!(" displayName=\"{}\"", xml_escape(name)))
		.unwrap_or_default();
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"27\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"{}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.11\"{display_name}/></observation></outboundRelationship2>",
		xml_escape(code)
	)
}

/// e2b:E.i.8
fn write_e_i_8(value: &Reaction) -> String {
	let Some(value) = value.medical_confirmation else {
		return String::new();
	};
	let value = if value { "true" } else { "false" };
	format!(
		"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"24\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{value}\"/></observation></outboundRelationship2>"
	)
}

/// e2b:E.i.9
fn write_e_i_9(value: &Reaction) -> Option<&str> {
	value.country_code.as_deref()
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

fn write_e_i_3_2(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> String {
	let value = value.filter(|value| *value);
	match (value, null_flavor) {
		(Some(value), None) => format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" value=\"{value}\"/></observation></outboundRelationship2>"
		),
		(None, null_flavor) => format!(
			"<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"{}\"/></observation></outboundRelationship2>",
			xml_escape(null_flavor.unwrap_or("NI"))
		),
		(Some(_), Some(_)) => unreachable!(
			"database invariant forbids a reaction criterion value and NullFlavor together"
		),
	}
}

fn write_e_i_4_or_5(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) {
	match (date, null_flavor) {
		(Some(value), _) => {
			out.push('<');
			out.push_str(tag);
			out.push_str(" value=\"");
			out.push_str(&xml_escape(&fmt_date_lexeme(value)));
			out.push_str("\"/>");
		}
		(None, Some(null_flavor)) => {
			out.push('<');
			out.push_str(tag);
			out.push_str(" nullFlavor=\"");
			out.push_str(&xml_escape(null_flavor));
			out.push_str("\"/>");
		}
		(None, None) => {}
	}
}

#[cfg(test)]
mod split_null_flavor_tests {
	use super::{write_e_i_7, write_fda_e_i_3_2h};

	#[test]
	fn exports_unknown_reaction_outcome_as_code_zero() {
		let xml = write_e_i_7(Some("0"));
		assert!(xml.contains("code=\"0\""));
		assert!(xml.contains("displayName=\"unknown\""));
		assert!(!xml.contains("nullFlavor=\"NI\""));
	}

	#[test]
	fn preserves_invalid_reaction_outcome_for_non_blocking_export() {
		assert!(write_e_i_7(None).is_empty());
		assert!(write_e_i_7(Some("")).is_empty());
		assert!(write_e_i_7(Some("99")).contains("code=\"99\""));
	}

	#[test]
	fn exports_required_intervention_companion_as_boolean_null_flavor() {
		let xml = write_fda_e_i_3_2h(None, Some("NI"));
		assert!(xml.contains("xsi:type=\"BL\" nullFlavor=\"NI\""));
		assert!(!xml.contains(" value=\""));
	}
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

#[cfg(test)]
mod registry_coverage_tests {
	use std::collections::BTreeSet;

	#[test]
	fn section_e_writers_cover_registry_fields() {
		let registry: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../../../registry/sections/e-reaction.json"
		))
		.expect("section E registry");
		let expected = registry
			.as_array()
			.expect("registry array")
			.iter()
			.filter(|entry| entry["local_only"] != true)
			.filter_map(|entry| entry["e2br3_code"].as_str())
			.collect::<BTreeSet<_>>();
		let implemented = include_str!("e.rs")
			.lines()
			.filter_map(|line| line.trim().strip_prefix("/// e2b:"))
			.collect::<BTreeSet<_>>();

		assert_eq!(implemented, expected);
	}
}

#[cfg(test)]
mod meddra_requirement_tests {
	use super::*;
	use sqlx::types::Uuid;
	use time::OffsetDateTime;

	fn reaction() -> Reaction {
		Reaction {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			sequence_number: 1,
			primary_source_reaction: Some("Headache".to_string()),
			primary_source_reaction_translation: None,
			reaction_language: Some("en".to_string()),
			reaction_meddra_version: Some("24.1".to_string()),
			reaction_meddra_code: Some("10019211".to_string()),
			term_highlighted: Some("4".to_string()),
			serious: Some(false),
			criteria_death: Some(false),
			criteria_death_null_flavor: None,
			criteria_life_threatening: Some(false),
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: Some(false),
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: Some(false),
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: Some(false),
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: Some(false),
			criteria_other_medically_important_null_flavor: None,
			required_intervention: None,
			required_intervention_null_flavor: None,
			expectedness: None,
			severity: None,
			mfds_device_ae_classification: None,
			mfds_device_ae_outcome: None,
			mfds_device_cause_medical_device: None,
			mfds_device_cause_procedure_issue: None,
			mfds_device_cause_patient_condition: None,
			mfds_device_cause_unable_to_assess: None,
			mfds_device_cause_other: None,
			mfds_device_action_reason: None,
			mfds_device_action_recall: None,
			mfds_device_action_repair: None,
			mfds_device_action_inspection: None,
			mfds_device_action_replacement: None,
			mfds_device_action_improvement: None,
			mfds_device_action_monitoring: None,
			mfds_device_action_notification: None,
			mfds_device_action_label_change: None,
			mfds_device_action_other: None,
			start_date: None,
			start_date_null_flavor: None,
			end_date: None,
			end_date_null_flavor: None,
			duration_value: None,
			duration_unit: None,
			outcome: Some("1".to_string()),
			medical_confirmation: Some(true),
			country_code: Some("US".to_string()),
			deleted: false,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn export_uses_ni_for_missing_or_blank_meddra_code() {
		for code in [None, Some("  ".to_string())] {
			let mut reaction = reaction();
			reaction.reaction_meddra_code = code;

			let xml = export_e_reactions_xml(&[reaction])
				.expect("semantic MedDRA issue must not block export");
			assert!(xml.contains("<value xsi:type=\"CE\" nullFlavor=\"NI\""));
		}
	}

	#[test]
	fn export_omits_absent_reported_text_without_inventing_null_flavor() {
		let mut reaction = reaction();
		reaction.primary_source_reaction = None;
		reaction.primary_source_reaction_translation = None;
		reaction.reaction_language = None;

		let xml = export_e_reactions_xml(&[reaction]).expect("reaction XML");
		assert!(!xml.contains("<originalText"));
		assert!(!xml.contains("<code code=\"30\""));
		assert!(!xml.contains("<value xsi:type=\"CE\" nullFlavor="));
		assert!(xml.contains("code=\"10019211\""));
		assert!(xml.contains("codeSystemVersion=\"24.1\""));
	}

	#[test]
	fn export_does_not_invent_fixed_code_system_versions() {
		let mut reaction = reaction();
		reaction.primary_source_reaction_translation = Some("Headache".to_string());
		let xml = export_e_reactions_xml(&[reaction]).expect("reaction XML");

		assert!(xml.contains(
			"<code code=\"30\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>"
		));
		assert!(xml.contains(
			"<code code=\"37\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>"
		));
		assert!(!xml.contains("code=\"30\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion="));
		assert!(!xml.contains("code=\"37\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\" codeSystemVersion="));
		assert!(xml.contains("codeSystemVersion=\"24.1\""));
	}

	#[test]
	fn exports_location_before_xsd_outbound_relationships() {
		let reaction = reaction();
		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("reaction XML");
		let value = xml.find("</originalText></value>").expect("reaction value");
		let location = xml
			.find("<location typeCode=\"LOC\"><locatedEntity classCode=\"LOCE\"><locatedPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"US\" codeSystem=\"1.0.3166.1.2.2\"/></locatedPlace></locatedEntity></location>")
			.expect("official E.i.9 location structure");
		let relationship = xml
			.find("<outboundRelationship2")
			.expect("reaction relationships");

		assert!(value < location && location < relationship);
		assert!(!xml.contains("<value xsi:type=\"BL\" value=\"false\"/>"));
		for code in ["34", "21", "33", "35", "12", "26"] {
			assert!(xml.contains(&format!(
				"<code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\" nullFlavor=\"NI\"/>"
			)));
		}

		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source =
			std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
				.expect("official FDA example");
		let exported = crate::export::roundtrip::patch_e_reactions(
			&source,
			std::slice::from_ref(&reaction),
		)
		.expect("patch official FDA example");
		let schema = crate::default_xsd_path().expect("official ICH schema");
		let errors =
			crate::validation::validate_e2b_xml_xsd(exported.as_bytes(), &schema)
				.expect("validate XSD");
		assert!(errors.is_empty(), "{errors:#?}");
	}

	#[test]
	fn exports_internal_duration_codes_as_ucum_and_omits_unknown_units() {
		let mut reaction = reaction();
		reaction.duration_value = Some("1".to_string());
		for (stored, ucum) in [
			("800", "10.a"),
			("801", "a"),
			("802", "mo"),
			("803", "wk"),
			("804", "d"),
			("805", "h"),
		] {
			reaction.duration_unit = Some(stored.to_string());
			let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
				.expect("supported reaction duration unit");
			assert!(xml.contains(&format!("<width value=\"1\" unit=\"{ucum}\"/>")));
		}

		reaction.duration_unit = Some("d".to_string());
		let xml = export_e_reactions_xml(&[reaction])
			.expect("semantic unit issue must not block export");
		assert!(xml.contains("<width value=\"1\"/>"));
		assert!(!xml.contains("unit=\"d\""));
	}

	#[test]
	fn exports_original_duration_value_lexeme_when_present() {
		let mut reaction = reaction();
		reaction.duration_value = Some("54.00".to_string());
		reaction.duration_unit = Some("804".to_string());

		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("duration export");
		assert!(xml.contains("<width value=\"54.00\" unit=\"d\"/>"));
	}

	#[test]
	fn exports_e_i_6_in_the_official_effective_time_shapes() {
		let mut reaction = reaction();
		reaction.duration_value = Some("24".to_string());
		reaction.duration_unit = Some("805".to_string());
		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("width-only export");
		assert!(xml.contains(
			"<effectiveTime xsi:type=\"IVL_TS\"><width value=\"24\" unit=\"h\"/></effectiveTime>"
		));

		reaction.start_date = Some("20030511".to_string());
		reaction.duration_value = Some("1.00".to_string());
		reaction.duration_unit = Some("803".to_string());
		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("low-and-width export");
		assert!(xml.contains(
			"<effectiveTime xsi:type=\"IVL_TS\"><low value=\"20030511\"/><width value=\"1.00\" unit=\"wk\"/></effectiveTime>"
		));

		reaction.start_date = None;
		reaction.end_date = Some("20030518".to_string());
		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("width-and-high export");
		assert!(xml.contains(
			"<effectiveTime xsi:type=\"IVL_TS\"><width value=\"1.00\" unit=\"wk\"/><high value=\"20030518\"/></effectiveTime>"
		));

		reaction.start_date = Some("20030511".to_string());
		reaction.end_date = Some("20030518".to_string());
		reaction.duration_value = Some("54".to_string());
		reaction.duration_unit = Some("804".to_string());
		let xml = export_e_reactions_xml(std::slice::from_ref(&reaction))
			.expect("SXPR export");
		assert!(xml.contains(
			"<effectiveTime xsi:type=\"SXPR_TS\"><comp xsi:type=\"IVL_TS\"><low value=\"20030511\"/><high value=\"20030518\"/></comp><comp xsi:type=\"IVL_TS\" operator=\"A\"><width value=\"54\" unit=\"d\"/></comp></effectiveTime>"
		));
	}

	#[test]
	fn exported_e_i_6_shapes_pass_the_official_ich_xsd() {
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.and_then(|path| path.parent())
			.and_then(|path| path.parent())
			.expect("workspace root")
			.to_path_buf();
		let source =
			std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
				.expect("official FDA example");
		let schema = crate::default_xsd_path().expect("official ICH schema");

		for (start, end, duration, unit) in [
			(None, None, "24", "805"),
			(Some("20030511"), None, "1.00", "803"),
			(None, Some("20030518"), "1.00", "803"),
			(Some("20030511"), Some("20030518"), "54", "804"),
		] {
			let mut reaction = reaction();
			reaction.start_date = start.map(str::to_string);
			reaction.end_date = end.map(str::to_string);
			reaction.duration_value = Some(duration.to_string());
			reaction.duration_unit = Some(unit.to_string());
			let exported = crate::export::roundtrip::patch_e_reactions(
				&source,
				std::slice::from_ref(&reaction),
			)
			.expect("patch official FDA example");
			let errors = crate::validation::validate_e2b_xml_xsd(
				exported.as_bytes(),
				&schema,
			)
			.expect("validate XSD");
			assert!(errors.is_empty(), "{errors:#?}");
		}
	}
}
