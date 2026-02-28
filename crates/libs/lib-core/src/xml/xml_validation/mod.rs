mod fda_profile;
mod ich_profile;

use crate::xml::error::Error;
use crate::xml::types::{XmlValidationError, XmlValidationReport};
use crate::xml::validate::{
	find_canonical_rule_for_phase, is_rule_condition_satisfied,
	is_rule_presence_valid, is_rule_value_valid, ExportNormalizationSpec,
	ExportNormalizeKind, RuleFacts, ValidationPhase,
};
use crate::xml::Result;
use fda_profile::collect_fda_profile_errors;
use ich_profile::{
	collect_ich_case_history_errors, collect_ich_identity_text_errors,
	collect_ich_profile_value_presence_errors, collect_ich_structural_value_errors,
};
use libxml::parser::Parser;
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};
use libxml::xpath::Context;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct XmlValidatorConfig {
	pub max_bytes: usize,
	pub allowed_roots: &'static [&'static str],
	pub xsd_path: Option<PathBuf>,
	pub require_schema_location: bool,
	pub require_its_version: Option<&'static str>,
}

impl Default for XmlValidatorConfig {
	fn default() -> Self {
		Self {
			max_bytes: 5 * 1024 * 1024,
			allowed_roots: &["MCCI_IN200100UV01", "MCCI_IN200101UV01"],
			xsd_path: xsd_path_from_env(),
			require_schema_location: true,
			require_its_version: Some("XML_1.0"),
		}
	}
}

