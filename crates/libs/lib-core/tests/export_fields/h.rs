use super::support::{
	begin_export_test, create_case_with_safety_report, export_base_xml,
	export_for_case, finish_export_test, parse_xpath, set_validated_raw_xml_case,
};
use crate::common::Result;
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, NarrativeInformationBmc,
	NarrativeInformationForCreate, NarrativeInformationForUpdate,
	SenderDiagnosisBmc, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};

#[tokio::test]
async fn export_h_patches_narrative_sender_diagnosis_and_summary() -> Result<()> {
	std::env::set_var("XML_V2_PATCH_H", "1");
	let (ctx, mm) = begin_export_test().await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let narrative_id = NarrativeInformationBmc::create(
		&ctx,
		&mm,
		NarrativeInformationForCreate {
			case_id,
			case_narrative: "Narrative".to_string(),
		},
	)
	.await?;
	NarrativeInformationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		NarrativeInformationForUpdate {
			case_narrative: None,
			reporter_comments: Some("Reporter comment".to_string()),
			sender_comments: Some("Sender comment".to_string()),
		},
	)
	.await?;
	let diagnosis_id = SenderDiagnosisBmc::create(
		&ctx,
		&mm,
		SenderDiagnosisForCreate {
			narrative_id,
			sequence_number: 1,
			diagnosis_meddra_code: Some("10047319".to_string()),
		},
	)
	.await?;
	SenderDiagnosisBmc::update(
		&ctx,
		&mm,
		diagnosis_id,
		SenderDiagnosisForUpdate {
			diagnosis_meddra_version: Some("12.0".to_string()),
			diagnosis_meddra_code: None,
		},
	)
	.await?;
	let summary_id = CaseSummaryInformationBmc::create(
		&ctx,
		&mm,
		CaseSummaryInformationForCreate {
			narrative_id,
			sequence_number: 1,
			summary_text: Some("Summary text".to_string()),
		},
	)
	.await?;
	CaseSummaryInformationBmc::update(
		&ctx,
		&mm,
		summary_id,
		CaseSummaryInformationForUpdate {
			summary_type: Some("1".to_string()),
			language_code: Some("en".to_string()),
			summary_text: None,
		},
	)
	.await?;

	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(
		&ctx, &mm, case_id, &raw_xml, false, false, false, false, false, true,
	)
	.await?;
	let xml = export_for_case(&ctx, &mm, case_id).await?;
	finish_export_test(&mm).await?;

	let (_doc, mut xpath) = parse_xpath(&xml);
	assert_eq!(
		xpath
			.findvalue("//hl7:investigationEvent/hl7:text", None)
			.unwrap(),
		"Narrative"
	);
	assert_eq!(
		xpath.findvalue("//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='10'] and hl7:author/hl7:assignedEntity/hl7:code[@code='3']]/hl7:value", None).unwrap(),
		"Reporter comment"
	);
	assert_eq!(
		xpath.findvalue("//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='10'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]/hl7:value", None).unwrap(),
		"Sender comment"
	);
	assert_eq!(
		xpath.findvalue("//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]/hl7:value/@code", None).unwrap(),
		"10047319"
	);
	assert_eq!(
		xpath.findvalue("//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value", None).unwrap(),
		"Summary text"
	);
	assert_eq!(
		xpath.findvalue("//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:author/hl7:assignedEntity/hl7:code/@code", None).unwrap(),
		"1"
	);
	Ok(())
}
