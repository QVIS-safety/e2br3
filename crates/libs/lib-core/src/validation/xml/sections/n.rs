use crate::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const FDA_N_FACT_BATCH_RECEIVER_XPATH: &str =
	"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension";
pub(crate) const FDA_N_FACT_MSG_RECEIVER_XPATH: &str =
	"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension";
pub(crate) const FDA_N_BATCH_RECEIVER_RULE_CODE: &str = "FDA.N.1.4.REQUIRED";
pub(crate) const FDA_N_BATCH_RECEIVER_RULE_MESSAGE: &str =
	"FDA.N.1.4 batch receiver identifier missing";

fn drain_section_errors(
	collected: Vec<XmlValidationError>,
	section_letter: char,
	errors: &mut Vec<XmlValidationError>,
) {
	errors.extend(collected.into_iter().filter(|error| {
		super::error_owned_by_section_letter(error, section_letter)
	}));
}

pub(crate) fn collect(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) {
	let mut collected = Vec::new();
	crate::validation::xml::ich_profile::collect_ich_identity_text_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::fda_profile::collect_fda_profile_errors(
		xpath,
		&mut collected,
	);
	drain_section_errors(collected, 'N', errors);
}
