use crate::xml::error::Error;
use crate::xml::Result;
use libxml::parser::Parser;
use libxml::tree::{Document, Node, NodeType};
use libxml::xpath::Context;
use sqlx::types::time::{Date, OffsetDateTime};
use time::{Duration, Month, PrimitiveDateTime, Time, UtcOffset};

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

pub(crate) fn fmt_datetime(dt: OffsetDateTime) -> String {
	let dt = dt.to_offset(UtcOffset::UTC);
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

pub(crate) fn clamp_14_digit_datetime_not_future(value: &str) -> String {
	if value.len() != 14 || !value.chars().all(|c| c.is_ascii_digit()) {
		return value.to_string();
	}
	let year: i32 = value[0..4].parse().unwrap_or(0);
	let month_num: u8 = value[4..6].parse().unwrap_or(0);
	let day: u8 = value[6..8].parse().unwrap_or(0);
	let hour: u8 = value[8..10].parse().unwrap_or(0);
	let minute: u8 = value[10..12].parse().unwrap_or(0);
	let second: u8 = value[12..14].parse().unwrap_or(0);

	let month = match month_num {
		1 => Month::January,
		2 => Month::February,
		3 => Month::March,
		4 => Month::April,
		5 => Month::May,
		6 => Month::June,
		7 => Month::July,
		8 => Month::August,
		9 => Month::September,
		10 => Month::October,
		11 => Month::November,
		12 => Month::December,
		_ => return value.to_string(),
	};

	let Ok(date) = Date::from_calendar_date(year, month, day) else {
		return value.to_string();
	};
	let Ok(time) = Time::from_hms(hour, minute, second) else {
		return value.to_string();
	};
	let dt = PrimitiveDateTime::new(date, time).assume_utc();
	let cutoff = OffsetDateTime::now_utc() - Duration::minutes(5);
	if dt > cutoff {
		fmt_datetime(cutoff)
	} else {
		value.to_string()
	}
}

pub(crate) fn fmt_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
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
	let fragment = wrap_fragment(fragment, "urn:hl7-org:v3");
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

fn wrap_fragment(fragment: &str, ns: &str) -> String {
	format!(
		"<wrapper xmlns=\"{ns}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">{fragment}</wrapper>"
	)
}

pub(crate) fn xml_escape(input: &str) -> String {
	input
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}
