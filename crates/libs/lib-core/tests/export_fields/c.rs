use crate::common::Result;
use crate::support::{create_case_with_safety_report, update_safety_report};
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::safety_report::{
	PrimarySourceBmc, PrimarySourceForCreate, SafetyReportIdentificationForUpdate,
	SenderInformationBmc, SenderInformationForCreate, SenderInformationForUpdate,
	StudyInformationBmc, StudyInformationForCreate, StudyRegistrationNumberBmc,
	StudyRegistrationNumberForCreate,
};
use serial_test::serial;
use sqlx::types::Uuid;

use super::support::{
	begin_export_test, export_base_xml, export_for_case, finish_export_test,
	parse_xpath, set_validated_raw_xml_case,
};

#[tokio::test]
#[serial]
async fn export_c_exports_c1_and_c3_fields_in_canonical_order() -> Result<()> {
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
	// C.1.2
	assert_eq!(
		xpath
			.findvalue("//hl7:controlActProcess/hl7:effectiveTime/@value", None)
			.unwrap(),
		"20240102030405"
	);
	// C.1.3
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code",
				None,
			)
			.unwrap(),
		"2"
	);
	// C.1.4
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:effectiveTime/hl7:low/@value",
				None,
			)
			.unwrap(),
		"20240101"
	);
	// C.1.5
	assert_eq!(
		xpath
			.findvalue("//hl7:investigationEvent/hl7:availabilityTime/@value", None,)
			.unwrap(),
		"20240101"
	);
	// C.1.6.1
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"true"
	);
	// C.1.7
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"false"
	);
	// C.1.7.1
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@code",
				None,
			)
			.unwrap(),
		"3"
	);
	// C.1.8.1
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']/@extension",
				None,
			)
			.unwrap(),
		"WW-C-123"
	);
	// C.1.8.2
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
	// C.1.11.1 / C.1.11.2 are compliance-gated and remain absent in this end-to-end path.
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']])",
				None,
			)
			.unwrap(),
		"0"
	);
	// FDA.C.1.12
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"true"
	);
	// C.3.1
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code",
				None,
			)
			.unwrap(),
		"3"
	);
	// C.3.2
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
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:addr/hl7:streetAddressLine",
				None,
			)
			.unwrap(),
		"1 Main St"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:addr/hl7:city",
				None,
			)
			.unwrap(),
		"Seoul"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:addr/hl7:state",
				None,
			)
			.unwrap(),
		"KR-11"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:addr/hl7:postalCode",
				None,
			)
			.unwrap(),
		"12345"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code/@code",
				None,
			)
			.unwrap(),
		"KR"
	);
	// C.3.3
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:prefix",
				None,
			)
			.unwrap(),
		"Dr"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[1]",
				None,
			)
			.unwrap(),
		"Min"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[2]",
				None,
			)
			.unwrap(),
		"Ji"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:family",
				None,
			)
			.unwrap(),
		"Kim"
	);
	// C.3.4
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom[starts-with(@value,'tel:')]/@value",
				None,
			)
			.unwrap(),
		"tel:01012345678"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom[starts-with(@value,'fax:')]/@value",
				None,
			)
			.unwrap(),
		"fax:0212345678"
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
#[serial]
async fn export_c_exports_primary_source_fields_in_canonical_order() -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	PrimarySourceBmc::create(
		&ctx,
		&mm,
		PrimarySourceForCreate {
			case_id,
			sequence_number: 1,
			reporter_title: Some("Prof".to_string()),
			reporter_given_name: Some("Jane".to_string()),
			reporter_middle_name: Some("A".to_string()),
			reporter_family_name: Some("Doe".to_string()),
			organization: Some("ACME Hospital".to_string()),
			department: Some("PV Dept".to_string()),
			street: Some("2 Safety Rd".to_string()),
			city: Some("Busan".to_string()),
			state: Some("KR-26".to_string()),
			postcode: Some("54321".to_string()),
			telephone: Some("0511234567".to_string()),
			country_code: Some("KR".to_string()),
			email: Some("jane.doe@example.com".to_string()),
			qualification: Some("1".to_string()),
			qualification_kr1: None,
			primary_source_regulatory: Some("1".to_string()),
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
	let base = "//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";

	// C.2.r.1
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
				None
			)
			.unwrap(),
		"Prof"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[1]"),
				None
			)
			.unwrap(),
		"Jane"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[2]"),
				None
			)
			.unwrap(),
		"A"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:assignedPerson/hl7:name/hl7:family"),
				None
			)
			.unwrap(),
		"Doe"
	);
	// C.2.r.2
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:representedOrganization/hl7:name"),
				None
			)
			.unwrap(),
		"ACME Hospital / PV Dept"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("{base}/hl7:addr/hl7:streetAddressLine"), None)
			.unwrap(),
		"2 Safety Rd"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("{base}/hl7:addr/hl7:city"), None)
			.unwrap(),
		"Busan"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("{base}/hl7:addr/hl7:state"), None)
			.unwrap(),
		"KR-26"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("{base}/hl7:addr/hl7:postalCode"), None)
			.unwrap(),
		"54321"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:telecom[starts-with(@value,'tel:')]/@value"),
				None,
			)
			.unwrap(),
		"tel:0511234567"
	);
	// C.2.r.3
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code/@code"),
				None,
			)
			.unwrap(),
		"KR"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("{base}/hl7:telecom[starts-with(@value,'mailto:')]/@value"),
				None,
			)
			.unwrap(),
		"mailto:jane.doe@example.com"
	);
	// C.2.r.4
	assert_eq!(
		xpath
			.findvalue(
				&format!(
					"{base}/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code/@code"
				),
				None,
			)
			.unwrap(),
		"1"
	);
	// MFDS.C.2.r.4.KR.1 currently has no canonical FDA XML export target and remains unsupported.
	// C.2.r.5
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='2']]/hl7:priorityNumber/@value",
				None,
			)
			.unwrap(),
		"1"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_c_exports_study_fields_in_canonical_order() -> Result<()> {
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let study_id = StudyInformationBmc::create(
		&ctx,
		&mm,
		StudyInformationForCreate {
			case_id,
			study_name: Some("Study Alpha".to_string()),
			sponsor_study_number: Some("CT-01-23".to_string()),
			study_type_reaction: Some("2".to_string()),
			study_type_reaction_kr1: None,
		},
	)
	.await?;
	StudyRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study_id,
			registration_number: "REG-001".to_string(),
			country_code: Some("KR".to_string()),
			sequence_number: 1,
		},
	)
	.await?;
	StudyRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study_id,
			registration_number: "REG-002".to_string(),
			country_code: Some("US".to_string()),
			sequence_number: 2,
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

	// C.5.2
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:title",
				None,
			)
			.unwrap(),
		"Study Alpha"
	);
	// C.5.3
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.5']/@extension",
				None,
			)
			.unwrap(),
		"CT-01-23"
	);
	// C.5.4
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:code/@code",
				None,
			)
			.unwrap(),
		"2"
	);
	// MFDS.C.5.4.KR.1 currently has no canonical FDA XML export target and remains unsupported.
	// C.5.5.r
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:authorization)",
				None,
			)
			.unwrap(),
		"2"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:authorization[1]/hl7:studyRegistration/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.6']/@extension",
				None,
			)
			.unwrap(),
		"REG-001"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:authorization[1]/hl7:studyRegistration/hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code/@code",
				None,
			)
			.unwrap(),
		"KR"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:authorization[2]/hl7:studyRegistration/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.6']/@extension",
				None,
			)
			.unwrap(),
		"REG-002"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf1/hl7:researchStudy/hl7:authorization[2]/hl7:studyRegistration/hl7:author/hl7:territorialAuthority/hl7:governingPlace/hl7:code/@code",
				None,
			)
			.unwrap(),
		"US"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_c_clears_optional_nodes_end_to_end() -> Result<()> {
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
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']])",
				None,
			)
			.unwrap(),
		"1"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']])",
				None,
			)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']])",
				None,
			)
			.unwrap(),
		"0"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_c_exports_report_date_nullflavors_end_to_end() -> Result<()> {
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
