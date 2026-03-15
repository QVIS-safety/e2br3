// Section C exporter (Safety Report Identification) - FDA mapping.

use crate::model::case::Case;
use crate::model::message_header::MessageHeader;
use crate::model::safety_report::{SafetyReportIdentification, SenderInformation};
use crate::xml::raw::patch::{patch_c_safety_report, CSafetyReportPatch};
use crate::xml::Result;
use libxml::parser::Parser;

pub fn export_c_safety_report_patch(
	raw_xml: &[u8],
	case: &Case,
	report: &SafetyReportIdentification,
	header: Option<&MessageHeader>,
	sender: Option<&SenderInformation>,
) -> Result<String> {
	// C.1 / C.1.1 / C.1.2 / C.1.4 - expedited/report-type local overrides.
	let combination_true = report
		.combination_product_report_indicator
		.as_deref()
		.map(is_true_like)
		.unwrap_or(false);
	let local_criteria_report_type =
		if !report.fulfil_expedited_criteria && !combination_true {
			Some("2")
		} else {
			report.local_criteria_report_type.as_deref()
		};

	let patch = CSafetyReportPatch {
		// C.1.8 / message envelope linkage.
		report_unique_id: &case.safety_report_id,
		transmission_date: report.transmission_date,
		transmission_date_null_flavor: report
			.transmission_date_null_flavor
			.as_deref(),
		transmission_date_value: header.map(|h| h.message_date.as_str()),
		transmission_date_time: header.and_then(|h| h.batch_transmission_date),

		// C.1.3 / C.1.5 / C.1.6 / C.1.7 - core report dates and type.
		report_type: &report.report_type,
		date_first_received: report.date_first_received_from_source,
		date_first_received_null_flavor: report
			.date_first_received_from_source_null_flavor
			.as_deref(),
		date_most_recent: report.date_of_most_recent_information,
		date_most_recent_null_flavor: report
			.date_of_most_recent_information_null_flavor
			.as_deref(),
		fulfil_expedited: report.fulfil_expedited_criteria,
		additional_documents_available: report.additional_documents_available,
		worldwide_unique_id: report.worldwide_unique_id.as_deref(),
		first_sender_type: report.first_sender_type.as_deref(),
		local_criteria_report_type,
		combination_product_indicator: report
			.combination_product_report_indicator
			.as_deref(),
		nullification_code: report.nullification_code.as_deref(),
		nullification_reason: report.nullification_reason.as_deref(),

		// C.2.r.* - sender details.
		sender_type: sender.and_then(|s| Some(s.sender_type.as_str())),
		sender_org_name: sender.and_then(|s| Some(s.organization_name.as_str())),
		sender_department: sender.and_then(|s| s.department.as_deref()),
		sender_street_address: sender.and_then(|s| s.street_address.as_deref()),
		sender_city: sender.and_then(|s| s.city.as_deref()),
		sender_state: sender.and_then(|s| s.state.as_deref()),
		sender_postcode: sender.and_then(|s| s.postcode.as_deref()),
		sender_country_code: sender.and_then(|s| s.country_code.as_deref()),
		sender_person_title: sender.and_then(|s| s.person_title.as_deref()),
		sender_person_given_name: sender
			.and_then(|s| s.person_given_name.as_deref()),
		sender_person_middle_name: sender
			.and_then(|s| s.person_middle_name.as_deref()),
		sender_person_family_name: sender
			.and_then(|s| s.person_family_name.as_deref()),
		sender_telephone: sender.and_then(|s| s.telephone.as_deref()),
		sender_fax: sender.and_then(|s| s.fax.as_deref()),
		sender_email: sender.and_then(|s| s.email.as_deref()),
	};

	patch_c_safety_report(raw_xml, &patch)
}

fn is_true_like(value: &str) -> bool {
	matches!(
		value.trim().to_ascii_lowercase().as_str(),
		"true" | "1" | "y" | "yes"
	)
}

/// Build a minimal ICSR XML skeleton and populate Section C using mapping-driven patching.
pub fn export_c_safety_report_xml(
	case: &Case,
	report: &SafetyReportIdentification,
	header: Option<&MessageHeader>,
	sender: Option<&SenderInformation>,
) -> Result<String> {
	let base_xml = base_icrs_skeleton();
	let parser = Parser::default();
	let doc = parser.parse_string(base_xml).map_err(|err| {
		crate::xml::error::Error::InvalidXml {
			message: format!("XML parse error (base skeleton): {err}"),
			line: None,
			column: None,
		}
	})?;
	let raw = doc.to_string();
	export_c_safety_report_patch(raw.as_bytes(), case, report, header, sender)
}

fn base_icrs_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}
