use crate::{Error, Result, XmlValidationError, XmlValidationReport};
use lib_core::regulatory::RegulatoryAuthority;
use libxml::parser::{Parser, ParserOptions};
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::{Path, PathBuf};

mod export_rules;
pub use export_rules::{validate_export_message_header, validate_export_rules};

#[derive(Debug, Clone)]
pub struct XmlValidatorConfig {
	pub max_bytes: usize,
	pub allowed_roots: &'static [&'static str],
	pub xsd_path: Option<PathBuf>,
	pub require_schema_location: bool,
	pub require_its_version: Option<&'static str>,
	pub authority: Option<RegulatoryAuthority>,
}

impl Default for XmlValidatorConfig {
	fn default() -> Self {
		Self {
			max_bytes: 10 * 1024 * 1024,
			allowed_roots: &["MCCI_IN200100UV01", "MCCI_IN200101UV01"],
			xsd_path: default_xsd_path(),
			require_schema_location: true,
			require_its_version: Some("XML_1.0"),
			authority: None,
		}
	}
}

pub fn default_xsd_path() -> Option<PathBuf> {
	default_xsd_candidates()
		.into_iter()
		.find(|candidate| candidate.exists())
}

fn strict_parser_options() -> ParserOptions<'static> {
	ParserOptions {
		recover: false,
		no_net: true,
		..Default::default()
	}
}

pub fn validate_e2b_xml(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = match config {
		Some(config) => config,
		None => XmlValidatorConfig::default(),
	};
	let mut report = validate_e2b_xml_structure(xml, &config)?;
	if report.errors.is_empty() {
		if let Some(authority) = config.authority {
			report
				.errors
				.append(&mut validate_export_rules(xml, authority)?);
		}
	}

	report.ok = report.errors.is_empty();
	Ok(report)
}

fn validate_e2b_xml_structure(
	xml: &[u8],
	config: &XmlValidatorConfig,
) -> Result<XmlValidationReport> {
	let mut report = validate_e2b_xml_basic(xml, Some(config.clone()))?;

	if let Some(xsd_path) = config.xsd_path.as_ref() {
		let mut xsd_errors = validate_e2b_xml_xsd(xml, xsd_path)?;
		report.errors.append(&mut xsd_errors);
	} else {
		report.errors.push(XmlValidationError {
			message: "XSD validation not configured (provide XmlValidatorConfig.xsd_path or place the schema at a default XML schema location)"
				.to_string(),
			code: None,
			section: None,
			field_path: None,
			line: None,
			column: None,
		});
	}
	report.ok = report.errors.is_empty();
	Ok(report)
}

/// Import checks safe XML decoding; submission owns XSD and business validity.
pub fn validate_e2b_xml_for_import(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = match config {
		Some(config) => config,
		None => XmlValidatorConfig::default(),
	};
	let oversized = xml.len() > config.max_bytes;
	let report = validate_e2b_xml_basic(xml, Some(config))?;
	if oversized {
		return Ok(report);
	}
	let doc = crate::import_sections::shared::parse_import_xml(
		std::str::from_utf8(xml).map_err(|error| Error::InvalidXml {
			message: format!("XML not valid UTF-8: {error}"),
			line: None,
			column: None,
		})?,
	)?;
	let mut xpath =
		libxml::xpath::Context::new(&doc).map_err(|_| Error::InvalidXml {
			message: "Cannot create XML context".into(),
			line: None,
			column: None,
		})?;
	let reports = xpath.findnodes("//*[local-name()='investigationEvent' and namespace-uri()='urn:hl7-org:v3']", None)
		.map_err(|_| Error::InvalidXml { message: "Cannot read report count".into(), line: None, column: None })?;
	if reports.len() != 1 {
		return Err(Error::InvalidXml {
			message: "Each XML file must contain exactly one HL7 investigationEvent; missing or multiple reports cannot be imported without data loss".into(),
			line: None,
			column: None,
		});
	}
	Ok(report)
}