pub fn validate_e2b_xml(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = config.unwrap_or_default();
	if xml.len() > config.max_bytes {
		return Ok(XmlValidationReport {
			ok: false,
			errors: vec![XmlValidationError {
				message: format!(
					"XML payload exceeds max size ({} bytes)",
					config.max_bytes
				),
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
	let mut errors: Vec<XmlValidationError> = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				if root.is_none() {
					let name_bytes = e.name().as_ref().to_vec();
					root = Some(String::from_utf8_lossy(&name_bytes).to_string());
				}
			}
			Ok(Event::Eof) => break,
			Ok(_) => {}
			Err(e) => {
				let pos = reader.buffer_position();
				errors.push(XmlValidationError {
					message: format!("XML parse error: {e}"),
					line: None,
					column: Some(pos),
				});
				break;
			}
		}
		buf.clear();
	}

	if root.is_none() {
		errors.push(XmlValidationError {
			message: "Missing root element".to_string(),
			line: None,
			column: None,
		});
	}

	if let Some(root_name) = &root {
		if !config.allowed_roots.iter().any(|v| *v == root_name) {
			errors.push(XmlValidationError {
				message: format!(
					"Unexpected root element '{root_name}', expected one of [{}]",
					config.allowed_roots.join(", ")
				),
				line: None,
				column: None,
			});
		}
	}

	if let Some(xsd_path) = config.xsd_path.as_ref() {
		let mut xsd_errors = validate_e2b_xml_xsd(xml, xsd_path)?;
		let xml_str = std::str::from_utf8(xml).unwrap_or_default();
		let has_porr = xml_str.contains("<PORR_IN049016UV")
			|| xml_str.contains("<PORR_IN049017UV")
			|| xml_str.contains("<PORR_IN049018UV");
		if has_porr {
			xsd_errors.retain(|err| {
				!err.message.contains("Missing child element(s)")
					|| !err.message.contains("PORR_IN04901")
			});
		}
		errors.append(&mut xsd_errors);
	} else {
		errors.push(XmlValidationError {
			message: "XSD validation not configured (set E2BR3_XSD_PATH)"
				.to_string(),
			line: None,
			column: None,
		});
	}

	Ok(XmlValidationReport {
		ok: errors.is_empty(),
		errors,
		root_element: root,
	})
}

/// Business/XML-structure validation only:
/// - lightweight XML parse/root checks
/// - catalog-driven XML structure/profile rules (ICH/FDA/MFDS overlays)
///
/// Does not run XSD validation.
pub fn validate_e2b_xml_business(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = config.unwrap_or_default();
	let mut base = validate_e2b_xml_basic(xml, Some(config.clone()))?;
	let mut rule_errors = validate_e2b_xml_rules(xml, &config)?;
	base.errors.append(&mut rule_errors);
	base.ok = base.errors.is_empty();
	Ok(base)
}

/// Lightweight validation:
/// - max size
/// - XML well-formedness (via quick_xml)
/// - root element presence + allowed roots
///
/// Does not run XSD validation or business rules.
pub fn validate_e2b_xml_basic(
	xml: &[u8],
	config: Option<XmlValidatorConfig>,
) -> Result<XmlValidationReport> {
	let config = config.unwrap_or_default();
	if xml.len() > config.max_bytes {
		return Ok(XmlValidationReport {
			ok: false,
			errors: vec![XmlValidationError {
				message: format!(
					"XML payload exceeds max size ({} bytes)",
					config.max_bytes
				),
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
	let mut errors: Vec<XmlValidationError> = Vec::new();

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				if root.is_none() {
					let name_bytes = e.name().as_ref().to_vec();
					root = Some(String::from_utf8_lossy(&name_bytes).to_string());
				}
			}
			Ok(Event::Eof) => break,
			Ok(_) => {}
			Err(e) => {
				let pos = reader.buffer_position();
				errors.push(XmlValidationError {
					message: format!("XML parse error: {e}"),
					line: None,
					column: Some(pos),
				});
				break;
			}
		}
		buf.clear();
	}

	if root.is_none() {
		errors.push(XmlValidationError {
			message: "Missing root element".to_string(),
			line: None,
			column: None,
		});
	}

	if let Some(root_name) = &root {
		if !config.allowed_roots.iter().any(|v| *v == root_name) {
			errors.push(XmlValidationError {
				message: format!(
					"Unexpected root element '{root_name}', expected one of [{}]",
					config.allowed_roots.join(", ")
				),
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

pub fn should_skip_xml_validation() -> bool {
	match std::env::var("E2BR3_SKIP_XML_VALIDATE") {
		Ok(value) => {
			let value = value.trim().to_ascii_lowercase();
			value == "1" || value == "true" || value == "yes"
		}
		Err(_) => false,
	}
}

pub fn validate_e2b_xml_xsd(
	xml: &[u8],
	xsd_path: &Path,
) -> Result<Vec<XmlValidationError>> {
	if !xsd_path.exists() {
		let schema_dir_hint = std::env::var("E2BR3_SCHEMAS_DIR")
			.unwrap_or_else(|_| "/opt/e2br3/schemas".to_string());
		return Err(Error::InvalidXml {
			message: format!(
				"XSD file not found at '{}'. Set E2BR3_XSD_PATH to a valid in-container path (for example '/app/schemas/multicacheschemas/MCCI_IN200100UV01.xsd') and mount your schema package via E2BR3_SCHEMAS_DIR (for example '{}').",
				xsd_path.display(),
				schema_dir_hint
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
		.parse_string(xml_str)
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
				errors
					.first()
					.and_then(|e| e.message.as_deref())
					.unwrap_or("unknown")
			),
			line: None,
			column: None,
		},
	)?;

	match ctx.validate_document(&doc) {
		Ok(()) => Ok(Vec::new()),
		Err(errors) => {
			let mut out = Vec::new();
			for err in errors {
				out.push(XmlValidationError {
					message: err
						.message
						.unwrap_or_else(|| "XSD validation error".to_string()),
					line: err.line.map(|v| v as usize),
					column: err.col.map(|v| v as usize),
				});
			}
			Ok(out)
		}
	}
}

fn xsd_path_from_env() -> Option<PathBuf> {
	let configured = std::env::var("E2BR3_XSD_PATH").ok().map(PathBuf::from)?;
	if configured.exists() {
		return Some(configured);
	}

	let file_name = configured.file_name()?;
	for root in schema_roots_from_env() {
		if !root.exists() {
			continue;
		}

		let direct = root.join(file_name);
		if direct.exists() {
			return Some(direct);
		}

		let multi_cache = root.join("multicacheschemas").join(file_name);
		if multi_cache.exists() {
			return Some(multi_cache);
		}

		if let Some(found) = find_named_file(&root, file_name, 5) {
			return Some(found);
		}
	}

	Some(configured)
}

fn schema_roots_from_env() -> Vec<PathBuf> {
	let mut roots = Vec::new();
	if let Ok(dir) = std::env::var("E2BR3_SCHEMAS_DIR") {
		roots.push(PathBuf::from(dir));
	}
	roots.push(PathBuf::from("/app/schemas"));
	roots
}

fn find_named_file(
	base: &Path,
	file_name: &OsStr,
	max_depth: usize,
) -> Option<PathBuf> {
	find_named_file_inner(base, file_name, 0, max_depth)
}

fn find_named_file_inner(
	base: &Path,
	file_name: &OsStr,
	depth: usize,
	max_depth: usize,
) -> Option<PathBuf> {
	if depth > max_depth {
		return None;
	}
	let entries = fs::read_dir(base).ok()?;
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_file() && path.file_name() == Some(file_name) {
			return Some(path);
		}
		if path.is_dir() {
			if let Some(found) =
				find_named_file_inner(&path, file_name, depth + 1, max_depth)
			{
				return Some(found);
			}
		}
	}
	None
}

fn validate_e2b_xml_rules(
	xml: &[u8],
	config: &XmlValidatorConfig,
) -> Result<Vec<XmlValidationError>> {
	let xml_str = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let parser = Parser::default();
	let doc = parser
		.parse_string(xml_str)
		.map_err(|err| Error::InvalidXml {
			message: format!("XML parse error: {err}"),
			line: None,
			column: None,
		})?;
	let root = doc.get_root_element().ok_or(Error::MissingRootElement)?;
	let root_name = root.get_name();
	let mut errors = Vec::new();
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

	if let Some(req) = config.require_its_version {
		match root.get_attribute("ITSVersion") {
			Some(value) if value == req => {}
			Some(value) => push_rule_error_with_detail(
				&mut errors,
				"ICH.XML.ROOT.ITSVERSION.REQUIRED",
				&format!("ITSVersion '{value}' does not match required '{req}'"),
			),
			None => push_rule_error_with_detail(
				&mut errors,
				"ICH.XML.ROOT.ITSVERSION.REQUIRED",
				"Missing ITSVersion attribute on root",
			),
		}
	}

	if config.require_schema_location {
		let schema_location = root
			.get_attribute_ns(
				"schemaLocation",
				"http://www.w3.org/2001/XMLSchema-instance",
			)
			.or_else(|| root.get_attribute("xsi:schemaLocation"));

		match schema_location {
			Some(value) => {
				let expected = format!("{root_name}.xsd");
				if !value.contains(&expected) {
					push_rule_error_with_detail(
						&mut errors,
						"ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
						&format!("schemaLocation missing expected '{expected}'"),
					);
				}
			}
			None => push_rule_error_with_detail(
				&mut errors,
				"ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
				"Missing xsi:schemaLocation on root",
			),
		}
	}

	collect_ich_identity_text_errors(&mut xpath, &mut errors);
	collect_ich_profile_value_presence_errors(&mut xpath, &mut errors);
	collect_ich_structural_value_errors(&mut xpath, &mut errors);
	collect_ich_case_history_errors(&mut xpath, &mut errors);

	collect_fda_profile_errors(&mut xpath, &mut errors);

	collect_placeholder_errors(&root, &mut errors);
	Ok(errors)
}

fn collect_placeholder_errors(
	root: &libxml::tree::Node,
	errors: &mut Vec<XmlValidationError>,
) {
	if root.get_type() == Some(libxml::tree::NodeType::ElementNode) {
		let content = root.get_content();
		if looks_placeholder(content.trim()) {
			push_rule_error_with_detail(
				errors,
				"ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
				&format!(
					"Placeholder value not allowed in <{}>: '{}'",
					root.get_name(),
					content.trim()
				),
			);
		}
		for (name, val) in root.get_attributes() {
			if !is_placeholder_checked_attr(&name) {
				continue;
			}
			if looks_placeholder(val.trim()) {
				push_rule_error_with_detail(
					errors,
					"ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
					&format!(
						"Placeholder value not allowed for <{}> attribute {}='{}'",
						root.get_name(),
						name,
						val.trim()
					),
				);
			}
		}
	}

	for child in root.get_child_nodes() {
		collect_placeholder_errors(&child, errors);
	}
}

fn is_placeholder_checked_attr(name: &str) -> bool {
	!matches!(
		name,
		"classCode" | "moodCode" | "typeCode" | "determinerCode"
	)
}

pub(crate) fn push_rule_error(
	errors: &mut Vec<XmlValidationError>,
	code: &str,
	fallback_message: &str,
) {
	push_rule_error_internal(errors, code, fallback_message, None);
}

pub(crate) fn push_rule_error_with_detail(
	errors: &mut Vec<XmlValidationError>,
	code: &str,
	fallback_message: &str,
) {
	push_rule_error_internal(errors, code, fallback_message, Some(fallback_message));
}

fn push_rule_error_internal(
	errors: &mut Vec<XmlValidationError>,
	code: &str,
	fallback_message: &str,
	detail: Option<&str>,
) {
	if !should_emit_rule_error(code) {
		return;
	}
	let message = find_canonical_rule_for_phase(code, ValidationPhase::Import)
		.map(|rule| match detail {
			Some(detail) => format!("[{}] {} ({detail})", rule.code, rule.message),
			None => format!("[{}] {}", rule.code, rule.message),
		})
		.unwrap_or_else(|| format!("[{code}] {fallback_message}"));
	errors.push(XmlValidationError {
		message,
		line: None,
		column: None,
	});
}

fn should_emit_rule_error(code: &str) -> bool {
	match find_canonical_rule_for_phase(code, ValidationPhase::Import) {
		Some(rule) => rule.blocking,
		None => false,
	}
}

pub(crate) fn validate_value_rule_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	value_attr: &str,
	rule_code: &str,
	facts: RuleFacts,
	fallback_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let value = node.get_attribute(value_attr);
		let null_flavor = node.get_attribute("nullFlavor");
		if !is_rule_value_valid(
			rule_code,
			value.as_deref(),
			null_flavor.as_deref(),
			facts,
		) {
			push_rule_error(errors, rule_code, fallback_message);
		}
	});
}

pub(crate) fn validate_attr_null_flavor_pair_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	value_attr: &str,
	required_code: &str,
	required_message: &str,
	forbidden_code: Option<&str>,
	forbidden_message: Option<&str>,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let value = node.get_attribute(value_attr);
		let has_value = value
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();
		if !has_value && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
		if has_value && has_null_flavor {
			if let (Some(code), Some(message)) = (forbidden_code, forbidden_message)
			{
				push_rule_error(errors, code, message);
			}
		}
	});
}

pub(crate) fn validate_attr_or_null_flavor_required_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	value_attr: &str,
	required_code: &str,
	required_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let value = node.get_attribute(value_attr);
		let has_value = value
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();
		if !has_value && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
	});
}

