use crate::export::policy::{
	export_attribute_strip_spec_for_rule, export_normalization_spec_for_rule,
	export_xpath_for_rule, export_xpaths_for_rule, has_export_policy_directive,
	ExportNormalizeKind, ExportPolicyDirective, EXPORT_NORMALIZE_INVALID_CODE_RULES,
	EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN,
	EXPORT_RULE_FDA_REQUIRED_INTERVENTION, EXPORT_RULE_GK11_EMPTY_PRUNE,
	EXPORT_RULE_OPTIONAL_PATH_EMPTY_PRUNE, EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE,
	EXPORT_RULE_RACE_EMPTY_PRUNE, EXPORT_RULE_RACE_NI_PRUNE,
	EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE, EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN,
};
use crate::export_utils::{remove_xsi_type, set_xsi_type};
use libxml::tree::{Document, Node, NodeType};
use libxml::xpath::Context;

pub(crate) fn postprocess_export_doc(
	doc: &mut Document,
	xpath: &mut Context,
) -> crate::Result<()> {
	normalize_export_values(xpath)?;
	prune_optional_nodes(doc, xpath);
	Ok(())
}

fn normalize_export_values(xpath: &mut Context) -> crate::Result<()> {
	for rule_code in EXPORT_NORMALIZE_INVALID_CODE_RULES {
		if !has_export_policy_directive(
			rule_code,
			ExportPolicyDirective::NormalizeInvalidCodeToNullFlavorNi,
		) {
			continue;
		}
		let Some(spec) = export_normalization_spec_for_rule(rule_code) else {
			continue;
		};
		if let Ok(nodes) = xpath.findnodes(spec.xpath, None) {
			for mut node in nodes {
				let value = match node.get_attribute(spec.attribute) {
					Some(value) => value,
					None => String::new(),
				};
				let valid = matches_normalization_kind(value.trim(), spec.kind);
				if !valid {
					let _ = node.set_attribute("nullFlavor", "NI");
					let _ = node.remove_attribute(spec.attribute);
					let _ = node.remove_attribute("codeSystem");
					let _ = node.remove_attribute("codeSystemVersion");
				}
			}
		}
	}

	if let Ok(nodes) = xpath.findnodes("//*[@type]", None) {
		for mut node in nodes {
			if let Some(value) = node.get_attribute("type") {
				set_xsi_type(&mut node, &value)?;
			}
		}
	}

	// Some imported FDA device operator rows come back as
	// <characteristic><value code="..."/></characteristic>. The schema expects
	// that coded operator marker to be the characteristic code node itself.
	if let Ok(nodes) = xpath.findnodes(
		"//hl7:characteristic[not(hl7:code)]/hl7:value[@code and not(@value)]",
		None,
	) {
		for mut node in nodes {
			remove_xsi_type(&mut node)?;
			let _ = node.remove_attribute("type");
			let _ = node.set_name("code");
		}
	}
	Ok(())
}

fn matches_normalization_kind(value: &str, kind: ExportNormalizeKind) -> bool {
	match kind {
		ExportNormalizeKind::AsciiDigitsLen(len) => {
			value.len() == len && value.chars().all(|c| c.is_ascii_digit())
		}
		ExportNormalizeKind::AsciiUpperLen(len) => {
			value.len() == len && value.chars().all(|c| c.is_ascii_uppercase())
		}
	}
}

fn prune_optional_nodes(_doc: &mut Document, xpath: &mut Context) {
	if has_export_policy_directive(
		EXPORT_RULE_OPTIONAL_PATH_EMPTY_PRUNE,
		ExportPolicyDirective::RemoveOptionalPathEmptyNodes,
	) {
		let optional_paths = include_str!("../../fda_optional_paths.txt");
		for raw in optional_paths
			.lines()
			.map(str::trim)
			.filter(|l| !l.is_empty())
		{
			let xp = path_to_xpath(raw);
			if let Ok(nodes) = xpath.findnodes(&xp, None) {
				for node in nodes {
					if node.get_name() == "substanceAdministration"
						&& node.get_parent().is_some_and(|parent| {
							parent.get_name() == "outboundRelationship2"
								&& parent.get_attribute("typeCode").as_deref()
									== Some("FLFS")
						}) {
						continue;
					}
					if !node_has_real_data(&node) {
						let mut n = node;
						n.unlink_node();
					}
				}
			}
		}
	}

	prune_placeholder_nodes(xpath);
	if has_export_policy_directive(
		EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE,
		ExportPolicyDirective::RemoveEmptyStructuralNodes,
	) {
		prune_empty_structural_nodes(xpath);
	}
}

