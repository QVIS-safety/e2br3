use super::validation_common::{
	assert_banner_issue, assert_section_rule_coverage, create_message_header,
	create_message_header_with_receiver, create_parent_information,
	create_parent_past_drug_history, create_past_drug_history, create_patient,
	create_primary_source, create_safety_report, create_sender, db_exec_case_sql,
	setup_case, update_parent_past_drug_history, update_patient, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

pub(crate) fn tested_rule_codes() -> &'static [&'static str] {
	&[
		"ICH.D.1.REQUIRED",
		"ICH.D.2.2a.REQUIRED",
		"ICH.D.2.2b.REQUIRED",
		"ICH.D.2.2.1a.REQUIRED",
		"ICH.D.2.2.1b.REQUIRED",
		"ICH.D.7.1.r.1a.REQUIRED",
		"ICH.D.7.1.r.1b.REQUIRED",
		"ICH.D.8.r.2a.REQUIRED",
		"ICH.D.8.r.3a.REQUIRED",
		"ICH.D.8.r.6a.REQUIRED",
		"ICH.D.8.r.6b.REQUIRED",
		"ICH.D.8.r.7a.REQUIRED",
		"ICH.D.8.r.7b.REQUIRED",
		"ICH.D.8.MPID_PHPID.EXCLUSIVE",
		"ICH.D.10.2.2a.REQUIRED",
		"ICH.D.10.2.2b.REQUIRED",
		"ICH.D.10.6.REQUIRED",
		"ICH.D.10.8.r.2a.REQUIRED",
		"ICH.D.10.8.r.3a.REQUIRED",
		"ICH.D.10.8.MPID_PHPID.EXCLUSIVE",
		"FDA.D.11.REQUIRED",
		"FDA.D.12.REQUIRED",
		"MFDS.D.8.r.1.KR.1b.REQUIRED",
		"MFDS.D.8.r.1.KR.1a.REQUIRED",
		"MFDS.D.10.8.r.1.KR.1b.REQUIRED",
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
	]
}

#[test]
fn d_rule_coverage_matches_backend_banner_contract() {
	assert_section_rule_coverage('D', tested_rule_codes());
}

#[serial]
#[tokio::test]
async fn d_ich_d_1_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, None, Some("1")).await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "patient_given_name": "Jane" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_2_2a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE patient_information SET age_at_time_of_onset = NULL, age_unit = '801' WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.2.2a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_2_2b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "age_at_time_of_onset": "42", "age_unit": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.2.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_2_2_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "gestation_period": null, "gestation_period_unit": "wk" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.2.2.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_2_2_1b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "gestation_period": "10", "gestation_period_unit": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.2.2.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_7_1_r_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO medical_history_episodes (id, patient_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) VALUES (gen_random_uuid(), '{patient_id}', 1, NULL, '10012345', now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.7.1.r.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_7_1_r_1b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO medical_history_episodes (id, patient_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) VALUES (gen_random_uuid(), '{patient_id}', 1, '27.0', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.7.1.r.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_8_pairs_and_exclusive_return_banner_issues() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let past_id = create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		Some("MPID-1"),
		None,
	)
	.await?;
	let _ = past_id;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE past_drug_history SET phpid = 'PHPID-1', phpid_version = NULL, indication_meddra_version = NULL, indication_meddra_code = '10054321', reaction_meddra_version = '27.0', reaction_meddra_code = NULL WHERE patient_id = (SELECT id FROM patient_information WHERE case_id = '{}' ORDER BY created_at DESC LIMIT 1)",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.8.r.2a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.8.r.3a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.8.r.6a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.8.r.7b.REQUIRED");
	assert_banner_issue(&report, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_8_r_6b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		None,
		None,
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE past_drug_history SET indication_meddra_version = '27.0', indication_meddra_code = NULL WHERE patient_id = (SELECT id FROM patient_information WHERE case_id = '{}' ORDER BY created_at DESC LIMIT 1)",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.8.r.6b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_8_r_7a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		None,
		None,
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE past_drug_history SET reaction_meddra_version = NULL, reaction_meddra_code = '10054321' WHERE patient_id = (SELECT id FROM patient_information WHERE case_id = '{}' ORDER BY created_at DESC LIMIT 1)",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.8.r.7a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_parent_rules_return_banner_issues() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let parent_id =
		create_parent_information(&ctx.app, &ctx.cookie, ctx.case_id, Some("2"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE parent_information SET parent_age = NULL, parent_age_unit = '801', sex = NULL WHERE id = '{}'",
			parent_id
		),
	)
	.await?;
	let parent_past_id = create_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		1,
		Some("Parent Past Drug"),
	)
	.await?;
	update_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		parent_past_id,
		json!({"data": { "mpid": "WHOMPID-001", "mpid_version": null, "phpid": "WPHPID-001", "phpid_version": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.10.2.2a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.10.6.REQUIRED");
	assert_banner_issue(&report, "ICH.D.10.8.r.2a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.10.8.r.3a.REQUIRED");
	assert_banner_issue(&report, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_ich_d_10_2_2b_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let parent_id =
		create_parent_information(&ctx.app, &ctx.cookie, ctx.case_id, Some("2"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE parent_information SET parent_age = '31', parent_age_unit = NULL WHERE id = '{}'",
			parent_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.D.10.2.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_fda_d_11_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.D.11.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_fda_d_12_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_banner_issue(&report, "FDA.D.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_mfds_past_drug_rules_return_banner_issues() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"KR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		None,
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.D.8.r.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_mfds_d_8_r_1_kr_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"FR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	create_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		1,
		Some("Past Drug"),
		Some("WHOMPID-001"),
		None,
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.D.8.r.1.KR.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_mfds_parent_past_drug_rules_return_banner_issues() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"KR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let parent_id =
		create_parent_information(&ctx.app, &ctx.cookie, ctx.case_id, Some("2"))
			.await?;
	create_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		1,
		Some("Parent Past Drug"),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.D.10.8.r.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_mfds_d_10_8_r_1_kr_1a_required_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header_with_receiver(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		Some("ZZMFDS"),
		"FR",
	)
	.await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let parent_id =
		create_parent_information(&ctx.app, &ctx.cookie, ctx.case_id, Some("2"))
			.await?;
	let parent_past_id = create_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		1,
		Some("Parent Past Drug"),
	)
	.await?;
	update_parent_past_drug_history(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		parent_id,
		parent_past_id,
		json!({"data": { "mpid": "WHOMPID-001", "mpid_version": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "mfds").await?;
	assert_banner_issue(&report, "MFDS.D.10.8.r.1.KR.1a.REQUIRED");
	Ok(())
}
