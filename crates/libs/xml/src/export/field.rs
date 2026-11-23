use crate::export_utils::xml_escape;
use crate::{Error, Result};
use libxml::xpath::Context;
use std::fmt::Display;

pub(crate) enum Value<'a, T> {
	Present(T),
	NullFlavor(&'a str),
	Missing,
}

pub(crate) fn invalid(field: &str, message: impl Display) -> Error {
	Error::InvalidXml {
		message: format!("{field}: {message}"),
		line: None,
		column: None,
	}
}

pub(crate) fn resolve<'a, T>(
	field: &str,
	value: Option<T>,
	null_flavor: Option<&'a str>,
) -> Result<Value<'a, T>> {
	match (value, null_flavor) {
		(Some(_), Some(_)) => Err(invalid(
			field,
			"value and nullFlavor cannot both be present",
		)),
		(Some(value), None) => Ok(Value::Present(value)),
		(None, Some(null_flavor)) => Ok(Value::NullFlavor(null_flavor)),
		(None, None) => Ok(Value::Missing),
	}
}

fn xml_text(field: &str, value: &str) -> Result<()> {
	if value.chars().any(|c| !matches!(c, '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')) {
   return Err(invalid(field, "stored text contains a character that XML 1.0 cannot represent"));
 }
	Ok(())
}

fn attribute_text(value: &str) -> String {
	xml_escape(value)
		.replace('\r', "&#13;")
		.replace('\n', "&#10;")
		.replace('\t', "&#9;")
}

pub(crate) fn attributes<T: Display>(
	field: &str,
	value: Option<T>,
	null_flavor: Option<&str>,
	attribute: &str,
) -> Result<String> {
	match resolve(field, value, null_flavor)? {
		Value::Present(value) => {
			let value = value.to_string();
			xml_text(field, &value)?;
			Ok(format!(" {attribute}=\"{}\"", attribute_text(&value)))
		}
		Value::NullFlavor(null_flavor) => {
			xml_text(field, null_flavor)?;
			Ok(format!(" nullFlavor=\"{}\"", attribute_text(null_flavor)))
		}
		Value::Missing => Ok(String::new()),
	}
}

pub(crate) fn text_element(
	field: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
	tag: &str,
) -> Result<String> {
	match resolve(field, value, null_flavor)? {
		Value::Present(value) => {
			xml_text(field, value)?;
			Ok(format!(
				"<{tag}>{}</{tag}>",
				xml_escape(value).replace('\r', "&#13;")
			))
		}
		Value::NullFlavor(null_flavor) => {
			xml_text(field, null_flavor)?;
			Ok(format!(
				"<{tag} nullFlavor=\"{}\"/>",
				attribute_text(null_flavor)
			))
		}
		Value::Missing => Ok(String::new()),
	}
}

pub(crate) fn set(
	field: &str,
	xpath: &mut Context,
	path: &str,
	value: Option<&str>,
	null_flavor: Option<&str>,
	attribute: Option<&str>,
) -> Result<()> {
	let resolved = resolve(field, value, null_flavor)?;
	match &resolved {
		Value::Present(value) | Value::NullFlavor(value) => xml_text(field, value)?,
		Value::Missing => {}
	}
	let nodes = xpath
		.findnodes(path, None)
		.map_err(|_| invalid(field, format!("invalid XPath: {path}")))?;
	if nodes.is_empty() && !matches!(resolved, Value::Missing) {
		return Err(invalid(field, format!("missing XML target: {path}")));
	}
	for mut node in nodes.into_iter().take(1) {
		for name in attribute.into_iter().chain(std::iter::once("nullFlavor")) {
			if node.get_attribute(name).is_some() {
				node.remove_attribute(name).map_err(|error| {
					invalid(field, format!("remove {name}: {error:?}"))
				})?;
			}
		}
		if attribute.is_none() {
			node.set_content("")
				.map_err(|error| invalid(field, format!("clear text: {error:?}")))?;
		}
		match &resolved {
			Value::Present(value) => match attribute {
				Some(name) => node.set_attribute(name, value).map_err(|error| {
					invalid(field, format!("write {name}: {error:?}"))
				})?,
				None => node.set_content(&xml_escape(value)).map_err(|error| {
					invalid(field, format!("write text: {error:?}"))
				})?,
			},
			Value::NullFlavor(null_flavor) => node
				.set_attribute("nullFlavor", null_flavor)
				.map_err(|error| {
					invalid(field, format!("write nullFlavor: {error:?}"))
				})?,
			Value::Missing => {}
		}
	}
	Ok(())
}

