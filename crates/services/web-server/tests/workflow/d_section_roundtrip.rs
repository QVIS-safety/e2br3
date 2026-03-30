use crate::persist_workflow::{
	create_case, db_count_by_case, disable_export_validation_for_test,
	export_case_xml, fill_section_d, force_case_validated_for_export, request_json,
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
async fn d_section_save_validate_export_roundtrip() -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_d(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;

	let count = db_count_by_case(&ctx, "patient_information", case_id).await?;
	assert!(count >= 1, "expected patients row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/patient"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	assert!(body["data"]["id"].as_str().is_some());

	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("PD"));
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_export_preserves_patient_and_past_drug_null_flavors(
) -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_d(&ctx, case_id).await?;

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient"),
		Some(json!({"data": {
			"birth_date": null,
			"birth_date_null_flavor": "ASKU"
		}})),
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK, "{body}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/patient/past-drugs"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK, "{body}");
	let past_drug_id = first_id(&body)?;

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"PUT",
		format!("/api/cases/{case_id}/patient/past-drugs/{past_drug_id}"),
		Some(json!({"data": {
			"start_date": null,
			"start_date_null_flavor": "UNK"
		}})),
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK, "{body}");

	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;
	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("birthTime nullFlavor=\"ASKU\""), "{xml}");
	assert!(xml.contains("Past Drug Persist"), "{xml}");
	assert!(xml.contains("low nullFlavor=\"UNK\""), "{xml}");
	Ok(())
}
