use crate::common::{
	audit_log_count, begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx,
	demo_org_id, demo_user_id, init_test_mm, reset_role, set_auditor_role,
	set_current_user, Result,
};
use lib_core::model::audit::AuditLogBmc;
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::patient::{PatientInformationBmc, PatientInformationForCreate};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_audit_trail_cases() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	assert_eq!(audit_log_count(&mm, "cases", case_id, "CREATE").await?, 1);

	let case_u = CaseForUpdate {
		safety_report_id: None,
		dg_prd_key: None,
		status: Some("validated".to_string()),
		validation_profile: None,
		appendices_json: None,
		review_receivers_json: None,
		workflow_routes_json: None,
		mfds_report_type: None,
		report_year: None,
		source_document_name: None,
		source_document_base64: None,
		source_document_media_type: None,
		submitted_by: None,
		submitted_at: None,
		raw_xml: None,
		dirty_c: None,
		dirty_d: None,
		dirty_e: None,
		dirty_f: None,
		dirty_g: None,
		dirty_h: None,
	};
	CaseBmc::update(&ctx, &mm, case_id, case_u).await?;

	assert_eq!(audit_log_count(&mm, "cases", case_id, "UPDATE").await?, 1);

	let patient_id = PatientInformationBmc::create(
		&ctx,
		&mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("AT".to_string()),
			sex: Some("1".to_string()),
			concomitant_therapy: None,
		},
	)
	.await?;
	assert_eq!(
		audit_log_count(&mm, "patient_information", patient_id, "CREATE").await?,
		1
	);

	CaseBmc::delete(&ctx, &mm, case_id).await?;
	assert_eq!(audit_log_count(&mm, "cases", case_id, "DELETE").await?, 1);

	set_auditor_role(&mm).await?;
	let logs = AuditLogBmc::list_by_record(&ctx, &mm, "cases", case_id).await?;
	reset_role(&mm).await?;
	assert!(logs.iter().any(|l| l.action == "CREATE"));
	assert!(logs.iter().any(|l| l.action == "UPDATE"));
	assert!(logs.iter().any(|l| l.action == "DELETE"));
	assert!(
		logs.iter()
			.any(|l| l.table_name == "patient_information"
				&& l.record_id == patient_id),
		"case audit query should include case-linked table audit rows"
	);

	// -- Verify user attribution: all audit logs should reference the correct user
	for log in &logs {
		assert_eq!(
			log.user_id,
			demo_user_id(),
			"Audit log for action '{}' should be attributed to the correct user",
			log.action
		);
	}

	// -- Verify CREATE log captures new_values
	let create_log = logs.iter().find(|l| l.action == "CREATE").unwrap();
	assert!(
		create_log.new_values.is_some(),
		"CREATE audit log should capture new_values"
	);
	assert!(
		create_log.old_values.is_none(),
		"CREATE audit log should not have old_values"
	);
	let create_values = create_log.new_values.as_ref().unwrap();
	assert_eq!(
		create_values.get("id").and_then(|v| v.as_str()),
		Some(case_id.to_string()).as_deref(),
		"CREATE audit log should contain correct record id"
	);

	// -- Verify UPDATE log captures both old and new values
	let update_log = logs.iter().find(|l| l.action == "UPDATE").unwrap();
	assert!(
		update_log.old_values.is_some(),
		"UPDATE audit log should capture old_values"
	);
	assert!(
		update_log.new_values.is_some(),
		"UPDATE audit log should capture new_values"
	);
	let old_values = update_log.old_values.as_ref().unwrap();
	let new_values = update_log.new_values.as_ref().unwrap();
	let changed_fields = update_log.changed_fields.as_ref().unwrap();
	assert_eq!(
		old_values.get("status").and_then(|v| v.as_str()),
		Some("draft"),
		"UPDATE audit log should capture old status"
	);
	assert_eq!(
		new_values.get("status").and_then(|v| v.as_str()),
		Some("validated"),
		"UPDATE audit log should capture new status"
	);
	assert_eq!(
		changed_fields
			.get("status")
			.and_then(|v| v.get("old"))
			.and_then(|v| v.as_str()),
		Some("draft"),
		"UPDATE changed_fields should capture old status"
	);
	assert_eq!(
		changed_fields
			.get("status")
			.and_then(|v| v.get("new"))
			.and_then(|v| v.as_str()),
		Some("validated"),
		"UPDATE changed_fields should capture new status"
	);

	// -- Verify DELETE log captures old_values
	let delete_log = logs.iter().find(|l| l.action == "DELETE").unwrap();
	assert!(
		delete_log.old_values.is_some(),
		"DELETE audit log should capture old_values"
	);
	assert!(
		delete_log.new_values.is_none(),
		"DELETE audit log should not have new_values"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_noop_update_does_not_create_audit_log() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	assert_eq!(audit_log_count(&mm, "cases", case_id, "CREATE").await?, 1);

	CaseBmc::update(
		&ctx,
		&mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: None,
			validation_profile: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: None,
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await?;

	set_auditor_role(&mm).await?;
	let logs = AuditLogBmc::list_by_record(&ctx, &mm, "cases", case_id).await?;
	reset_role(&mm).await?;
	assert!(
		!logs
			.iter()
			.any(|log| log.table_name == "cases" && log.action == "UPDATE"),
		"metadata-only no-op update must not be visible in audit trail"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_log_hash_chain_integrity() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	CaseBmc::update(
		&ctx,
		&mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: Some("validated".to_string()),
			validation_profile: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: None,
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await?;

	CaseBmc::delete(&ctx, &mm, case_id).await?;

	set_auditor_role(&mm).await?;
	let case_log_ids: Vec<(i64,)> = mm
		.dbx()
		.fetch_all(
			sqlx::query_as(
				"SELECT id FROM audit_logs WHERE table_name = 'cases' AND record_id = $1 ORDER BY id ASC",
			)
			.bind(case_id),
		)
		.await?;
	assert!(
		case_log_ids.len() >= 3,
		"expected at least CREATE/UPDATE/DELETE case logs"
	);
	let min_id = case_log_ids.first().map(|(id,)| *id).unwrap();
	let max_id = case_log_ids.last().map(|(id,)| *id).unwrap();
	let chain_rows: Vec<(i64, String, String)> = mm
		.dbx()
		.fetch_all(
			sqlx::query_as(
				"SELECT id, prev_hash, entry_hash FROM audit_logs WHERE id BETWEEN $1 AND $2 ORDER BY id ASC",
			)
			.bind(min_id)
			.bind(max_id),
		)
		.await?;
	reset_role(&mm).await?;

	assert!(chain_rows.len() >= 2, "expected contiguous chain rows");
	for (_, prev_hash, entry_hash) in &chain_rows {
		assert_eq!(prev_hash.len(), 64, "prev_hash must be 64 hex chars");
		assert_eq!(entry_hash.len(), 64, "entry_hash must be 64 hex chars");
		assert!(
			prev_hash.chars().all(|c| c.is_ascii_hexdigit()),
			"prev_hash must be hex-encoded"
		);
		assert!(
			entry_hash.chars().all(|c| c.is_ascii_hexdigit()),
			"entry_hash must be hex-encoded"
		);
	}

	for idx in 1..chain_rows.len() {
		let (prev_id, _, prev_entry_hash) = &chain_rows[idx - 1];
		let (curr_id, curr_prev_hash, _) = &chain_rows[idx];
		assert_eq!(
			curr_prev_hash, prev_entry_hash,
			"chain mismatch at id {} -> {}",
			prev_id, curr_id
		);
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_log_hash_chain_verification_report_is_clean() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	CaseBmc::update(
		&ctx,
		&mm,
		case_id,
		CaseForUpdate {
			safety_report_id: None,
			dg_prd_key: None,
			status: Some("validated".to_string()),
			validation_profile: None,
			appendices_json: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			report_year: None,
			source_document_name: None,
			source_document_base64: None,
			source_document_media_type: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: None,
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await?;

	set_auditor_role(&mm).await?;
	let case_log_ids: Vec<(i64,)> = mm
		.dbx()
		.fetch_all(
			sqlx::query_as(
				"SELECT id FROM audit_logs WHERE table_name = 'cases' AND record_id = $1 ORDER BY id ASC",
			)
			.bind(case_id),
		)
		.await?;
	let min_case_log_id = case_log_ids.first().map(|(id,)| *id).unwrap();
	reset_role(&mm).await?;

	set_auditor_role(&mm).await?;
	let report =
		AuditLogBmc::verify_hash_chain_since(&ctx, &mm, Some(min_case_log_id))
			.await?;
	reset_role(&mm).await?;

	assert!(
		report.total_rows > 0,
		"verification report should include at least one row"
	);
	assert_eq!(
		report.broken_rows, 0,
		"expected no chain break in fresh scoped window, got report={report:?}"
	);
	assert_eq!(
		report.verified_ok_rows + report.broken_rows,
		report.total_rows,
		"verified + broken should equal total"
	);
	assert!(report.first_broken_id.is_none());
	assert!(report.first_broken_reason.is_none());

	CaseBmc::delete(&ctx, &mm, case_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}
