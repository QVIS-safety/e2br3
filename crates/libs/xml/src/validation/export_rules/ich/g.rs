use crate::{Result, XmlValidationError};
use libxml::xpath::Context;

pub(super) fn run(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) -> Result<()> {
	g_k_1(xpath, errors)
}

fn g_k_1(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) -> Result<()> {
	super::super::reject(
		xpath,
		errors,
		"G.k.1",
		"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:subject/hl7:investigationEvent[not(.//hl7:causalityAssessment[hl7:code[@code='20' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19'] and hl7:value[@codeSystem='2.16.840.1.113883.3.989.2.1.1.13' and (@code='1' or @code='3' or @code='4')]])]",
		"Each ICSR must contain at least one suspect, interacting, or not-administered product (G.k.1).",
	)
}