pub(crate) fn validate_attr_or_text_or_null_required_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	value_attr: &str,
	required_code: &str,
	required_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let has_attr = node
			.get_attribute(value_attr)
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_original_text = node.get_child_elements().iter().any(|c| {
			c.get_name() == "originalText" && !c.get_content().trim().is_empty()
		});
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();
		if !has_attr && !has_original_text && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
	});
}

pub(crate) fn validate_code_or_codesystem_or_text_or_null_required_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	required_code: &str,
	required_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let has_code = node
			.get_attribute("code")
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_code_system = node
			.get_attribute("codeSystem")
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_original_text = node.get_child_elements().iter().any(|c| {
			c.get_name() == "originalText" && !c.get_content().trim().is_empty()
		});
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();
		if !has_code && !has_code_system && !has_original_text && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
	});
}

pub(crate) fn validate_code_or_codesystem_or_text_required_with_nullflavor_forbidden_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	required_code: &str,
	required_message: &str,
	forbidden_code: &str,
	forbidden_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let has_code = node
			.get_attribute("code")
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_code_system = node
			.get_attribute("codeSystem")
			.as_deref()
			.map(|v| !v.trim().is_empty())
			.unwrap_or(false);
		let has_original_text = node.get_child_elements().iter().any(|c| {
			c.get_name() == "originalText" && !c.get_content().trim().is_empty()
		});
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();

		if !has_code && !has_code_system && !has_original_text && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
		if has_code && has_null_flavor {
			push_rule_error(errors, forbidden_code, forbidden_message);
		}
	});
}

