use super::save_fields_common::{
	assert_null, assert_str, get_ok, post_created, put_ok, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn message_header_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/message-header",
	}
}

fn receiver_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/receiver",
	}
}

macro_rules! message_header_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let initial_message_number = format!("MSG-{case_id}");

			post_created(
				&ctx,
				message_header_field($canonical),
				format!("/api/cases/{case_id}/message-header"),
				json!({"data": {
					"case_id": case_id,
					"message_number": initial_message_number,
					"message_sender_identifier": "SENDER",
					"message_receiver_identifier": "RECV",
					"message_date": "20240102030405"
				}}),
			)
			.await?;

			put_ok(
				&ctx,
				message_header_field($canonical),
				format!("/api/cases/{case_id}/message-header"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				message_header_field($canonical),
				format!("/api/cases/{case_id}/message-header"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! receiver_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;

			post_created(
				&ctx,
				receiver_field($canonical),
				format!("/api/cases/{case_id}/receiver"),
				json!({"data": {
					"case_id": case_id,
					"receiver_type": "2",
					"organization_name": "Receiver"
				}}),
			)
			.await?;

			put_ok(
				&ctx,
				receiver_field($canonical),
				format!("/api/cases/{case_id}/receiver"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				receiver_field($canonical),
				format!("/api/cases/{case_id}/receiver"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

message_header_single_field_test!(
	save_n_batch_number_only,
	"N.batch_number",
	json!({"batch_number": "BATCH"}),
	|value| {
		assert_str(value, "batch_number", "BATCH");
	}
);
message_header_single_field_test!(
	save_n_batch_sender_identifier_only,
	"N.batch_sender_identifier",
	json!({"batch_sender_identifier": "BS"}),
	|value| {
		assert_str(value, "batch_sender_identifier", "BS");
	}
);
message_header_single_field_test!(
	save_n_batch_receiver_identifier_only,
	"N.batch_receiver_identifier",
	json!({"batch_receiver_identifier": "BR"}),
	|value| {
		assert_str(value, "batch_receiver_identifier", "BR");
	}
);
#[tokio::test]
#[serial]
async fn save_n_message_number_only() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	let initial_message_number = format!("MSG-{case_id}");
	let updated_message_number =
		format!("MSG-UPDATED-{}", Uuid::new_v4().simple());

	post_created(
		&ctx,
		message_header_field("N.message_number"),
		format!("/api/cases/{case_id}/message-header"),
		json!({"data": {
			"case_id": case_id,
			"message_number": initial_message_number,
			"message_sender_identifier": "SENDER",
			"message_receiver_identifier": "RECV",
			"message_date": "20240102030405"
		}}),
	)
	.await?;

	put_ok(
		&ctx,
		message_header_field("N.message_number"),
		format!("/api/cases/{case_id}/message-header"),
		json!({ "data": { "message_number": updated_message_number } }),
	)
	.await?;

	let value = get_ok(
		&ctx,
		message_header_field("N.message_number"),
		format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_str(&value, "message_number", &updated_message_number);
	Ok(())
}
message_header_single_field_test!(
	save_n_message_sender_identifier_only,
	"N.message_sender_identifier",
	json!({"message_sender_identifier": "SENDER-2"}),
	|value| {
		assert_str(value, "message_sender_identifier", "SENDER-2");
	}
);
message_header_single_field_test!(
	save_n_message_receiver_identifier_only,
	"N.message_receiver_identifier",
	json!({"message_receiver_identifier": "RECV-2"}),
	|value| {
		assert_str(value, "message_receiver_identifier", "RECV-2");
	}
);
message_header_single_field_test!(
	save_n_message_date_only,
	"N.message_date",
	json!({"message_date": "20240203040506"}),
	|value| {
		assert_str(value, "message_date", "20240203040506");
	}
);

#[tokio::test]
#[serial]
async fn save_n_batch_transmission_date_only() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	let initial_message_number = format!("MSG-{case_id}");
	let batch_transmission_date = json!([2024, 32, 1, 1, 1, 0, 0, 0, 0]);

	post_created(
		&ctx,
		message_header_field("N.batch_transmission_date"),
		format!("/api/cases/{case_id}/message-header"),
		json!({"data": {
			"case_id": case_id,
			"message_number": initial_message_number,
			"message_sender_identifier": "SENDER",
			"message_receiver_identifier": "RECV",
			"message_date": "20240102030405"
		}}),
	)
	.await?;

	put_ok(
		&ctx,
		message_header_field("N.batch_transmission_date"),
		format!("/api/cases/{case_id}/message-header"),
		json!({ "data": { "batch_transmission_date": batch_transmission_date } }),
	)
	.await?;

	let value = get_ok(
		&ctx,
		message_header_field("N.batch_transmission_date"),
		format!("/api/cases/{case_id}/message-header"),
	)
	.await?;
	assert_eq!(
		value["data"]["batch_transmission_date"],
		json!([2024, 32, 1, 1, 1, 0, 0, 0, 0])
	);
	Ok(())
}

receiver_single_field_test!(
	save_n_receiver_type_only,
	"N.receiver_type",
	json!({"receiver_type": "3"}),
	|value| {
		assert_str(value, "receiver_type", "3");
	}
);
receiver_single_field_test!(
	save_n_receiver_organization_name_only,
	"N.organization_name",
	json!({"organization_name": "Receiver 2"}),
	|value| {
		assert_str(value, "organization_name", "Receiver 2");
	}
);
receiver_single_field_test!(
	save_n_receiver_department_only,
	"N.department",
	json!({"department": "PV"}),
	|value| {
		assert_str(value, "department", "PV");
	}
);
receiver_single_field_test!(
	save_n_receiver_street_address_only,
	"N.street_address",
	json!({"street_address": "Street"}),
	|value| {
		assert_str(value, "street_address", "Street");
	}
);
receiver_single_field_test!(
	save_n_receiver_city_only,
	"N.city",
	json!({"city": "Seoul"}),
	|value| {
		assert_str(value, "city", "Seoul");
	}
);
receiver_single_field_test!(
	save_n_receiver_state_province_only,
	"N.state_province",
	json!({"state_province": "11"}),
	|value| {
		assert_str(value, "state_province", "11");
	}
);
receiver_single_field_test!(
	save_n_receiver_postcode_only,
	"N.postcode",
	json!({"postcode": "12345"}),
	|value| {
		assert_str(value, "postcode", "12345");
	}
);
receiver_single_field_test!(
	save_n_receiver_country_code_only,
	"N.country_code",
	json!({"country_code": "KR"}),
	|value| {
		assert_str(value, "country_code", "KR");
	}
);
receiver_single_field_test!(
	save_n_receiver_telephone_only,
	"N.telephone",
	json!({"telephone": "010"}),
	|value| {
		assert_str(value, "telephone", "010");
	}
);
receiver_single_field_test!(
	save_n_receiver_fax_only,
	"N.fax",
	json!({"fax": "020"}),
	|value| {
		assert_str(value, "fax", "020");
	}
);
receiver_single_field_test!(
	save_n_receiver_email_only,
	"N.email",
	json!({"email": "recv@example.com"}),
	|value| {
		assert_str(value, "email", "recv@example.com");
		assert_null(value, "review_receivers_json");
	}
);
