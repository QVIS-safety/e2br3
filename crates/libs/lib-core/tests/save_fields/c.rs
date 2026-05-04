use super::common::{date, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::case::{CaseBmc, CaseForCreate, CaseForUpdate};
use lib_core::model::case_identifiers::{
	LinkedReportNumberBmc, LinkedReportNumberForCreate, LinkedReportNumberForUpdate,
	OtherCaseIdentifierBmc, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use lib_core::model::safety_report::{
	DocumentsHeldBySenderBmc, DocumentsHeldBySenderForCreate,
	DocumentsHeldBySenderForUpdate, LiteratureReferenceBmc,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySourceBmc,
	PrimarySourceForCreate, PrimarySourceForUpdate, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SafetyReportIdentificationForUpdate,
	SenderInformationBmc, SenderInformationForCreate, SenderInformationForUpdate,
	StudyInformationBmc, StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumberBmc, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
};
use serial_test::serial;
use time::Month;
use uuid::Uuid;

fn case_seed_id() -> String {
	format!("SAVE-C-{}", Uuid::new_v4().simple())
}

#[tokio::test]
#[serial]
async fn save_c_case_create_non_e2b_fields() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let row = CaseBmc::get(&ctx, &mm, case_id).await?;
	let org_id = row.organization_id;
	let new_case_id = CaseBmc::create(
		&ctx,
		&mm,
		CaseForCreate {
			organization_id: org_id,
			safety_report_id: case_seed_id(),
			dg_prd_key: None,
			status: Some("draft".to_string()),
			appendices_json: Some("[\"mfds\"]".to_string()),
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: Some("Spontaneous".to_string()),
			report_year: Some("2026".to_string()),
			source_document_name: Some("source.txt".to_string()),
			source_document_base64: Some("U09VUkNF".to_string()),
			source_document_media_type: Some("text/plain".to_string()),
			version: Some(1),
		},
	)
	.await?;
	let saved = CaseBmc::get(&ctx, &mm, new_case_id).await?;
	assert_eq!(saved.mfds_report_type.as_deref(), Some("Spontaneous"));
	assert_eq!(saved.report_year.as_deref(), Some("2026"));
	assert_eq!(saved.source_document_name.as_deref(), Some("source.txt"));
	assert_eq!(saved.source_document_base64.as_deref(), Some("U09VUkNF"));
	assert_eq!(
		saved.source_document_media_type.as_deref(),
		Some("text/plain")
	);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_case_update_non_e2b_fields() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let updated_safety_report_id = case_seed_id();
	CaseBmc::update(
		&ctx,
		&mm,
		case_id,
		CaseForUpdate {
			safety_report_id: Some(updated_safety_report_id.clone()),
			dg_prd_key: None,
			status: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: Some("Spontaneous".to_string()),
			report_year: Some("2026".to_string()),
			source_document_name: Some("source.txt".to_string()),
			source_document_base64: Some("U09VUkNF".to_string()),
			source_document_media_type: Some("text/plain".to_string()),
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: None,
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await?;
	let saved = CaseBmc::get(&ctx, &mm, case_id).await?;
	assert_eq!(saved.safety_report_id, updated_safety_report_id);
	assert_eq!(saved.mfds_report_type.as_deref(), Some("Spontaneous"));
	assert_eq!(saved.report_year.as_deref(), Some("2026"));
	assert_eq!(saved.source_document_name.as_deref(), Some("source.txt"));
	assert_eq!(saved.source_document_base64.as_deref(), Some("U09VUkNF"));
	assert_eq!(
		saved.source_document_media_type.as_deref(),
		Some("text/plain")
	);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_1_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	SafetyReportIdentificationBmc::create(
		&ctx,
		&mm,
		SafetyReportIdentificationForCreate {
			case_id,
			transmission_date: Some(date(2024, Month::January, 1)),
			transmission_date_null_flavor: None,
			report_type: Some("1".to_string()),
			date_first_received_from_source: Some(date(2024, Month::January, 2)),
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: Some(date(2024, Month::January, 3)),
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria: Some(true),
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			first_sender_type: Some("2".to_string()),
			additional_documents_available: Some(true),
			other_case_identifiers_exist: None,
			worldwide_unique_id: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await?;
	let row = SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.transmission_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.transmission_date_null_flavor, None);
	assert_eq!(row.report_type.as_deref(), Some("1"));
	assert_eq!(
		row.date_first_received_from_source,
		Some(date(2024, Month::January, 2))
	);
	assert_eq!(row.date_first_received_from_source_null_flavor, None);
	assert_eq!(
		row.date_of_most_recent_information,
		Some(date(2024, Month::January, 3))
	);
	assert_eq!(row.date_of_most_recent_information_null_flavor, None);
	assert_eq!(row.fulfil_expedited_criteria, Some(true));
	assert_eq!(row.local_criteria_report_type, None);
	assert_eq!(row.combination_product_report_indicator, None);
	assert_eq!(row.worldwide_unique_id, None);
	assert_eq!(row.first_sender_type.as_deref(), Some("2"));
	assert_eq!(row.additional_documents_available, Some(true));
	assert_eq!(row.nullification_code, None);
	assert_eq!(row.nullification_reason, None);
	assert_eq!(row.receiver_organization, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_1_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	SafetyReportIdentificationBmc::create(
		&ctx,
		&mm,
		SafetyReportIdentificationForCreate {
			case_id,
			transmission_date: None,
			transmission_date_null_flavor: Some("UNK".to_string()),
			report_type: Some("1".to_string()),
			date_first_received_from_source: None,
			date_first_received_from_source_null_flavor: Some("NI".to_string()),
			date_of_most_recent_information: None,
			date_of_most_recent_information_null_flavor: Some("ASKU".to_string()),
			fulfil_expedited_criteria: Some(false),
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			first_sender_type: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			worldwide_unique_id: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await?;
	SafetyReportIdentificationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		SafetyReportIdentificationForUpdate {
			transmission_date: Some(date(2024, Month::February, 1)),
			transmission_date_null_flavor: None,
			report_type: lib_core::model::safety_report::PatchValue::Value(
				"2".to_string(),
			),
			date_first_received_from_source: Some(date(2024, Month::February, 2)),
			date_first_received_from_source_null_flavor: None,
			date_of_most_recent_information: Some(date(2024, Month::February, 3)),
			date_of_most_recent_information_null_flavor: None,
			fulfil_expedited_criteria:
				lib_core::model::safety_report::PatchValue::Value(true),
			local_criteria_report_type: Some("LOCAL".to_string()),
			combination_product_report_indicator: Some("1".to_string()),
			worldwide_unique_id: Some("WID".to_string()),
			first_sender_type: Some("2".to_string()),
			additional_documents_available: Some(false),
			other_case_identifiers_exist: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: Some("Receiver".to_string()),
		},
	)
	.await?;
	let row = SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.transmission_date, Some(date(2024, Month::February, 1)));
	assert_eq!(row.transmission_date_null_flavor, None);
	assert_eq!(row.report_type.as_deref(), Some("2"));
	assert_eq!(
		row.date_first_received_from_source,
		Some(date(2024, Month::February, 2))
	);
	assert_eq!(row.date_first_received_from_source_null_flavor, None);
	assert_eq!(
		row.date_of_most_recent_information,
		Some(date(2024, Month::February, 3))
	);
	assert_eq!(row.date_of_most_recent_information_null_flavor, None);
	assert_eq!(row.fulfil_expedited_criteria, Some(true));
	assert_eq!(row.local_criteria_report_type.as_deref(), Some("LOCAL"));
	assert_eq!(
		row.combination_product_report_indicator.as_deref(),
		Some("1")
	);
	assert_eq!(row.worldwide_unique_id.as_deref(), Some("WID"));
	assert_eq!(row.first_sender_type.as_deref(), Some("2"));
	assert_eq!(row.additional_documents_available, Some(false));
	assert_eq!(row.nullification_code, None);
	assert_eq!(row.nullification_reason, None);
	assert_eq!(row.receiver_organization.as_deref(), Some("Receiver"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_2_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = SenderInformationBmc::create(
		&ctx,
		&mm,
		SenderInformationForCreate {
			case_id,
			sender_type: Some("1".to_string()),
			organization_name: Some("Org".to_string()),
			department: Some("Dept".to_string()),
			street_address: Some("123 St".to_string()),
			city: Some("Seoul".to_string()),
			state: Some("11".to_string()),
			postcode: Some("12345".to_string()),
			country_code: Some("KR".to_string()),
			person_title: Some("Dr".to_string()),
			person_given_name: Some("Given".to_string()),
			person_middle_name: Some("Mid".to_string()),
			person_family_name: Some("Family".to_string()),
			telephone: Some("010".to_string()),
			fax: Some("020".to_string()),
			email: Some("sender@example.com".to_string()),
		},
	)
	.await?;
	let row = SenderInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sender_type.as_deref(), Some("1"));
	assert_eq!(row.organization_name.as_deref(), Some("Org"));
	assert_eq!(row.department.as_deref(), Some("Dept"));
	assert_eq!(row.street_address.as_deref(), Some("123 St"));
	assert_eq!(row.city.as_deref(), Some("Seoul"));
	assert_eq!(row.state.as_deref(), Some("11"));
	assert_eq!(row.postcode.as_deref(), Some("12345"));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.person_title.as_deref(), Some("Dr"));
	assert_eq!(row.person_given_name.as_deref(), Some("Given"));
	assert_eq!(row.person_middle_name.as_deref(), Some("Mid"));
	assert_eq!(row.person_family_name.as_deref(), Some("Family"));
	assert_eq!(row.telephone.as_deref(), Some("010"));
	assert_eq!(row.fax.as_deref(), Some("020"));
	assert_eq!(row.email.as_deref(), Some("sender@example.com"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_2_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = SenderInformationBmc::create(
		&ctx,
		&mm,
		SenderInformationForCreate {
			case_id,
			sender_type: Some("1".to_string()),
			organization_name: Some("Org".to_string()),
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
		},
	)
	.await?;
	SenderInformationBmc::update(
		&ctx,
		&mm,
		id,
		SenderInformationForUpdate {
			sender_type: Some("2".to_string()),
			organization_name: Some("Org 2".to_string()),
			department: Some("Dept".to_string()),
			street_address: Some("123 St".to_string()),
			city: Some("Seoul".to_string()),
			state: Some("11".to_string()),
			postcode: Some("12345".to_string()),
			country_code: Some("KR".to_string()),
			person_title: Some("Dr".to_string()),
			person_given_name: Some("Given".to_string()),
			person_middle_name: Some("Mid".to_string()),
			person_family_name: Some("Family".to_string()),
			telephone: Some("010".to_string()),
			fax: Some("020".to_string()),
			email: Some("sender@example.com".to_string()),
		},
	)
	.await?;
	let row = SenderInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sender_type.as_deref(), Some("2"));
	assert_eq!(row.organization_name.as_deref(), Some("Org 2"));
	assert_eq!(row.department.as_deref(), Some("Dept"));
	assert_eq!(row.street_address.as_deref(), Some("123 St"));
	assert_eq!(row.city.as_deref(), Some("Seoul"));
	assert_eq!(row.state.as_deref(), Some("11"));
	assert_eq!(row.postcode.as_deref(), Some("12345"));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.person_title.as_deref(), Some("Dr"));
	assert_eq!(row.person_given_name.as_deref(), Some("Given"));
	assert_eq!(row.person_middle_name.as_deref(), Some("Mid"));
	assert_eq!(row.person_family_name.as_deref(), Some("Family"));
	assert_eq!(row.telephone.as_deref(), Some("010"));
	assert_eq!(row.fax.as_deref(), Some("020"));
	assert_eq!(row.email.as_deref(), Some("sender@example.com"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_receiver_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: Some("3".to_string()),
			organization_name: Some("Receiver".to_string()),
			department: Some("PV".to_string()),
			street_address: Some("Street".to_string()),
			city: Some("Seoul".to_string()),
			state_province: Some("11".to_string()),
			postcode: Some("12345".to_string()),
			country_code: Some("KR".to_string()),
			telephone: Some("010".to_string()),
			fax: Some("020".to_string()),
			email: Some("recv@example.com".to_string()),
		},
	)
	.await?;
	let row = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.receiver_type.as_deref(), Some("3"));
	assert_eq!(row.organization_name.as_deref(), Some("Receiver"));
	assert_eq!(row.department.as_deref(), Some("PV"));
	assert_eq!(row.street_address.as_deref(), Some("Street"));
	assert_eq!(row.city.as_deref(), Some("Seoul"));
	assert_eq!(row.state_province.as_deref(), Some("11"));
	assert_eq!(row.postcode.as_deref(), Some("12345"));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.telephone.as_deref(), Some("010"));
	assert_eq!(row.fax.as_deref(), Some("020"));
	assert_eq!(row.email.as_deref(), Some("recv@example.com"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_receiver_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: Some("2".to_string()),
			organization_name: Some("Receiver 1".to_string()),
			department: None,
			street_address: None,
			city: None,
			state_province: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	ReceiverInformationBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		ReceiverInformationForUpdate {
			receiver_type: Some("3".to_string()),
			organization_name: Some("Receiver 2".to_string()),
			department: Some("PV".to_string()),
			street_address: Some("Street".to_string()),
			city: Some("Seoul".to_string()),
			state_province: Some("11".to_string()),
			postcode: Some("12345".to_string()),
			country_code: Some("KR".to_string()),
			telephone: Some("010".to_string()),
			fax: Some("020".to_string()),
			email: Some("recv@example.com".to_string()),
		},
	)
	.await?;
	let row = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.receiver_type.as_deref(), Some("3"));
	assert_eq!(row.organization_name.as_deref(), Some("Receiver 2"));
	assert_eq!(row.department.as_deref(), Some("PV"));
	assert_eq!(row.street_address.as_deref(), Some("Street"));
	assert_eq!(row.city.as_deref(), Some("Seoul"));
	assert_eq!(row.state_province.as_deref(), Some("11"));
	assert_eq!(row.postcode.as_deref(), Some("12345"));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.telephone.as_deref(), Some("010"));
	assert_eq!(row.fax.as_deref(), Some("020"));
	assert_eq!(row.email.as_deref(), Some("recv@example.com"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_2_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = PrimarySourceBmc::create(
		&ctx,
		&mm,
		PrimarySourceForCreate {
			case_id,
			sequence_number: 1,
			reporter_title: Some("Dr".to_string()),
			reporter_given_name: Some("Jane".to_string()),
			reporter_middle_name: Some("Q".to_string()),
			reporter_family_name: Some("Doe".to_string()),
			organization: Some("Hospital".to_string()),
			department: Some("ER".to_string()),
			street: Some("Street".to_string()),
			city: Some("Seoul".to_string()),
			state: Some("11".to_string()),
			postcode: Some("12345".to_string()),
			telephone: Some("010".to_string()),
			country_code: Some("KR".to_string()),
			email: Some("jane@example.com".to_string()),
			qualification: Some("1".to_string()),
			qualification_kr1: Some("1".to_string()),
			primary_source_regulatory: Some("2".to_string()),
		},
	)
	.await?;
	let row = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.reporter_title.as_deref(), Some("Dr"));
	assert_eq!(row.reporter_given_name.as_deref(), Some("Jane"));
	assert_eq!(row.reporter_middle_name.as_deref(), Some("Q"));
	assert_eq!(row.reporter_family_name.as_deref(), Some("Doe"));
	assert_eq!(row.organization.as_deref(), Some("Hospital"));
	assert_eq!(row.department.as_deref(), Some("ER"));
	assert_eq!(row.street.as_deref(), Some("Street"));
	assert_eq!(row.city.as_deref(), Some("Seoul"));
	assert_eq!(row.state.as_deref(), Some("11"));
	assert_eq!(row.postcode.as_deref(), Some("12345"));
	assert_eq!(row.telephone.as_deref(), Some("010"));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.email.as_deref(), Some("jane@example.com"));
	assert_eq!(row.qualification.as_deref(), Some("1"));
	assert_eq!(row.qualification_kr1.as_deref(), Some("1"));
	assert_eq!(row.primary_source_regulatory.as_deref(), Some("2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_2_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = PrimarySourceBmc::create(
		&ctx,
		&mm,
		PrimarySourceForCreate {
			case_id,
			sequence_number: 1,
			reporter_title: None,
			reporter_given_name: None,
			reporter_middle_name: None,
			reporter_family_name: None,
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
			primary_source_regulatory: None,
		},
	)
	.await?;
	PrimarySourceBmc::update(
		&ctx,
		&mm,
		id,
		PrimarySourceForUpdate {
			reporter_title: Some("Prof".to_string()),
			reporter_given_name: Some("John".to_string()),
			reporter_middle_name: Some("M".to_string()),
			reporter_family_name: Some("Smith".to_string()),
			organization: Some("Clinic".to_string()),
			department: Some("PV".to_string()),
			street: Some("Road".to_string()),
			city: Some("Busan".to_string()),
			state: Some("26".to_string()),
			postcode: Some("54321".to_string()),
			telephone: Some("021".to_string()),
			country_code: Some("US".to_string()),
			email: Some("john@example.com".to_string()),
			qualification: Some("2".to_string()),
			qualification_kr1: Some("2".to_string()),
			primary_source_regulatory: Some("1".to_string()),
		},
	)
	.await?;
	let row = PrimarySourceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.reporter_title.as_deref(), Some("Prof"));
	assert_eq!(row.reporter_given_name.as_deref(), Some("John"));
	assert_eq!(row.reporter_middle_name.as_deref(), Some("M"));
	assert_eq!(row.reporter_family_name.as_deref(), Some("Smith"));
	assert_eq!(row.organization.as_deref(), Some("Clinic"));
	assert_eq!(row.department.as_deref(), Some("PV"));
	assert_eq!(row.street.as_deref(), Some("Road"));
	assert_eq!(row.city.as_deref(), Some("Busan"));
	assert_eq!(row.state.as_deref(), Some("26"));
	assert_eq!(row.postcode.as_deref(), Some("54321"));
	assert_eq!(row.telephone.as_deref(), Some("021"));
	assert_eq!(row.country_code.as_deref(), Some("US"));
	assert_eq!(row.email.as_deref(), Some("john@example.com"));
	assert_eq!(row.qualification.as_deref(), Some("2"));
	assert_eq!(row.qualification_kr1.as_deref(), Some("2"));
	assert_eq!(row.primary_source_regulatory.as_deref(), Some("1"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_1_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = OtherCaseIdentifierBmc::create(
		&ctx,
		&mm,
		OtherCaseIdentifierForCreate {
			case_id,
			sequence_number: 1,
			source_of_identifier: "FDA".to_string(),
			case_identifier: "CASE-1".to_string(),
		},
	)
	.await?;
	let row = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.source_of_identifier, "FDA");
	assert_eq!(row.case_identifier, "CASE-1");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_1_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = OtherCaseIdentifierBmc::create(
		&ctx,
		&mm,
		OtherCaseIdentifierForCreate {
			case_id,
			sequence_number: 1,
			source_of_identifier: "FDA".to_string(),
			case_identifier: "CASE-1".to_string(),
		},
	)
	.await?;
	OtherCaseIdentifierBmc::update(
		&ctx,
		&mm,
		id,
		OtherCaseIdentifierForUpdate {
			source_of_identifier: Some("MFDS".to_string()),
			case_identifier: Some("CASE-2".to_string()),
		},
	)
	.await?;
	let row = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.source_of_identifier, "MFDS");
	assert_eq!(row.case_identifier, "CASE-2");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_2_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = LinkedReportNumberBmc::create(
		&ctx,
		&mm,
		LinkedReportNumberForCreate {
			case_id,
			sequence_number: 1,
			linked_report_number: "LINK-1".to_string(),
		},
	)
	.await?;
	let row = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.linked_report_number, "LINK-1");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_3_2_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = LinkedReportNumberBmc::create(
		&ctx,
		&mm,
		LinkedReportNumberForCreate {
			case_id,
			sequence_number: 1,
			linked_report_number: "LINK-1".to_string(),
		},
	)
	.await?;
	LinkedReportNumberBmc::update(
		&ctx,
		&mm,
		id,
		LinkedReportNumberForUpdate {
			linked_report_number: Some("LINK-2".to_string()),
		},
	)
	.await?;
	let row = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.linked_report_number, "LINK-2");
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_4_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = DocumentsHeldBySenderBmc::create(
		&ctx,
		&mm,
		DocumentsHeldBySenderForCreate {
			case_id,
			title: Some("Title".to_string()),
			document_base64: Some("BASE64".to_string()),
			media_type: Some("application/pdf".to_string()),
			representation: Some("B64".to_string()),
			compression: Some("none".to_string()),
			sequence_number: 1,
		},
	)
	.await?;
	let row = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.title.as_deref(), Some("Title"));
	assert_eq!(row.document_base64.as_deref(), Some("BASE64"));
	assert_eq!(row.media_type.as_deref(), Some("application/pdf"));
	assert_eq!(row.representation.as_deref(), Some("B64"));
	assert_eq!(row.compression.as_deref(), Some("none"));
	assert_eq!(row.sequence_number, 1);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_4_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = DocumentsHeldBySenderBmc::create(
		&ctx,
		&mm,
		DocumentsHeldBySenderForCreate {
			case_id,
			title: None,
			document_base64: None,
			media_type: None,
			representation: None,
			compression: None,
			sequence_number: 1,
		},
	)
	.await?;
	DocumentsHeldBySenderBmc::update(
		&ctx,
		&mm,
		id,
		DocumentsHeldBySenderForUpdate {
			title: Some("Title 2".to_string()),
			document_base64: Some("BASE64-2".to_string()),
			media_type: Some("text/plain".to_string()),
			representation: Some("TXT".to_string()),
			compression: Some("zip".to_string()),
			sequence_number: Some(2),
		},
	)
	.await?;
	let row = DocumentsHeldBySenderBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.title.as_deref(), Some("Title 2"));
	assert_eq!(row.document_base64.as_deref(), Some("BASE64-2"));
	assert_eq!(row.media_type.as_deref(), Some("text/plain"));
	assert_eq!(row.representation.as_deref(), Some("TXT"));
	assert_eq!(row.compression.as_deref(), Some("zip"));
	assert_eq!(row.sequence_number, 2);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_4_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = LiteratureReferenceBmc::create(
		&ctx,
		&mm,
		LiteratureReferenceForCreate {
			case_id,
			reference_text: "Ref".to_string(),
			sequence_number: 1,
			document_base64: Some("BASE64".to_string()),
			media_type: Some("application/pdf".to_string()),
			representation: Some("B64".to_string()),
			compression: Some("none".to_string()),
		},
	)
	.await?;
	let row = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.reference_text, "Ref");
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.document_base64.as_deref(), Some("BASE64"));
	assert_eq!(row.media_type.as_deref(), Some("application/pdf"));
	assert_eq!(row.representation.as_deref(), Some("B64"));
	assert_eq!(row.compression.as_deref(), Some("none"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_4_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = LiteratureReferenceBmc::create(
		&ctx,
		&mm,
		LiteratureReferenceForCreate {
			case_id,
			reference_text: "Ref".to_string(),
			sequence_number: 1,
			document_base64: None,
			media_type: None,
			representation: None,
			compression: None,
		},
	)
	.await?;
	LiteratureReferenceBmc::update(
		&ctx,
		&mm,
		id,
		LiteratureReferenceForUpdate {
			reference_text: Some("Ref 2".to_string()),
			sequence_number: Some(2),
			document_base64: Some("BASE64-2".to_string()),
			media_type: Some("text/plain".to_string()),
			representation: Some("TXT".to_string()),
			compression: Some("zip".to_string()),
		},
	)
	.await?;
	let row = LiteratureReferenceBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.reference_text, "Ref 2");
	assert_eq!(row.sequence_number, 2);
	assert_eq!(row.document_base64.as_deref(), Some("BASE64-2"));
	assert_eq!(row.media_type.as_deref(), Some("text/plain"));
	assert_eq!(row.representation.as_deref(), Some("TXT"));
	assert_eq!(row.compression.as_deref(), Some("zip"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_5_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = StudyInformationBmc::create(
		&ctx,
		&mm,
		StudyInformationForCreate {
			case_id,
			study_name: Some("Study".to_string()),
			sponsor_study_number: Some("SP-1".to_string()),
			study_type_reaction: Some("1".to_string()),
			study_type_reaction_kr1: Some("1".to_string()),
		},
	)
	.await?;
	let row = StudyInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.study_name.as_deref(), Some("Study"));
	assert_eq!(row.sponsor_study_number.as_deref(), Some("SP-1"));
	assert_eq!(row.study_type_reaction.as_deref(), Some("1"));
	assert_eq!(row.study_type_reaction_kr1.as_deref(), Some("1"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_5_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = StudyInformationBmc::create(
		&ctx,
		&mm,
		StudyInformationForCreate {
			case_id,
			study_name: None,
			sponsor_study_number: None,
			study_type_reaction: None,
			study_type_reaction_kr1: None,
		},
	)
	.await?;
	StudyInformationBmc::update(
		&ctx,
		&mm,
		id,
		StudyInformationForUpdate {
			study_name: Some("Study 2".to_string()),
			sponsor_study_number: Some("SP-2".to_string()),
			study_type_reaction: Some("2".to_string()),
			study_type_reaction_kr1: Some("2".to_string()),
		},
	)
	.await?;
	let row = StudyInformationBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.study_name.as_deref(), Some("Study 2"));
	assert_eq!(row.sponsor_study_number.as_deref(), Some("SP-2"));
	assert_eq!(row.study_type_reaction.as_deref(), Some("2"));
	assert_eq!(row.study_type_reaction_kr1.as_deref(), Some("2"));
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_5_r_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let study_id = StudyInformationBmc::create(
		&ctx,
		&mm,
		StudyInformationForCreate {
			case_id,
			study_name: Some("Study".to_string()),
			sponsor_study_number: Some("SP-1".to_string()),
			study_type_reaction: Some("1".to_string()),
			study_type_reaction_kr1: None,
		},
	)
	.await?;
	let id = StudyRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study_id,
			registration_number: "REG-1".to_string(),
			country_code: Some("KR".to_string()),
			sequence_number: 1,
		},
	)
	.await?;
	let row = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.study_information_id, study_id);
	assert_eq!(row.registration_number, "REG-1");
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	assert_eq!(row.sequence_number, 1);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_c_5_r_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let study_id = StudyInformationBmc::create(
		&ctx,
		&mm,
		StudyInformationForCreate {
			case_id,
			study_name: Some("Study".to_string()),
			sponsor_study_number: Some("SP-1".to_string()),
			study_type_reaction: Some("1".to_string()),
			study_type_reaction_kr1: None,
		},
	)
	.await?;
	let id = StudyRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyRegistrationNumberForCreate {
			study_information_id: study_id,
			registration_number: "REG-1".to_string(),
			country_code: None,
			sequence_number: 1,
		},
	)
	.await?;
	StudyRegistrationNumberBmc::update(
		&ctx,
		&mm,
		id,
		StudyRegistrationNumberForUpdate {
			registration_number: Some("REG-2".to_string()),
			country_code: Some("US".to_string()),
			sequence_number: Some(2),
		},
	)
	.await?;
	let row = StudyRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.study_information_id, study_id);
	assert_eq!(row.registration_number, "REG-2");
	assert_eq!(row.country_code.as_deref(), Some("US"));
	assert_eq!(row.sequence_number, 2);
	finish(&mm).await
}
