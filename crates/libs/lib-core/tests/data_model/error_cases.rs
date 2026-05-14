use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, rollback_test_ctx, set_current_user, unique_suffix,
	Result,
};
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::drug::{
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_core::model::organization::{OrganizationBmc, OrganizationForUpdate};
use lib_core::model::patient::{PatientInformationBmc, PatientInformationForCreate};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::user::{UserBmc, UserForCreate, UserForUpdate};
use lib_core::model::Error as ModelError;
use serial_test::serial;
use sqlx::types::Uuid;

// ============================================================================
// SECTION 1: NotFound Errors - GET operations
// ============================================================================

#[serial]
#[tokio::test]
async fn test_case_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = CaseBmc::get(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "cases");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result =
		UserBmc::get::<lib_core::model::user::User>(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "users");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_organization_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = OrganizationBmc::get(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "organizations");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_information_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = DrugInformationBmc::get(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "drug_information");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = ReactionBmc::get(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "reactions");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_patient_get_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = PatientInformationBmc::get(&ctx, &mm, fake_id).await;

	// NOTE: PatientInformationBmc::get currently returns EntityNotFound (legacy)
	// rather than EntityUuidNotFound. Test matches current behavior.
	match result {
		Err(ModelError::EntityUuidNotFound { entity, .. }) => {
			assert_eq!(entity, "patient_information");
		}
		Err(other) => {
			return Err(format!("expected EntityNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 2: NotFound Errors - UPDATE operations
// ============================================================================

#[serial]
#[tokio::test]
async fn test_case_update_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let fake_id = Uuid::new_v4();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let case_u = CaseForUpdate {
		safety_report_id: None,
		dg_prd_key: None,
		status: Some("validated".to_string()),
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

	let result = CaseBmc::update(&ctx, &mm, fake_id, case_u).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "cases");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_update_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let user_u = UserForUpdate {
		email: None,
		username: None,
		role: Some("admin".to_string()),
		permission_profile_id: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		access_blind_allowed: None,
		active: None,
		last_login_at: None,
	};

	let result = UserBmc::update(&ctx, &mm, fake_id, user_u).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "users");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_organization_update_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let org_u = OrganizationForUpdate {
		name: Some("Updated".to_string()),
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

	let result = OrganizationBmc::update(&ctx, &mm, fake_id, org_u).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "organizations");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 3: NotFound Errors - DELETE operations
// ============================================================================

#[serial]
#[tokio::test]
async fn test_case_delete_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let fake_id = Uuid::new_v4();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let result = CaseBmc::delete(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "cases");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_user_delete_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = UserBmc::delete(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "users");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_organization_delete_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = OrganizationBmc::delete(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "organizations");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_delete_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = DrugInformationBmc::delete(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "drug_information");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_delete_not_found() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_id = Uuid::new_v4();

	let result = ReactionBmc::delete(&ctx, &mm, fake_id).await;

	match result {
		Err(ModelError::EntityUuidNotFound { entity, id }) => {
			assert_eq!(entity, "reactions");
			assert_eq!(id, fake_id);
		}
		Err(other) => {
			return Err(format!("expected EntityUuidNotFound, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error, got success".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 4: Unique Constraint Violations
// ============================================================================

#[serial]
#[tokio::test]
async fn test_user_duplicate_username() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let suffix = unique_suffix();
	let fx_username = format!("test_dup_username_user-{suffix}");

	let user_c_1 = UserForCreate {
		organization_id: demo_org_id(),
		email: format!("{fx_username}@example.com"),
		username: Some(fx_username.to_string()),
		pwd_clear: "password123".to_string(),
		role: Some("user".to_string()),
		permission_profile_id: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		access_blind_allowed: None,
	};
	let user_c_2 = UserForCreate {
		organization_id: demo_org_id(),
		email: format!("different-{suffix}@example.com"),
		username: Some(fx_username.to_string()), // Same username
		pwd_clear: "password456".to_string(),
		role: Some("user".to_string()),
		permission_profile_id: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		access_blind_allowed: None,
	};

	begin_test_ctx(&mm, &ctx).await?;
	let user_id_1 = UserBmc::create(&ctx, &mm, user_c_1).await?;
	commit_test_ctx(&mm).await?;

	begin_test_ctx(&mm, &ctx).await?;
	let result = UserBmc::create(&ctx, &mm, user_c_2).await;

	match result {
		Err(ModelError::UniqueViolation { table, constraint }) => {
			assert_eq!(table, "users");
			assert!(constraint.contains("username"));
		}
		Err(other) => {
			rollback_test_ctx(&mm).await?;
			begin_test_ctx(&mm, &ctx).await?;
			UserBmc::delete(&ctx, &mm, user_id_1).await?;
			commit_test_ctx(&mm).await?;
			return Err(format!("expected UniqueViolation, got: {other:?}").into());
		}
		Ok(user_id_2) => {
			rollback_test_ctx(&mm).await?;
			begin_test_ctx(&mm, &ctx).await?;
			UserBmc::delete(&ctx, &mm, user_id_2).await?;
			UserBmc::delete(&ctx, &mm, user_id_1).await?;
			commit_test_ctx(&mm).await?;
			return Err("expected duplicate username error".into());
		}
	}

	rollback_test_ctx(&mm).await?;
	begin_test_ctx(&mm, &ctx).await?;
	UserBmc::delete(&ctx, &mm, user_id_1).await?;
	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 5: Foreign Key Constraint Violations
// ============================================================================

#[serial]
#[tokio::test]
async fn test_user_create_invalid_organization() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_org_id = Uuid::new_v4();
	let suffix = unique_suffix();

	let user_c = UserForCreate {
		organization_id: fake_org_id,
		email: format!("fk_test-{suffix}@example.com"),
		username: Some(format!("fk_test_user-{suffix}")),
		pwd_clear: "password123".to_string(),
		role: Some("user".to_string()),
		permission_profile_id: None,
		comments: None,
		other_information: None,
		access_start_at: None,
		access_end_at: None,
		access_sender_ids: None,
		access_product_ids: None,
		access_study_ids: None,
		access_blind_allowed: None,
	};

	let result = UserBmc::create(&ctx, &mm, user_c).await;

	// Should fail due to FK constraint on organization_id
	assert!(result.is_err(), "expected FK violation error");

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_create_invalid_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let fake_case_id = Uuid::new_v4();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let drug_c = DrugInformationForCreate {
		case_id: fake_case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
		drug_generic_name: None,
		..Default::default()
	};

	let result = DrugInformationBmc::create(&ctx, &mm, drug_c).await;

	// Should fail due to FK constraint on case_id
	assert!(
		result.is_err(),
		"expected FK violation error for invalid case_id"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_create_invalid_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let fake_case_id = Uuid::new_v4();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let reaction_c = ReactionForCreate {
		case_id: fake_case_id,
		sequence_number: 1,
		primary_source_reaction: "Test Reaction".to_string(),
	};

	let result = ReactionBmc::create(&ctx, &mm, reaction_c).await;

	// Should fail due to FK constraint on case_id
	assert!(
		result.is_err(),
		"expected FK violation error for invalid case_id"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_patient_create_invalid_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let fake_case_id = Uuid::new_v4();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	let patient_c = PatientInformationForCreate {
		case_id: fake_case_id,
		patient_initials: Some("XX".to_string()),
		sex: Some("1".to_string()),
		concomitant_therapy: None,
	};

	let result = PatientInformationBmc::create(&ctx, &mm, patient_c).await;

	// Should fail due to FK constraint on case_id
	assert!(
		result.is_err(),
		"expected FK violation error for invalid case_id"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 6: get_in_case / update_in_case with wrong case_id
// ============================================================================

#[serial]
#[tokio::test]
async fn test_drug_get_in_wrong_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	// Create two cases
	let case_id_1 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let case_id_2 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug in case 1
	let drug_c = DrugInformationForCreate {
		case_id: case_id_1,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
		drug_generic_name: None,
		..Default::default()
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Try to get drug using case 2 (wrong case)
	let result =
		DrugInformationBmc::get_in_case(&ctx, &mm, case_id_2, drug_id).await;

	assert!(result.is_err(), "should not find drug in wrong case");

	// Cleanup
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id_1).await?;
	CaseBmc::delete(&ctx, &mm, case_id_2).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_update_in_wrong_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	// Create two cases
	let case_id_1 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let case_id_2 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug in case 1
	let drug_c = DrugInformationForCreate {
		case_id: case_id_1,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
		drug_generic_name: None,
		..Default::default()
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Try to update drug using case 2 (wrong case)
	let drug_u = DrugInformationForUpdate {
		medicinal_product: Some("Hacked Drug".to_string()),
		drug_characterization: None,
		brand_name: None,
		drug_generic_name: None,
		drug_authorization_number: None,
		manufacturer_name: None,
		manufacturer_country: None,
		batch_lot_number: None,
		cumulative_dose_first_reaction_value: None,
		cumulative_dose_first_reaction_unit: None,
		gestation_period_exposure_value: None,
		gestation_period_exposure_unit: None,
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
		drug_additional_info_codes_json: None,
		drug_additional_information: None,
		fda_specialized_product_category: None,
		fda_device_info_json: None,
	};
	let result =
		DrugInformationBmc::update_in_case(&ctx, &mm, case_id_2, drug_id, drug_u)
			.await;

	assert!(result.is_err(), "should not update drug in wrong case");

	// Verify drug was not modified
	let drug = DrugInformationBmc::get(&ctx, &mm, drug_id).await?;
	assert_eq!(
		drug.medicinal_product, "Test Drug",
		"drug should not be modified"
	);

	// Cleanup
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id_1).await?;
	CaseBmc::delete(&ctx, &mm, case_id_2).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_get_in_wrong_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;

	// Create two cases
	let case_id_1 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;
	let case_id_2 = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create reaction in case 1
	let reaction_c = ReactionForCreate {
		case_id: case_id_1,
		sequence_number: 1,
		primary_source_reaction: "Test Reaction".to_string(),
	};
	let reaction_id = ReactionBmc::create(&ctx, &mm, reaction_c).await?;

	// Try to get reaction using case 2 (wrong case)
	let result = ReactionBmc::get_in_case(&ctx, &mm, case_id_2, reaction_id).await;

	assert!(result.is_err(), "should not find reaction in wrong case");

	// Cleanup
	ReactionBmc::delete(&ctx, &mm, reaction_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id_1).await?;
	CaseBmc::delete(&ctx, &mm, case_id_2).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