pub(crate) fn element<T: Display>(
	field: &str,
	value: Option<T>,
	null_flavor: Option<&str>,
	tag: &str,
	attribute: &str,
	fixed: &str,
) -> Result<String> {
	let attrs = attributes(field, value, null_flavor, attribute)?;
	if attrs.is_empty() {
		return Ok(String::new());
	}
	Ok(format!("<{tag}{fixed}{attrs}/>"))
}

pub(crate) fn stored_date(
	field: &str,
	date: Option<String>,
	raw: Option<&str>,
) -> Result<Option<String>> {
	match (date, raw) {
		(Some(_), Some(_)) => {
			Err(invalid(field, "conflicting stored date representations"))
		}
		(Some(value), None) => Ok(Some(value)),
		(None, Some(value)) => Ok(Some(value.to_owned())),
		(None, None) => Ok(None),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn dom_writes_preserve_values_and_reject_conflicts_before_mutation() {
		let doc = libxml::parser::Parser::default()
			.parse_string(
				"<root><name nullFlavor=\"OLD\"/><id extension=\"old\"/></root>",
			)
			.expect("fixture");
		let mut xpath = Context::new(&doc).expect("XPath");
		for value in [" <&> \"한글\" ", "", "   ", "line\n\tvalue\r"] {
			set("name", &mut xpath, "/root/name", Some(value), None, None)
				.expect("text");
			assert_eq!(xpath.findvalue("/root/name", None).expect("text"), value);
			set(
				"id",
				&mut xpath,
				"/root/id",
				Some(value),
				None,
				Some("extension"),
			)
			.expect("attribute");
			let nodes = xpath.findnodes("/root/id", None).expect("id");
			assert_eq!(nodes[0].get_attribute("extension").as_deref(), Some(value));
		}
		let before = doc.to_string();
		assert!(set(
			"id",
			&mut xpath,
			"/root/id",
			Some("bad\u{1}"),
			None,
			Some("extension")
		)
		.is_err());
		assert!(attributes("id", Some("bad\u{1}"), None, "value").is_err());
		assert!(text_element("name", Some("bad\u{1}"), None, "name").is_err());
		assert!(set(
			"id",
			&mut xpath,
			"/root/id",
			Some(""),
			Some("UNK"),
			Some("extension")
		)
		.is_err());
		assert_eq!(doc.to_string(), before);
		set(
			"id",
			&mut xpath,
			"/root/id",
			None,
			Some("CUSTOM"),
			Some("extension"),
		)
		.expect("raw nullFlavor");
		let nodes = xpath.findnodes("/root/id", None).expect("id");
		assert_eq!(nodes[0].get_attribute("extension"), None);
		assert_eq!(
			nodes[0].get_attribute("nullFlavor").as_deref(),
			Some("CUSTOM")
		);
		assert!(set(
			"id",
			&mut xpath,
			"/root/missing",
			Some("stored"),
			None,
			Some("extension")
		)
		.is_err());
	}

	#[test]
	fn stored_dates_never_choose_between_two_representations() {
		assert!(stored_date("date", Some("20240101".into()), Some("2024")).is_err());
		assert_eq!(
			stored_date("date", None, Some("2024")).expect("raw"),
			Some("2024".into())
		);
		assert_eq!(stored_date("date", None, None).expect("missing"), None);
	}
	#[test]
	fn serialization_preserves_stored_values_without_business_validation() {
		assert_eq!(
			attributes("test", Some(false), None, "value").expect("false"),
			" value=\"false\""
		);
		assert_eq!(
			attributes("test", Some(0), None, "value").expect("zero"),
			" value=\"0\""
		);
		assert_eq!(
			attributes::<bool>("test", None, None, "value").expect("missing"),
			""
		);
		for code in ["UNK", "NA", "OTH", "CUSTOM", "", " UNK "] {
			assert_eq!(
				attributes::<bool>("test", None, Some(code), "value")
					.expect("stored nullFlavor"),
				format!(" nullFlavor=\"{code}\"")
			);
			assert!(resolve("test", Some(false), Some(code)).is_err());
		}
		assert!(resolve("test", Some(""), Some("UNK")).is_err());
		assert_eq!(
			text_element("test", Some(" <&> "), None, "name").expect("text"),
			"<name> &lt;&amp;&gt; </name>"
		);
	}
}
