use crate::common::Result;
use crate::support::{create_case_with_safety_report, update_safety_report};
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::safety_report::{
	SafetyReportIdentificationForUpdate, SenderInformationBmc,
	SenderInformationForCreate, SenderInformationForUpdate,
};
use sqlx::types::Uuid;

use super::support::{
	begin_export_test, export_base_xml, export_for_case, finish_export_test,
	parse_xpath, set_validated_raw_xml_case,
};

#[tokio::test]
async fn export_c_patches_end_to_end_fields() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	update_safety_report(
		&ctx,
		&mm,
		case_id,
		SafetyReportIdentificationForUpdate {
			transmission_date: None,
			transmission_date_null_flavor: None,
			report_type: Some("2".to_string()),
			date_first_received_from_source: None,
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: None,
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria: Some(false),
			local_criteria_report_type: Some("3".to_string()),
			combination_product_report_indicator: Some("true".to_string()),
			worldwide_unique_id: Some("WW-C-123".to_string()),
			first_sender_type: Some("2".to_string()),
			additional_documents_available: Some(true),
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: Some("Receiver Org".to_string()),
		},
	)
	.await?;
	let sender_id = SenderInformationBmc::create(
		&ctx,
		&mm,
		SenderInformationForCreate {
			case_id,
			sender_type: "3".to_string(),
			organization_name: "Sender Org".to_string(),
		},
	)
	.await?;
	SenderInformationBmc::update(
		&ctx,
		&mm,
		sender_id,
		SenderInformationForUpdate {
			sender_type: None,
			organization_name: None,
			department: Some("Safety".to_string()),
			street_address: Some("1 Main St".to_string()),
			city: Some("Seoul".to_string()),
			state: Some("KR-11".to_string()),
			postcode: Some("12345".to_string()),
			country_code: Some("KR".to_string()),
			person_title: Some("Dr".to_string()),
			person_given_name: Some("Min".to_string()),
			person_middle_name: Some("Ji".to_string()),
			person_family_name: Some("Kim".to_string()),
			telephone: Some("01012345678".to_string()),
			fax: Some("0212345678".to_string()),
			email: Some("sender@example.com".to_string()),
		},
	)
	.await?;
	let message_number = format!("MSG-C-{}", Uuid::new_v4());
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number,
			message_sender_identifier: "MSG-SENDER".to_string(),
			message_receiver_identifier: "MSG-RECV".to_string(),
			message_date: "20240102030405".to_string(),
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, true, false, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']/@extension",
				None,
			)
			.unwrap(),
		"WW-C-123"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"true"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
				None,
			)
			.unwrap(),
		"2"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588']]/hl7:value/@code",
				None,
			)
			.unwrap(),
		"3"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"true"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
				None,
			)
			.unwrap(),
		"Safety"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name",
				None,
			)
			.unwrap(),
		"Sender Org"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom[starts-with(@value,'mailto:')]/@value",
				None,
			)
			.unwrap(),
		"mailto:sender@example.com"
	);
	Ok(())
}

#[tokio::test]
async fn export_c_clears_optional_nodes_end_to_end() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, true, false, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2'])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue("count(//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']])", None)
			.unwrap(),
		"0"
	);
	Ok(())
}

#[tokio::test]
async fn export_c_exports_report_date_nullflavors_end_to_end() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	update_safety_report(
		&ctx,
		&mm,
		case_id,
		SafetyReportIdentificationForUpdate {
			transmission_date: None,
			transmission_date_null_flavor: Some("UNK".to_string()),
			report_type: None,
			date_first_received_from_source: None,
			date_first_received_from_source_null_flavor: Some("ASKU".to_string()),
			date_of_most_recent_information: None,
			date_of_most_recent_information_null_flavor: Some("MSK".to_string()),
			fulfil_expedited_criteria: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			additional_documents_available: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, true, false, false, false, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:controlActProcess/hl7:effectiveTime/@nullFlavor",
				None
			)
			.unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:effectiveTime/hl7:low/@nullFlavor",
				None
			)
			.unwrap(),
		"ASKU"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:availabilityTime/@nullFlavor",
				None
			)
			.unwrap(),
		"MSK"
	);
	Ok(())
}
