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
		|| reaction.duration_unit.is_some()
	{
		let has_duration =
			reaction.duration_value.is_some() || reaction.duration_unit.is_some();
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
		)?;
		if !use_sxpr && has_duration && has_low {
			write_e_i_6_width(&mut out, reaction);
		}
		write_e_i_5(
			&mut out,
			"high",
			reaction.end_date.as_deref(),
			reaction.end_date_null_flavor.as_deref(),
		)?;
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
	let code = write_e_i_2_1b(reaction);
	let version = write_e_i_2_1a(reaction);
	if let Some(meddracode) = code {
		out.push_str(" code=\"");
		out.push_str(&xml_escape(meddracode));
		out.push('"');
	}
	if code.is_some() || version.is_some() {
		out.push_str(" codeSystem=\"2.16.840.1.113883.6.163\"");
	}
	if let Some(version) = version {
		out.push_str(" codeSystemVersion=\"");
		out.push_str(&xml_escape(version));
		out.push('"');
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
		out.push_str("<location typeCode=\"LOC\"><locatedEntity classCode=\"LOCE\"><locatedPlace classCode=\"COUNTRY\" determinerCode=\"INSTANCE\"><code code=\"");
		out.push_str(&xml_escape(country));
		out.push_str("\" codeSystem=\"1.0.3166.1.2.2\"/></locatedPlace></locatedEntity></location>");
	}
	out.push_str(&write_e_i_1_2(reaction));
	out.push_str(&write_e_i_3_1(reaction));
	out.push_str(&write_e_i_3_2a(
		"34",
		reaction.criteria_death,
		reaction.criteria_death_null_flavor.as_deref(),
	)?);
	out.push_str(&write_e_i_3_2b(
		"21",
		reaction.criteria_life_threatening,
		reaction.criteria_life_threatening_null_flavor.as_deref(),
	)?);
	out.push_str(&write_e_i_3_2c(
		"33",
		reaction.criteria_hospitalization,
		reaction.criteria_hospitalization_null_flavor.as_deref(),
	)?);
	out.push_str(&write_e_i_3_2d(
		"35",
		reaction.criteria_disabling,
		reaction.criteria_disabling_null_flavor.as_deref(),
	)?);
	out.push_str(&write_e_i_3_2e(
		"12",
		reaction.criteria_congenital_anomaly,
		reaction.criteria_congenital_anomaly_null_flavor.as_deref(),
	)?);
	out.push_str(&write_e_i_3_2f(
		"26",
		reaction.criteria_other_medically_important,
		reaction
			.criteria_other_medically_important_null_flavor
			.as_deref(),
	)?);
	if matches!(authority, lib_core::regulatory::RegulatoryAuthority::Fda) {
		out.push_str(&write_fda_e_i_3_2h(
			reaction.required_intervention,
			reaction.required_intervention_null_flavor.as_deref(),
		)?);
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
	value.primary_source_reaction.as_deref()
}

/// e2b:E.i.1.1b
fn write_e_i_1_1b(value: &Reaction) -> Option<&str> {
	value.reaction_language.as_deref()
}

