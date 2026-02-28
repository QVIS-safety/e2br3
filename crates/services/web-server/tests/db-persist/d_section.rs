use crate::persist_workflow::{
	create_case, db_count_by_case, fill_section_d, request_json, save_case, setup,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn d_section_forms_persist_to_db_after_save() -> crate::common::Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_d(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;

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
	Ok(())
}
