mod common;

use common::{
	audit_log_count, begin_test_ctx, commit_test_ctx, create_case_fixture,
	delete_case_fixture, demo_ctx, demo_org_id, demo_user_id, init_test_mm,
	reset_role, set_auditor_role, set_current_user, Result,
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

	delete_case_fixture(&mm, case_id).await.ok();
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

	delete_case_fixture(&mm, case_id).await.ok();
	commit_test_ctx(&mm).await?;
	Ok(())
}