pub(crate) fn validate_text_null_flavor_pair_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	required_code: &str,
	required_message: &str,
	forbidden_code: Option<&str>,
	forbidden_message: Option<&str>,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let content = node.get_content();
		let has_text = !content.trim().is_empty();
		let has_null_flavor = node.get_attribute("nullFlavor").is_some();
		if !has_text && !has_null_flavor {
			push_rule_error(errors, required_code, required_message);
		}
		if has_text && has_null_flavor {
			if let (Some(code), Some(message)) = (forbidden_code, forbidden_message)
			{
				push_rule_error(errors, code, message);
			}
		}
	});
}

pub(crate) fn validate_attr_prefix_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	attr_name: &str,
	allowed_prefixes: &[&str],
	rule_code: &str,
	value_label: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let Some(value) = node.get_attribute(attr_name) else {
			return;
		};
		if value.trim().is_empty() {
			return;
		}
		if allowed_prefixes
			.iter()
			.any(|prefix| value.starts_with(prefix))
		{
			return;
		}
		let expected = allowed_prefixes.join(", ");
		push_rule_error(
			errors,
			rule_code,
			&format!("{value_label} must start with {expected}, got '{value}'"),
		);
	});
}

pub(crate) fn validate_normalized_code_format_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rule_code: &str,
	spec: ExportNormalizationSpec,
	format_error_message: impl Fn(&str) -> String,
	extra_required_attr: Option<(&str, &str, &'static str)>,
) {
	for_each_xpath_node(xpath, spec.xpath, |node| {
		let Some(code) = node.get_attribute(spec.attribute) else {
			return;
		};
		if !matches_normalization_kind(code.trim(), spec.kind) {
			push_rule_error(errors, rule_code, &format_error_message(&code));
		}
		if let Some((attr, missing_rule, missing_message)) = extra_required_attr {
			let value = node.get_attribute(attr);
			if value.as_deref().unwrap_or("").trim().is_empty() {
				push_rule_error(errors, missing_rule, missing_message);
			}
		}
	});
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

pub(crate) fn for_each_xpath_node(
	xpath: &mut Context,
	node_xpath: &str,
	mut visitor: impl FnMut(libxml::tree::Node),
) {
	if let Ok(nodes) = xpath.findnodes(node_xpath, None) {
		for node in nodes {
			visitor(node);
		}
	}
}

pub(crate) fn xpath_has_nodes(xpath: &mut Context, node_xpath: &str) -> bool {
	xpath
		.findnodes(node_xpath, None)
		.ok()
		.map(|nodes| !nodes.is_empty())
		.unwrap_or(false)
}

pub(crate) fn xpath_any_node(
	xpath: &mut Context,
	node_xpath: &str,
	predicate: impl Fn(&libxml::tree::Node) -> bool,
) -> bool {
	xpath
		.findnodes(node_xpath, None)
		.ok()
		.map(|nodes| nodes.into_iter().any(|n| predicate(&n)))
		.unwrap_or(false)
}

pub(crate) fn xpath_any_value_prefix(
	xpath: &mut Context,
	expr: &str,
	prefix: &str,
) -> bool {
	xpath
		.findvalues(expr, None)
		.ok()
		.map(|vals| vals.iter().any(|v| v.starts_with(prefix)))
		.unwrap_or(false)
}

pub(crate) fn validate_presence_rule(
	errors: &mut Vec<XmlValidationError>,
	rule_code: &str,
	present: bool,
	facts: RuleFacts,
	fallback_message: &str,
) {
	if !is_rule_presence_valid(rule_code, present, facts) {
		push_rule_error(errors, rule_code, fallback_message);
	}
}

pub(crate) fn validate_condition_rule_violation(
	errors: &mut Vec<XmlValidationError>,
	rule_code: &str,
	facts: RuleFacts,
	fallback_message: &str,
) {
	if is_rule_condition_satisfied(rule_code, facts) {
		push_rule_error(errors, rule_code, fallback_message);
	}
}

pub(crate) fn validate_required_child_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	parent_xpath: &str,
	required_child_name: &str,
	rule_code: &str,
	fallback_message: &str,
) {
	for_each_xpath_node(xpath, parent_xpath, |node| {
		let has_child = node
			.get_child_elements()
			.into_iter()
			.any(|child| child.get_name() == required_child_name);
		if !has_child {
			push_rule_error(errors, rule_code, fallback_message);
		}
	});
}

