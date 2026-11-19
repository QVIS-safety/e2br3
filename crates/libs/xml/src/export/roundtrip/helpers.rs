use super::*;

fn has_nodes(xpath: &mut Context, path: &str) -> Result<bool> {
	xpath
		.findnodes(path, None)
		.map(|nodes| !nodes.is_empty())
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to evaluate XPath: {path}"),
			line: None,
			column: None,
		})
}

pub(super) fn ensure_investigation_id(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	root: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='{root}']"
	);
	if has_nodes(xpath, &path)? {
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent",
		&format!("<id root=\"{root}\"/>"),
	)
}

pub(super) fn ensure_primary_role(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if has_nodes(xpath, "//hl7:primaryRole/hl7:player1")? {
		return Ok(());
	}
	let fragment = "\
		<subject1 typeCode=\"SBJ\">\
			<primaryRole classCode=\"INVSBJ\">\
				<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\">\
					<name/>\
					<administrativeGenderCode codeSystem=\"1.0.5218\"/>\
					<birthTime/>\
				</player1>\
			</primaryRole>\
		</subject1>";
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:adverseEventAssessment",
		fragment,
	)
}

pub(super) fn ensure_subject_observation(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:subjectOf2/hl7:observation[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if has_nodes(xpath, &path)? {
		return Ok(());
	}
	let fragment = format!(
		"<subjectOf2 typeCode=\"SBJ\">\
			<observation classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"{value_type}\"/>\
			</observation>\
		</subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:primaryRole", &fragment)
}

pub(super) fn ensure_control_act_effective_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if has_nodes(xpath, "//hl7:controlActProcess/hl7:effectiveTime")? {
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:controlActProcess",
		"<effectiveTime/>",
	)
}

pub(super) fn ensure_investigation_effective_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if has_nodes(
		xpath,
		"//hl7:investigationEvent/hl7:effectiveTime/hl7:low",
	)? {
		return Ok(());
	}
	if has_nodes(xpath, "//hl7:investigationEvent/hl7:effectiveTime")? {
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent/hl7:effectiveTime",
			"<low/>",
		)
	} else {
		append_fragment_child(
			doc,
			parser,
			xpath,
			"//hl7:investigationEvent",
			"<effectiveTime><low/></effectiveTime>",
		)
	}
}

pub(super) fn ensure_investigation_availability_time(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if has_nodes(xpath, "//hl7:investigationEvent/hl7:availabilityTime")? {
		return Ok(());
	}
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:investigationEvent",
		"<availabilityTime/>",
	)
}

pub(super) fn ensure_investigation_text(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
) -> Result<()> {
	if has_nodes(xpath, "//hl7:investigationEvent/hl7:text")? {
		return Ok(());
	}
	append_fragment_child(doc, parser, xpath, "//hl7:investigationEvent", "<text/>")
}

pub(super) fn ensure_investigation_characteristic(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_code_system: Option<&str>,
) -> Result<()> {
	let path = format!(
		"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if has_nodes(xpath, &path)? {
		return Ok(());
	}
	let value_cs = match value_code_system {
		Some(cs) => format!(" codeSystem=\"{cs}\""),
		None => String::new(),
	};
	let fragment = format!(
		"<subjectOf2 typeCode=\"SUBJ\">\
			<investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"CE\"{value_cs}><originalText/></value>\
			</investigationCharacteristic>\
		</subjectOf2>"
	);
	append_fragment_child(doc, parser, xpath, "//hl7:investigationEvent", &fragment)
}

pub(super) fn ensure_observation_event_component(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	code: &str,
	code_system: &str,
	value_type: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:component/hl7:observationEvent[hl7:code[@code='{code}' and @codeSystem='{code_system}']]"
	);
	if has_nodes(xpath, &path)? {
		return Ok(());
	}
	let fragment = format!(
		"<component typeCode=\"COMP\">\
			<observationEvent classCode=\"OBS\" moodCode=\"EVN\">\
				<code code=\"{code}\" codeSystem=\"{code_system}\"/>\
				<value xsi:type=\"{value_type}\"/>\
			</observationEvent>\
		</component>"
	);
	append_fragment_child(
		doc,
		parser,
		xpath,
		"//hl7:investigationEvent",
		&fragment,
	)?;
	reorder_investigation_event_children(xpath);
	Ok(())
}

pub(crate) fn reorder_investigation_event_children(xpath: &mut Context) {
	// InvestigationEvent schema order ends with outboundRelationship, subjectOf1, subjectOf2.
	for name in ["outboundRelationship", "subjectOf1", "subjectOf2"] {
		if let Ok(nodes) =
			xpath.findnodes(&format!("//hl7:investigationEvent/hl7:{name}"), None)
		{
			for mut node in nodes {
				if let Some(mut parent) = node.get_parent() {
					node.unlink_node();
					let _ = parent.add_child(&mut node);
				}
			}
		}
	}

	// C.5 subjectOf1 must precede the D/G subjectOf2 children in primaryRole.
	if let Ok(mut roles) = xpath.findnodes(
		"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:subject1/hl7:primaryRole",
		None,
	) {
		for mut role in roles.drain(..) {
			for name in ["subjectOf1", "subjectOf2"] {
				if let Ok(nodes) =
					xpath.findnodes(&format!("./hl7:{name}"), Some(&role))
				{
					for mut node in nodes {
						node.unlink_node();
						let _ = role.add_child(&mut node);
					}
				}
			}
		}
	}
}

pub(crate) fn reorder_patient_player_children(xpath: &mut Context) {
	// Patient schema tail: D.9.1, address, race, identifiers, parent role.
	for name in [
		"deceasedTime",
		"addr",
		"raceCode",
		"asIdentifiedEntity",
		"role",
	] {
		if let Ok(nodes) = xpath
			.findnodes(&format!("//hl7:primaryRole/hl7:player1/hl7:{name}"), None)
		{
			for mut node in nodes {
				if let Some(mut parent) = node.get_parent() {
					node.unlink_node();
					let _ = parent.add_child(&mut node);
				}
			}
		}
	}
}

pub(super) fn clear_null_flavor_if_export_policy(
	xpath: &mut Context,
	rule_code: &str,
	path: &str,
) {
	if should_clear_null_flavor_on_value(rule_code) {
		remove_attr_first(xpath, path, "nullFlavor");
	}
}

pub(super) fn fmt_date(date: Date) -> String {
	let year = date.year();
	let month: u8 = date.month().into();
	let day = date.day();
	format!("{year:04}{month:02}{day:02}")
}

pub(super) fn normalize_bl_value(value: &str) -> Option<&'static str> {
	match value.trim().to_ascii_lowercase().as_str() {
		"true" | "1" | "y" | "yes" => Some("true"),
		"false" | "0" | "2" | "n" | "no" => Some("false"),
		_ => None,
	}
}
