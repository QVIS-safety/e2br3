use crate::error::Error;
use crate::export::mode::{apply_c_d_h_sections, build_fresh_export_from_db};
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::{self, ModelManager};
use lib_core::regulatory::RegulatoryAuthority;

#[derive(Debug, Clone)]
pub struct OutboundMessageHeader {
	pub batch_number: String,
	pub batch_sender_identifier: String,
	pub batch_receiver_identifier: String,
	pub batch_transmission_date: sqlx::types::time::OffsetDateTime,
	pub message_sender_identifier: String,
	pub message_receiver_identifier: String,
}

#[derive(Debug, Clone)]
pub struct ExportXmlOptions {
	pub apply_comments: bool,
	pub authority: RegulatoryAuthority,
	pub outbound_message_header: OutboundMessageHeader,
}

pub(crate) mod mode;
pub mod policy;
pub mod roundtrip;
pub mod sections;
pub(crate) mod shared;

pub async fn export_case_xml_with_options(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	options: ExportXmlOptions,
) -> Result<String> {
	serialize_case_xml_for_authority(
		ctx,
		mm,
		case_id,
		options.authority,
		&options.outbound_message_header,
	)
	.await
	.map(|xml| apply_export_xml_options(xml, options.apply_comments))
}

pub async fn serialize_case_xml_for_authority(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	authority: RegulatoryAuthority,
	outbound_message_header: &OutboundMessageHeader,
) -> Result<String> {
	let mm = mm.new_with_txn()?;
	mm.dbx().begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let result = async {
		let case = CaseBmc::get(ctx, &mm, case_id).await.map_err(Error::from)?;
		let xml =
			build_fresh_export_from_db(ctx, &mm, case_id, &case, authority).await?;
		apply_c_d_h_sections(
			ctx,
			&mm,
			case_id,
			xml,
			authority,
			outbound_message_header,
		)
		.await
	}
	.await;
	match result {
		Ok(xml) => {
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
			Ok(xml)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(crate) fn base_export_skeleton() -> &'static str {
	include_str!("fixtures/base_export_skeleton.xml")
}

fn apply_export_xml_options(xml: String, apply_comments: bool) -> String {
	if apply_comments {
		xml
	} else {
		strip_xml_comments(&xml)
	}
}

fn strip_xml_comments(xml: &str) -> String {
	let mut output = String::with_capacity(xml.len());
	let mut rest = xml;
	while let Some(start) = rest.find("<!--") {
		output.push_str(&rest[..start]);
		let after_start = &rest[start + 4..];
		if let Some(end) = after_start.find("-->") {
			rest = &after_start[end + 3..];
		} else {
			return output;
		}
	}
	output.push_str(rest);
	output
}

#[cfg(test)]
mod tests {
	use super::{apply_export_xml_options, base_export_skeleton};
	use crate::export::roundtrip::{
		patch_c_safety_report, patch_d_patient, patch_e_reactions,
		patch_f_test_results, patch_g_drugs, patch_h_narrative, CSafetyReportPatch,
		DPatientPatch,
	};
	use lib_core::model::narrative::NarrativeInformation;
	use libxml::parser::Parser;
	use libxml::xpath::Context;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

	#[test]
	fn export_options_preserve_or_strip_comments_without_changing_payload() {
		let xml = "<root><!-- label --><case>value</case><!-- tail --></root>";
		assert_eq!(apply_export_xml_options(xml.to_string(), true), xml);
		assert_eq!(
			apply_export_xml_options(xml.to_string(), false),
			"<root><case>value</case></root>"
		);
	}

	#[test]
	fn fresh_export_skeleton_contains_only_required_parent_structure() {
		let xml = base_export_skeleton();
		assert!(!xml.contains("FAERS2022Scenario"));
		assert!(!xml.contains("US-APHARMA"));
		assert!(!xml.contains("CureAll"));

		let doc = Parser::default().parse_string(xml).expect("parse skeleton");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		for path in [
			"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:controlActProcess/hl7:subject/hl7:investigationEvent",
			"//hl7:adverseEventAssessment",
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity",
		] {
			assert_eq!(xpath.findnodes(path, None).expect("find parent").len(), 1);
		}
	}

	#[test]
	fn fresh_export_skeleton_accepts_all_section_writers() {
		let c = CSafetyReportPatch {
			report_unique_id: "CASE-1",
			transmission_date: Some("20240101"),
			report_type: "1",
			date_first_received: None,
			date_most_recent: None,
			fulfil_expedited: Some(false),
			fulfil_expedited_null_flavor: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			local_criteria_report_type: None,
			combination_product_indicator: None,
			combination_product_indicator_null_flavor: None,
			nullification_code: None,
			nullification_reason: None,
			sender_type: None,
			sender_health_professional_type_kr1: None,
			sender_org_name: None,
			sender_department: None,
			sender_street_address: None,
			sender_city: None,
			sender_state: None,
			sender_postcode: None,
			sender_country_code: None,
			sender_person_title: None,
			sender_person_title_null_flavor: None,
			sender_person_given_name: None,
			sender_person_given_name_null_flavor: None,
			sender_person_middle_name: None,
			sender_person_middle_name_null_flavor: None,
			sender_person_family_name: None,
			sender_person_family_name_null_flavor: None,
			sender_telephone: None,
			sender_fax: None,
			sender_email: None,
		};
		let d = DPatientPatch {
			patient_name: None,
			sex: None,
			birth_date: None,
			age_value: None,
			age_unit: None,
			weight_kg: None,
			height_cm: None,
			date_of_death: None,
			autopsy_performed: None,
			autopsy_performed_null_flavor: None,
			reported_causes: &[],
			autopsy_causes: &[],
		};
		let narrative = NarrativeInformation {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			source_narrative_presave_id: None,
			case_narrative: "Narrative".to_string(),
			reporter_comments: None,
			sender_comments: None,
			additional_information: None,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		};

		let mut xml = patch_c_safety_report(base_export_skeleton().as_bytes(), &c)
			.expect("write C");
		xml = patch_d_patient(xml.as_bytes(), &d).expect("write D");
		xml = patch_e_reactions(xml.as_bytes(), &[]).expect("write E");
		xml = patch_f_test_results(xml.as_bytes(), &[]).expect("write F");
		xml = patch_g_drugs(
			xml.as_bytes(),
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
			&[],
		)
		.expect("write G");
		xml = patch_h_narrative(xml.as_bytes(), &narrative).expect("write H");

		let doc = Parser::default().parse_string(&xml).expect("parse result");
		let mut xpath = Context::new(&doc).expect("xpath");
		xpath
			.register_namespace("hl7", "urn:hl7-org:v3")
			.expect("namespace");
		assert_eq!(
			xpath
				.findvalue("//hl7:investigationEvent/hl7:text", None)
				.expect("narrative"),
			"Narrative"
		);
		assert_eq!(
			xpath
				.findvalue("//hl7:primaryRole/@classCode", None)
				.expect("patient role"),
			"INVSBJ"
		);
	}
}
