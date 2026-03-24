use crate::persist_workflow::{
	create_case, db_count_by_case, disable_export_validation_for_test,
	export_case_xml, fill_section_d, force_case_validated_for_export, request_json,
	save_case, setup, validate_case_fda,
};
use serial_test::serial;

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
