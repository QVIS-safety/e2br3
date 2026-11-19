// Section H importer (Narrative) - FDA mapping.

use crate::error::Error;
use crate::import_constraint;
use crate::mapping::fda::h_narrative::HNarrativePaths;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	NarrativeInformationBmc, NarrativeInformationForCreate, SenderDiagnosisBmc,
	SenderDiagnosisForCreate,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use libxml::parser::Parser;
use libxml::xpath::Context;

#[derive(Debug)]
pub struct HNarrativeImport {
	pub case_narrative: String,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
}

#[derive(Debug)]
pub struct HSenderDiagnosisImport {
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
}

#[derive(Debug)]
pub struct HCaseSummaryImport {
	pub sequence_number: i32,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
}

pub fn parse_h_narrative(xml: &[u8]) -> Result<Option<HNarrativeImport>> {
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
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let case_narrative = read_h_1(&mut xpath)?;
	let reporter_comments = read_h_2(&mut xpath)?;
	let sender_comments = read_h_4(&mut xpath)?;

	Ok(Some(HNarrativeImport {
		case_narrative,
		reporter_comments,
		sender_comments,
	}))
}

pub fn parse_h_sender_diagnoses(xml: &[u8]) -> Result<Vec<HSenderDiagnosisImport>> {
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
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:component1//hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query sender diagnoses".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		let (diagnosis_meddra_version, diagnosis_meddra_code) =
			read_h_3_r_1(&mut xpath, &node)?;
		items.push(HSenderDiagnosisImport {
			sequence_number: (idx + 1) as i32,
			diagnosis_meddra_version,
			diagnosis_meddra_code,
		});
	}

	Ok(items)
}

pub fn parse_h_case_summaries(xml: &[u8]) -> Result<Vec<HCaseSummaryImport>> {
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
	let mut xpath = Context::new(&doc).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");

	let nodes = xpath
		.findnodes(
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36'] and hl7:author/hl7:assignedEntity/hl7:code[@code='2']]",
			None,
		)
		.map_err(|_| Error::InvalidXml {
			message: "Failed to query case summaries".to_string(),
			line: None,
			column: None,
		})?;

	let mut items = Vec::new();
	for (idx, node) in nodes.into_iter().enumerate() {
		let (language_code, summary_text) = read_h_5_r_1(&mut xpath, &node)?;
		items.push(HCaseSummaryImport {
			sequence_number: (idx + 1) as i32,
			language_code,
			summary_text,
		});
	}

	Ok(items)
}

pub(crate) async fn import_section_h(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: sqlx::types::Uuid,
) -> Result<()> {
	let Some(narrative) = parse_h_narrative(xml)? else {
		return Ok(());
	};
	let sender_diagnoses = parse_h_sender_diagnoses(xml)?;
	let case_summaries = parse_h_case_summaries(xml)?;
	set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role())
		.await
		.map_err(Error::Model)?;
	let narrative_id = NarrativeInformationBmc::create(
		ctx,
		mm,
		NarrativeInformationForCreate {
			case_id,
			source_narrative_presave_id: None,
			case_narrative: narrative.case_narrative,
			reporter_comments: narrative.reporter_comments,
			sender_comments: narrative.sender_comments,
			additional_information: None,
		},
	)
	.await?;
	for item in sender_diagnoses {
		SenderDiagnosisBmc::create(
			ctx,
			mm,
			SenderDiagnosisForCreate {
				narrative_id,
				sequence_number: item.sequence_number,
				diagnosis_meddra_version: item.diagnosis_meddra_version,
				diagnosis_meddra_code: item.diagnosis_meddra_code,
			},
		)
		.await?;
	}
	for item in case_summaries {
		CaseSummaryInformationBmc::create(
			ctx,
			mm,
			CaseSummaryInformationForCreate {
				narrative_id,
				sequence_number: item.sequence_number,
				language_code: item.language_code,
				summary_text: item.summary_text,
			},
		)
		.await?;
	}
	Ok(())
}

