use crate::common::{create_case_fixture, demo_org_id, demo_user_id, Result};
use lib_core::ctx::Ctx;
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SafetyReportIdentificationForCreate,
	SafetyReportIdentificationForUpdate,
};
use lib_core::model::ModelManager;
use lib_core::xml::validate::{
	validate_case_for_profile, CaseValidationReport, ValidationProfile,
};
use lib_core::xml::{validate_e2b_xml_business, XmlValidationReport};
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use std::fs;
use std::path::PathBuf;
use time::Month;

fn sample_date() -> Date {
	Date::from_calendar_date(2024, Month::January, 1)
		.expect("sample date should be valid")
}

pub fn sample_safety_report_create(
	case_id: Uuid,
) -> SafetyReportIdentificationForCreate {
	SafetyReportIdentificationForCreate {
		case_id,
		transmission_date: Some(sample_date()),
		transmission_date_null_flavor: None,
		report_type: "1".to_string(),
		date_first_received_from_source: Some(sample_date()),
		date_first_received_from_source_null_flavor: None,
		date_of_most_recent_information: Some(sample_date()),
		date_of_most_recent_information_null_flavor: None,
		fulfil_expedited_criteria: true,
		first_sender_type: None,
		additional_documents_available: None,
	}
}

pub fn blank_safety_report_update() -> SafetyReportIdentificationForUpdate {
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
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		additional_documents_available: None,
	}
}

pub async fn create_case_with_safety_report(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<Uuid> {
	let case_id = create_case_fixture(mm, demo_org_id(), demo_user_id()).await?;
	SafetyReportIdentificationBmc::create(
		ctx,
		mm,
		sample_safety_report_create(case_id),
	)
	.await?;
	Ok(case_id)
}

pub async fn update_safety_report(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	update: SafetyReportIdentificationForUpdate,
) -> Result<()> {
	SafetyReportIdentificationBmc::update_by_case(ctx, mm, case_id, update).await?;
	Ok(())
}

pub async fn validate_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profile: ValidationProfile,
) -> Result<CaseValidationReport> {
	Ok(validate_case_for_profile(ctx, mm, case_id, profile).await?)
}

pub fn assert_has_issue(report: &CaseValidationReport, code: &str) {
	assert!(
		report.issues.iter().any(|issue| issue.code == code),
		"expected report to contain {code}, got {:?}",
		report
			.issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>()
	);
}

pub fn assert_lacks_issue(report: &CaseValidationReport, code: &str) {
	assert!(
		report.issues.iter().all(|issue| issue.code != code),
		"expected report to omit {code}, got {:?}",
		report
			.issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>()
	);
}

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

pub fn read_base_xml_fixture() -> Result<String> {
	let path = workspace_root().join("docs/refs/instances/FAERS2022Scenario1.xml");
	Ok(fs::read_to_string(path)?)
}

pub fn validate_business_xml(xml: &str) -> Result<XmlValidationReport> {
	Ok(validate_e2b_xml_business(xml.as_bytes(), None)?)
}

pub fn assert_has_xml_rule(report: &XmlValidationReport, code: &str) {
	assert!(
		report
			.errors
			.iter()
			.any(|error| error.message.contains(code)),
		"expected XML report to contain {code}, got {:?}",
		report
			.errors
			.iter()
			.map(|error| error.message.as_str())
			.collect::<Vec<_>>()
	);
}

pub fn assert_lacks_xml_rule(report: &XmlValidationReport, code: &str) {
	assert!(
		report
			.errors
			.iter()
			.all(|error| !error.message.contains(code)),
		"expected XML report to omit {code}, got {:?}",
		report
			.errors
			.iter()
			.map(|error| error.message.as_str())
			.collect::<Vec<_>>()
	);
}
