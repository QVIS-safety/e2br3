use super::*;

pub(super) fn ensure_investigation_id(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	root: &str,
) -> Result<()> {
	let path = format!(
		"//hl7:controlActProcess/hl7:subject/hl7:investigationEvent/hl7:id[@root='{root}']"
	);
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes("//hl7:primaryRole/hl7:player1", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let fragment = "\
		<subject1 typeCode=\"SBJ\">\
			<primaryRole classCode=\"PAT\">\
				<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\">\
					<name/>\
					<administrativeGenderCode code=\"0\" codeSystem=\"1.0.5218\"/>\
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
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes("//hl7:controlActProcess/hl7:effectiveTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:effectiveTime/hl7:low", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:effectiveTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:availabilityTime", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes("//hl7:investigationEvent/hl7:text", None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
		return Ok(());
	}
	let value_cs = value_code_system
		.map(|cs| format!(" codeSystem=\"{cs}\""))
		.unwrap_or_default();
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
	if xpath
		.findnodes(&path, None)
		.map(|n| !n.is_empty())
		.unwrap_or(false)
	{
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

pub(super) fn reorder_investigation_event_children(xpath: &mut Context) {
	if let Ok(subject_nodes) =
		xpath.findnodes("//hl7:investigationEvent/hl7:subjectOf2", None)
	{
		for mut node in subject_nodes {
			if let Some(mut parent) = node.get_parent() {
				node.unlink_node();
				let _ = parent.add_child(&mut node);
			}
		}
	}
}

pub(super) fn clear_null_flavor_if_catalog_directive(
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
	format!("{:04}{:02}{:02}", year, month, day)
}

pub(super) fn fmt_date_time_fallback(date: Date) -> String {
	format!("{}000000", fmt_date(date))
}

pub(super) fn fmt_offset_datetime(dt: OffsetDateTime) -> String {
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}",
		dt.year(),
		u8::from(dt.month()),
		dt.day(),
		dt.hour(),
		dt.minute(),
		dt.second()
	)
}

pub(super) fn is_14_digit_datetime(value: &str) -> bool {
	value.len() == 14 && value.chars().all(|c| c.is_ascii_digit())
}

pub(super) fn normalize_bl_value(value: &str) -> Option<&'static str> {
	match value.trim().to_ascii_lowercase().as_str() {
		"true" | "1" | "y" | "yes" => Some("true"),
		"false" | "0" | "2" | "n" | "no" => Some("false"),
		_ => None,
	}
}
