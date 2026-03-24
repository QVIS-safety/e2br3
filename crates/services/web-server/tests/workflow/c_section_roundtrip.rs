use crate::persist_workflow::{
	create_case, db_count_by_case, db_fetch_json_by_case, fill_section_c,
	request_json, save_case, setup, validate_case_fda,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn c_section_save_and_validate_persists_expected_fields(
) -> crate::common::Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_c(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;

	let count =
		db_count_by_case(&ctx, "safety_report_identification", case_id).await?;
	assert!(count >= 1, "expected safety_reports row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/safety-report"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	assert!(body["data"]["id"].as_str().is_some());
	assert_eq!(body["data"]["first_sender_type"], "2");
	assert_eq!(body["data"]["additional_documents_available"], true);

	let db_report = db_fetch_json_by_case(
		&ctx,
		"SELECT json_build_object(
			'first_sender_type', first_sender_type,
			'additional_documents_available', additional_documents_available
		) FROM safety_report_identification WHERE case_id = $1",
		case_id,
	)
	.await?;
	assert_eq!(db_report["first_sender_type"], "2");
	assert_eq!(db_report["additional_documents_available"], true);
	Ok(())
}
