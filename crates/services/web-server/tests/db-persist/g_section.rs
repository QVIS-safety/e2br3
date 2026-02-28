use crate::persist_workflow::{
	create_case, db_count_by_case, fill_section_g, request_json, save_case, setup,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn g_section_forms_persist_to_db_after_save() -> crate::common::Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_g(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;

	let count = db_count_by_case(&ctx, "drug_information", case_id).await?;
	assert!(count >= 1, "expected drugs row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	assert!(body["data"]
		.as_array()
		.map(|v| !v.is_empty())
		.unwrap_or(false));
	Ok(())
}
