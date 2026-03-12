use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate,
};
use lib_core::model::message_header::{MessageHeaderBmc, MessageHeaderForCreate};
use lib_core::model::narrative::{
	CaseSummaryInformationBmc, CaseSummaryInformationForCreate,
	NarrativeInformationBmc, NarrativeInformationForCreate, SenderDiagnosisBmc,
	SenderDiagnosisForCreate,
};
use lib_core::model::patient::{
	MedicalHistoryEpisodeBmc, MedicalHistoryEpisodeForCreate, PatientInformationBmc,
	PatientInformationForCreate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::safety_report::{
	LiteratureReferenceBmc, LiteratureReferenceForCreate, PrimarySourceBmc,
	PrimarySourceForCreate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SenderInformationBmc,
	SenderInformationForCreate,
};
use lib_core::model::test_result::{TestResultBmc, TestResultForCreate};
use lib_core::model::Error as ModelError;
use serial_test::serial;
use sqlx::types::time::Date;
use time::Month;

// ============================================================================
// SECTION 1: Case Cascade Delete Tests
// Deleting a case should cascade delete all related child records
// ============================================================================

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_message_header() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create message header
	let header_c = MessageHeaderForCreate {
		case_id,
		message_number: format!("MSG-CASCADE-{case_id}"),
		message_sender_identifier: "SENDER-1".to_string(),
		message_receiver_identifier: "RECEIVER-1".to_string(),
		message_date: "20240101120000".to_string(),
	};
	let header_id = MessageHeaderBmc::create(&ctx, &mm, header_c).await?;

	// Verify header exists
	let header = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(header.id, header_id);

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify header is also deleted (cascade)
	let result = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await;
	assert!(
		result.is_err(),
		"message header should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_drug_information() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug information
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Cascade Test Drug".to_string(),
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Verify drug exists
	let drug = DrugInformationBmc::get(&ctx, &mm, drug_id).await?;
	assert_eq!(drug.medicinal_product, "Cascade Test Drug");

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify drug is also deleted
	let result = DrugInformationBmc::get(&ctx, &mm, drug_id).await;
	assert!(result.is_err(), "drug should be cascade deleted with case");

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_reactions() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create reaction
	let reaction_c = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Cascade Test Reaction".to_string(),
	};
	let reaction_id = ReactionBmc::create(&ctx, &mm, reaction_c).await?;

	// Verify reaction exists
	let reaction = ReactionBmc::get(&ctx, &mm, reaction_id).await?;
	assert_eq!(reaction.primary_source_reaction, "Cascade Test Reaction");

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify reaction is also deleted
	let result = ReactionBmc::get(&ctx, &mm, reaction_id).await;
	assert!(
		result.is_err(),
		"reaction should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_patient_information() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create patient
	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("CT".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;

	// Verify patient exists
	let patient = PatientInformationBmc::get(&ctx, &mm, patient_id).await?;
	assert_eq!(patient.patient_initials.as_deref(), Some("CT"));

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify patient is also deleted
	let result = PatientInformationBmc::get(&ctx, &mm, patient_id).await;
	assert!(
		result.is_err(),
		"patient should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_test_results() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create test result
	let test_c = TestResultForCreate {
		case_id,
		sequence_number: 1,
		test_name: "Cascade Test Result".to_string(),
	};
	let test_id = TestResultBmc::create(&ctx, &mm, test_c).await?;

	// Verify test exists
	let test = TestResultBmc::get(&ctx, &mm, test_id).await?;
	assert_eq!(test.test_name, "Cascade Test Result");

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify test is also deleted
	let result = TestResultBmc::get(&ctx, &mm, test_id).await;
	assert!(
		result.is_err(),
		"test result should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_safety_report_identification() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create safety report identification
	let report_c = SafetyReportIdentificationForCreate {
		case_id,
		transmission_date: Date::from_calendar_date(2024, Month::January, 1)?,
		report_type: "1".to_string(),
		date_first_received_from_source: Date::from_calendar_date(
			2024,
			Month::January,
			1,
		)?,
		date_of_most_recent_information: Date::from_calendar_date(
			2024,
			Month::January,
			1,
		)?,
		fulfil_expedited_criteria: true,
	};
	let report_id =
		SafetyReportIdentificationBmc::create(&ctx, &mm, report_c).await?;

	// Verify report exists
	let report =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(report.id, report_id);

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify report is also deleted
	let result =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await;
	assert!(
		result.is_err(),
		"safety report should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_to_narrative() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create narrative
	let narrative_c = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Cascade test narrative".to_string(),
	};
	let narrative_id =
		NarrativeInformationBmc::create(&ctx, &mm, narrative_c).await?;

	// Verify narrative exists
	let narrative = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(narrative.id, narrative_id);

	// Delete case
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	// Verify narrative is also deleted
	let result = NarrativeInformationBmc::get_by_case(&ctx, &mm, case_id).await;
	assert!(
		result.is_err(),
		"narrative should be cascade deleted with case"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 2: Deep Cascade Delete Tests (Parent -> Child -> Grandchild)
// ============================================================================

#[serial]
#[tokio::test]
async fn test_drug_delete_cascades_to_dosage_and_substances() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Parent Drug".to_string(),
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Create child: dosage
	let dosage_c = DosageInformationForCreate {
		drug_id,
		sequence_number: 1,
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_value: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_time: None,
		last_administration_date: None,
		last_administration_time: None,
		duration_value: None,
		duration_unit: None,
		batch_lot_number: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	};
	let dosage_id = DosageInformationBmc::create(&ctx, &mm, dosage_c).await?;

	// Create child: active substance
	let substance_c = DrugActiveSubstanceForCreate {
		drug_id,
		sequence_number: 1,
		substance_name: Some("Test Substance".to_string()),
		substance_termid: None,
		substance_termid_version: None,
		strength_value: None,
		strength_unit: None,
	};
	let substance_id =
		DrugActiveSubstanceBmc::create(&ctx, &mm, substance_c).await?;

	// Create child: indication
	let indication_c = DrugIndicationForCreate {
		drug_id,
		sequence_number: 1,
		indication_text: Some("Test Indication".to_string()),
		indication_meddra_version: None,
		indication_meddra_code: None,
	};
	let indication_id = DrugIndicationBmc::create(&ctx, &mm, indication_c).await?;

	// Delete drug
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;

	// Verify all children are deleted
	let dosage_result = DosageInformationBmc::get(&ctx, &mm, dosage_id).await;
	assert!(dosage_result.is_err(), "dosage should be cascade deleted");

	let substance_result =
		DrugActiveSubstanceBmc::get(&ctx, &mm, substance_id).await;
	assert!(
		substance_result.is_err(),
		"substance should be cascade deleted"
	);

	let indication_result = DrugIndicationBmc::get(&ctx, &mm, indication_id).await;
	assert!(
		indication_result.is_err(),
		"indication should be cascade deleted"
	);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_patient_delete_cascades_to_medical_history() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create patient
	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("PD".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;

	// Create child: medical history
	let history_c = MedicalHistoryEpisodeForCreate {
		patient_id,
		sequence_number: 1,
		meddra_code: Some("12345678".to_string()),
	};
	let history_id = MedicalHistoryEpisodeBmc::create(&ctx, &mm, history_c).await?;

	// Delete patient
	PatientInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;

	// Verify medical history is deleted
	let history_result = MedicalHistoryEpisodeBmc::get(&ctx, &mm, history_id).await;
	assert!(
		history_result.is_err(),
		"medical history should be cascade deleted"
	);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_narrative_delete_cascades_to_sender_diagnosis() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create narrative
	let narrative_c = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Test narrative".to_string(),
	};
	let narrative_id =
		NarrativeInformationBmc::create(&ctx, &mm, narrative_c).await?;

	// Create child: sender diagnosis
	let diagnosis_c = SenderDiagnosisForCreate {
		narrative_id,
		sequence_number: 1,
		diagnosis_meddra_code: Some("12345678".to_string()),
	};
	let diagnosis_id = SenderDiagnosisBmc::create(&ctx, &mm, diagnosis_c).await?;

	// Create child: case summary
	let summary_c = CaseSummaryInformationForCreate {
		narrative_id,
		sequence_number: 1,
		summary_text: Some("Test summary".to_string()),
	};
	let summary_id = CaseSummaryInformationBmc::create(&ctx, &mm, summary_c).await?;

	// Delete narrative
	NarrativeInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;

	// Verify children are deleted
	let diagnosis_result = SenderDiagnosisBmc::get(&ctx, &mm, diagnosis_id).await;
	assert!(
		diagnosis_result.is_err(),
		"sender diagnosis should be cascade deleted"
	);

	let summary_result = CaseSummaryInformationBmc::get(&ctx, &mm, summary_id).await;
	assert!(
		summary_result.is_err(),
		"case summary should be cascade deleted"
	);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 3: Full Case Delete with All Children
// ============================================================================

#[serial]
#[tokio::test]
async fn test_case_delete_cascades_all_children_comprehensive() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create message header
	let header_c = MessageHeaderForCreate {
		case_id,
		message_number: format!("MSG-FULL-{case_id}"),
		message_sender_identifier: "SENDER-1".to_string(),
		message_receiver_identifier: "RECEIVER-1".to_string(),
		message_date: "20240101120000".to_string(),
	};
	MessageHeaderBmc::create(&ctx, &mm, header_c).await?;

	// Create safety report
	let report_c = SafetyReportIdentificationForCreate {
		case_id,
		transmission_date: Date::from_calendar_date(2024, Month::January, 1)?,
		report_type: "1".to_string(),
		date_first_received_from_source: Date::from_calendar_date(
			2024,
			Month::January,
			1,
		)?,
		date_of_most_recent_information: Date::from_calendar_date(
			2024,
			Month::January,
			1,
		)?,
		fulfil_expedited_criteria: true,
	};
	SafetyReportIdentificationBmc::create(&ctx, &mm, report_c).await?;

	// Create sender info
	let sender_c = SenderInformationForCreate {
		case_id,
		sender_type: "1".to_string(),
		organization_name: "Test Sender".to_string(),
	};
	SenderInformationBmc::create(&ctx, &mm, sender_c).await?;

	// Create primary source
	let primary_c = PrimarySourceForCreate {
		case_id,
		sequence_number: 1,
		qualification: Some("1".to_string()),
		qualification_kr1: None,
	};
	PrimarySourceBmc::create(&ctx, &mm, primary_c).await?;

	// Create literature reference
	let lit_c = LiteratureReferenceForCreate {
		case_id,
		reference_text: "Test Reference".to_string(),
		sequence_number: 1,
		document_base64: None,
		media_type: None,
		representation: None,
		compression: None,
	};
	LiteratureReferenceBmc::create(&ctx, &mm, lit_c).await?;

	// Create patient
	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("FC".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	PatientInformationBmc::create(&ctx, &mm, patient_c).await?;

	// Create reaction
	let reaction_c = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Test Reaction".to_string(),
	};
	ReactionBmc::create(&ctx, &mm, reaction_c).await?;

	// Create drug with children
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	let dosage_c = DosageInformationForCreate {
		drug_id,
		sequence_number: 1,
		dose_value: None,
		dose_unit: None,
		number_of_units: None,
		frequency_value: None,
		frequency_unit: None,
		first_administration_date: None,
		first_administration_time: None,
		last_administration_date: None,
		last_administration_time: None,
		duration_value: None,
		duration_unit: None,
		batch_lot_number: None,
		dosage_text: None,
		dose_form: None,
		dose_form_termid: None,
		dose_form_termid_version: None,
		route_of_administration: None,
		route_termid_version: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		first_administration_date_null_flavor: None,
		last_administration_date_null_flavor: None,
	};
	DosageInformationBmc::create(&ctx, &mm, dosage_c).await?;

	// Create test result
	let test_c = TestResultForCreate {
		case_id,
		sequence_number: 1,
		test_name: "Test Result".to_string(),
	};
	TestResultBmc::create(&ctx, &mm, test_c).await?;

	// Create narrative with children
	let narrative_c = NarrativeInformationForCreate {
		case_id,
		case_narrative: "Test narrative".to_string(),
	};
	let narrative_id =
		NarrativeInformationBmc::create(&ctx, &mm, narrative_c).await?;

	let diagnosis_c = SenderDiagnosisForCreate {
		narrative_id,
		sequence_number: 1,
		diagnosis_meddra_code: Some("12345678".to_string()),
	};
	SenderDiagnosisBmc::create(&ctx, &mm, diagnosis_c).await?;

	// Delete case - this should cascade delete everything
	let delete_result = CaseBmc::delete(&ctx, &mm, case_id).await;
	assert!(delete_result.is_ok(), "case deletion should succeed");

	// Verify case is gone
	let case_result = CaseBmc::get(&ctx, &mm, case_id).await;
	match case_result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "cases");
			assert_eq!(id, case_id);
		}
		_ => return Err("case should not exist after deletion".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}