fn prune_placeholder_nodes(xpath: &mut Context) {
	if has_export_policy_directive(
		EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE,
		ExportPolicyDirective::RemovePlaceholderValueNodes,
	) {
		for path in export_xpaths_for_rule(EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE) {
			unlink_nodes(xpath, path, true);
		}
	}

	if has_export_policy_directive(
		EXPORT_RULE_RACE_NI_PRUNE,
		ExportPolicyDirective::RemoveRaceNiNodes,
	) {
		if let Some(path) = export_xpath_for_rule(EXPORT_RULE_RACE_NI_PRUNE) {
			unlink_nodes(xpath, path, true);
		}
	}
	if has_export_policy_directive(
		EXPORT_RULE_RACE_EMPTY_PRUNE,
		ExportPolicyDirective::RemoveRaceEmptyNodes,
	) {
		if let Some(path) = export_xpath_for_rule(EXPORT_RULE_RACE_EMPTY_PRUNE) {
			unlink_nodes(xpath, path, true);
		}
	}

	if has_export_policy_directive(
		EXPORT_RULE_GK11_EMPTY_PRUNE,
		ExportPolicyDirective::RemoveEmptyGk11Relationships,
	) {
		if let Some(path) = export_xpath_for_rule(EXPORT_RULE_GK11_EMPTY_PRUNE) {
			unlink_nodes(xpath, path, false);
		}
	}

	if has_export_policy_directive(
		EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN,
		ExportPolicyDirective::RemoveDocumentTextCompression,
	) {
		if let Some(spec) = export_attribute_strip_spec_for_rule(
			EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN,
		) {
			remove_attribute_on_nodes(xpath, spec.xpath, spec.attribute);
		}
	}

	if has_export_policy_directive(
		EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN,
		ExportPolicyDirective::RemoveSummaryLanguageJa,
	) {
		if let Some(spec) = export_attribute_strip_spec_for_rule(
			EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN,
		) {
			remove_attribute_on_nodes(xpath, spec.xpath, spec.attribute);
		}
	}

	if has_export_policy_directive(
		EXPORT_RULE_FDA_REQUIRED_INTERVENTION,
		ExportPolicyDirective::RequiredInterventionNullFlavorNi,
	) {
		if let Some(path) =
			export_xpath_for_rule(EXPORT_RULE_FDA_REQUIRED_INTERVENTION)
		{
			if let Ok(nodes) = xpath.findnodes(path, None) {
				for mut node in nodes {
					if !required_intervention_rule_applies(&node) {
						continue;
					}
					let val = match node.get_attribute("value") {
						Some(value) => value,
						None => String::new(),
					};
					let trimmed = val.trim();
					if !trimmed.is_empty() && !looks_placeholder(trimmed) {
						let _ = node.remove_attribute("nullFlavor");
						continue;
					}
					let _ = node.remove_attribute("value");
					if node.get_attribute("nullFlavor").is_none() {
						let _ = node.set_attribute("nullFlavor", "NI");
					}
				}
			}
		}
	}
}

fn prune_empty_structural_nodes(xpath: &mut Context) {
	for path in export_xpaths_for_rule(EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE) {
		if let Ok(nodes) = xpath.findnodes(path, None) {
			for node in nodes {
				let has_element_children = node
					.get_child_nodes()
					.into_iter()
					.any(|c| c.get_type() == Some(NodeType::ElementNode));
				if !has_element_children && !node_has_real_data(&node) {
					let mut n = node;
					n.unlink_node();
				}
			}
		}
	}
}

fn path_to_xpath(path: &str) -> String {
	let mut out = String::from("/");
	for (i, part) in path.split('/').enumerate() {
		if i > 0 {
			out.push('/');
		}
		out.push_str("hl7:");
		out.push_str(part);
	}
	out
}

fn remove_attribute_on_nodes(xpath: &mut Context, node_xpath: &str, attr: &str) {
	if let Ok(nodes) = xpath.findnodes(node_xpath, None) {
		for mut node in nodes {
			let _ = node.remove_attribute(attr);
		}
	}
}

