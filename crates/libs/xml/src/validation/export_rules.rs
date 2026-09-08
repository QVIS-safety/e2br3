use crate::{Error, Result, XmlValidationError};
use lib_core::regulatory::RegulatoryAuthority;
use libxml::parser::Parser;
use libxml::xpath::Context;

mod authority;
mod ich;

/// Validates XML invariants that would make an authority payload internally
/// inconsistent. Case business rules are intentionally not evaluated here.
pub fn validate_export_rules(
	xml: &[u8],
	authority: RegulatoryAuthority,
) -> Result<Vec<XmlValidationError>> {
	let xml = std::str::from_utf8(xml).map_err(|err| Error::InvalidXml {
		message: format!("XML not valid UTF-8: {err}"),
		line: None,
		column: None,
	})?;
	let document =
		Parser::default()
			.parse_string(xml)
			.map_err(|err| Error::InvalidXml {
				message: format!("XML parse error: {err}"),
				line: None,
				column: None,
			})?;
	let mut xpath = Context::new(&document).map_err(|_| Error::InvalidXml {
		message: "Failed to initialize XPath context".to_string(),
		line: None,
		column: None,
	})?;
	let _ = xpath.register_namespace("hl7", "urn:hl7-org:v3");
	let _ =
		xpath.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");

	let mut errors = Vec::new();
	ich::run(&mut xpath, authority, &mut errors);
	authority::run(&mut xpath, authority, &mut errors);
	Ok(errors)
}

fn reject(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	code: &str,
	expression: &str,
	message: &str,
) {
	if matches(xpath, expression) {
		errors.push(XmlValidationError {
			message: message.to_string(),
			code: Some(code.to_string()),
			section: Some("xml".to_string()),
			field_path: None,
			line: None,
			column: None,
		});
	}
}

