use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};

#[tokio::test]
async fn export_f_rebuilds_test_results_in_sequence_order_and_exports_fields(
) -> Result<()> {
	std::env::set_var("XML_V2_PATCH_F", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;

	let second_id = TestResultBmc::create(
		&ctx,
		&mm,
		TestResultForCreate {
			case_id,
			sequence_number: 2,
			test_name: "AST".to_string(),
		},
	)
	.await?;
	TestResultBmc::update(
		&ctx,
		&mm,
		second_id,
		TestResultForUpdate {
			test_name: None,
			test_date: None,
			test_date_null_flavor: Some("UNK".to_string()),
			test_meddra_version: Some("27.0".to_string()),
			test_meddra_code: Some("10003561".to_string()),
			test_result_code: Some("H".to_string()),
			test_result_value: Some("55".to_string()),
			test_result_unit: Some("U/L".to_string()),
			result_unstructured: Some("Above range".to_string()),
			normal_low_value: Some("0".to_string()),
			normal_high_value: Some("40".to_string()),
			comments: Some("Needs follow-up".to_string()),
			more_info_available: Some(true),
		},
	)
	.await?;

	let first_id = TestResultBmc::create(
		&ctx,
		&mm,
		TestResultForCreate {
			case_id,
			sequence_number: 1,
			test_name: "ALT".to_string(),
		},
	)
	.await?;
	TestResultBmc::update(
		&ctx,
		&mm,
		first_id,
		TestResultForUpdate {
			test_name: None,
			test_date: Some(
				time::Date::from_calendar_date(2024, time::Month::January, 3)
					.unwrap(),
			),
			test_date_null_flavor: None,
			test_meddra_version: Some("27.0".to_string()),
			test_meddra_code: Some("10001552".to_string()),
			test_result_code: Some("N".to_string()),
			test_result_value: Some("25".to_string()),
			test_result_unit: Some("U/L".to_string()),
			result_unstructured: Some("Normal".to_string()),
			normal_low_value: Some("0".to_string()),
			normal_high_value: Some("40".to_string()),
			comments: Some("All normal".to_string()),
			more_info_available: Some(false),
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, false, false, true, false, false,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath.findvalue("count(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])", None).unwrap(),
		"2"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[1]//hl7:code/hl7:originalText", None).unwrap(),
		"ALT"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[2]//hl7:code/hl7:originalText", None).unwrap(),
		"AST"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[1]//hl7:effectiveTime/@value", None).unwrap(),
		"20240103"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[2]//hl7:effectiveTime/@nullFlavor", None).unwrap(),
		"UNK"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[1]//hl7:value/@value", None).unwrap(),
		"25"
	);
	assert_eq!(
		xpath.findvalue("(//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='3']])[2]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='11']]/hl7:value/@value", None).unwrap(),
		"true"
	);
	Ok(())
}
