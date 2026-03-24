use crate::xml::types::XmlValidationError;
use libxml::xpath::Context;

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
	crate::validation::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	drain_section_errors(collected, 'H', errors);
}
