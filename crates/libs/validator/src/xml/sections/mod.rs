pub(crate) mod c;
pub(crate) mod d;
pub(crate) mod e;
pub(crate) mod f;
pub(crate) mod g;
pub(crate) mod h;
pub(crate) mod n;

use lib_core::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) fn error_owned_by_section_letter(
	error: &XmlValidationError,
	section_letter: char,
) -> bool {
	error
		.code
		.as_deref()
		.map(|code| code.contains(&format!(".{section_letter}.")))
		.unwrap_or(false)
}

fn collect_generic_xml_errors(xpath: &mut Context) -> Vec<XmlValidationError> {
	let mut collected = Vec::new();
	crate::xml::ich_profile::collect_ich_identity_text_errors(xpath, &mut collected);
	crate::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	crate::xml::ich_profile::collect_ich_structural_value_errors(
		xpath,
		&mut collected,
	);
	crate::xml::ich_profile::collect_ich_case_history_errors(xpath, &mut collected);
	crate::xml::fda_profile::collect_fda_profile_errors(xpath, &mut collected);
	collected
		.into_iter()
		.filter(|error| error.section.as_deref() == Some("xml"))
		.collect()
}

pub(crate) fn collect_business_rule_errors(
	xpath: &mut Context,
) -> Vec<XmlValidationError> {
	let mut errors = collect_generic_xml_errors(xpath);
	c::collect(xpath, &mut errors);
	d::collect(xpath, &mut errors);
	e::collect(xpath, &mut errors);
	f::collect(xpath, &mut errors);
	g::collect(xpath, &mut errors);
	h::collect(xpath, &mut errors);
	n::collect(xpath, &mut errors);
	errors
}