pub(crate) fn validate_required_attrs_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	required_attrs: &[&str],
	rule_code: &str,
	fallback_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let missing = required_attrs.iter().any(|attr| {
			node.get_attribute(attr)
				.as_deref()
				.map(|v| v.trim().is_empty())
				.unwrap_or(true)
		});
		if missing {
			push_rule_error(errors, rule_code, fallback_message);
		}
	});
}

pub(crate) fn validate_when_child_present_require_any_children(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	trigger_child_name: &str,
	required_child_names: &[&str],
	rule_code: &str,
	fallback_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		let children = node.get_child_elements();
		let has_trigger = children
			.iter()
			.any(|child| child.get_name() == trigger_child_name);
		if !has_trigger {
			return;
		}
		let has_required = children
			.iter()
			.any(|child| required_child_names.contains(&child.get_name().as_str()));
		if !has_required {
			push_rule_error(errors, rule_code, fallback_message);
		}
	});
}

pub(crate) fn validate_when_attr_equals_require_any_children(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	attr_name: &str,
	expected_attr_value: &str,
	required_child_names: &[&str],
	rule_code: &str,
	fallback_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		if node.get_attribute(attr_name).as_deref() != Some(expected_attr_value) {
			return;
		}
		let children = node.get_child_elements();
		let has_required = children
			.iter()
			.any(|child| required_child_names.contains(&child.get_name().as_str()));
		if !has_required {
			push_rule_error(errors, rule_code, fallback_message);
		}
	});
}

