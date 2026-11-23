use crate::export::policy::{
	export_xpaths_for_rule, EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE,
};
use crate::export_utils::{remove_xsi_type, set_xsi_type};
use libxml::tree::{Document, Node, NodeType};
use libxml::xpath::Context;

pub(crate) fn postprocess_export_doc(
	_doc: &mut Document,
	xpath: &mut Context,
) -> crate::Result<()> {
	normalize_export_values(xpath)?;
	prune_empty_structural_nodes(xpath);
	Ok(())
}

fn normalize_export_values(xpath: &mut Context) -> crate::Result<()> {
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

fn node_has_real_data(node: &Node) -> bool {
	if node.get_type() != Some(NodeType::ElementNode) {
		return false;
	}
	let content = node.get_content();
	if !content.is_empty() {
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
		if node.get_attribute(attr).is_some() {
			return true;
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn postprocess_preserves_placeholder_shaped_stored_values() {
		// Isolated draft lexemes, not clinical data or a submission-validity claim.
		let mut doc = libxml::parser::Parser::default().parse_string(br#"<root xmlns="urn:hl7-org:v3">
<observation><value code="G.k.10.r"/></observation>
<investigationCharacteristic><code code="3" codeSystem="2.16.840.1.113883.3.989.2.1.1.23"/><value code="C.1.11.1"/></investigationCharacteristic>
<observation><value code="D.2.3"/></observation>
<observation><value value="4" unit="D.2.2.1b"/></observation>
<observation><value code="AB.123"/></observation>
<observation>AB.123</observation>
</root>"#).expect("parse isolated draft lexemes");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		let before = doc.to_string();
		postprocess_export_doc(&mut doc, &mut xpath)
			.expect("postprocess draft export");
		assert_eq!(
			doc.to_string(),
			before,
			"stored values must not be guessed to be template placeholders"
		);
	}

	#[test]
	fn postprocess_preserves_draft_codes_and_explicit_null_flavors() {
		let parser = libxml::parser::Parser::default();
		let mut doc = parser.parse_string(br#"<root xmlns="urn:hl7-org:v3">
<value code="draft-term" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="24.1"/>
<value code="123" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="24.1"/>
<value nullFlavor="UNK" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="24.1"/>
<value code="10019211" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="24.1"/>
<code code="us" codeSystem="1.0.3166.1.2.2" codeSystemVersion="isolated"/>
<code code="USA" codeSystem="1.0.3166.1.2.2" codeSystemVersion="isolated"/>
<code nullFlavor="UNK" codeSystem="1.0.3166.1.2.2" codeSystemVersion="isolated"/>
<code code="US" codeSystem="1.0.3166.1.2.2" codeSystemVersion="isolated"/>
</root>"#).expect("parse isolated draft codes");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		let attributes = |context: &mut Context| {
			context
				.findnodes("/hl7:root/*", None)
				.expect("draft nodes")
				.iter()
				.map(|node| {
					["code", "codeSystem", "codeSystemVersion", "nullFlavor"]
						.map(|attribute| node.get_attribute(attribute))
				})
				.collect::<Vec<_>>()
		};
		let before = attributes(&mut xpath);
		assert_eq!(before.len(), 8);
		postprocess_export_doc(&mut doc, &mut xpath)
			.expect("postprocess draft export");
		assert_eq!(
			attributes(&mut xpath),
			before,
			"export must not replace explicit draft values or UNK with NI"
		);
	}

	#[test]
	fn postprocess_preserves_document_compression_and_summary_language() {
		let parser = libxml::parser::Parser::default();
		let mut doc = parser.parse_string(br#"<root xmlns="urn:hl7-org:v3"><document><text mediaType="text/plain" representation="B64" compression="GZ">H4sIAAAAAAAAA8sszs9JLElNUcjLz0vOyczLTE7MUUgsKUlMzshNzSvhAgCw1cKiIAAAAA==</text></document><component><observationEvent><code code="36"/><value language="JA">Isolated draft summary</value></observationEvent></component></root>"#).expect("parse isolated fixture");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		postprocess_export_doc(&mut doc, &mut xpath)
			.expect("postprocess draft export");
		assert_eq!(
			[
				xpath
					.findvalue("//hl7:document/hl7:text/@compression", None)
					.expect("compression"),
				xpath
					.findvalue("//hl7:observationEvent/hl7:value/@language", None)
					.expect("language"),
			],
			["GZ", "JA"],
			"stored metadata must not be deleted to satisfy submission rules"
		);
		assert_eq!(xpath.findvalue("//hl7:document/hl7:text", None).expect("document bytes"), "H4sIAAAAAAAAA8sszs9JLElNUcjLz0vOyczLTE7MUUgsKUlMzshNzSvhAgCw1cKiIAAAAA==");
		assert_eq!(
			xpath
				.findvalue("//hl7:observationEvent/hl7:value", None)
				.expect("summary text"),
			"Isolated draft summary"
		);
		assert_eq!(
			xpath
				.findvalue("//hl7:document/hl7:text/@mediaType", None)
				.expect("media type"),
			"text/plain"
		);
		assert_eq!(
			xpath
				.findvalue("//hl7:document/hl7:text/@representation", None)
				.expect("representation"),
			"B64"
		);
	}

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

#[cfg(test)]
mod lossless_tests {
	use super::*;
	#[test]
	fn postprocessing_never_synthesizes_null_flavor_or_drops_empty_stored_text() {
		let mut doc = libxml::parser::Parser::default().parse_string(r#"<root xmlns="urn:hl7-org:v3"><observation><code code="29"/><outboundRelationship2><observation><code code="26"/><value value="true"/></observation></outboundRelationship2><outboundRelationship2><observation><code code="7"/><value/></observation></outboundRelationship2></observation><observation><code code="C17049" codeSystem="2.16.840.1.113883.3.26.1.1"/><value code="NI"/></observation><outboundRelationship2><observation><code code="2"/><value/></observation></outboundRelationship2></root>"#).expect("fixture");
		let mut xpath = Context::new(&doc).expect("XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		let before = doc.to_string();
		postprocess_export_doc(&mut doc, &mut xpath).expect("postprocess");
		assert_eq!(doc.to_string(), before);
	}
}
