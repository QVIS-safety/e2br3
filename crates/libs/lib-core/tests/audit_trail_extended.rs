mod common;

use common::{
	audit_log_count, begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx,
	demo_org_id, demo_user_id, init_test_mm, reset_role, set_auditor_role,
	set_current_user, unique_suffix, Result,
};
use lib_core::model::audit::{AuditLogBmc, AuditLogFilter};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::organization::{
	OrganizationBmc, OrganizationForCreate, OrganizationForUpdate,
};
use lib_core::model::patient::{
	PatientInformationBmc, PatientInformationForCreate, PatientInformationForUpdate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use lib_core::model::user::{UserBmc, UserForCreate, UserForUpdate};
use modql::filter::ListOptions;
use serial_test::serial;

// ============================================================================
// SECTION 1: Drug Information Audit Trail
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_trail_drug_information() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// CREATE
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Audit Test Drug".to_string(),
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;
	assert_eq!(
		audit_log_count(&mm, "drug_information", drug_id, "CREATE").await?,
		1
	);

	// UPDATE
	let drug_u = DrugInformationForUpdate {
		medicinal_product: Some("Updated Audit Drug".to_string()),
		drug_characterization: None,
		brand_name: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		dosage_text: None,
		action_taken: None,
		rechallenge: None,
		investigational_product_blinded: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		obtain_drug_country: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
	};
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;
	assert_eq!(
		audit_log_count(&mm, "drug_information", drug_id, "UPDATE").await?,
		1
	);

	// DELETE
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	assert_eq!(
		audit_log_count(&mm, "drug_information", drug_id, "DELETE").await?,
		1
	);

	// Verify all audit logs
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "drug_information", drug_id).await?;
	reset_role(&mm).await?;
	assert_eq!(logs.len(), 3, "should have CREATE, UPDATE, DELETE logs");

	// Verify user attribution
	for log in &logs {
		assert_eq!(log.user_id, demo_user_id());
	}

	// Verify UPDATE captures old and new values
	let update_log = logs.iter().find(|l| l.action == "UPDATE").unwrap();
	assert!(update_log.old_values.is_some());
	assert!(update_log.new_values.is_some());
	let old = update_log.old_values.as_ref().unwrap();
	let new = update_log.new_values.as_ref().unwrap();
	assert_eq!(
		old.get("medicinal_product").and_then(|v| v.as_str()),
		Some("Audit Test Drug")
	);
	assert_eq!(
		new.get("medicinal_product").and_then(|v| v.as_str()),
		Some("Updated Audit Drug")
	);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 2: Reaction Audit Trail
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_trail_reactions() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// CREATE
	let reaction_c = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Audit Test Reaction".to_string(),
	};
	let reaction_id = ReactionBmc::create(&ctx, &mm, reaction_c).await?;
	assert_eq!(
		audit_log_count(&mm, "reactions", reaction_id, "CREATE").await?,
		1
	);

	// UPDATE
		let reaction_u = ReactionForUpdate {
			primary_source_reaction: Some("Updated Reaction".to_string()),
			primary_source_reaction_translation: None,
			reaction_meddra_code: None,
		reaction_meddra_version: None,
		reaction_language: None,
		term_highlighted: None,
		serious: Some(true),
		criteria_death: None,
		criteria_life_threatening: None,
		criteria_hospitalization: None,
		criteria_disabling: None,
		criteria_congenital_anomaly: None,
		criteria_other_medically_important: None,
		required_intervention: None,
		country_code: None,
		start_date: None,
		end_date: None,
		duration_value: None,
		duration_unit: None,
		medical_confirmation: None,
		outcome: None,
	};
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;
	assert_eq!(
		audit_log_count(&mm, "reactions", reaction_id, "UPDATE").await?,
		1
	);

	// DELETE
	ReactionBmc::delete(&ctx, &mm, reaction_id).await?;
	assert_eq!(
		audit_log_count(&mm, "reactions", reaction_id, "DELETE").await?,
		1
	);

	// Verify
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "reactions", reaction_id).await?;
	reset_role(&mm).await?;
	assert_eq!(logs.len(), 3);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 3: Patient Information Audit Trail
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_trail_patient_information() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// CREATE
	let patient_c = PatientInformationForCreate {
		case_id,
		patient_initials: Some("AT".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};
	let patient_id = PatientInformationBmc::create(&ctx, &mm, patient_c).await?;
	assert_eq!(
		audit_log_count(&mm, "patient_information", patient_id, "CREATE").await?,
		1
	);

	// UPDATE
	let patient_u = PatientInformationForUpdate {
		patient_initials: Some("UP".to_string()),
		patient_given_name: None,
		patient_family_name: None,
		birth_date: None,
		age_at_time_of_onset: None,
		age_unit: None,
		gestation_period: None,
		gestation_period_unit: None,
		age_group: None,
		race_code: None,
		ethnicity_code: None,
		weight_kg: None,
		height_cm: None,
		sex: None,
		last_menstrual_period_date: None,
		medical_history_text: Some("Test history".to_string()),
		concomitant_therapy: None,
	};
	PatientInformationBmc::update_by_case(&ctx, &mm, case_id, patient_u).await?;
	assert_eq!(
		audit_log_count(&mm, "patient_information", patient_id, "UPDATE").await?,
		1
	);

	// DELETE
	PatientInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(
		audit_log_count(&mm, "patient_information", patient_id, "DELETE").await?,
		1
	);

	// Verify
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "patient_information", patient_id)
			.await?;
	reset_role(&mm).await?;
	assert_eq!(logs.len(), 3);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 4: Organization Audit Trail
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_trail_organizations() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let suffix = unique_suffix();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	// CREAET
	let org_c = OrganizationForCreate {
		name: format!("Audit Test Org {suffix}"),
		org_type: Some("internal".to_string()),
		address: None,
		contact_email: Some(format!("audit-{suffix}@test.com")),
	};
	let org_id = OrganizationBmc::create(&ctx, &mm, org_c).await?;
	assert_eq!(
		audit_log_count(&mm, "organizations", org_id, "CREATE").await?,
		1
	);

	// UPDATE
	let org_u = OrganizationForUpdate {
		name: Some("Updated Audit Org".to_string()),
		org_type: None,
		address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		contact_email: None,
		contact_phone: None,
		active: None,
	};
	OrganizationBmc::update(&ctx, &mm, org_id, org_u).await?;
	assert_eq!(
		audit_log_count(&mm, "organizations", org_id, "UPDATE").await?,
		1
	);

	// DELETE
	OrganizationBmc::delete(&ctx, &mm, org_id).await?;
	assert_eq!(
		audit_log_count(&mm, "organizations", org_id, "DELETE").await?,
		1
	);

	// Verify
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "organizations", org_id).await?;
	reset_role(&mm).await?;
	assert_eq!(logs.len(), 3);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 5: User Audit Trail
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_trail_users() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let suffix = unique_suffix();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	// CREATE (note: UserBmc::create also calls update_pwd which creates an UPDATE log)
	let user_c = UserForCreate {
		organization_id: demo_org_id(),
		email: format!("audit_test_user-{suffix}@example.com"),
		username: format!("audit_test_user-{suffix}"),
		pwd_clear: "password123".to_string(),
		role: Some("user".to_string()),
		first_name: Some("Audit".to_string()),
		last_name: Some("User".to_string()),
	};
	let user_id = UserBmc::create(&ctx, &mm, user_c).await?;
	assert_eq!(audit_log_count(&mm, "users", user_id, "CREATE").await?, 1);
	// Password is set via update_pwd during create, generating 1 UPDATE log
	let update_count_after_create =
		audit_log_count(&mm, "users", user_id, "UPDATE").await?;
	assert!(
		update_count_after_create >= 1,
		"create should generate at least 1 UPDATE from password setup"
	);

	// UPDATE
	let user_u = UserForUpdate {
		email: None,
		role: Some("admin".to_string()),
		first_name: Some("Updated".to_string()),
		last_name: None,
		active: None,
		last_login_at: None,
	};
	UserBmc::update(&ctx, &mm, user_id, user_u).await?;
	let update_count_after_update =
		audit_log_count(&mm, "users", user_id, "UPDATE").await?;
	assert!(
		update_count_after_update > update_count_after_create,
		"explicit update should add another UPDATE log"
	);

	// DELETE
	UserBmc::delete(&ctx, &mm, user_id).await?;
	assert_eq!(audit_log_count(&mm, "users", user_id, "DELETE").await?, 1);

	// Verify total logs (CREATE + UPDATE(s) + DELETE)
	set_auditor_role(&mm).await?;
	let logs = AuditLogBmc::list_by_record(&ctx, &mm, "users", user_id).await?;
	reset_role(&mm).await?;
	assert!(
		logs.len() >= 3,
		"should have at least CREATE, UPDATE, DELETE logs"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 6: Audit Log Query Tests
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_log_list_all() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create multiple drugs to generate audit logs
	let mut drug_ids = Vec::new();
	for i in 1..=3 {
		let drug_c = DrugInformationForCreate {
			case_id,
			sequence_number: i,
			drug_characterization: "1".to_string(),
			medicinal_product: format!("Query Test Drug {i}"),
		};
		let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;
		drug_ids.push(drug_id);
	}

	// List all audit logs
	set_auditor_role(&mm).await?;
	let logs = AuditLogBmc::list(&ctx, &mm, None, None).await?;
	reset_role(&mm).await?;
	assert!(!logs.is_empty(), "should have audit logs");

	// Verify logs contain our drug records
	let drug_logs: Vec<_> = logs
		.iter()
		.filter(|l| {
			l.table_name == "drug_information" && drug_ids.contains(&l.record_id)
		})
		.collect();
	assert!(
		drug_logs.len() >= 3,
		"should have at least 3 CREATE logs for our drugs"
	);

	// Cleanup
	for drug_id in drug_ids {
		DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	}
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_log_chronological_order() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// CREATE
	let reaction_c = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Chrono Test".to_string(),
	};
	let reaction_id = ReactionBmc::create(&ctx, &mm, reaction_c).await?;

	// UPDATE
		let reaction_u = ReactionForUpdate {
			primary_source_reaction: Some("Chrono Updated".to_string()),
			primary_source_reaction_translation: None,
			reaction_meddra_code: None,
		reaction_meddra_version: None,
		reaction_language: None,
		term_highlighted: None,
		serious: None,
		criteria_death: None,
		criteria_life_threatening: None,
		criteria_hospitalization: None,
		criteria_disabling: None,
		criteria_congenital_anomaly: None,
		criteria_other_medically_important: None,
		required_intervention: None,
		country_code: None,
		start_date: None,
		end_date: None,
		duration_value: None,
		duration_unit: None,
		medical_confirmation: None,
		outcome: None,
	};
	ReactionBmc::update_in_case(&ctx, &mm, case_id, reaction_id, reaction_u).await?;

	// DELETE
	ReactionBmc::delete(&ctx, &mm, reaction_id).await?;

	// Get logs - should be in chronological order
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "reactions", reaction_id).await?;
	reset_role(&mm).await?;
	assert_eq!(logs.len(), 3);

	// Verify order: CREATE should have earliest timestamp
	let create_log = logs.iter().find(|l| l.action == "CREATE").unwrap();
	let update_log = logs.iter().find(|l| l.action == "UPDATE").unwrap();
	let delete_log = logs.iter().find(|l| l.action == "DELETE").unwrap();

	assert!(
		create_log.created_at <= update_log.created_at,
		"CREATE should be before UPDATE"
	);
	assert!(
		update_log.created_at <= delete_log.created_at,
		"UPDATE should be before DELETE"
	);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_log_list_honors_filters_and_list_options() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let suffix = unique_suffix();
	let org_id = OrganizationBmc::create(
		&ctx,
		&mm,
		OrganizationForCreate {
			name: format!("AuditFilterOrg-{suffix}"),
			org_type: Some("sponsor".to_string()),
			address: None,
			contact_email: None,
		},
	)
	.await?;

	// Ensure we have a known matching row.
	set_auditor_role(&mm).await?;
	let org_filter: AuditLogFilter = serde_json::from_value(
		serde_json::json!({ "table_name": { "$eq": "organizations" } }),
	)?;
	let org_logs =
		AuditLogBmc::list(&ctx, &mm, Some(vec![org_filter]), None).await?;
	assert!(
		!org_logs.is_empty(),
		"expected at least one organizations audit log"
	);

	// Verify filters are actually applied.
	let none_filter: AuditLogFilter = serde_json::from_value(
		serde_json::json!({ "table_name": { "$eq": "__no_such_table__" } }),
	)?;
	let no_logs =
		AuditLogBmc::list(&ctx, &mm, Some(vec![none_filter]), None).await?;
	assert!(
		no_logs.is_empty(),
		"expected no logs for impossible table_name"
	);

	// Verify list_options limit is honored.
	let limited = AuditLogBmc::list(
		&ctx,
		&mm,
		Some(vec![serde_json::from_value(
			serde_json::json!({ "table_name": { "$eq": "organizations" } }),
		)?]),
		Some(ListOptions {
			limit: Some(1),
			offset: None,
			order_bys: Some("!created_at".into()),
		}),
	)
	.await?;
	assert!(limited.len() <= 1, "expected at most one row with limit=1");

	reset_role(&mm).await?;
	OrganizationBmc::delete(&ctx, &mm, org_id).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 7: Audit Trail Data Integrity