/// e2b:H.1
fn read_h_1(xpath: &mut Context) -> Result<String> {
	let value = first_text_root(xpath, HNarrativePaths::CASE_NARRATIVE);
	import_constraint::string(
		"caseNarrative",
		value.as_deref(),
		None,
		input_contracts::generated::h::h_1,
	)?;
	value.ok_or_else(|| Error::InvalidImportRequest {
		message: "case narrative is required".to_string(),
	})
}

/// e2b:H.2
fn read_h_2(xpath: &mut Context) -> Result<Option<String>> {
	input_string(
		first_text_root(xpath, HNarrativePaths::REPORTER_COMMENTS),
		"reporterComments",
		input_contracts::generated::h::h_2,
	)
}

/// e2b:H.3.r.1a
/// e2b:H.3.r.1b
fn read_h_3_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	Ok((
		input_string(
			first_attr(xpath, node, "hl7:value", "codeSystemVersion"),
			"senderDiagnoses[].diagnosisMeddraVersion",
			input_contracts::generated::h::h_3_r_1a,
		)?,
		input_string(
			first_attr(xpath, node, "hl7:value", "code"),
			"senderDiagnoses[].diagnosisMeddraCode",
			input_contracts::generated::h::h_3_r_1b,
		)?,
	))
}

/// e2b:H.4
fn read_h_4(xpath: &mut Context) -> Result<Option<String>> {
	input_string(
		first_text_root(xpath, HNarrativePaths::SENDER_COMMENTS),
		"senderComments",
		input_contracts::generated::h::h_4,
	)
}

/// e2b:H.5.r.1a
/// e2b:H.5.r.1b
fn read_h_5_r_1(
	xpath: &mut Context,
	node: &libxml::tree::Node,
) -> Result<(Option<String>, Option<String>)> {
	let language = normalize_lang3(first_attr(xpath, node, "hl7:value", "language"));
	import_constraint::string(
		"caseSummaryInformation[].languageCode",
		language.as_deref(),
		None,
		input_contracts::generated::h::h_5_r_1b,
	)?;
	Ok((
		language,
		input_string(
			first_text(xpath, node, "hl7:value"),
			"caseSummaryInformation[].summaryText",
			input_contracts::generated::h::h_5_r_1a,
		)?,
	))
}

fn input_string(
	value: Option<String>,
	field: &str,
	check: impl for<'a> Fn(
		input_contracts::FieldInput<'a>,
	) -> Vec<input_contracts::InputIssue>,
) -> Result<Option<String>> {
	import_constraint::string(field, value.as_deref(), None, check)?;
	Ok(value)
}

fn first_text_root(xpath: &mut Context, expr: &str) -> Option<String> {
	let nodes = xpath.findnodes(expr, None).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn normalize_lang3(value: Option<String>) -> Option<String> {
	let v = value?.trim().to_ascii_lowercase();
	if v.len() == 3 && v.chars().all(|c| c.is_ascii_lowercase()) {
		return Some(v);
	}
	None
}

#[cfg(test)]
mod tests {
	use super::normalize_lang3;

	#[test]
	fn keeps_only_iso_639_2_language_codes() {
		assert_eq!(
			normalize_lang3(Some(" ENG ".into())).as_deref(),
			Some("eng")
		);
		assert_eq!(normalize_lang3(Some("en".into())), None);
	}
}

fn first_text(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	expr: &str,
) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		let content = n.get_content();
		if !content.trim().is_empty() {
			return Some(content);
		}
	}
	None
}

fn first_attr(
	xpath: &mut Context,
	node: &libxml::tree::Node,
	expr: &str,
	attr: &str,
) -> Option<String> {
	let nodes = xpath.findnodes(expr, Some(node)).ok()?;
	for n in nodes {
		if let Some(value) = n.get_attribute(attr) {
			if !value.trim().is_empty() {
				return Some(value);
			}
		}
	}
	None
}
