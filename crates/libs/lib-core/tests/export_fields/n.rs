use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SafetyReportIdentificationForUpdate,
};
use sqlx::types::time::{Date, PrimitiveDateTime, Time};
use sqlx::types::Uuid;
use time::Month;

#[tokio::test]
async fn export_n_patches_message_header_and_receiver_end_to_end() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let message_number = format!("MSG-N-{}", Uuid::new_v4());
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number: message_number.clone(),
			message_sender_identifier: "MSG-SENDER".to_string(),
			message_receiver_identifier: "MSG-RECV".to_string(),
			message_date: "20240102030405".to_string(),
		},
	)
	.await?;
	MessageHeaderBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		MessageHeaderForUpdate {
			batch_number: Some("BATCH-123".to_string()),
			batch_sender_identifier: Some("BATCH-SENDER".to_string()),
			batch_receiver_identifier: Some("BATCH-RECV".to_string()),
			batch_transmission_date: Some(
				PrimitiveDateTime::new(
					Date::from_calendar_date(2024, Month::January, 2).unwrap(),
					Time::from_hms(3, 4, 5).unwrap(),
				)
				.assume_utc(),
			),
			message_number: None,
			message_sender_identifier: None,
			message_receiver_identifier: None,
			message_date: None,
		},
	)
	.await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: Some("2".to_string()),
			organization_name: Some("Receiver Org".to_string()),
		},
	)
	.await?;
	ReceiverInformationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		ReceiverInformationForUpdate {
			receiver_type: None,
			organization_name: None,
			department: Some("PV".to_string()),
			street_address: Some("2 Main St".to_string()),
			city: Some("Busan".to_string()),
			state_province: Some("KR-26".to_string()),
			postcode: Some("54321".to_string()),
			country_code: Some("KR".to_string()),
			telephone: Some("0511234567".to_string()),
			fax: Some("0517654321".to_string()),
			email: Some("receiver@example.com".to_string()),
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
			.findvalue("/hl7:MCCI_IN200100UV01/hl7:id/@extension", None)
			.unwrap(),
		"BATCH-123"
	);
	assert_eq!(
		xpath
			.findvalue(
				"/hl7:MCCI_IN200100UV01/hl7:sender/hl7:device/hl7:id/@extension",
				None
			)
			.unwrap(),
		"BATCH-SENDER"
	);
	assert_eq!(
		xpath
			.findvalue(
				"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension",
				None
			)
			.unwrap(),
		"BATCH-RECV"
	);
	assert_eq!(
		xpath
			.findvalue(
				"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:id/@extension",
				None
			)
			.unwrap(),
		message_number
	);
	assert_eq!(
		xpath
			.findvalue("/hl7:MCCI_IN200100UV01/hl7:name/@displayName", None)
			.unwrap(),
		"ichicsr"
	);
	assert_eq!(
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:code/@code", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name", None).unwrap(),
		"Receiver Org"
	);
	assert_eq!(
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:desc", None).unwrap(),
		"PV"
	);
	assert_eq!(
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:addr/hl7:country/@code", None).unwrap(),
		"KR"
	);
	assert_eq!(
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:telecom[starts-with(@value,'mailto:')]/@value", None).unwrap(),
		"mailto:receiver@example.com"
	);
	Ok(())
}

#[tokio::test]
async fn export_n_falls_back_to_report_receiver_organization() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_C", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number: format!("MSG-N-{}", Uuid::new_v4()),
			message_sender_identifier: "MSG-SENDER".to_string(),
			message_receiver_identifier: "MSG-RECV".to_string(),
			message_date: "20240102030405".to_string(),
		},
	)
	.await?;
	SafetyReportIdentificationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		SafetyReportIdentificationForUpdate {
			transmission_date: None,
			transmission_date_null_flavor: None,
			report_type: None,
			date_first_received_from_source: None,
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: None,
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			worldwide_unique_id: None,
			first_sender_type: None,
			additional_documents_available: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: Some("Header Receiver Org".to_string()),
		},
	)
	.await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: Some("2".to_string()),
			organization_name: None,
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
		xpath.findvalue("/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:asAgent/hl7:representedOrganization/hl7:name", None).unwrap(),
		"Header Receiver Org"
	);
	Ok(())
}