// ============================================================================

#[serial]
#[tokio::test]
async fn test_audit_log_captures_all_changed_fields() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// CREATE drug with minimal fields
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Field Test Drug".to_string(),
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// UPDATE with multiple fields
	let drug_u = DrugInformationForUpdate {
		medicinal_product: Some("Updated Product".to_string()),
		drug_characterization: Some("2".to_string()),
		brand_name: Some("Test Brand".to_string()),
		manufacturer_name: Some("Test Manufacturer".to_string()),
		manufacturer_country: None,
		batch_lot_number: None,
		dosage_text: None,
		action_taken: Some("1".to_string()),
		rechallenge: None,
		investigational_product_blinded: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		obtain_drug_country: None,
		parent_route: None,
		parent_route_termid: None,
		parent_route_termid_version: None,
		parent_dosage_text: None,
		fda_additional_info_coded: None,
	};
	DrugInformationBmc::update_in_case(&ctx, &mm, case_id, drug_id, drug_u).await?;

	// Get UPDATE log
	set_auditor_role(&mm).await?;
	let logs =
		AuditLogBmc::list_by_record(&ctx, &mm, "drug_information", drug_id).await?;
	reset_role(&mm).await?;
	let update_log = logs.iter().find(|l| l.action == "UPDATE").unwrap();

	let new_values = update_log.new_values.as_ref().unwrap();
	let changed_fields = update_log.changed_fields.as_ref().unwrap();

	// Verify all updated fields are captured
	assert!(new_values.get("medicinal_product").is_some());
	assert!(new_values.get("drug_characterization").is_some());
	assert!(new_values.get("brand_name").is_some());
	assert!(new_values.get("manufacturer_name").is_some());
	assert!(new_values.get("action_taken").is_some());
	assert!(changed_fields.get("medicinal_product").is_some());
	assert!(changed_fields.get("drug_characterization").is_some());
	assert!(changed_fields.get("brand_name").is_some());
	assert!(changed_fields.get("manufacturer_name").is_some());
	assert!(changed_fields.get("action_taken").is_some());

	// Cleanup
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
