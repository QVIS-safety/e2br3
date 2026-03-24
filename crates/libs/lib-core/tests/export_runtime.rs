mod common;
#[path = "rules/support.rs"]
mod support;

use crate::common::{
	begin_test_ctx, commit_test_ctx, demo_ctx, demo_user_id, init_test_mm,
	set_current_user, Result,
};
use crate::support::{
	blank_safety_report_update, create_case_with_safety_report, update_safety_report,
};
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate, SenderDiagnosisBmc,
	SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
};
use lib_core::model::patient::{
	PatientInformationBmc, PatientInformationForCreate, PatientInformationForUpdate,
};
use lib_core::xml::export_case_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;
use rust_decimal::Decimal;
use serial_test::serial;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;

fn export_base_xml() -> Result<String> {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root");
	Ok(std::fs::read_to_string(
		root.join("docs/refs/instances/FAERS2022Scenario1.xml"),
	)?)
}

async fn set_validated_raw_xml_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	case_id: Uuid,
	raw_xml: &str,
	dirty_c: bool,
	dirty_d: bool,
	dirty_h: bool,
) -> Result<()> {
	CaseBmc::update(
		ctx,
		mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: Some("validated".to_string()),
			validation_profile: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: Some(raw_xml.as_bytes().to_vec()),
			dirty_c: Some(dirty_c),
			dirty_d: Some(dirty_d),
			dirty_e: Some(false),
			dirty_f: Some(false),
			dirty_g: Some(false),
			dirty_h: Some(dirty_h),
		},
	)
	.await?;
	Ok(())
}

fn parse_xpath(xml: &str) -> (libxml::tree::Document, Context) {
	let parser = Parser::default();
	let doc = parser.parse_string(xml).expect("parse xml");
	let xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	xpath
		.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
		.unwrap();
	(doc, xpath)
}

#[tokio::test]
#[serial]
async fn export_runtime_patches_patient_gestation_age_group_and_concomitant(
) -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("JD".to_string()),
			sex: Some("2".to_string()),
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	PatientInformationBmc::update(
		&ctx,
		&mm,
		patient_id,
		PatientInformationForUpdate {
			patient_initials: None,
			patient_given_name: None,
			patient_family_name: None,
			patient_initials_null_flavor: None,
			birth_date: None,
			birth_date_null_flavor: None,
			age_at_time_of_onset: None,
			age_at_time_of_onset_null_flavor: None,
			age_unit: None,
			gestation_period: Some(Decimal::new(10, 0)),
			gestation_period_unit: Some("wk".to_string()),
			age_group: Some("4".to_string()),
			weight_kg: None,
			height_cm: None,
			sex: None,
			sex_null_flavor: None,
			race_code: None,
			ethnicity_code: None,
			last_menstrual_period_date: Some(
				Date::from_calendar_date(2024, Month::January, 1).unwrap(),
			),
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: Some("History".to_string()),
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(&ctx, &mm, case_id, &raw_xml, false, true, false)
		.await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	let (_doc, mut xpath) = parse_xpath(&xml);

	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"10.00"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='16']]/hl7:value/@unit",
				None,
			)
			.unwrap(),
		"wk"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='4']]/hl7:value/@code",
				None,
			)
			.unwrap(),
		"4"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:primaryRole/hl7:subjectOf2/hl7:organizer[hl7:code[@code='1']]/hl7:component/hl7:observation[hl7:code[@code='11']]/hl7:value/@value",
				None,
			)
			.unwrap(),
		"true"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_runtime_patches_sender_diagnosis_in_h_section() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
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
	let raw_xml = export_base_xml()?;
	set_validated_raw_xml_case(&ctx, &mm, case_id, &raw_xml, false, false, true)
		.await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	let (_doc, mut xpath) = parse_xpath(&xml);

	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]/hl7:value/@code",
				None,
			)
			.unwrap(),
		"10047319"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:component/hl7:adverseEventAssessment/hl7:component1/hl7:observationEvent[hl7:code[@code='15'] and hl7:author/hl7:assignedEntity/hl7:code[@code='1']]/hl7:value/@codeSystemVersion",
				None,
			)
			.unwrap(),
		"12.0"
	);
	Ok(())
}

#[tokio::test]
#[serial]
async fn export_runtime_clears_stale_optional_c_nodes() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_with_safety_report(&ctx, &mm).await?;
	update_safety_report(&ctx, &mm, case_id, blank_safety_report_update()).await?;
	let raw_xml = export_base_xml()?
		.replacen(
			"</investigationEvent>",
			"<id root=\"2.16.840.1.113883.3.989.2.1.3.2\" extension=\"STALE-WORLD\"/></investigationEvent>",
			1,
		)
		.replacen(
			"</investigationEvent>",
			"<subjectOf2 typeCode=\"SBJ\"><investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.23\"/><value code=\"1\"/></investigationCharacteristic></subjectOf2><subjectOf2 typeCode=\"SBJ\"><investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\"><code code=\"4\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.23\"/><value><originalText>stale reason</originalText></value></investigationCharacteristic></subjectOf2></investigationEvent>",
			1,
		)
		.replacen(
			"</investigationEvent>",
			"<component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"C54588\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/><value xsi:type=\"CE\" code=\"1\"/></observationEvent></component><component typeCode=\"COMP\"><observationEvent classCode=\"OBS\" moodCode=\"EVN\"><code code=\"C156384\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/><value xsi:type=\"BL\" value=\"true\"/></observationEvent></component></investigationEvent>",
			1,
		);
	set_validated_raw_xml_case(&ctx, &mm, case_id, &raw_xml, true, false, false)
		.await?;

	let xml = export_case_xml(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	let (_doc, mut xpath) = parse_xpath(&xml);

	assert_eq!(
		xpath
			.findnodes(
				"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	assert_eq!(
		xpath
			.findnodes(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C54588' and @codeSystem='2.16.840.1.113883.3.26.1.1']]",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	assert_eq!(
		xpath
			.findnodes(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='C156384' and @codeSystem='2.16.840.1.113883.3.26.1.1']]",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	assert_eq!(
		xpath
			.findnodes(
				"//hl7:component/hl7:observationEvent[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	assert_eq!(
		xpath
			.findnodes(
				"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='3' or @code='4']]",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	assert_eq!(
		xpath
			.findnodes(
				"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]",
				None,
			)
			.unwrap()
			.len(),
		0
	);
	Ok(())
}
