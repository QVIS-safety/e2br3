use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::safety_report::{
	LiteratureReferenceBmc, LiteratureReferenceForCreate,
	LiteratureReferenceForUpdate, PrimarySourceBmc, PrimarySourceForCreate,
	PrimarySourceForUpdate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SafetyReportIdentificationForUpdate,
	SenderInformationBmc, SenderInformationForCreate, SenderInformationForUpdate,
	StudyInformationBmc, StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumberBmc, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
};
use serial_test::serial;
use sqlx::types::time::Date;
use time::Month;

#[serial]
#[tokio::test]
async fn test_safety_report_identification_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

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
	let report =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(report.id, report_id);

	let report_u = SafetyReportIdentificationForUpdate {
		transmission_date: None,
		report_type: Some("2".to_string()),
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		worldwide_unique_id: Some("WUID-1".to_string()),
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: Some("Receiver".to_string()),
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
	};
	SafetyReportIdentificationBmc::update_by_case(&ctx, &mm, case_id, report_u)
		.await?;
	let report =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(report.report_type, "2");

	SafetyReportIdentificationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_safety_report_nullification_requires_compliance_context() -> Result<()>
{
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

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

	let report_u = SafetyReportIdentificationForUpdate {
		transmission_date: None,
		report_type: None,
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		worldwide_unique_id: None,
		nullification_code: Some("1".to_string()),
		nullification_reason: Some("duplicate report".to_string()),
		receiver_organization: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
	};

	let err =
		SafetyReportIdentificationBmc::update_by_case(&ctx, &mm, case_id, report_u)
			.await
			.expect_err("nullification without compliance context must fail");
	assert!(
		err.to_string()
			.contains("compliance context required for nullification"),
		"unexpected error: {err}"
	);

	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_safety_report_submodels_crud() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	let sender_c = SenderInformationForCreate {
		case_id,
		sender_type: "1".to_string(),
		organization_name: "Demo Sender".to_string(),
	};
	let sender_id = SenderInformationBmc::create(&ctx, &mm, sender_c).await?;
	let sender = SenderInformationBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(sender.organization_name, "Demo Sender");

	let sender_u = SenderInformationForUpdate {
		sender_type: None,
		organization_name: Some("Updated Sender".to_string()),
		department: None,
		street_address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		person_title: None,
		person_given_name: None,
		person_middle_name: None,
		person_family_name: None,
		telephone: None,
		fax: None,
		email: None,
	};
	SenderInformationBmc::update(&ctx, &mm, sender_id, sender_u).await?;
	let sender = SenderInformationBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(sender.organization_name, "Updated Sender");

	let primary_c = PrimarySourceForCreate {
		case_id,
		sequence_number: 1,
		qualification: Some("1".to_string()),
		qualification_kr1: None,
	};
	let primary_id = PrimarySourceBmc::create(&ctx, &mm, primary_c).await?;
	let primary = PrimarySourceBmc::get(&ctx, &mm, primary_id).await?;
	assert_eq!(primary.sequence_number, 1);

	let primary_u = PrimarySourceForUpdate {
		reporter_title: None,
		reporter_given_name: Some("Jane".to_string()),
		reporter_middle_name: None,
		reporter_family_name: Some("Doe".to_string()),
		organization: None,
		department: None,
		street: None,
		city: None,
		state: None,
		postcode: None,
		telephone: None,
		country_code: None,
		email: None,
		qualification: None,
		qualification_kr1: None,
		primary_source_regulatory: Some("1".to_string()),
	};
	PrimarySourceBmc::update(&ctx, &mm, primary_id, primary_u).await?;
	let primary = PrimarySourceBmc::get(&ctx, &mm, primary_id).await?;
	assert_eq!(primary.reporter_family_name.as_deref(), Some("Doe"));

	let lit_c = LiteratureReferenceForCreate {
		case_id,
		reference_text: "Ref 1".to_string(),
		sequence_number: 1,
		document_base64: None,
		media_type: None,
		representation: None,
		compression: None,
	};
	let lit_id = LiteratureReferenceBmc::create(&ctx, &mm, lit_c).await?;
	let lit = LiteratureReferenceBmc::get(&ctx, &mm, lit_id).await?;
	assert_eq!(lit.reference_text, "Ref 1");

	let lit_u = LiteratureReferenceForUpdate {
		reference_text: Some("Ref 1 updated".to_string()),
		sequence_number: None,
		document_base64: None,
		media_type: None,
		representation: None,
		compression: None,
	};
	LiteratureReferenceBmc::update(&ctx, &mm, lit_id, lit_u).await?;
	let lit = LiteratureReferenceBmc::get(&ctx, &mm, lit_id).await?;
	assert_eq!(lit.reference_text, "Ref 1 updated");

	let study_c = StudyInformationForCreate {
		case_id,
		study_name: Some("Study A".to_string()),
		sponsor_study_number: Some("SSN-1".to_string()),
		study_type_reaction: None,
		study_type_reaction_kr1: None,
	};
	let study_id = StudyInformationBmc::create(&ctx, &mm, study_c).await?;
	let study = StudyInformationBmc::get(&ctx, &mm, study_id).await?;
	assert_eq!(study.study_name.as_deref(), Some("Study A"));

	let study_u = StudyInformationForUpdate {
		study_name: Some("Study B".to_string()),
		sponsor_study_number: None,
		study_type_reaction: Some("01".to_string()),
		study_type_reaction_kr1: None,
	};
	StudyInformationBmc::update(&ctx, &mm, study_id, study_u).await?;
	let study = StudyInformationBmc::get(&ctx, &mm, study_id).await?;
	assert_eq!(study.study_name.as_deref(), Some("Study B"));

	let reg_c = StudyRegistrationNumberForCreate {
		study_information_id: study_id,
		registration_number: "REG-001".to_string(),
		country_code: Some("US".to_string()),
		sequence_number: 1,
	};
	let reg_id = StudyRegistrationNumberBmc::create(&ctx, &mm, reg_c).await?;
	let reg = StudyRegistrationNumberBmc::get(&ctx, &mm, reg_id).await?;
	assert_eq!(reg.registration_number, "REG-001");

	let reg_u = StudyRegistrationNumberForUpdate {
		registration_number: Some("REG-002".to_string()),
		country_code: None,
		sequence_number: None,
	};
	StudyRegistrationNumberBmc::update(&ctx, &mm, reg_id, reg_u).await?;
	let reg = StudyRegistrationNumberBmc::get(&ctx, &mm, reg_id).await?;
	assert_eq!(reg.registration_number, "REG-002");

	StudyRegistrationNumberBmc::delete(&ctx, &mm, reg_id).await?;
	StudyInformationBmc::delete(&ctx, &mm, study_id).await?;
	LiteratureReferenceBmc::delete(&ctx, &mm, lit_id).await?;
	PrimarySourceBmc::delete(&ctx, &mm, primary_id).await?;
	SenderInformationBmc::delete(&ctx, &mm, sender_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
