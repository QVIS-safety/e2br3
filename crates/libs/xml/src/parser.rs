use crate::error::Error;
use crate::types::ParsedE2b;
use crate::Result;
use serde_json::json;

pub fn parse_e2b_xml(xml: &[u8]) -> Result<ParsedE2b> {
	// Import performs validation before parse; keep parser focused on parsing metadata.
	let root = extract_root_element_name(xml).ok_or_else(|| Error::InvalidXml {
		message: "XML root element missing".to_string(),
		line: None,
		column: None,
	})?;
	let json = json!({
		"root": root,
		"size_bytes": xml.len(),
	});

	Ok(ParsedE2b {
		root_element: root,
		json,
	})
}

fn extract_root_element_name(xml: &[u8]) -> Option<String> {
	let xml_str = match std::str::from_utf8(xml) {
		Ok(xml) => xml,
		Err(_) => return None,
	};
	let mut rest = xml_str;
	loop {
		let start = rest.find('<')?;
		rest = &rest[start + 1..];
		if rest.starts_with('?') || rest.starts_with('!') {
			rest = rest.get(rest.find('>')? + 1..)?;
			continue;
		}
		let end =
			match rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/') {
				Some(end) => end,
				None => rest.len(),
			};
		let name = rest[..end].trim();
		return (!name.is_empty()).then(|| name.to_string());
	}
}

#[cfg(test)]
mod tests {
	use super::extract_root_element_name;

	#[test]
	fn root_name_skips_xml_declaration_and_comments() {
		assert_eq!(
			extract_root_element_name(
				br#"<?xml version=\"1.0\"?><!----><MCCI_IN200100UV01/>"#
			),
			Some("MCCI_IN200100UV01".to_string())
		);
	}
}
