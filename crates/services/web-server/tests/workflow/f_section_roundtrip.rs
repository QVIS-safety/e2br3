use crate::persist_workflow::{
	create_case, db_count_by_case, disable_export_validation_for_test,
	export_case_xml, fill_section_f, force_case_validated_for_export, request_json,
	save_case, setup, validate_case_fda,
};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

fn first_id(body: &Value) -> crate::common::Result<Uuid> {
	let id = body["data"]
		.as_array()
		.and_then(|rows| rows.first())
		.and_then(|row| row.get("id"))
		.and_then(Value::as_str)
		.ok_or("missing first row id")?;
	Ok(Uuid::parse_str(id)?)
}

#[serial]
#[tokio::test]
async fn f_section_save_validate_export_roundtrip() -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_f(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;

	let count = db_count_by_case(&ctx, "test_results", case_id).await?;
	assert!(count >= 1, "expected test_results row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/test-results"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	assert!(body["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("Persist Blood Test"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn f_section_export_preserves_test_date_null_flavor(
) -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_f(&ctx, case_id).await?;

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/test-results"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK, "{body}");
	let test_result_id = first_id(&body)?;

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/test-results/{test_result_id}"),
		Some(json!({"data": {
			"test_date": null,
			"test_date_null_flavor": "UNK"
		}})),
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK, "{body}");

	save_case(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;
	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("Persist Blood Test"), "{xml}");
	assert!(xml.contains("effectiveTime nullFlavor=\"UNK\""), "{xml}");
	Ok(())
}
