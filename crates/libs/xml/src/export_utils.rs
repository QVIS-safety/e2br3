use crate::error::Error;
use crate::Result;
use libxml::parser::Parser;
use libxml::tree::{Document, Namespace, Node, NodeType};
use libxml::xpath::Context;
use quick_xml::events::Event;
use quick_xml::Reader;
use sqlx::types::time::{Date, OffsetDateTime};
use time::UtcOffset;

pub(crate) fn set_attr_first(
	xpath: &mut Context,
	path: &str,
	attr: &str,
	value: &str,
) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_attribute(attr, value);
		}
	}
}

pub(crate) fn set_xsi_type_first(
	xpath: &mut Context,
	path: &str,
	value: &str,
) -> Result<()> {
	let mut node = xpath
		.findnodes(path, None)
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to find nodes for path {path}"),
			line: None,
			column: None,
		})?
		.into_iter()
		.next()
		.ok_or_else(|| Error::InvalidXml {
			message: format!("Failed to find nodes for path {path}"),
			line: None,
			column: None,
		})?;
	set_xsi_type(&mut node, value)
}

pub(crate) fn set_xsi_type(node: &mut Node, value: &str) -> Result<()> {
	let namespace = match xsi_namespace(node) {
		Some(namespace) => namespace,
		None => Namespace::new("xsi", XSI_NAMESPACE, node).map_err(|err| {
			Error::InvalidXml {
				message: format!("Failed to declare xsi namespace: {err}"),
				line: None,
				column: None,
			}
		})?,
	};
	node.remove_attribute_no_ns("type")
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to remove unqualified type attribute: {err}"),
			line: None,
			column: None,
		})?;
	node.remove_attribute_no_ns("xsi:type")
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to remove malformed xsi:type attribute: {err}"),
			line: None,
			column: None,
		})?;
	node.set_attribute_ns("type", value, &namespace)
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to set xsi:type attribute: {err}"),
			line: None,
			column: None,
		})
}

pub(crate) fn set_text_first(xpath: &mut Context, path: &str, value: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.set_content(value);
		}
	}
}

pub(crate) fn remove_attr_first(xpath: &mut Context, path: &str, attr: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		if let Some(mut node) = nodes.into_iter().next() {
			let _ = node.remove_attribute(attr);
		}
	}
}

pub(crate) fn remove_xsi_type(node: &mut Node) -> Result<()> {
	node.remove_attribute_ns("type", XSI_NAMESPACE)
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to remove xsi:type attribute: {err}"),
			line: None,
			column: None,
		})?;
	node.remove_attribute_no_ns("xsi:type")
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to remove malformed xsi:type attribute: {err}"),
			line: None,
			column: None,
		})
}

const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

fn xsi_namespace(node: &Node) -> Option<Namespace> {
	let mut current = Some(node.clone());
	while let Some(element) = current {
		if let Some(namespace) = element
			.get_namespace_declarations()
			.into_iter()
			.find(|namespace| {
				namespace.get_prefix() == "xsi"
					&& namespace.get_href() == XSI_NAMESPACE
			}) {
			return Some(namespace);
		}
		current = element.get_parent();
	}
	None
}

pub(crate) fn fmt_datetime(dt: OffsetDateTime) -> String {
	let dt = dt.to_offset(UtcOffset::UTC);
	format!(
		"{:04}{:02}{:02}{:02}{:02}{:02}+0000",
		dt.year(),
		u8::from(dt.month()),
		dt.day(),
		dt.hour(),
		dt.minute(),
		dt.second()
	)
}

pub(crate) fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

pub(crate) fn fmt_date_lexeme(value: &str) -> String {
	let value = value.trim();
	if value.len() == 10
		&& value.as_bytes()[4] == b'-'
		&& value.as_bytes()[7] == b'-'
		&& value
			.bytes()
			.enumerate()
			.all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
	{
		return value.replace('-', "");
	}
	value.to_string()
}

pub(crate) fn append_fragment_child(
	doc: &mut Document,
	parser: &Parser,
	xpath: &mut Context,
	parent_path: &str,
	fragment: &str,
) -> Result<()> {
	let mut parent = xpath
		.findnodes(parent_path, None)
		.map_err(|_| Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?
		.into_iter()
		.next()
		.ok_or(Error::InvalidXml {
			message: format!("Failed to find nodes for path {parent_path}"),
			line: None,
			column: None,
		})?;

	let mut node = node_from_fragment(doc, parser, fragment)?;
	parent
		.add_child(&mut node)
		.map_err(|err| Error::InvalidXml {
			message: format!("Failed to append fragment: {err}"),
			line: None,
			column: None,
		})?;
	Ok(())
}

pub(crate) fn remove_nodes(xpath: &mut Context, path: &str) {
	if let Ok(nodes) = xpath.findnodes(path, None) {
		for mut node in nodes {
			node.unlink_node();
		}
	}
}

fn node_from_fragment(
	doc: &mut Document,
	parser: &Parser,
	fragment: &str,
) -> Result<Node> {
	let fragment = wrap_fragment(fragment, "urn:hl7-org:v3")?;
	let frag_doc =
		parser
			.parse_string(&fragment)
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})?;
	let root = frag_doc.get_root_element().ok_or(Error::InvalidXml {
		message: "Failed to get fragment root".to_string(),
		line: None,
		column: None,
	})?;
	let mut child = root
		.get_child_nodes()
		.into_iter()
		.find(|n| n.get_type() == Some(NodeType::ElementNode))
		.ok_or(Error::InvalidXml {
			message: "Failed to get fragment child".to_string(),
			line: None,
			column: None,
		})?;
	child.unlink_node();
	doc.import_node(&mut child).map_err(|_| Error::InvalidXml {
		message: "Failed to import cloned node".to_string(),
		line: None,
		column: None,
	})
}

