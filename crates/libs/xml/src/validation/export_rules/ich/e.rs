use crate::{Result, XmlValidationError};
use libxml::xpath::Context;

const REACTION: &str = "/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:subject/hl7:investigationEvent[not(.//hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19'] and hl7:value[@codeSystem='2.16.840.1.113883.6.163' and normalize-space(@code) != '']])]";
const REACTION_VERSION: &str = "/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:subject/hl7:investigationEvent[not(.//hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19'] and hl7:value[@codeSystem='2.16.840.1.113883.6.163' and normalize-space(@codeSystemVersion) != '']])]";

pub(super) fn run(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) -> Result<()> {
	e_i_2_1a(xpath, errors)?;
	e_i_2_1b(xpath, errors)
}

fn e_i_2_1a(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) -> Result<()> {
	super::super::reject(
		xpath,
		errors,
		"E.i.2.1a",
		REACTION_VERSION,
		"E.i.2.1a MedDRA version is required for a reaction/event.",
	)
}

fn e_i_2_1b(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) -> Result<()> {
	super::super::reject(
		xpath,
		errors,
		"E.i.2.1b",
		REACTION,
		"E.i.2.1b MedDRA code is required for a reaction/event.",
	)
}
