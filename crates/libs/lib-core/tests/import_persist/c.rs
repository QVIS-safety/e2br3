use lib_core::model::case_identifiers::{LinkedReportNumber, OtherCaseIdentifier};
use lib_core::model::receiver::ReceiverInformation;
use lib_core::model::safety_report::{
	DocumentsHeldBySender, LiteratureReference, PrimarySource,
	SafetyReportIdentification, SenderInformation, StudyInformation,
	StudyRegistrationNumber,
};
use serial_test::serial;

use crate::common::{
	date, fetch_one_by_uuid, fetch_optional_by_uuid, import_fixture,
	import_fixture_with_profile, list_by_uuid,
};

#[serial]
#[tokio::test]
async fn imports_c_header_and_source_models() {
	let imported =
		import_fixture_with_profile("FAERS2022Scenario6.xml", Some("fda")).await;
	let report: SafetyReportIdentification = fetch_one_by_uuid(
		&imported,
		"SELECT * FROM safety_report_identification WHERE case_id = $1 LIMIT 1",
		imported.case_id,
	)
	.await;
	let primary_sources: Vec<PrimarySource> = list_by_uuid(
		&imported,
		"SELECT * FROM primary_sources WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;
	let other_ids: Vec<OtherCaseIdentifier> = list_by_uuid(
		&imported,
		"SELECT * FROM other_case_identifiers WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;
	let linked: Vec<LinkedReportNumber> = list_by_uuid(
		&imported,
		"SELECT * FROM linked_report_numbers WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;
	let documents: Vec<DocumentsHeldBySender> = list_by_uuid(
		&imported,
		"SELECT * FROM documents_held_by_sender WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;
	let literature: Vec<LiteratureReference> = list_by_uuid(
		&imported,
		"SELECT * FROM literature_references WHERE case_id = $1 ORDER BY sequence_number",
		imported.case_id,
	)
	.await;

	assert_c_header(&report);

	assert_eq!(primary_sources.len(), 2);
	assert_eq!(primary_sources[0].case_id, imported.case_id);
	assert_eq!(primary_sources[0].sequence_number, 1);
	assert_eq!(primary_sources[0].reporter_title.as_deref(), Some("Dr."));
	assert_eq!(
		primary_sources[0].reporter_given_name.as_deref(),
		Some("Jerome")
	);
	assert_eq!(
		primary_sources[0].reporter_middle_name.as_deref(),
		Some("James")
	);
	assert_eq!(
		primary_sources[0].reporter_family_name.as_deref(),
		Some("Jacobs")
	);
	assert_eq!(
		primary_sources[0].organization.as_deref(),
		Some("Pharma Company")
	);
	assert_eq!(primary_sources[0].department.as_deref(), Some("Reporting"));
	assert_eq!(primary_sources[0].street.as_deref(), Some("123 Main St."));
	assert_eq!(primary_sources[0].city.as_deref(), Some("Anytown"));
	assert_eq!(primary_sources[0].state.as_deref(), Some("NJ"));
	assert_eq!(primary_sources[0].postcode.as_deref(), Some("87654"));
	assert_eq!(primary_sources[0].telephone.as_deref(), Some("3334445555"));
	assert_eq!(primary_sources[0].country_code.as_deref(), Some("US"));
	assert_eq!(primary_sources[0].email.as_deref(), Some("abc@gmail.com"));
	assert_eq!(primary_sources[0].qualification.as_deref(), Some("1"));
	assert_eq!(primary_sources[0].qualification_kr1, None);
	assert_eq!(
		primary_sources[0].primary_source_regulatory.as_deref(),
		Some("1")
	);

	assert_eq!(primary_sources[1].case_id, imported.case_id);
	assert_eq!(primary_sources[1].sequence_number, 2);
	assert_eq!(
		primary_sources[1].reporter_title.as_deref(),
		Some("Professor")
	);
	assert_eq!(
		primary_sources[1].reporter_given_name.as_deref(),
		Some("Ronald")
	);
	assert_eq!(
		primary_sources[1].reporter_middle_name.as_deref(),
		Some("Robert")
	);
	assert_eq!(
		primary_sources[1].reporter_family_name.as_deref(),
		Some("Rhodes")
	);
	assert_eq!(
		primary_sources[1].organization.as_deref(),
		Some("Medium Size Pharma")
	);
	assert_eq!(primary_sources[1].department.as_deref(), Some("Reporting"));
	assert_eq!(primary_sources[1].street.as_deref(), Some("89 Central Ave"));
	assert_eq!(primary_sources[1].city.as_deref(), Some("Anytown"));
	assert_eq!(primary_sources[1].state.as_deref(), Some("IL"));
	assert_eq!(primary_sources[1].postcode.as_deref(), Some("01340"));
	assert_eq!(primary_sources[1].telephone.as_deref(), Some("8884562344"));
	assert_eq!(primary_sources[1].country_code.as_deref(), Some("US"));
	assert_eq!(primary_sources[1].email.as_deref(), Some("abc@gmail.com"));
	assert_eq!(primary_sources[1].qualification.as_deref(), Some("2"));
	assert_eq!(primary_sources[1].qualification_kr1, None);
	assert_eq!(
		primary_sources[1].primary_source_regulatory.as_deref(),
		Some("2")
	);

	assert_eq!(other_ids.len(), 2);
	assert_eq!(other_ids[0].case_id, imported.case_id);
	assert_eq!(other_ids[0].sequence_number, 1);
	assert_eq!(
		other_ids[0].source_of_identifier,
		"Reporting Ltd Corporation"
	);
	assert_eq!(other_ids[0].case_identifier, "FR-RPTLTD-15998");
	assert_eq!(other_ids[1].case_id, imported.case_id);
	assert_eq!(other_ids[1].sequence_number, 2);
	assert_eq!(
		other_ids[1].source_of_identifier,
		"European Medicines Agency"
	);
	assert_eq!(other_ids[1].case_identifier, "EU-EMA-999999");

	assert_eq!(linked.len(), 2);
	assert_eq!(linked[0].case_id, imported.case_id);
	assert_eq!(linked[0].sequence_number, 1);
	assert_eq!(linked[0].linked_report_number, "FR-PHARMA-3448976655");
	assert_eq!(linked[1].case_id, imported.case_id);
	assert_eq!(linked[1].sequence_number, 2);
	assert_eq!(linked[1].linked_report_number, "FR-QPHARMA-12345678");

	assert_eq!(documents.len(), 2);
	assert_eq!(documents[0].case_id, imported.case_id);
	assert_eq!(documents[0].sequence_number, 1);
	assert_eq!(documents[0].title.as_deref(), Some("Sample Autopsy Report"));
	assert!(documents[0].document_base64.is_some());
	assert_eq!(documents[0].media_type.as_deref(), Some("text/plain"));
	assert_eq!(documents[0].representation.as_deref(), Some("B64"));
	assert_eq!(documents[0].compression, None);
	assert_eq!(documents[1].case_id, imported.case_id);
	assert_eq!(documents[1].sequence_number, 2);
	assert_eq!(
		documents[1].title.as_deref(),
		Some("Medication Information")
	);
	assert!(documents[1].document_base64.is_some());
	assert_eq!(documents[1].media_type.as_deref(), Some("text/plain"));
	assert_eq!(documents[1].representation.as_deref(), Some("B64"));
	assert_eq!(documents[1].compression, None);

	assert_eq!(literature.len(), 1);
	assert_eq!(literature[0].case_id, imported.case_id);
	assert_eq!(literature[0].sequence_number, 1);
	assert_eq!(
		literature[0].reference_text,
		"\n\nAuthor (last name first), \"Article Title.\" Name of newspaper, city, state of publication. (date): edition if available, section, page number(s).\n"
	);
	assert!(literature[0].document_base64.is_some());
	assert_eq!(literature[0].media_type.as_deref(), Some("application/pdf"));
	assert_eq!(literature[0].representation.as_deref(), Some("B64"));
	assert_eq!(literature[0].compression, None);
}

#[serial]
#[tokio::test]
async fn imports_c_sender_and_receiver_models() {
	let imported = import_fixture("FAERS2022Scenario1.xml").await;
	let sender: SenderInformation = fetch_one_by_uuid(
		&imported,
		"SELECT * FROM sender_information WHERE case_id = $1 LIMIT 1",
		imported.case_id,
	)
	.await;
	let receiver: Option<ReceiverInformation> = fetch_optional_by_uuid(
		&imported,
		"SELECT * FROM receiver_information WHERE case_id = $1 LIMIT 1",
		imported.case_id,
	)
	.await;

	assert_eq!(sender.case_id, imported.case_id);
	assert_eq!(sender.sender_type, "1");
	assert_eq!(sender.organization_name, "Management");
	assert_eq!(sender.department, None);
	assert_eq!(sender.street_address.as_deref(), Some("13 Elm St."));
	assert_eq!(sender.city.as_deref(), Some("Metropolis"));
	assert_eq!(sender.state.as_deref(), Some("UT"));
	assert_eq!(sender.postcode.as_deref(), Some("65498"));
	assert_eq!(sender.country_code, None);
	assert_eq!(sender.person_title.as_deref(), Some("Doctor"));
	assert_eq!(sender.person_given_name.as_deref(), Some("Roger"));
	assert_eq!(sender.person_middle_name, None);
	assert_eq!(sender.person_family_name.as_deref(), Some("Robertson"));
	assert_eq!(sender.telephone.as_deref(), Some("6102227777"));
	assert_eq!(sender.fax.as_deref(), Some("6109991122"));
	assert_eq!(sender.email.as_deref(), Some("alladdresses@site.com"));

	let receiver = receiver.expect("expected receiver_information row");
	assert_eq!(receiver.case_id, imported.case_id);
	assert_eq!(receiver.receiver_type, None);
	assert_eq!(receiver.organization_name.as_deref(), Some("CDER"));
	assert_eq!(receiver.department, None);
	assert_eq!(receiver.street_address, None);
	assert_eq!(receiver.city, None);
	assert_eq!(receiver.state_province, None);
	assert_eq!(receiver.postcode, None);
	assert_eq!(receiver.country_code, None);
	assert_eq!(receiver.telephone, None);
	assert_eq!(receiver.fax, None);
	assert_eq!(receiver.email, None);
}

#[serial]
#[tokio::test]
async fn imports_c_study_models() {
	let imported =
		import_fixture_with_profile("FAERS2022Scenario3.xml", Some("fda")).await;
	let regs: Vec<StudyRegistrationNumber> = list_by_uuid(
		&imported,
		"SELECT * FROM study_registration_numbers WHERE study_information_id IN (SELECT id FROM study_information WHERE case_id = $1) ORDER BY sequence_number",
		imported.case_id,
	)
	.await;
	assert_eq!(regs.len(), 4);
	let study: StudyInformation = fetch_one_by_uuid(
		&imported,
		"SELECT * FROM study_information WHERE id = $1",
		regs[0].study_information_id,
	)
	.await;

	assert_eq!(study.case_id, imported.case_id);
	assert_eq!(
		study.study_name.as_deref(),
		Some("Study ID$Abbreviated Trial Name")
	);
	assert_eq!(study.sponsor_study_number.as_deref(), Some("CT-00-00"));
	assert_eq!(study.study_type_reaction.as_deref(), Some("1"));
	assert_eq!(study.study_type_reaction_kr1, None);

	assert_eq!(regs[0].study_information_id, study.id);
	assert_eq!(regs[0].registration_number, "STUDY-US-001");
	assert_eq!(regs[0].country_code.as_deref(), Some("US"));
	assert_eq!(regs[0].sequence_number, 1);
	assert_eq!(regs[1].study_information_id, study.id);
	assert_eq!(regs[1].registration_number, "123456");
	assert_eq!(regs[1].country_code, None);
	assert_eq!(regs[1].sequence_number, 2);
	assert_eq!(regs[2].study_information_id, study.id);
	assert_eq!(regs[2].registration_number, "222222");
	assert_eq!(regs[2].country_code, None);
	assert_eq!(regs[2].sequence_number, 3);
	assert_eq!(regs[3].study_information_id, study.id);
	assert_eq!(regs[3].registration_number, "333333");
	assert_eq!(regs[3].country_code, None);
	assert_eq!(regs[3].sequence_number, 4);
}

fn assert_c_header(report: &SafetyReportIdentification) {
	assert_eq!(report.transmission_date, Some(date(2014, 6, 14)));
	assert_eq!(report.transmission_date_null_flavor, None);
	assert_eq!(report.report_type, "1");
	assert_eq!(
		report.date_first_received_from_source,
		Some(date(2022, 6, 14))
	);
	assert_eq!(report.date_first_received_from_source_null_flavor, None);
	assert_eq!(
		report.date_of_most_recent_information,
		Some(date(2022, 6, 14))
	);
	assert_eq!(report.date_of_most_recent_information_null_flavor, None);
	assert!(report.fulfil_expedited_criteria);
	assert_eq!(report.local_criteria_report_type.as_deref(), Some("1"));
	assert_eq!(
		report.combination_product_report_indicator.as_deref(),
		Some("true")
	);
	assert_eq!(
		report.worldwide_unique_id.as_deref(),
		Some("US-APHARMA-8744554B")
	);
	assert_eq!(report.first_sender_type.as_deref(), Some("1"));
	assert_eq!(report.additional_documents_available, Some(true));
	assert_eq!(report.nullification_code, None);
	assert_eq!(report.nullification_reason, None);
	assert_eq!(report.receiver_organization.as_deref(), Some("CDER"));
}