/// e2b:E.i.1.2
fn write_e_i_1_2(value: &Reaction) -> String {
	let Some(text) = value.primary_source_reaction_translation.as_deref() else {
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
	value.reaction_meddra_code.as_deref()
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
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2b
fn write_e_i_3_2b(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2c
fn write_e_i_3_2c(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2d
fn write_e_i_3_2d(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2e
fn write_e_i_3_2e(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:E.i.3.2f
fn write_e_i_3_2f(
	code: &str,
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	write_e_i_3_2(code, value, null_flavor)
}

/// e2b:FDA.E.i.3.2h
fn write_fda_e_i_3_2h(
	value: Option<bool>,
	null_flavor: Option<&str>,
) -> Result<String> {
	let attrs =
		field::attributes("write_fda_e_i_3_2h", value, null_flavor, "value")?;
	if attrs.is_empty() {
		return Ok(String::new());
	}
	Ok(format!("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"7\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/><value xsi:type=\"BL\"{attrs}/></observation></outboundRelationship2>"))
}

/// e2b:E.i.4
fn write_e_i_4(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) -> Result<()> {
	write_e_i_4_or_5(out, tag, date, null_flavor)
}

/// e2b:E.i.5
fn write_e_i_5(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) -> Result<()> {
	write_e_i_4_or_5(out, tag, date, null_flavor)
}

/// e2b:E.i.6a
fn write_e_i_6a(value: &Reaction) -> Option<&str> {
	value.duration_value.as_deref()
}

/// e2b:E.i.6b
fn write_e_i_6b(value: &Reaction) -> Option<&str> {
	let unit = value.duration_unit.as_deref()?;
	Some(
		match crate::mapping::fda::e_reaction::reaction_duration_unit_to_ucum(unit) {
			Some(mapped) => mapped,
			// Draft export preserves unknown stored units; submission validates vocabulary.
			None => unit,
		},
	)
}

fn write_e_i_6_width(out: &mut String, reaction: &Reaction) {
	out.push_str("<width");
	if let Some(width) = write_e_i_6a(reaction) {
		out.push_str(" value=\"");
		out.push_str(&xml_escape(width));
		out.push('"');
	}
	if let Some(unit) = write_e_i_6b(reaction) {
		out.push_str(" unit=\"");
		out.push_str(&xml_escape(unit));
		out.push('"');
	}
	out.push_str("/>");
}

/// e2b:E.i.7
fn write_e_i_7(value: Option<&str>) -> String {
	let Some(code) = value else {
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
	let display_name = match display_name {
		Some(name) => format!(" displayName=\"{}\"", xml_escape(name)),
		None => String::new(),
	};
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
	let Some(value) = value else {
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
) -> Result<String> {
	let attrs = field::attributes("write_e_i_3_2", value, null_flavor, "value")?;
	if attrs.is_empty() {
		return Ok(String::new());
	}
	Ok(format!("<outboundRelationship2 typeCode=\"PERT\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"{code}\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"BL\"{attrs}/></observation></outboundRelationship2>"))
}

fn write_e_i_4_or_5(
	out: &mut String,
	tag: &str,
	date: Option<&str>,
	null_flavor: Option<&str>,
) -> Result<()> {
	out.push_str(&field::element(
		"E.i.4/5",
		date.map(fmt_date_lexeme),
		null_flavor,
		tag,
		"value",
		"",
	)?);
	Ok(())
}

#[cfg(test)]
mod split_null_flavor_tests {
	use super::{write_e_i_3_2, write_e_i_7, write_fda_e_i_3_2h};

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
		assert!(write_e_i_7(Some("")).contains("code=\"\""));
		assert!(write_e_i_7(Some("99")).contains("code=\"99\""));
	}

	#[test]
	fn exports_required_intervention_companion_as_boolean_null_flavor() {
		let xml = write_fda_e_i_3_2h(None, Some("NI")).expect("serialize");
		assert!(xml.contains("xsi:type=\"BL\" nullFlavor=\"NI\""));
		assert!(!xml.contains(" value=\""));
	}

	#[test]
	fn reaction_null_flavors_are_raw_but_conflicts_are_errors() {
		assert!(write_e_i_3_2("1", Some(false), Some("UNK")).is_err());
		assert!(write_fda_e_i_3_2h(Some(false), Some("CUSTOM")).is_err());
		assert!(write_e_i_3_2("1", None, Some("CUSTOM"))
			.expect("raw NF")
			.contains("nullFlavor=\"CUSTOM\""));
	}

	#[test]
	fn does_not_invent_reaction_null_flavors() {
		assert!(write_e_i_3_2("1", None, None)
			.expect("serialize")
			.is_empty());
		assert!(write_fda_e_i_3_2h(None, None)
			.expect("serialize")
			.is_empty());
		assert!(write_e_i_3_2("1", Some(false), None)
			.expect("serialize")
			.contains("value=\"false\""));
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
			// These true-only markers use explicit NI when no criterion is provided.
			criteria_death: None,
			criteria_death_null_flavor: Some("NI".into()),
			criteria_life_threatening: None,
			criteria_life_threatening_null_flavor: Some("NI".into()),
			criteria_hospitalization: None,
			criteria_hospitalization_null_flavor: Some("NI".into()),
			criteria_disabling: None,
			criteria_disabling_null_flavor: Some("NI".into()),
			criteria_congenital_anomaly: None,
			criteria_congenital_anomaly_null_flavor: Some("NI".into()),
			criteria_other_medically_important: None,
			criteria_other_medically_important_null_flavor: Some("NI".into()),
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
	fn duration_export_preserves_missing_or_unsupported_unit() {
		for unit in [None, Some("bogus")] {
			let mut r = reaction();
			r.duration_value = Some("1".into());
			r.duration_unit = unit.map(str::to_string);
			let xml = export_e_reactions_xml(&[r]).expect("export draft duration");
			let imported = crate::import_sections::e_reaction::parse_e_reactions(
				xml.as_bytes(),
			)
			.expect("import draft duration");
			assert_eq!(imported[0].duration_value.as_deref(), Some("1"));
			assert_eq!(imported[0].duration_unit.as_deref(), unit);
		}
	}

	#[test]
	fn export_import_preserves_independent_meddra_code_and_version() {
		use lib_core::regulatory::RegulatoryAuthority;
		let baseline = reaction();
		for authority in [
			RegulatoryAuthority::Ich,
			RegulatoryAuthority::Fda,
			RegulatoryAuthority::Mfds,
		] {
			let initial = export_e_reactions_xml_for_authority(
				std::slice::from_ref(&baseline),
				authority,
			)
			.expect("initial XML");
			for code in [None, Some("10019211")] {
				for version in [Some("24.1"), None] {
					let mut current = baseline.clone();
					current.reaction_meddra_code = code.map(str::to_string);
					current.reaction_meddra_version = version.map(str::to_string);
					let fresh = export_e_reactions_xml_for_authority(
						std::slice::from_ref(&current),
						authority,
					)
					.expect("fresh draft XML");
					let patched =
						crate::export::roundtrip::patch_e_reactions_for_authority(
							initial.as_bytes(),
							std::slice::from_ref(&current),
							authority,
						)
						.expect("patched draft XML");
					for (mode, xml) in [("fresh", fresh), ("patched", patched)] {
						let parser = libxml::parser::Parser::default();
						let mut doc =
							parser.parse_string(xml.as_bytes()).expect("parse XML");
						let mut xpath =
							libxml::xpath::Context::new(&doc).expect("XPath");
						xpath
							.register_namespace("hl7", "urn:hl7-org:v3")
							.expect("namespace");
						crate::export::shared::patch_doc::postprocess_export_doc(
							&mut doc, &mut xpath,
						)
						.expect("postprocess XML");
						let nodes = xpath
							.findnodes(
								"//hl7:observation[hl7:code[@code='29']]/hl7:value",
								None,
							)
							.expect("reaction value");
						assert_eq!(nodes.len(), 1);
						assert_eq!([nodes[0].get_attribute("code"), nodes[0].get_attribute("codeSystemVersion"), nodes[0].get_attribute("nullFlavor")], [current.reaction_meddra_code.clone(), current.reaction_meddra_version.clone(), None], "{authority:?} {mode}: preserve independent MedDRA fields without invented NI");
						let imported =
							crate::import_sections::e_reaction::parse_e_reactions(
								doc.to_string().as_bytes(),
							)
							.expect("import draft reaction");
						assert_eq!(imported.len(), 1);
						assert_eq!(
							imported[0].reaction_meddra_code,
							current.reaction_meddra_code
						);
						assert_eq!(
							imported[0].reaction_meddra_version,
							current.reaction_meddra_version
						);
						assert_eq!(
							imported[0].primary_source_reaction,
							current.primary_source_reaction
						);
					}
				}
			}
		}
	}

	#[test]
	fn export_preserves_explicit_blank_meddra_code_without_inventing_ni() {
		for code in ["", "  "] {
			let mut current = reaction();
			current.reaction_meddra_code = Some(code.to_string());
			let xml = export_e_reactions_xml(&[current])
				.expect("export explicit blank draft value");
			let doc = libxml::parser::Parser::default()
				.parse_string(xml.as_bytes())
				.expect("parse XML");
			let mut xpath = libxml::xpath::Context::new(&doc).expect("XPath");
			xpath
				.register_namespace("hl7", "urn:hl7-org:v3")
				.expect("namespace");
			let nodes = xpath
				.findnodes("//hl7:observation[hl7:code[@code='29']]/hl7:value", None)
				.expect("reaction value");
			assert_eq!(nodes.len(), 1);
			assert_eq!(nodes[0].get_attribute("code").as_deref(), Some(code));
			assert_eq!(nodes[0].get_attribute("nullFlavor"), None);
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
		let mut reaction = reaction();
		// NI must be explicit; the exporter must not turn a saved false into NI.
		reaction.criteria_death = None;
		reaction.criteria_death_null_flavor = Some("NI".into());
		reaction.criteria_life_threatening = None;
		reaction.criteria_life_threatening_null_flavor = Some("NI".into());
		reaction.criteria_hospitalization = None;
		reaction.criteria_hospitalization_null_flavor = Some("NI".into());
		reaction.criteria_disabling = None;
		reaction.criteria_disabling_null_flavor = Some("NI".into());
		reaction.criteria_congenital_anomaly = None;
		reaction.criteria_congenital_anomaly_null_flavor = Some("NI".into());
		reaction.criteria_other_medically_important = None;
		reaction.criteria_other_medically_important_null_flavor = Some("NI".into());
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
	fn exports_internal_duration_codes_as_ucum() {
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
		let xml =
			export_e_reactions_xml(&[reaction]).expect("UCUM alias is supported");
		assert!(xml.contains("<width value=\"1\" unit=\"d\"/>"));
	}

	fn assert_draft_duration_roundtrip(value: Option<&str>, unit: Option<&str>) {
		use lib_core::regulatory::RegulatoryAuthority::{Fda, Ich, Mfds};
		let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../../docs/exporter/fda/FAERS2022Scenario1.xml");
		let source = std::fs::read(root).expect("existing FDA fixture");
		for authority in [Ich, Fda, Mfds] {
			for (start, end) in [
				(None, None),
				(Some("20030511"), None),
				(None, Some("20030518")),
				(Some("20030511"), Some("20030518")),
			] {
				let mut current = reaction();
				current.duration_value = value.map(str::to_string);
				current.duration_unit = unit.map(str::to_string);
				current.start_date = start.map(str::to_string);
				current.end_date = end.map(str::to_string);
				let fresh = export_e_reactions_xml_for_authority(
					std::slice::from_ref(&current),
					authority,
				)
				.expect("draft duration must not require submission validity");
				let patched =
					crate::export::roundtrip::patch_e_reactions_for_authority(
						&source,
						std::slice::from_ref(&current),
						authority,
					)
					.expect("patch draft duration");
				for (mode, xml) in [("fresh", fresh), ("patched", patched)] {
					let parser = libxml::parser::Parser::default();
					let mut doc = parser
						.parse_string(xml.as_bytes())
						.expect("parse exported XML");
					let mut xpath =
						libxml::xpath::Context::new(&doc).expect("XPath");
					xpath
						.register_namespace("hl7", "urn:hl7-org:v3")
						.expect("namespace");
					crate::export::shared::patch_doc::postprocess_export_doc(
						&mut doc, &mut xpath,
					)
					.expect("postprocess draft");
					let imported =
						crate::import_sections::e_reaction::parse_e_reactions(
							doc.to_string().as_bytes(),
						)
						.expect("import partial duration");
					assert_eq!(imported.len(), 1);
					assert_eq!(
						imported[0].duration_value.as_deref(),
						value,
						"{authority:?} {mode}"
					);
					assert_eq!(
						imported[0].duration_unit.as_deref(),
						unit,
						"{authority:?} {mode}"
					);
					assert_eq!(imported[0].start_date.as_deref(), start);
					assert_eq!(imported[0].end_date.as_deref(), end);
				}
			}
		}
	}

	#[test]
	fn draft_duration_value_without_unit_is_preserved() {
		assert_draft_duration_roundtrip(Some("1.00"), None);
	}

	#[test]
	fn draft_duration_unit_without_value_is_preserved() {
		assert_draft_duration_roundtrip(None, Some("d"));
	}

	#[test]
	fn draft_duration_unknown_unit_is_preserved() {
		assert_draft_duration_roundtrip(Some("1.00"), Some("fortnight"));
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