fn unlink_nodes(
	xpath: &mut Context,
	node_xpath: &str,
	unlink_parent_if_present: bool,
) {
	if let Ok(nodes) = xpath.findnodes(node_xpath, None) {
		for mut node in nodes {
			if unlink_parent_if_present {
				if let Some(mut parent) = node.get_parent() {
					parent.unlink_node();
					continue;
				}
			}
			node.unlink_node();
		}
	}
}

fn node_has_real_data(node: &Node) -> bool {
	if node.get_type() != Some(NodeType::ElementNode) {
		return false;
	}
	let content = node.get_content();
	if !content.trim().is_empty() && !looks_placeholder(content.trim()) {
		return true;
	}
	for attr in [
		"code",
		"value",
		"extension",
		"root",
		"unit",
		"displayName",
		"codeSystem",
		"codeSystemVersion",
		"nullFlavor",
	] {
		if let Some(val) = node.get_attribute(attr) {
			if !val.trim().is_empty() && !looks_placeholder(val.trim()) {
				return true;
			}
		}
	}
	for child in node.get_child_nodes() {
		if child.get_type() == Some(NodeType::ElementNode)
			&& node_has_real_data(&child)
		{
			return true;
		}
	}
	false
}

fn looks_placeholder(value: &str) -> bool {
	let v = value.trim();
	if v.is_empty() {
		return true;
	}
	let Some(first) = v.chars().next() else {
		return false;
	};
	if !first.is_ascii_uppercase() || !v.contains('.') {
		return false;
	}
	if !v.chars().any(|c| c.is_ascii_digit()) {
		return false;
	}
	v.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn required_intervention_rule_applies(value_node: &Node) -> bool {
	let mut current = value_node.clone();
	let mut reaction_observation: Option<Node> = None;

	for _ in 0..4 {
		let Some(parent) = current.get_parent() else {
			break;
		};
		if parent.get_name() == "observation"
			&& child_observation_code(&parent).as_deref() == Some("29")
		{
			reaction_observation = Some(parent);
			break;
		}
		current = parent;
	}

	let other_medically_important = reaction_observation
		.as_ref()
		.is_some_and(reaction_has_other_medically_important_true);

	other_medically_important
}

fn reaction_has_other_medically_important_true(reaction_obs: &Node) -> bool {
	for rel in reaction_obs.get_child_nodes() {
		if rel.get_name() != "outboundRelationship2" {
			continue;
		}
		for obs in rel.get_child_nodes() {
			if obs.get_name() != "observation"
				|| child_observation_code(&obs).as_deref() != Some("26")
			{
				continue;
			}
			for child in obs.get_child_nodes() {
				if child.get_name() != "value" {
					continue;
				}
					let explicit_true = child
						.get_attribute("value")
						.is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1");
					let coded_true = child
						.get_attribute("code")
						.is_some_and(|v| v == "true" || v == "1");
				if explicit_true || coded_true {
					return true;
				}
			}
		}
	}
	false
}

fn child_observation_code(obs: &Node) -> Option<String> {
	obs.get_child_nodes().into_iter().find_map(|child| {
		if child.get_name() == "code" {
			child.get_attribute("code")
		} else {
			None
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn postprocess_does_not_invent_null_flavor_for_empty_text() {
		let parser = libxml::parser::Parser::default();
		let mut doc = parser
			.parse_string(
				br#"<root xmlns="urn:hl7-org:v3"><text/><originalText/><text nullFlavor="UNK"/></root>"#,
			)
			.expect("parse fixture");
		let mut xpath = Context::new(&doc).expect("create xpath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("register namespace");

		postprocess_export_doc(&mut doc, &mut xpath).expect("postprocess export");

		assert_eq!(
			xpath
				.findvalue("/hl7:root/hl7:text[1]/@nullFlavor", None)
				.expect("read empty text nullFlavor"),
			""
		);
		assert_eq!(
			xpath
				.findvalue("/hl7:root/hl7:originalText/@nullFlavor", None)
				.expect("read empty originalText nullFlavor"),
			""
		);
		assert_eq!(
			xpath
				.findvalue("/hl7:root/hl7:text[2]/@nullFlavor", None)
				.expect("read explicit nullFlavor"),
			"UNK"
		);
	}
}