/// Lightweight validation that checks payload size, XML well-formedness, and
/// the allowed root element without running XSD or business rules.
pub fn validate_e2b_xml_basic(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = match config {
		Some(config) => config,
		None => XmlValidatorConfig::default(),
	};
	if xml.len() > config.max_bytes {
		return Ok(XmlValidationReport {
			ok: false,
			errors: vec![XmlValidationError {
				message: format!(
					"XML payload exceeds max size ({} bytes)",
					config.max_bytes
				),
				code: None,
				section: None,
				field_path: None,
				line: None,
				column: None,
			}],
			root_element: None,
		});
	}

	let mut reader = Reader::from_reader(xml);
	reader.trim_text(true);
	let mut buf = Vec::new();
	let mut root: Option<String> = None;
	let mut errors = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(element)) => {
				if root.is_none() {
					let name = element.name().as_ref().to_vec();
					root = Some(String::from_utf8_lossy(&name).to_string());
				}
			}
			Ok(Event::Eof) => break,
			Ok(_) => {}
			Err(err) => {
				errors.push(XmlValidationError {
					message: format!("XML parse error: {err}"),
					code: None,
					section: None,
					field_path: None,
					line: None,
					column: Some(reader.buffer_position()),
				});
				break;
			}
		}
		buf.clear();
	}

	if root.is_none() {
		errors.push(XmlValidationError {
			message: "Missing root element".to_string(),
			code: None,
			section: None,
			field_path: None,
			line: None,
			column: None,
		});
	}

	if let Some(root_name) = &root {
		if !config
			.allowed_roots
			.iter()
			.any(|allowed| *allowed == root_name)
		{
			errors.push(XmlValidationError {
				message: format!(
					"Unexpected root element '{root_name}', expected one of [{}]",
					config.allowed_roots.join(", ")
				),
				code: None,
				section: None,
				field_path: None,
				line: None,
				column: None,
			});
		}
	}

	Ok(XmlValidationReport {
		ok: errors.is_empty(),
		errors,
		root_element: root,
	})
}

pub fn validate_e2b_xml_xsd(
	xml: &[u8],
	xsd_path: &Path,
) -> Result<Vec<XmlValidationError>> {
	if !xsd_path.exists() {
		return Err(Error::InvalidXml {
			message: format!(
				"XSD file not found at '{}'. Provide XmlValidatorConfig.xsd_path explicitly or place the schema at one of the default XML schema locations.",
				xsd_path.display()
			),
			line: None,
			column: None,
		});
	}
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string_with_options(xml_str, strict_parser_options())
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;

	let mut schema_parser = SchemaParserContext::from_file(
		xsd_path.to_str().ok_or(Error::InvalidXml {
			message: "XSD path is not valid UTF-8".to_string(),
			line: None,
			column: None,
		})?,
	);
	let mut ctx = SchemaValidationContext::from_parser(&mut schema_parser).map_err(
		|errors| Error::InvalidXml {
			message: format!(
				"XSD parse error: {}",
				match errors.first().and_then(|err| err.message.as_deref()) {
					Some(message) => message,
					None => "unknown",
				}
			),
			line: None,
			column: None,
		},
	)?;

	match ctx.validate_document(&doc) {
		Ok(()) => Ok(Vec::new()),
		Err(errors) => Ok(errors
			.into_iter()
			.map(|err| XmlValidationError {
				message: match err.message {
					Some(message) => message,
					None => "XSD validation error".to_string(),
				},
				code: None,
				section: None,
				field_path: None,
				line: err.line.map(|value| value as usize),
				column: err.col.map(|value| value as usize),
			})
			.collect()),
	}
}

fn default_xsd_candidates() -> Vec<PathBuf> {
	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let workspace_root = manifest_dir
		.parent()
		.and_then(|path| path.parent())
		.and_then(|path| path.parent())
		.map(PathBuf::from);
	let mut candidates = Vec::new();
	if let Some(workspace_root) = workspace_root {
		candidates.push(
			workspace_root.join(
				"docs/exporter/schema/multicacheschemas/MCCI_IN200100UV01.xsd",
			),
		);
	}
	candidates.push(PathBuf::from(
		"/app/schemas/multicacheschemas/MCCI_IN200100UV01.xsd",
	));
	candidates.push(PathBuf::from(
		"/opt/e2br3/schemas/multicacheschemas/MCCI_IN200100UV01.xsd",
	));
	candidates
}

#[cfg(test)]
mod tests {
	use super::*;

	fn issue() -> XmlValidationError {
		XmlValidationError {
			message: String::new(),
			code: None,
			section: None,
			field_path: None,
			line: None,
			column: None,
		}
	}

	#[test]
	fn xml_error_has_no_severity_flag() {
		let value = serde_json::to_value(issue()).unwrap();
		assert!(value.get("blocking").is_none());
	}
}
