use crate::persist_workflow::{
	create_case, db_count_by_case, disable_export_validation_for_test,
	export_case_xml, fill_section_e, force_case_validated_for_export, request_json,
	save_case, setup, validate_case_fda,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn e_section_save_validate_export_roundtrip() -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_e(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;

	let count = db_count_by_case(&ctx, "reactions", case_id).await?;
	assert!(count >= 1, "expected reactions row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/reactions"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	assert!(body["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));

	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("Persist Reaction Headache"));
	Ok(())
}
