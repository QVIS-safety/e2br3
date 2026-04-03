use super::validation_common::{
	assert_banner_issue, assert_lacks_code, assert_section_rule_coverage,
	create_message_header, create_other_case_identifier, create_primary_source,
	create_safety_report, create_safety_report_with, create_sender,
	create_study_information, db_exec_case_sql, post_json, put_json, setup_case,
	update_primary_source, update_safety_report, validate_case,
};
use crate::common::Result;
use axum::http::StatusCode;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.C.1.REQUIRED",
		"ICH.C.1.1.REQUIRED",
		"ICH.C.1.2.REQUIRED",
		"ICH.C.1.3.REQUIRED",
		"ICH.C.1.4.REQUIRED",
		"ICH.C.1.5.REQUIRED",
		"ICH.C.1.7.REQUIRED",
		"ICH.C.1.9.1.r.1.REQUIRED",
		"ICH.C.1.11.2.REQUIRED",
		"ICH.C.3.1.REQUIRED",
		"MFDS.C.3.1.KR.1.REQUIRED",
		"ICH.C.3.2.REQUIRED",
		"ICH.C.2.r.4.REQUIRED",
		"ICH.C.5.4.REQUIRED",
		"FDA.C.1.7.1.REQUIRED",
		"FDA.C.1.12.RECOMMENDED",
		"FDA.C.1.12.REQUIRED",
		"FDA.C.2.r.2.EMAIL.REQUIRED",
	]
}

#[test]
fn c_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('C', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	let blankish = ctx
		.case_id
		.as_bytes()
		.iter()
		.map(|byte| if byte % 2 == 0 { ' ' } else { '\t' })
		.collect::<String>();
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE cases SET safety_report_id = E'{}' WHERE id = '{}'",
			blankish, ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_2_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET transmission_date = NULL, transmission_date_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_2_allows_transmission_date_null_flavor() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": {
			"transmission_date_null_flavor": "UNK"
		}}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_lacks_code(&report, "ICH.C.1.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_3_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/safety-report", ctx.case_id),
		json!({"data": { "report_type": null }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_4_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET date_first_received_from_source = NULL, date_first_received_from_source_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_5_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET date_of_most_recent_information = NULL, date_of_most_recent_information_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.5.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_7_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/safety-report", ctx.case_id),
		json!({"data": { "fulfil_expedited_criteria": null }}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.7.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_9_1_r_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let other_id = create_other_case_identifier(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		"Regulatory authority",
		"CASE-123",
	)
	.await?;
	let (status, body) = put_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/other-identifiers/{other_id}", ctx.case_id),
		json!({"data": {
			"source_of_identifier": "",
			"case_identifier": "CASE-123"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.9.1.r.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_11_2_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({
			"data": {
				"nullification_code": "1",
				"nullification_reason": null
			},
			"reason_for_change": "validation test nullification transition",
			"e_signature": {
				"meaning": "nullify case for validation test",
				"password": "adminpwd"
			}
		}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.11.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_3_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.3.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_3_2_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.3.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_3_1_required_returns_banner_issue_when_sender_row_lacks_type(
) -> Result<()> {
	let ctx = setup_case().await?;
	post_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/safety-report/senders", ctx.case_id),
		json!({
			"data": {
				"case_id": ctx.case_id,
				"organization_name": "Sender Org"
			}
		}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.3.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_3_2_required_returns_banner_issue_when_sender_row_lacks_org(
) -> Result<()> {
	let ctx = setup_case().await?;
	post_json(
		&ctx.app,
		&ctx.cookie,
		format!("/api/cases/{}/safety-report/senders", ctx.case_id),
		json!({
			"data": {
				"case_id": ctx.case_id,
				"sender_type": "1"
			}
		}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.3.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_2_r_4_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	let ps_id =
		create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, None).await?;
	update_primary_source(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		ps_id,
		json!({"data": { "organization": "Reporter Org" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.2.r.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_5_4_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report_with(&ctx.app, &ctx.cookie, ctx.case_id, "2", false)
		.await?;
	create_study_information(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("Study"),
		Some("S-1"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.5.4.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_fda_c_1_7_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.C.1.7.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_fda_c_1_12_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.C.1.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_fda_c_1_12_recommended_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.C.1.12.RECOMMENDED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_fda_c_2_r_2_email_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, true).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.C.2.r.2.EMAIL.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_mfds_c_3_1_kr_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZMFDS"))
		.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "3", "Sender Org").await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.C.3.1.KR.1.REQUIRED");
	Ok(())
}
