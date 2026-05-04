use super::save_fields_common::{
	assert_bool, assert_date_tuple, assert_i64, assert_null, assert_str,
	create_case_with_field, extract_id, get_ok, post_created, put_ok, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, request_json, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

fn case_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}",
	}
}

fn safety_report_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/safety-report",
	}
}

fn sender_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/safety-report/senders/{sender_id}",
	}
}

fn receiver_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/receiver",
	}
}

fn primary_source_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/safety-report/primary-sources/{primary_source_id}",
	}
}

fn other_identifier_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/other-identifiers/{identifier_id}",
	}
}

fn linked_report_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/linked-reports/{linked_report_id}",
	}
}

fn document_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/safety-report/documents/{document_id}",
	}
}

fn literature_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/safety-report/literature/{literature_id}",
	}
}

fn study_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/safety-report/studies/{study_id}",
	}
}

fn study_registration_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/safety-report/studies/{study_id}/registrations/{registration_id}",
	}
}

async fn create_safety_report(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	post_created(
		ctx,
		safety_report_field("C.1"),
		format!("/api/cases/{case_id}/safety-report"),
		json!({"data": {
			"case_id": case_id
		}}),
	)
	.await?;
	Ok(())
}

async fn create_sender(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		sender_field("C.2"),
		format!("/api/cases/{case_id}/safety-report/senders"),
		json!({"data": {
			"case_id": case_id
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_receiver(ctx: &PersistTestCtx, case_id: Uuid) -> Result<()> {
	post_created(
		ctx,
		receiver_field("C.3.receiver"),
		format!("/api/cases/{case_id}/receiver"),
		json!({"data": {
			"case_id": case_id
		}}),
	)
	.await?;
	Ok(())
}

async fn create_primary_source(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		primary_source_field("C.2.r"),
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

#[tokio::test]
#[serial]
async fn save_c_2_full_surface_on_first_create_persists() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	let value = post_created(
		&ctx,
		sender_field("C.2.create"),
		format!("/api/cases/{case_id}/safety-report/senders"),
		json!({"data": {
			"case_id": case_id,
			"sender_type": "2",
			"organization_name": "Org 2",
			"department": "Dept",
			"street_address": "123 St",
			"city": "Seoul",
			"state": "11",
			"postcode": "12345",
			"country_code": "KR",
			"person_title": "Dr",
			"person_given_name": "Given",
			"person_middle_name": "Mid",
			"person_family_name": "Family",
			"telephone": "010",
			"fax": "020",
			"email": "sender@example.com"
		}}),
	)
	.await?;
	let sender_id = extract_id(&value)?;

	let value = get_ok(
		&ctx,
		sender_field("C.2.create"),
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
	)
	.await?;
	assert_str(&value, "sender_type", "2");
	assert_str(&value, "organization_name", "Org 2");
	assert_str(&value, "department", "Dept");
	assert_str(&value, "street_address", "123 St");
	assert_str(&value, "city", "Seoul");
	assert_str(&value, "state", "11");
	assert_str(&value, "postcode", "12345");
	assert_str(&value, "country_code", "KR");
	assert_str(&value, "person_title", "Dr");
	assert_str(&value, "person_given_name", "Given");
	assert_str(&value, "person_middle_name", "Mid");
	assert_str(&value, "person_family_name", "Family");
	assert_str(&value, "telephone", "010");
	assert_str(&value, "fax", "020");
	assert_str(&value, "email", "sender@example.com");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_3_receiver_full_surface_on_first_create_persists() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	post_created(
		&ctx,
		receiver_field("C.3.receiver.create"),
		format!("/api/cases/{case_id}/receiver"),
		json!({"data": {
			"case_id": case_id,
			"receiver_type": "3",
			"organization_name": "Receiver",
			"department": "PV",
			"street_address": "Street",
			"city": "Seoul",
			"state_province": "11",
			"postcode": "12345",
			"country_code": "KR",
			"telephone": "010",
			"fax": "020",
			"email": "recv@example.com"
		}}),
	)
	.await?;

	let value = get_ok(
		&ctx,
		receiver_field("C.3.receiver.create"),
		format!("/api/cases/{case_id}/receiver"),
	)
	.await?;
	assert_str(&value, "receiver_type", "3");
	assert_str(&value, "organization_name", "Receiver");
	assert_str(&value, "department", "PV");
	assert_str(&value, "street_address", "Street");
	assert_str(&value, "city", "Seoul");
	assert_str(&value, "state_province", "11");
	assert_str(&value, "postcode", "12345");
	assert_str(&value, "country_code", "KR");
	assert_str(&value, "telephone", "010");
	assert_str(&value, "fax", "020");
	assert_str(&value, "email", "recv@example.com");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_2_r_full_surface_on_first_create_persists() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	let value = post_created(
		&ctx,
		primary_source_field("C.2.r.create"),
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"reporter_title": "Dr",
			"reporter_given_name": "Jane",
			"reporter_middle_name": "Q",
			"reporter_family_name": "Doe",
			"organization": "Hospital",
			"department": "ER",
			"street": "Street",
			"city": "Seoul",
			"state": "11",
			"postcode": "12345",
			"telephone": "010",
			"country_code": "KR",
			"email": "jane@example.com",
			"qualification": "1",
			"qualification_kr1": "1",
			"primary_source_regulatory": "1"
		}}),
	)
	.await?;
	let primary_source_id = extract_id(&value)?;

	let value = get_ok(
		&ctx,
		primary_source_field("C.2.r.create"),
		format!(
			"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
		),
	)
	.await?;
	assert_i64(&value, "sequence_number", 1);
	assert_str(&value, "reporter_title", "Dr");
	assert_str(&value, "reporter_given_name", "Jane");
	assert_str(&value, "reporter_middle_name", "Q");
	assert_str(&value, "reporter_family_name", "Doe");
	assert_str(&value, "organization", "Hospital");
	assert_str(&value, "department", "ER");
	assert_str(&value, "street", "Street");
	assert_str(&value, "city", "Seoul");
	assert_str(&value, "state", "11");
	assert_str(&value, "postcode", "12345");
	assert_str(&value, "telephone", "010");
	assert_str(&value, "country_code", "KR");
	assert_str(&value, "email", "jane@example.com");
	assert_str(&value, "qualification", "1");
	assert_str(&value, "qualification_kr1", "1");
	assert_str(&value, "primary_source_regulatory", "1");
	Ok(())
}

async fn create_other_identifier(
	ctx: &PersistTestCtx,
	case_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		other_identifier_field("C.3.1.r"),
		format!("/api/cases/{case_id}/other-identifiers"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"source_of_identifier": "FDA",
			"case_identifier": "CASE-1"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_linked_report(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		linked_report_field("C.3.2.r"),
		format!("/api/cases/{case_id}/linked-reports"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"linked_report_number": "LINK-1"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_document(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		document_field("C.4.r"),
		format!("/api/cases/{case_id}/safety-report/documents"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_literature(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		literature_field("C.4"),
		format!("/api/cases/{case_id}/safety-report/literature"),
		json!({"data": {
			"case_id": case_id,
			"reference_text": "Seed reference",
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_study(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		study_field("C.5"),
		format!("/api/cases/{case_id}/safety-report/studies"),
		json!({"data": {
			"case_id": case_id,
			"study_name": "Study",
			"sponsor_study_number": "SP-1"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_study_registration(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	study_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		study_registration_field("C.5.r"),
		format!(
			"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"
		),
		json!({"data": {
			"study_information_id": study_id,
			"registration_number": "REG-1",
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

macro_rules! safety_report_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			create_safety_report(&ctx, case_id).await?;

			put_ok(
				&ctx,
				safety_report_field($canonical),
				format!("/api/cases/{case_id}/safety-report"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				safety_report_field($canonical),
				format!("/api/cases/{case_id}/safety-report"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! sender_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let sender_id = create_sender(&ctx, case_id).await?;

			put_ok(
				&ctx,
				sender_field($canonical),
				format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				sender_field($canonical),
				format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
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
			create_receiver(&ctx, case_id).await?;

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

macro_rules! primary_source_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let primary_source_id = create_primary_source(&ctx, case_id).await?;

			put_ok(
				&ctx,
				primary_source_field($canonical),
				format!(
					"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				primary_source_field($canonical),
				format!(
					"/api/cases/{case_id}/safety-report/primary-sources/{primary_source_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! other_identifier_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let identifier_id = create_other_identifier(&ctx, case_id).await?;

			put_ok(
				&ctx,
				other_identifier_field($canonical),
				format!("/api/cases/{case_id}/other-identifiers/{identifier_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				other_identifier_field($canonical),
				format!("/api/cases/{case_id}/other-identifiers/{identifier_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! linked_report_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let linked_report_id = create_linked_report(&ctx, case_id).await?;

			put_ok(
				&ctx,
				linked_report_field($canonical),
				format!("/api/cases/{case_id}/linked-reports/{linked_report_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				linked_report_field($canonical),
				format!("/api/cases/{case_id}/linked-reports/{linked_report_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! document_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let document_id = create_document(&ctx, case_id).await?;

			put_ok(
				&ctx,
				document_field($canonical),
				format!("/api/cases/{case_id}/safety-report/documents/{document_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				document_field($canonical),
				format!("/api/cases/{case_id}/safety-report/documents/{document_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! literature_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let literature_id = create_literature(&ctx, case_id).await?;

			put_ok(
				&ctx,
				literature_field($canonical),
				format!("/api/cases/{case_id}/safety-report/literature/{literature_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				literature_field($canonical),
				format!("/api/cases/{case_id}/safety-report/literature/{literature_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! study_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let study_id = create_study(&ctx, case_id).await?;

			put_ok(
				&ctx,
				study_field($canonical),
				format!("/api/cases/{case_id}/safety-report/studies/{study_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				study_field($canonical),
				format!("/api/cases/{case_id}/safety-report/studies/{study_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! study_registration_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let study_id = create_study(&ctx, case_id).await?;
			let registration_id = create_study_registration(&ctx, case_id, study_id).await?;

			put_ok(
				&ctx,
				study_registration_field($canonical),
				format!(
					"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{registration_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				study_registration_field($canonical),
				format!(
					"/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{registration_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

#[tokio::test]
#[serial]
async fn save_c_1_1_safety_report_id_only() -> Result<()> {
	let ctx = setup().await?;
	let safety_report_id = format!("SAVE-C-1-{}", Uuid::new_v4().simple());
	let (_, case_id) = create_case_with_field(
		&ctx,
		"C.1.1",
		"safety_report_id",
		json!(safety_report_id),
	)
	.await?;

	let value =
		get_ok(&ctx, case_field("C.1.1"), format!("/api/cases/{case_id}")).await?;
	assert_str(&value, "safety_report_id", &safety_report_id);
	assert_str(&value, "status", "draft");
	Ok(())
}

macro_rules! case_create_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let mut data = json!({
				"organization_id": ctx.org_id,
				"safety_report_id": format!("SAVE-C-{}", Uuid::new_v4().simple()),
				"status": "draft",
				"appendices_json": "[\"mfds\"]"
			});
			if let Some(data_obj) = data.as_object_mut() {
				if let Some(extra_obj) = json!($payload).as_object() {
					for (key, value) in extra_obj {
						data_obj.insert(key.clone(), value.clone());
					}
				}
			}

			let value = post_created(
				&ctx,
				case_field($canonical),
				"/api/cases".to_string(),
				json!({ "data": data }),
			)
			.await?;
			let case_id = extract_id(&value)?;
			let value = get_ok(&ctx, case_field($canonical), format!("/api/cases/{case_id}"))
				.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

async fn get_safety_report_with_deadlock_retry(
	ctx: &PersistTestCtx,
	canonical_id: &'static str,
	case_id: Uuid,
) -> Result<serde_json::Value> {
	let uri = format!("/api/cases/{case_id}/safety-report");
	let field = safety_report_field(canonical_id);
	let mut last_error = None;

	for _ in 0..5 {
		let (status, body) =
			request_json(&ctx.app, &ctx.cookie, "GET", uri.clone(), None).await?;
		if status.is_success() {
			return Ok(body);
		}
		if body.to_string().contains("deadlock detected") {
			last_error = Some(format!(
				"{} read via {} hit transient deadlock: status={} body={}",
				field.canonical_id, field.endpoint, status, body
			));
			sleep(Duration::from_millis(100)).await;
			continue;
		}
		return Err(format!(
			"{} read via {} failed: status={} uri={} body={}",
			field.canonical_id, field.endpoint, status, uri, body
		)
		.into());
	}

	Err(last_error
		.unwrap_or_else(|| {
			format!(
				"{} read via {} failed after retries",
				field.canonical_id, field.endpoint
			)
		})
		.into())
}

safety_report_single_field_test!(
	save_c_1_transmission_date_only,
	"C.1.2",
	json!({"transmission_date": [2024, 2, 1]}),
	|value| {
		assert_date_tuple(value, "transmission_date", &[2024, 32]);
	}
);
safety_report_single_field_test!(
	save_c_1_transmission_date_null_flavor_only,
	"C.1.2.null_flavor",
	json!({"transmission_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "transmission_date_null_flavor", "UNK");
	}
);
safety_report_single_field_test!(
	save_c_1_report_type_only,
	"C.1.3",
	json!({"report_type": "2"}),
	|value| {
		assert_str(value, "report_type", "2");
	}
);
safety_report_single_field_test!(
	save_c_1_date_first_received_from_source_only,
	"C.1.4",
	json!({"date_first_received_from_source": [2024, 2, 2]}),
	|value| {
		assert_date_tuple(value, "date_first_received_from_source", &[2024, 33]);
	}
);
safety_report_single_field_test!(
	save_c_1_date_first_received_from_source_null_flavor_only,
	"C.1.4.null_flavor",
	json!({"date_first_received_from_source_null_flavor": "NI"}),
	|value| {
		assert_str(value, "date_first_received_from_source_null_flavor", "NI");
	}
);
safety_report_single_field_test!(
	save_c_1_date_of_most_recent_information_only,
	"C.1.5",
	json!({"date_of_most_recent_information": [2024, 2, 3]}),
	|value| {
		assert_date_tuple(value, "date_of_most_recent_information", &[2024, 34]);
	}
);
safety_report_single_field_test!(
	save_c_1_date_of_most_recent_information_null_flavor_only,
	"C.1.5.null_flavor",
	json!({"date_of_most_recent_information_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "date_of_most_recent_information_null_flavor", "ASKU");
	}
);
safety_report_single_field_test!(
	save_c_1_fulfil_expedited_criteria_only,
	"C.1.7",
	json!({"fulfil_expedited_criteria": false}),
	|value| {
		assert_bool(value, "fulfil_expedited_criteria", false);
	}
);
safety_report_single_field_test!(
	save_c_1_local_criteria_report_type_only,
	"FDA.C.1.7.1",
	json!({"local_criteria_report_type": "LOCAL"}),
	|value| {
		assert_str(value, "local_criteria_report_type", "LOCAL");
	}
);
safety_report_single_field_test!(
	save_c_1_combination_product_report_indicator_only,
	"FDA.C.1.12",
	json!({"combination_product_report_indicator": "1"}),
	|value| {
		assert_str(value, "combination_product_report_indicator", "1");
	}
);
safety_report_single_field_test!(
	save_c_1_worldwide_unique_id_only,
	"C.1.8.1",
	json!({"worldwide_unique_id": "WID"}),
	|value| {
		assert_str(value, "worldwide_unique_id", "WID");
	}
);
safety_report_single_field_test!(
	save_c_1_first_sender_type_only,
	"C.1.8.2",
	json!({"first_sender_type": "1"}),
	|value| {
		assert_str(value, "first_sender_type", "1");
	}
);
safety_report_single_field_test!(
	save_c_1_additional_documents_available_only,
	"C.1.6.1",
	json!({"additional_documents_available": false}),
	|value| {
		assert_bool(value, "additional_documents_available", false);
	}
);
safety_report_single_field_test!(
	save_c_1_other_case_identifiers_exist_only,
	"C.1.9.1",
	json!({"other_case_identifiers_exist": true}),
	|value| {
		assert_bool(value, "other_case_identifiers_exist", true);
	}
);

#[tokio::test]
#[serial]
async fn save_c_1_local_criteria_report_type_on_first_create_persists() -> Result<()>
{
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	post_created(
		&ctx,
		safety_report_field("FDA.C.1.7.1.create"),
		format!("/api/cases/{case_id}/safety-report"),
		json!({"data": {
			"case_id": case_id,
			"fulfil_expedited_criteria": true,
			"local_criteria_report_type": "1"
		}}),
	)
	.await?;

	let value = get_ok(
		&ctx,
		safety_report_field("FDA.C.1.7.1.create"),
		format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_bool(&value, "fulfil_expedited_criteria", true);
	assert_str(&value, "local_criteria_report_type", "1");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_1_combination_product_report_indicator_on_first_create_persists(
) -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	post_created(
		&ctx,
		safety_report_field("FDA.C.1.12.create"),
		format!("/api/cases/{case_id}/safety-report"),
		json!({"data": {
			"case_id": case_id,
			"combination_product_report_indicator": "2"
		}}),
	)
	.await?;

	let value = get_ok(
		&ctx,
		safety_report_field("FDA.C.1.12.create"),
		format!("/api/cases/{case_id}/safety-report"),
	)
	.await?;
	assert_str(&value, "combination_product_report_indicator", "2");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_1_nullification_code_only() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	create_safety_report(&ctx, case_id).await?;

	put_ok(
		&ctx,
		safety_report_field("C.1.11.2"),
		format!("/api/cases/{case_id}/safety-report"),
		json!({ "data": { "nullification_reason": "Seed reason" } }),
	)
	.await?;

	put_ok(
		&ctx,
		safety_report_field("C.1.11.1"),
		format!("/api/cases/{case_id}/safety-report"),
		json!({
			"data": { "nullification_code": "1" },
			"reason_for_change": "strict save-fields nullification transition",
			"e_signature": {
				"meaning": "nullify case",
				"password": "adminpwd"
			}
		}),
	)
	.await?;

	let value =
		get_safety_report_with_deadlock_retry(&ctx, "C.1.11.1", case_id).await?;
	assert_str(&value, "nullification_code", "1");
	Ok(())
}
safety_report_single_field_test!(
	save_c_1_nullification_reason_only,
	"C.1.11.2",
	json!({"nullification_reason": "Reason"}),
	|value| {
		assert_str(value, "nullification_reason", "Reason");
	}
);
safety_report_single_field_test!(
	save_c_1_receiver_organization_only,
	"C.3.receiver.1",
	json!({"receiver_organization": "Receiver"}),
	|value| {
		assert_str(value, "receiver_organization", "Receiver");
	}
);

case_create_single_field_test!(
	save_c_mfds_ci_on_create_only,
	"MFDS.CI.create",
	json!({"mfds_report_type": "Spontaneous"}),
	|value| {
		assert_str(value, "mfds_report_type", "Spontaneous");
	}
);
case_create_single_field_test!(
	save_c_report_year_on_create_only,
	"Non-E2B.reportYear.create",
	json!({"report_year": "2026"}),
	|value| {
		assert_str(value, "report_year", "2026");
	}
);
case_create_single_field_test!(
	save_c_source_document_name_on_create_only,
	"Non-E2B.sourceDocument.name.create",
	json!({"source_document_name": "source.txt"}),
	|value| {
		assert_str(value, "source_document_name", "source.txt");
	}
);
case_create_single_field_test!(
	save_c_source_document_base64_on_create_only,
	"Non-E2B.sourceDocument.base64.create",
	json!({"source_document_base64": "U09VUkNF"}),
	|value| {
		assert_str(value, "source_document_base64", "U09VUkNF");
	}
);
case_create_single_field_test!(
	save_c_source_document_media_type_on_create_only,
	"Non-E2B.sourceDocument.mediaType.create",
	json!({"source_document_media_type": "text/plain"}),
	|value| {
		assert_str(value, "source_document_media_type", "text/plain");
	}
);

#[tokio::test]
#[serial]
async fn save_c_mfds_ci_only() -> Result<()> {
	let ctx = setup().await?;
	let (_, case_id) = create_case_with_field(
		&ctx,
		"MFDS.CI",
		"mfds_report_type",
		json!("Spontaneous"),
	)
	.await?;
	let value =
		get_ok(&ctx, case_field("MFDS.CI"), format!("/api/cases/{case_id}")).await?;
	assert_str(&value, "mfds_report_type", "Spontaneous");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_report_year_only() -> Result<()> {
	let ctx = setup().await?;
	let (_, case_id) = create_case_with_field(
		&ctx,
		"Non-E2B.reportYear",
		"report_year",
		json!("2026"),
	)
	.await?;
	let value = get_ok(
		&ctx,
		case_field("Non-E2B.reportYear"),
		format!("/api/cases/{case_id}"),
	)
	.await?;
	assert_str(&value, "report_year", "2026");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_source_document_name_only() -> Result<()> {
	let ctx = setup().await?;
	let (_, case_id) = create_case_with_field(
		&ctx,
		"Non-E2B.sourceDocument.name",
		"source_document_name",
		json!("source.txt"),
	)
	.await?;
	let value = get_ok(
		&ctx,
		case_field("Non-E2B.sourceDocument.name"),
		format!("/api/cases/{case_id}"),
	)
	.await?;
	assert_str(&value, "source_document_name", "source.txt");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_source_document_base64_only() -> Result<()> {
	let ctx = setup().await?;
	let (_, case_id) = create_case_with_field(
		&ctx,
		"Non-E2B.sourceDocument.base64",
		"source_document_base64",
		json!("U09VUkNF"),
	)
	.await?;
	let value = get_ok(
		&ctx,
		case_field("Non-E2B.sourceDocument.base64"),
		format!("/api/cases/{case_id}"),
	)
	.await?;
	assert_str(&value, "source_document_base64", "U09VUkNF");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_source_document_media_type_only() -> Result<()> {
	let ctx = setup().await?;
	let (_, case_id) = create_case_with_field(
		&ctx,
		"Non-E2B.sourceDocument.mediaType",
		"source_document_media_type",
		json!("text/plain"),
	)
	.await?;
	let value = get_ok(
		&ctx,
		case_field("Non-E2B.sourceDocument.mediaType"),
		format!("/api/cases/{case_id}"),
	)
	.await?;
	assert_str(&value, "source_document_media_type", "text/plain");
	Ok(())
}

sender_single_field_test!(
	save_c_2_sender_type_only,
	"C.3.1",
	json!({"sender_type": "2"}),
	|value| {
		assert_str(value, "sender_type", "2");
	}
);
sender_single_field_test!(
	save_c_2_organization_name_only,
	"C.3.2",
	json!({"organization_name": "Org 2"}),
	|value| {
		assert_str(value, "organization_name", "Org 2");
	}
);
sender_single_field_test!(
	save_c_2_department_only,
	"C.3.3.1",
	json!({"department": "Dept"}),
	|value| {
		assert_str(value, "department", "Dept");
	}
);
sender_single_field_test!(
	save_c_2_street_address_only,
	"C.3.4.1",
	json!({"street_address": "123 St"}),
	|value| {
		assert_str(value, "street_address", "123 St");
	}
);
sender_single_field_test!(
	save_c_2_city_only,
	"C.3.4.2",
	json!({"city": "Seoul"}),
	|value| {
		assert_str(value, "city", "Seoul");
	}
);
sender_single_field_test!(
	save_c_2_state_only,
	"C.3.4.3",
	json!({"state": "11"}),
	|value| {
		assert_str(value, "state", "11");
	}
);
sender_single_field_test!(
	save_c_2_postcode_only,
	"C.3.4.4",
	json!({"postcode": "12345"}),
	|value| {
		assert_str(value, "postcode", "12345");
	}
);
sender_single_field_test!(
	save_c_2_country_code_only,
	"C.3.4.5",
	json!({"country_code": "KR"}),
	|value| {
		assert_str(value, "country_code", "KR");
	}
);
sender_single_field_test!(
	save_c_2_person_title_only,
	"C.3.3.2",
	json!({"person_title": "Dr"}),
	|value| {
		assert_str(value, "person_title", "Dr");
	}
);
sender_single_field_test!(
	save_c_2_person_given_name_only,
	"C.3.3.3",
	json!({"person_given_name": "Given"}),
	|value| {
		assert_str(value, "person_given_name", "Given");
	}
);
sender_single_field_test!(
	save_c_2_person_middle_name_only,
	"C.3.3.4",
	json!({"person_middle_name": "Mid"}),
	|value| {
		assert_str(value, "person_middle_name", "Mid");
	}
);
sender_single_field_test!(
	save_c_2_person_family_name_only,
	"C.3.3.5",
	json!({"person_family_name": "Family"}),
	|value| {
		assert_str(value, "person_family_name", "Family");
	}
);
sender_single_field_test!(
	save_c_2_telephone_only,
	"C.3.4.6",
	json!({"telephone": "010"}),
	|value| {
		assert_str(value, "telephone", "010");
	}
);
sender_single_field_test!(
	save_c_2_fax_only,
	"C.3.4.7",
	json!({"fax": "020"}),
	|value| {
		assert_str(value, "fax", "020");
	}
);
sender_single_field_test!(
	save_c_2_email_only,
	"C.3.4.8",
	json!({"email": "sender@example.com"}),
	|value| {
		assert_str(value, "email", "sender@example.com");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_type_only,
	"C.3.receiver.2",
	json!({"receiver_type": "3"}),
	|value| {
		assert_str(value, "receiver_type", "3");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_department_only,
	"C.3.receiver.3",
	json!({"department": "PV"}),
	|value| {
		assert_str(value, "department", "PV");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_street_only,
	"C.3.receiver.4",
	json!({"street_address": "Street"}),
	|value| {
		assert_str(value, "street_address", "Street");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_city_only,
	"C.3.receiver.5",
	json!({"city": "Seoul"}),
	|value| {
		assert_str(value, "city", "Seoul");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_state_only,
	"C.3.receiver.6",
	json!({"state_province": "11"}),
	|value| {
		assert_str(value, "state_province", "11");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_postcode_only,
	"C.3.receiver.7",
	json!({"postcode": "12345"}),
	|value| {
		assert_str(value, "postcode", "12345");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_telephone_only,
	"C.3.receiver.8",
	json!({"telephone": "010"}),
	|value| {
		assert_str(value, "telephone", "010");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_fax_only,
	"C.3.receiver.9",
	json!({"fax": "020"}),
	|value| {
		assert_str(value, "fax", "020");
	}
);
receiver_single_field_test!(
	save_c_3_receiver_email_only,
	"C.3.receiver.10",
	json!({"email": "recv@example.com"}),
	|value| {
		assert_str(value, "email", "recv@example.com");
	}
);

#[tokio::test]
#[serial]
async fn save_c_3_1_sender_type_without_sender_organization_persists() -> Result<()>
{
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	let value = post_created(
		&ctx,
		sender_field("C.3.1"),
		format!("/api/cases/{case_id}/safety-report/senders"),
		json!({"data": {
			"case_id": case_id,
			"sender_type": "2"
		}}),
	)
	.await?;
	let sender_id = extract_id(&value)?;

	let value = get_ok(
		&ctx,
		sender_field("C.3.1"),
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
	)
	.await?;
	assert_str(&value, "sender_type", "2");
	assert_null(&value, "organization_name");
	Ok(())
}

#[tokio::test]
#[serial]
async fn save_c_3_2_sender_organization_without_sender_type_persists() -> Result<()>
{
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;

	let value = post_created(
		&ctx,
		sender_field("C.3.2"),
		format!("/api/cases/{case_id}/safety-report/senders"),
		json!({"data": {
			"case_id": case_id,
			"organization_name": "Partial Sender Org"
		}}),
	)
	.await?;
	let sender_id = extract_id(&value)?;

	let value = get_ok(
		&ctx,
		sender_field("C.3.2"),
		format!("/api/cases/{case_id}/safety-report/senders/{sender_id}"),
	)
	.await?;
	assert_null(&value, "sender_type");
	assert_str(&value, "organization_name", "Partial Sender Org");
	Ok(())
}

primary_source_single_field_test!(
	save_c_2_r_reporter_title_only,
	"C.2.r.1.1",
	json!({"reporter_title": "Prof"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "reporter_title", "Prof");
	}
);
primary_source_single_field_test!(
	save_c_2_r_reporter_given_name_only,
	"C.2.r.1.2",
	json!({"reporter_given_name": "John"}),
	|value| {
		assert_str(value, "reporter_given_name", "John");
	}
);
primary_source_single_field_test!(
	save_c_2_r_reporter_middle_name_only,
	"C.2.r.1.3",
	json!({"reporter_middle_name": "M"}),
	|value| {
		assert_str(value, "reporter_middle_name", "M");
	}
);
primary_source_single_field_test!(
	save_c_2_r_reporter_family_name_only,
	"C.2.r.1.4",
	json!({"reporter_family_name": "Smith"}),
	|value| {
		assert_str(value, "reporter_family_name", "Smith");
	}
);
primary_source_single_field_test!(
	save_c_2_r_organization_only,
	"C.2.r.2.1",
	json!({"organization": "Clinic"}),
	|value| {
		assert_str(value, "organization", "Clinic");
	}
);
primary_source_single_field_test!(
	save_c_2_r_department_only,
	"C.2.r.2.2",
	json!({"department": "PV"}),
	|value| {
		assert_str(value, "department", "PV");
	}
);
primary_source_single_field_test!(
	save_c_2_r_street_only,
	"C.2.r.2.3",
	json!({"street": "Road"}),
	|value| {
		assert_str(value, "street", "Road");
	}
);
primary_source_single_field_test!(
	save_c_2_r_city_only,
	"C.2.r.2.4",
	json!({"city": "Busan"}),
	|value| {
		assert_str(value, "city", "Busan");
	}
);
primary_source_single_field_test!(
	save_c_2_r_state_only,
	"C.2.r.2.5",
	json!({"state": "26"}),
	|value| {
		assert_str(value, "state", "26");
	}
);
primary_source_single_field_test!(
	save_c_2_r_postcode_only,
	"C.2.r.2.6",
	json!({"postcode": "54321"}),
	|value| {
		assert_str(value, "postcode", "54321");
	}
);
primary_source_single_field_test!(
	save_c_2_r_telephone_only,
	"C.2.r.2.7",
	json!({"telephone": "021"}),
	|value| {
		assert_str(value, "telephone", "021");
	}
);
primary_source_single_field_test!(
	save_c_2_r_country_code_only,
	"C.2.r.3",
	json!({"country_code": "US"}),
	|value| {
		assert_str(value, "country_code", "US");
	}
);
primary_source_single_field_test!(
	save_c_2_r_email_only,
	"C.2.r.2.8",
	json!({"email": "john@example.com"}),
	|value| {
		assert_str(value, "email", "john@example.com");
	}
);
primary_source_single_field_test!(
	save_c_2_r_qualification_only,
	"C.2.r.4",
	json!({"qualification": "2"}),
	|value| {
		assert_str(value, "qualification", "2");
	}
);
primary_source_single_field_test!(
	save_c_2_r_qualification_kr1_only,
	"C.2.r.qualification_kr1",
	json!({"qualification_kr1": "2"}),
	|value| {
		assert_str(value, "qualification_kr1", "2");
	}
);
primary_source_single_field_test!(
	save_c_2_r_primary_source_regulatory_only,
	"C.2.r.5",
	json!({"primary_source_regulatory": "1"}),
	|value| {
		assert_str(value, "primary_source_regulatory", "1");
	}
);

other_identifier_single_field_test!(
	save_c_3_1_r_source_of_identifier_only,
	"C.3.1.r.source_of_identifier",
	json!({"source_of_identifier": "MFDS"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "source_of_identifier", "MFDS");
	}
);
other_identifier_single_field_test!(
	save_c_3_1_r_case_identifier_only,
	"C.3.1.r.case_identifier",
	json!({"case_identifier": "CASE-2"}),
	|value| {
		assert_str(value, "case_identifier", "CASE-2");
	}
);

linked_report_single_field_test!(
	save_c_3_2_r_linked_report_number_only,
	"C.3.2.r.linked_report_number",
	json!({"linked_report_number": "LINK-2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "linked_report_number", "LINK-2");
	}
);

document_single_field_test!(
	save_c_4_r_title_only,
	"C.4.r.title",
	json!({"title": "Title 2"}),
	|value| {
		assert_str(value, "title", "Title 2");
	}
);
document_single_field_test!(
	save_c_4_r_document_base64_only,
	"C.4.r.document_base64",
	json!({"document_base64": "BASE64-2"}),
	|value| {
		assert_str(value, "document_base64", "BASE64-2");
	}
);
document_single_field_test!(
	save_c_4_r_media_type_only,
	"C.4.r.media_type",
	json!({"media_type": "text/plain"}),
	|value| {
		assert_str(value, "media_type", "text/plain");
	}
);
document_single_field_test!(
	save_c_4_r_representation_only,
	"C.4.r.representation",
	json!({"representation": "TXT"}),
	|value| {
		assert_str(value, "representation", "TXT");
	}
);
document_single_field_test!(
	save_c_4_r_compression_only,
	"C.4.r.compression",
	json!({"compression": "zip"}),
	|value| {
		assert_str(value, "compression", "zip");
	}
);
document_single_field_test!(
	save_c_4_r_sequence_number_only,
	"C.4.r.sequence_number",
	json!({"sequence_number": 2}),
	|value| {
		assert_i64(value, "sequence_number", 2);
	}
);

literature_single_field_test!(
	save_c_4_reference_text_only,
	"C.4.r.1",
	json!({"reference_text": "Ref 2"}),
	|value| {
		assert_str(value, "reference_text", "Ref 2");
	}
);
literature_single_field_test!(
	save_c_4_sequence_number_only,
	"C.4.sequence_number",
	json!({"sequence_number": 2}),
	|value| {
		assert_i64(value, "sequence_number", 2);
	}
);
literature_single_field_test!(
	save_c_4_document_base64_only,
	"C.4.document_base64",
	json!({"document_base64": "BASE64-2"}),
	|value| {
		assert_str(value, "document_base64", "BASE64-2");
	}
);
literature_single_field_test!(
	save_c_4_media_type_only,
	"C.4.media_type",
	json!({"media_type": "text/plain"}),
	|value| {
		assert_str(value, "media_type", "text/plain");
	}
);
literature_single_field_test!(
	save_c_4_representation_only,
	"C.4.representation",
	json!({"representation": "TXT"}),
	|value| {
		assert_str(value, "representation", "TXT");
	}
);
literature_single_field_test!(
	save_c_4_compression_only,
	"C.4.compression",
	json!({"compression": "zip"}),
	|value| {
		assert_str(value, "compression", "zip");
	}
);

study_single_field_test!(
	save_c_5_study_name_only,
	"C.5.2",
	json!({"study_name": "Study 2"}),
	|value| {
		assert_str(value, "study_name", "Study 2");
	}
);
study_single_field_test!(
	save_c_5_sponsor_study_number_only,
	"C.5.3",
	json!({"sponsor_study_number": "SP-2"}),
	|value| {
		assert_str(value, "sponsor_study_number", "SP-2");
	}
);
study_single_field_test!(
	save_c_5_study_type_reaction_only,
	"C.5.4",
	json!({"study_type_reaction": "2"}),
	|value| {
		assert_str(value, "study_type_reaction", "2");
	}
);
study_single_field_test!(
	save_c_5_study_type_reaction_kr1_only,
	"C.5.study_type_reaction_kr1",
	json!({"study_type_reaction_kr1": "2"}),
	|value| {
		assert_str(value, "study_type_reaction_kr1", "2");
	}
);

study_registration_single_field_test!(
	save_c_5_r_registration_number_only,
	"C.5.1.r.1",
	json!({"registration_number": "REG-2"}),
	|value| {
		assert_str(value, "registration_number", "REG-2");
	}
);
study_registration_single_field_test!(
	save_c_5_r_country_code_only,
	"C.5.1.r.2",
	json!({"country_code": "US"}),
	|value| {
		assert_str(value, "country_code", "US");
	}
);
study_registration_single_field_test!(
	save_c_5_r_sequence_number_only,
	"C.5.r.sequence_number",
	json!({"sequence_number": 2}),
	|value| {
		assert_i64(value, "sequence_number", 2);
	}
);
