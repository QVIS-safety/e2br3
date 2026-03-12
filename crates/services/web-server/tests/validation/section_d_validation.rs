use super::validation_common::{
	assert_has_code, create_message_header, create_message_header_with_receiver,
	create_parent_information, create_parent_past_drug_history,
	create_past_drug_history, create_patient, create_primary_source,
	create_safety_report, create_sender, db_exec_case_sql, setup_case,
	update_parent_past_drug_history, update_patient, validate_case,
};
use crate::common::Result;
use serde_json::json;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn d_section_patient_initials_rule_is_enforced() -> Result<()> {
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
	assert_has_code(&report, "ICH.D.1.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_fda_race_and_ethnicity_rules_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.D.11.REQUIRED");
	assert_has_code(&report, "FDA.D.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_fda_race_and_ethnicity_reject_invalid_codes() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_sender(&ctx.app, &ctx.cookie, ctx.case_id, "1", "Sender Org").await?;
	create_primary_source(&ctx.app, &ctx.cookie, ctx.case_id, 1, Some("1")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("1"), Some("1")).await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "fda").await?;
	assert_has_code(&report, "FDA.D.11.REQUIRED");
	assert_has_code(&report, "FDA.D.12.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_age_value_and_unit_pair_rules_are_enforced() -> Result<()> {
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
	assert_has_code(&report, "ICH.D.2.2b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_gestation_value_and_unit_pair_rules_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
		.await?;
	update_patient(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": { "gestation_period_unit": "wk", "gestation_period": null }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.2.2.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_mfds_past_drug_code_required_for_kr_receiver() -> Result<()> {
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
	let _ = create_past_drug_history(
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
	assert_has_code(&report, "MFDS.D.8.r.1.KR.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_mfds_parent_past_drug_version_required_for_fr_when_code_present(
) -> Result<()> {
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
	assert_has_code(&report, "MFDS.D.10.8.r.1.KR.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_medical_history_meddra_pairs_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO medical_history_episodes \
			 (id, patient_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{patient_id}', 1, '27.0', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.7.1.r.1b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_past_drug_meddra_pairs_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO past_drug_history \
			 (id, patient_id, sequence_number, drug_name, indication_meddra_version, indication_meddra_code, reaction_meddra_version, reaction_meddra_code, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{patient_id}', 1, 'Past Drug', NULL, '10054321', '27.0', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.8.r.6a.REQUIRED");
	assert_has_code(&report, "ICH.D.8.r.7b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_death_cause_meddra_pairs_are_enforced() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO patient_death_information \
			 (id, patient_id, date_of_death, autopsy_performed, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{patient_id}', '2024-01-01', true, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"WITH death_row AS ( \
			 SELECT id FROM patient_death_information WHERE patient_id = '{patient_id}' ORDER BY created_at DESC LIMIT 1 \
			 ) \
			 INSERT INTO reported_causes_of_death \
			 (id, death_info_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) \
			 SELECT gen_random_uuid(), id, 1, '27.0', NULL, now(), now(), '{}' FROM death_row",
			ctx.admin_id
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"WITH death_row AS ( \
			 SELECT id FROM patient_death_information WHERE patient_id = '{patient_id}' ORDER BY created_at DESC LIMIT 1 \
			 ) \
			 INSERT INTO autopsy_causes_of_death \
			 (id, death_info_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) \
			 SELECT gen_random_uuid(), id, 1, NULL, '10099992', now(), now(), '{}' FROM death_row",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.9.2.r.1b.REQUIRED");
	assert_has_code(&report, "ICH.D.9.4.r.1a.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_death_cause_comments_are_required() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO patient_death_information \
			 (id, patient_id, date_of_death, autopsy_performed, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{patient_id}', '2024-01-01', true, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"WITH death_row AS ( \
			 SELECT id FROM patient_death_information WHERE patient_id = '{patient_id}' ORDER BY created_at DESC LIMIT 1 \
			 ) \
			 INSERT INTO reported_causes_of_death \
			 (id, death_info_id, sequence_number, meddra_version, meddra_code, comments, created_at, updated_at, created_by) \
			 SELECT gen_random_uuid(), id, 1, '27.0', '10099991', NULL, now(), now(), '{}' FROM death_row",
			ctx.admin_id
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"WITH death_row AS ( \
			 SELECT id FROM patient_death_information WHERE patient_id = '{patient_id}' ORDER BY created_at DESC LIMIT 1 \
			 ) \
			 INSERT INTO autopsy_causes_of_death \
			 (id, death_info_id, sequence_number, meddra_version, meddra_code, comments, created_at, updated_at, created_by) \
			 SELECT gen_random_uuid(), id, 1, '27.0', '10099992', NULL, now(), now(), '{}' FROM death_row",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.9.2.r.2.REQUIRED");
	assert_has_code(&report, "ICH.D.9.4.r.2.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_autopsy_flag_is_required_when_date_of_death_present() -> Result<()>
{
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	create_message_header(&ctx.app, &ctx.cookie, ctx.case_id, Some("ZZFDA")).await?;
	let patient_id =
		create_patient(&ctx.app, &ctx.cookie, ctx.case_id, Some("AB"), Some("1"))
			.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO patient_death_information \
			 (id, patient_id, date_of_death, autopsy_performed, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{patient_id}', '2024-01-01', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.9.3.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_parent_rules_are_enforced() -> Result<()> {
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
			"UPDATE parent_information SET parent_age = 40, parent_age_unit = NULL, sex = NULL WHERE id = '{parent_id}'"
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO parent_medical_history \
			 (id, parent_id, sequence_number, meddra_version, meddra_code, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{parent_id}', 1, '27.0', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"INSERT INTO parent_past_drug_history \
			 (id, parent_id, sequence_number, drug_name, indication_meddra_version, indication_meddra_code, reaction_meddra_version, reaction_meddra_code, created_at, updated_at, created_by) \
			 VALUES (gen_random_uuid(), '{parent_id}', 1, 'Parent Past Drug', NULL, '10011111', '27.0', NULL, now(), now(), '{}')",
			ctx.admin_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.10.2.2b.REQUIRED");
	assert_has_code(&report, "ICH.D.10.6.REQUIRED");
	assert_has_code(&report, "ICH.D.10.7.1.r.1b.REQUIRED");
	assert_has_code(&report, "ICH.D.10.8.r.6a.REQUIRED");
	assert_has_code(&report, "ICH.D.10.8.r.7b.REQUIRED");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_past_drug_identifiers_are_mutually_exclusive() -> Result<()> {
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
		Some("1"),
	)
	.await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE past_drug_history SET phpid = 'PHPID-1' WHERE id = '{past_id}'"
		),
	)
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
		json!({"data": { "mpid": "MPID-2", "phpid": "PHPID-2" }}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.8.MPID_PHPID.EXCLUSIVE");
	assert_has_code(&report, "ICH.D.10.8.MPID_PHPID.EXCLUSIVE");
	Ok(())
}

#[serial]
#[tokio::test]
async fn d_section_past_drug_identifier_versions_are_required_when_codes_present(
) -> Result<()> {
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
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE past_drug_history \
			 SET phpid = 'PHPID-1', phpid_version = NULL \
			 WHERE id = '{past_id}'"
		),
	)
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
		json!({"data": {
			"mpid": "MPID-2",
			"mpid_version": "",
			"phpid": "PHPID-2",
			"phpid_version": ""
		}}),
	)
	.await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_has_code(&report, "ICH.D.8.r.2a.REQUIRED");
	assert_has_code(&report, "ICH.D.8.r.3a.REQUIRED");
	assert_has_code(&report, "ICH.D.10.8.r.2a.REQUIRED");
	assert_has_code(&report, "ICH.D.10.8.r.3a.REQUIRED");
	Ok(())
}