fn matches(xpath: &mut Context, expression: &str) -> bool {
	!xpath
		.evaluate(expression)
		.expect("static XML-integrity XPath must compile")
		.get_nodes_as_vec()
		.is_empty()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reports_only_xml_integrity_errors() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="message"/><creationTime value="20260810"/><controlActProcess><effectiveTime value="20260811"/><subject><investigationEvent><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="case"/></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let errors = validate_export_rules(xml, RegulatoryAuthority::Ich).unwrap();
		let codes = errors
			.iter()
			.filter_map(|error| error.code.as_deref())
			.filter(|code| code.starts_with("N."))
			.collect::<Vec<_>>();
		assert_eq!(codes, ["N.2.r.1", "N.2.r.4"]);
	}

	#[test]
	fn ignores_case_business_rules() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><name code="wrong"/><PORR_IN049016UV><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="not-a-country-profile"/><creationTime value="20990101"/><controlActProcess><effectiveTime value="20990101"/><subject><investigationEvent><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="not-a-country-profile"/></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let errors = validate_export_rules(xml, RegulatoryAuthority::Ich).unwrap();
		assert!(!errors.iter().any(|error| {
			matches!(error.code.as_deref(), Some("C.1.1" | "C.1.2"))
		}));
	}

	#[test]
	fn regional_fields_are_a_final_authority_invariant() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><raceCode/></MCCI_IN200100UV01>"#;
		assert!(!validate_export_rules(xml, RegulatoryAuthority::Fda)
			.unwrap()
			.iter()
			.any(|error| error.message.contains("regional fields")));
		let errors = validate_export_rules(xml, RegulatoryAuthority::Ich).unwrap();
		assert!(errors.iter().any(
			|error| error.message == "XML for ICH contains FDA regional fields."
		));
	}

	#[test]
	fn rejects_icsr_without_required_reaction_product_and_narrative() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent/></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let errors = validate_export_rules(xml, RegulatoryAuthority::Fda).unwrap();
		assert!(errors
			.iter()
			.any(|error| error.code.as_deref() == Some("ICH.N.1.5.REQUIRED")));
		let codes = errors
			.iter()
			.filter_map(|error| error.code.as_deref())
			.filter(|code| {
				matches!(*code, "E.i.2.1a" | "E.i.2.1b" | "G.k.1" | "H.1")
			})
			.collect::<Vec<_>>();
		assert_eq!(codes, ["E.i.2.1a", "E.i.2.1b", "G.k.1", "H.1"]);
	}

	#[test]
	fn accepts_required_ich_case_content() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><PORR_IN049016UV><controlActProcess><subject><investigationEvent><text>Case narrative</text><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="10019211" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="28.1"/></observation><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/></causalityAssessment></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		let errors = validate_export_rules(xml, RegulatoryAuthority::Fda).unwrap();
		assert!(!errors.iter().any(|error| {
			matches!(
				error.code.as_deref(),
				Some("E.i.2.1a" | "E.i.2.1b" | "G.k.1" | "H.1")
			)
		}));
	}

	#[test]
	fn validates_generated_message_header_only_at_xml_boundary() {
		let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><id extension="batch"/><creationTime value="20260814000000"/><receiver><device><id extension="ZZFDA"/></device></receiver><sender><device><id extension="SENDER"/></device></sender><PORR_IN049016UV><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="case"/><creationTime value="20260814000000"/><receiver><device><id extension="CDER"/></device></receiver><sender><device><id extension="SENDER"/></device></sender><controlActProcess><effectiveTime value="20260814000000"/><subject><investigationEvent><id root="2.16.840.1.113883.3.989.2.1.3.1" extension="case"/><text>Case narrative</text><observation><code code="29" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="10019211" codeSystem="2.16.840.1.113883.6.163" codeSystemVersion="28.1"/></observation><causalityAssessment><code code="20" codeSystem="2.16.840.1.113883.3.989.2.1.1.19"/><value code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.13"/></causalityAssessment></investigationEvent></subject></controlActProcess></PORR_IN049016UV></MCCI_IN200100UV01>"#;
		for (authority, batch_receiver, message_receiver) in [
			(RegulatoryAuthority::Ich, "ICHTEST", "ICHTEST"),
			(RegulatoryAuthority::Fda, "ZZFDA_PREMKT", "CDER_IND"),
			(
				RegulatoryAuthority::Fda,
				"ZZFDA_PREMKT",
				"CDER_IND_EXEMPT_BA_BE",
			),
			(RegulatoryAuthority::Fda, "ZZFDA_PREMKT", "CBER_IND"),
			(RegulatoryAuthority::Fda, "ZZFDA", "CDER"),
			(RegulatoryAuthority::Mfds, "MFDS-O-CT", "MFDS-O-CT"),
			(RegulatoryAuthority::Mfds, "MFDS-O-CU", "MFDS-O-CU"),
			(RegulatoryAuthority::Mfds, "MFDS-O-KR", "MFDS-O-KR"),
			(RegulatoryAuthority::Mfds, "MFDS-O-FR", "MFDS-O-FR"),
			(RegulatoryAuthority::Mfds, "MFDS-O-CF", "MFDS-O-CF"),
		] {
			let candidate = String::from_utf8(xml.to_vec())
				.unwrap()
				.replace(
					"extension=\"ZZFDA\"",
					&format!("extension=\"{batch_receiver}\""),
				)
				.replace(
					"extension=\"CDER\"",
					&format!("extension=\"{message_receiver}\""),
				);
			let errors =
				validate_export_rules(candidate.as_bytes(), authority).unwrap();
			assert!(
				!errors.iter().any(|error| {
					error.code.as_deref().is_some_and(|code| {
						code.contains(".N.")
							|| code.starts_with("N.")
							|| code.starts_with("FDA.R")
							|| code.starts_with("MFDS.N")
					})
				}),
				"{authority:?} {batch_receiver}/{message_receiver}: {errors:?}"
			);
		}

		let invalid_fda = String::from_utf8(xml.to_vec())
			.unwrap()
			.replace("extension=\"ZZFDA\"", "extension=\"ZZFDA_PREMKT\"");
		assert!(validate_export_rules(
			invalid_fda.as_bytes(),
			RegulatoryAuthority::Fda
		)
		.unwrap()
		.iter()
		.any(|error| error.code.as_deref() == Some("FDA.R0007")));
		let future = String::from_utf8(xml.to_vec()).unwrap().replacen(
			"20260814000000",
			"20990101000000",
			1,
		);
		assert!(
			validate_export_rules(future.as_bytes(), RegulatoryAuthority::Fda)
				.unwrap()
				.iter()
				.any(|error| {
					error.code.as_deref() == Some("ICH.N.1.5.FUTURE_DATE.FORBIDDEN")
				})
		);

		let invalid_mfds = String::from_utf8(xml.to_vec())
			.unwrap()
			.replace("extension=\"ZZFDA\"", "extension=\"MFDS-O-CT\"")
			.replace("extension=\"CDER\"", "extension=\"CT\"");
		let errors = validate_export_rules(
			invalid_mfds.as_bytes(),
			RegulatoryAuthority::Mfds,
		)
		.unwrap();
		assert!(errors
			.iter()
			.any(|error| error.code.as_deref() == Some("MFDS.N.2.r.3.ROUTE")));
		assert!(errors
			.iter()
			.any(|error| error.code.as_deref() == Some("MFDS.N.ROUTE.PAIR")));
	}
}
