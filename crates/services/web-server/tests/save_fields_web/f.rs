use super::save_fields_common::{
	assert_bool, assert_date_tuple, assert_null, assert_str, extract_id, get_ok,
	post_created, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn test_result_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/test-results/{test_result_id}",
	}
}

async fn create_test_result(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	create_test_result_with_payload(
		ctx,
		case_id,
		json!({
			"sequence_number": 1,
			"test_name": "ALT"
		}),
	)
	.await
}

async fn create_test_result_with_payload(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	mut payload: serde_json::Value,
) -> Result<Uuid> {
	payload["case_id"] = json!(case_id);
	let value = post_created(
		ctx,
		test_result_field("F.r"),
		format!("/api/cases/{case_id}/test-results"),
		json!({"data": payload}),
	)
	.await?;
	extract_id(&value)
}

#[tokio::test]
#[serial]
async fn create_f_r_full_payload_persists_exposed_fields() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	let value = post_created(
		&ctx,
		test_result_field("F.r"),
		format!("/api/cases/{case_id}/test-results"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"test_date": [2024, 1, 1],
			"test_date_null_flavor": "UNK",
			"test_name": "ALT",
			"test_meddra_version": "27.0",
			"test_meddra_code": "10001552",
			"test_result_code": "N",
			"test_result_value": "25",
			"test_result_unit": "U/L",
			"result_unstructured": "Normal",
			"normal_low_value": "0",
			"normal_high_value": "40",
			"comments": "All normal",
			"more_info_available": true
		}}),
	)
	.await?;
	let test_result_id = extract_id(&value)?;

	let value = get_ok(
		&ctx,
		test_result_field("F.r"),
		format!("/api/cases/{case_id}/test-results/{test_result_id}"),
	)
	.await?;
	assert_date_tuple(&value, "test_date", &[2024, 1]);
	assert_null(&value, "test_date_null_flavor");
	assert_str(&value, "test_name", "ALT");
	assert_str(&value, "test_meddra_version", "27.0");
	assert_str(&value, "test_meddra_code", "10001552");
	assert_str(&value, "test_result_code", "N");
	assert_str(&value, "test_result_value", "25");
	assert_str(&value, "test_result_unit", "U/L");
	assert_str(&value, "result_unstructured", "Normal");
	assert_str(&value, "normal_low_value", "0");
	assert_str(&value, "normal_high_value", "40");
	assert_str(&value, "comments", "All normal");
	assert_bool(&value, "more_info_available", true);
	Ok(())
}

macro_rules! test_result_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let mut payload = $payload;
			if payload.get("sequence_number").is_none() {
				payload["sequence_number"] = json!(1);
			}
			if payload.get("test_name").is_none() {
				payload["test_name"] = json!("ALT");
			}

			let test_result_id =
				create_test_result_with_payload(&ctx, case_id, payload).await?;

			let value = get_ok(
				&ctx,
				test_result_field($canonical),
				format!("/api/cases/{case_id}/test-results/{test_result_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

test_result_single_field_test!(
	save_f_r_test_name_only,
	"F.r.2.1",
	json!({"test_name": "AST"}),
	|value| {
		assert_str(value, "test_name", "AST");
	}
);
test_result_single_field_test!(
	save_f_r_test_date_only,
	"F.r.1",
	json!({"test_date": [2024, 1, 1]}),
	|value| {
		assert_date_tuple(value, "test_date", &[2024, 1]);
	}
);
test_result_single_field_test!(
	save_f_r_test_date_null_flavor_only,
	"F.r.test_date_null_flavor",
	json!({"test_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "test_date_null_flavor", "UNK");
	}
);
test_result_single_field_test!(
	save_f_r_test_meddra_version_only,
	"F.r.2.2a",
	json!({"test_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "test_meddra_version", "27.0");
	}
);
test_result_single_field_test!(
	save_f_r_test_meddra_code_only,
	"F.r.2.2b",
	json!({"test_meddra_code": "1000"}),
	|value| {
		assert_str(value, "test_meddra_code", "1000");
	}
);
test_result_single_field_test!(
	save_f_r_test_result_code_only,
	"F.r.3.1",
	json!({"test_result_code": "N"}),
	|value| {
		assert_str(value, "test_result_code", "N");
	}
);
test_result_single_field_test!(
	save_f_r_test_result_value_only,
	"F.r.3.2",
	json!({"test_result_value": "11"}),
	|value| {
		assert_str(value, "test_result_value", "11");
	}
);
test_result_single_field_test!(
	save_f_r_test_result_unit_only,
	"F.r.3.3",
	json!({"test_result_unit": "mg/dL"}),
	|value| {
		assert_str(value, "test_result_unit", "mg/dL");
	}
);
test_result_single_field_test!(
	save_f_r_result_unstructured_only,
	"F.r.3.4",
	json!({"result_unstructured": "Normal"}),
	|value| {
		assert_str(value, "result_unstructured", "Normal");
	}
);
test_result_single_field_test!(
	save_f_r_normal_low_value_only,
	"F.r.4",
	json!({"normal_low_value": "1"}),
	|value| {
		assert_str(value, "normal_low_value", "1");
	}
);
test_result_single_field_test!(
	save_f_r_normal_high_value_only,
	"F.r.5",
	json!({"normal_high_value": "20"}),
	|value| {
		assert_str(value, "normal_high_value", "20");
	}
);
test_result_single_field_test!(
	save_f_r_comments_only,
	"F.r.6",
	json!({"comments": "Comment"}),
	|value| {
		assert_str(value, "comments", "Comment");
	}
);
test_result_single_field_test!(
	save_f_r_more_info_available_only,
	"F.r.7",
	json!({"more_info_available": true}),
	|value| {
		assert_bool(value, "more_info_available", true);
	}
);
