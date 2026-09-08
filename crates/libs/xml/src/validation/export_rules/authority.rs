use crate::XmlValidationError;
use lib_core::regulatory::RegulatoryAuthority;
use libxml::xpath::Context;

pub(super) fn run(
	xpath: &mut Context,
	authority: RegulatoryAuthority,
	errors: &mut Vec<XmlValidationError>,
) {
	match authority {
		RegulatoryAuthority::Fda => reject(xpath, errors, MFDS, "FDA", "MFDS"),
		RegulatoryAuthority::Mfds => reject(xpath, errors, FDA, "MFDS", "FDA"),
		RegulatoryAuthority::Ich => {
			reject(xpath, errors, MFDS, "ICH", "MFDS");
			reject(xpath, errors, FDA, "ICH", "FDA");
		}
	}
}

const MFDS: &str = "//*[@codeSystem and starts-with(@codeSystem, '2.16.840.1.113883.3.989.5.1.10.')] | //*[@root and starts-with(@root, '2.16.840.1.113883.3.989.5.1.10.')]";
const FDA: &str = "//*[@codeSystem and starts-with(@codeSystem, '2.16.840.1.113883.3.989.5.1.2.')] | //*[@root and starts-with(@root, '2.16.840.1.113883.3.989.5.1.2.')] | //hl7:partProduct[@classCode='DEV'] | //hl7:raceCode | //hl7:observation[hl7:code[@code='C16564' or @code='C54588' or @code='C156384' or @code='C17049']] | //hl7:characteristic[hl7:code[@code='C54026' or @code='C54592' or @code='C54451' or @code='C54594' or @code='C54595' or @code='C94031']]";

fn reject(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	expression: &str,
	authority: &str,
	regional_authority: &str,
) {
	if super::matches(xpath, expression) {
		errors.push(XmlValidationError {
			message: format!(
				"XML for {authority} contains {regional_authority} regional fields."
			),
			code: None,
			section: Some("xml".to_string()),
			field_path: None,
			line: None,
			column: None,
		});
	}
}
