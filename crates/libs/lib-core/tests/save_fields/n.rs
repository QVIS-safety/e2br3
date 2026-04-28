use super::common::{datetime_utc, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::message_header::{
	MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::model::receiver::{
	ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use serial_test::serial;
use time::Month;

#[tokio::test]
#[serial]
async fn save_n_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let message_number = format!("MSG-{case_id}");
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number: message_number.clone(),
			message_sender_identifier: "SENDER".to_string(),
			message_receiver_identifier: "RECV".to_string(),
			message_date: "20240102030405".to_string(),
		},
	)
	.await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: Some("2".to_string()),
			organization_name: Some("Receiver".to_string()),
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
	let header = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(header.case_id, case_id);
	assert_eq!(header.batch_number, None);
	assert_eq!(header.batch_sender_identifier, None);
	assert_eq!(header.batch_receiver_identifier, None);
	assert_eq!(header.batch_transmission_date, None);
	assert_eq!(header.message_type, "ichicsr");
	assert_eq!(header.message_format_version, "2.1");
	assert_eq!(header.message_format_release, "2.0");
	assert_eq!(header.message_number, message_number);
	assert_eq!(header.message_sender_identifier, "SENDER");
	assert_eq!(header.message_receiver_identifier, "RECV");
	assert_eq!(header.message_date_format, "204");
	assert_eq!(header.message_date, "20240102030405");
	let receiver = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(receiver.case_id, case_id);
	assert_eq!(receiver.receiver_type.as_deref(), Some("2"));
	assert_eq!(receiver.organization_name.as_deref(), Some("Receiver"));
	assert_eq!(receiver.department, None);
	assert_eq!(receiver.street_address, None);
	assert_eq!(receiver.city, None);
	assert_eq!(receiver.state_province, None);
	assert_eq!(receiver.postcode, None);
	assert_eq!(receiver.country_code, None);
	assert_eq!(receiver.telephone, None);
	assert_eq!(receiver.fax, None);
	assert_eq!(receiver.email, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_n_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let message_number = format!("MSG-{case_id}");
	let updated_message_number = format!("MSG-{case_id}-2");
	MessageHeaderBmc::create(
		&ctx,
		&mm,
		MessageHeaderForCreate {
			case_id,
			message_number,
			message_sender_identifier: "SENDER".to_string(),
			message_receiver_identifier: "RECV".to_string(),
			message_date: "20240102030405".to_string(),
		},
	)
	.await?;
	ReceiverInformationBmc::create(
		&ctx,
		&mm,
		ReceiverInformationForCreate {
			case_id,
			receiver_type: None,
			organization_name: None,
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
	MessageHeaderBmc::update_by_case(
		&ctx,
		&mm,
		case_id,
		MessageHeaderForUpdate {
			batch_number: Some("BATCH".to_string()),
			batch_sender_identifier: Some("BS".to_string()),
			batch_receiver_identifier: Some("BR".to_string()),
			batch_transmission_date: Some(datetime_utc(
				2024,
				Month::January,
				2,
				3,
				4,
				5,
			)),
			message_number: Some(updated_message_number.clone()),
			message_sender_identifier: Some("SENDER-2".to_string()),
			message_receiver_identifier: Some("RECV-2".to_string()),
			message_date: Some("20240203040506".to_string()),
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
	let header = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(header.case_id, case_id);
	assert_eq!(header.batch_number.as_deref(), Some("BATCH"));
	assert_eq!(header.batch_sender_identifier.as_deref(), Some("BS"));
	assert_eq!(header.batch_receiver_identifier.as_deref(), Some("BR"));
	assert_eq!(
		header.batch_transmission_date,
		Some(datetime_utc(2024, Month::January, 2, 3, 4, 5))
	);
	assert_eq!(header.message_type, "ichicsr");
	assert_eq!(header.message_format_version, "2.1");
	assert_eq!(header.message_format_release, "2.0");
	assert_eq!(header.message_number, updated_message_number);
	assert_eq!(header.message_sender_identifier, "SENDER-2");
	assert_eq!(header.message_receiver_identifier, "RECV-2");
	assert_eq!(header.message_date_format, "204");
	assert_eq!(header.message_date, "20240203040506");
	let receiver = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(receiver.case_id, case_id);
	assert_eq!(receiver.receiver_type.as_deref(), Some("3"));
	assert_eq!(receiver.organization_name.as_deref(), Some("Receiver 2"));
	assert_eq!(receiver.department.as_deref(), Some("PV"));
	assert_eq!(receiver.street_address.as_deref(), Some("Street"));
	assert_eq!(receiver.city.as_deref(), Some("Seoul"));
	assert_eq!(receiver.state_province.as_deref(), Some("11"));
	assert_eq!(receiver.postcode.as_deref(), Some("12345"));
	assert_eq!(receiver.country_code.as_deref(), Some("KR"));
	assert_eq!(receiver.telephone.as_deref(), Some("010"));
	assert_eq!(receiver.fax.as_deref(), Some("020"));
	assert_eq!(receiver.email.as_deref(), Some("recv@example.com"));
	finish(&mm).await
}