fn xsi_type_of(node: &libxml::tree::Node) -> Option<String> {
	node.get_attribute_ns("type", "http://www.w3.org/2001/XMLSchema-instance")
		.or_else(|| node.get_attribute("xsi:type"))
}

pub(crate) fn validate_supported_xsi_types_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	allowed_types: &[&str],
	rule_code: &str,
	fallback_message_prefix: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		if let Some(xsi_type) = xsi_type_of(&node) {
			if !allowed_types.contains(&xsi_type.as_str()) {
				push_rule_error(
					errors,
					rule_code,
					&format!("{fallback_message_prefix} '{xsi_type}'"),
				);
			}
		}
	});
}

pub(crate) fn validate_typed_children_attrs_or_nullflavor_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	required_xsi_type: &str,
	child_names: &[&str],
	required_attrs: &[&str],
	component_required_rule_code: &str,
	component_required_message: &str,
	attr_rule_code: &str,
	attr_rule_message: &str,
) {
	for_each_xpath_node(xpath, node_xpath, |node| {
		if xsi_type_of(&node).as_deref() != Some(required_xsi_type) {
			return;
		}
		let mut has_any = false;
		for child in node.get_child_elements() {
			if !child_names.contains(&child.get_name().as_str()) {
				continue;
			}
			has_any = true;
			let missing_attr = required_attrs.iter().any(|attr| {
				child
					.get_attribute(attr)
					.as_deref()
					.map(|v| v.trim().is_empty())
					.unwrap_or(true)
			});
			let has_null_flavor = child.get_attribute("nullFlavor").is_some();
			if missing_attr && !has_null_flavor {
				push_rule_error(errors, attr_rule_code, attr_rule_message);
			}
		}
		if !has_any {
			push_rule_error(
				errors,
				component_required_rule_code,
				component_required_message,
			);
		}
	});
}

fn looks_placeholder(value: &str) -> bool {
	let v = value.trim();
	if v.is_empty() {
		return false;
	}
	if v.chars().any(|c| c.is_whitespace()) {
		return false;
	}
	if v.len() > 24 {
		return false;
	}
	let mut chars = v.chars();
	let Some(first) = chars.next() else {
		return false;
	};
	if !first.is_ascii_uppercase() {
		return false;
	}
	if !v.contains('.') {
		return false;
	}
	if !v.chars().any(|c| c.is_ascii_digit()) {
		return false;
	}
	v.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}