fn wrap_fragment(fragment: &str, ns: &str) -> Result<String> {
	let mut reader = Reader::from_str(fragment);
	loop {
		let (element, closing_len) = match reader.read_event() {
			Ok(Event::Start(element)) => (element, 1),
			Ok(Event::Empty(element)) => (element, 2),
			Ok(Event::Eof) => {
				return Err(Error::InvalidXml {
					message: "Fragment has no root element".to_string(),
					line: None,
					column: None,
				})
			}
			Ok(_) => continue,
			Err(err) => {
				return Err(Error::InvalidXml {
					message: format!("XML parse error: {err}"),
					line: None,
					column: None,
				})
			}
		};

		let has_default_namespace = match element
			.try_get_attribute("xmlns")
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})? {
			Some(attr) if attr.value.as_ref() == ns.as_bytes() => true,
			Some(_) => {
				return Err(Error::InvalidXml {
					message: "Fragment root has an invalid default namespace"
						.to_string(),
					line: None,
					column: None,
				})
			}
			None => false,
		};
		let xsi = "http://www.w3.org/2001/XMLSchema-instance";
		let has_xsi_namespace = match element
			.try_get_attribute("xmlns:xsi")
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})? {
			Some(attr) if attr.value.as_ref() == xsi.as_bytes() => true,
			Some(_) => {
				return Err(Error::InvalidXml {
					message: "Fragment root has an invalid xsi namespace"
						.to_string(),
					line: None,
					column: None,
				})
			}
			None => false,
		};

		let insert_at = reader.buffer_position() - closing_len;
		let mut rooted =
			String::with_capacity(fragment.len() + ns.len() + xsi.len() + 25);
		rooted.push_str(&fragment[..insert_at]);
		if !has_default_namespace {
			rooted.push_str(" xmlns=\"");
			rooted.push_str(ns);
			rooted.push('"');
		}
		if !has_xsi_namespace {
			rooted.push_str(" xmlns:xsi=\"");
			rooted.push_str(xsi);
			rooted.push('"');
		}
		rooted.push_str(&fragment[insert_at..]);
		return Ok(format!("<wrapper>{rooted}</wrapper>"));
	}
}

pub(crate) fn xml_escape(input: &str) -> String {
	input
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
	use super::{
		append_fragment_child, fmt_date_lexeme, set_xsi_type_first, wrap_fragment,
		xml_escape, XSI_NAMESPACE,
	};
	use libxml::parser::Parser;
	use libxml::xpath::Context;

	#[test]
	fn xml_escape_preserves_text_and_escapes_entities_once() {
		assert_eq!(xml_escape(""), "");
		assert_eq!(xml_escape("환자 A"), "환자 A");
		assert_eq!(xml_escape("&<>\"'"), "&amp;&lt;&gt;&quot;&apos;");
		assert_eq!(xml_escape("&amp;"), "&amp;amp;");
	}

	#[test]
	fn date_lexeme_compacts_ui_dates_without_rewriting_e2b_precision() {
		assert_eq!(fmt_date_lexeme("2026-04-07"), "20260407");
		assert_eq!(fmt_date_lexeme("202604"), "202604");
		assert_eq!(fmt_date_lexeme("invalid"), "invalid");
	}

	#[test]
	fn appended_fragment_keeps_namespace_bindings() {
		let parser = Parser::default();
		let mut doc = parser
			.parse_string("<root xmlns=\"urn:hl7-org:v3\"><parent/></root>")
			.expect("destination document");
		let mut xpath = Context::new(&doc).expect("destination XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("HL7 namespace");

		append_fragment_child(
			&mut doc,
			&parser,
			&mut xpath,
			"//hl7:parent",
			"<value xsi:type=\"ED\"/>",
		)
		.expect("append fragment");

		let reparsed = parser
			.parse_string(&doc.to_string())
			.expect("serialized document with bound xsi prefix");
		let mut reparsed_xpath = Context::new(&reparsed).expect("reparsed XPath");
		reparsed_xpath
			.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
			.expect("xsi namespace");
		assert_eq!(
			reparsed_xpath
				.findnodes("//*[@xsi:type='ED']", None)
				.expect("xsi:type lookup")
				.len(),
			1
		);
	}

	#[test]
	fn fragment_root_namespaces_are_not_duplicated() {
		let wrapped = wrap_fragment(
			"<value xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"ED\"/>",
			"urn:hl7-org:v3",
		)
		.expect("wrap fragment");

		assert_eq!(wrapped.matches("xmlns=\"").count(), 1);
		assert_eq!(wrapped.matches("xmlns:xsi=\"").count(), 1);
	}

	#[test]
	fn namespaced_attribute_update_does_not_duplicate_xsi_type() {
		let parser = Parser::default();
		let doc = parser
			.parse_string(
				"<root xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><value xsi:type=\"BL\"/></root>",
			)
			.expect("source document");
		let mut xpath = Context::new(&doc).expect("source XPath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("HL7 namespace");

		set_xsi_type_first(&mut xpath, "//hl7:value", "CE")
			.expect("update xsi:type");

		let serialized = doc.to_string();
		assert_eq!(serialized.matches("xsi:type=\"").count(), 1, "{serialized}");
		let reparsed = parser.parse_string(&serialized).expect("serialized XML");
		let mut reparsed_xpath = Context::new(&reparsed).expect("reparsed XPath");
		reparsed_xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("HL7 namespace");
		reparsed_xpath
			.register_namespace("xsi", XSI_NAMESPACE)
			.expect("xsi namespace");
		assert_eq!(
			reparsed_xpath
				.findnodes("//hl7:value[@xsi:type='CE']", None)
				.expect("xsi:type lookup")
				.len(),
			1
		);
	}
}
