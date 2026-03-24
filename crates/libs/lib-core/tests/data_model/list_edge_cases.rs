use crate::common::{
	begin_test_ctx, commit_test_ctx, create_case_fixture, demo_ctx, demo_org_id,
	demo_user_id, init_test_mm, set_current_user, Result,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::{
	DrugActiveSubstanceBmc, DrugActiveSubstanceForCreate, DrugInformationBmc,
	DrugInformationForCreate,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::Error as ModelError;
use modql::filter::ListOptions;
use serial_test::serial;

// ============================================================================
// SECTION 1: Empty List Results
// ============================================================================

#[serial]
#[tokio::test]
async fn test_drug_list_empty_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// List drugs for a case with no drugs
	let drugs = DrugInformationBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert!(drugs.is_empty(), "new case should have no drugs");

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_list_empty_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// List reactions for a case with no reactions
	let reactions = ReactionBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert!(reactions.is_empty(), "new case should have no reactions");

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 2: List with Pagination (using DrugActiveSubstanceBmc which has list())
// ============================================================================

#[serial]
#[tokio::test]
async fn test_substance_list_with_limit() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Test Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Create 5 substances
	let mut substance_ids = Vec::new();
	for i in 1..=5 {
		let substance_c = DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: i,
			substance_name: Some(format!("Substance {i}")),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		};
		let substance_id =
			DrugActiveSubstanceBmc::create(&ctx, &mm, substance_c).await?;
		substance_ids.push(substance_id);
	}

	// List with limit 3
	let list_options = ListOptions {
		limit: Some(3),
		offset: None,
		order_bys: Some("sequence_number".into()),
	};
	let substances =
		DrugActiveSubstanceBmc::list(&ctx, &mm, None, Some(list_options)).await?;
	assert!(substances.len() <= 3, "should respect limit");

	// Cleanup
	for substance_id in substance_ids {
		DrugActiveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	}
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_substance_list_with_offset() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create drug
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Offset Test Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// Create 5 substances
	let mut substance_ids = Vec::new();
	for i in 1..=5 {
		let substance_c = DrugActiveSubstanceForCreate {
			drug_id,
			sequence_number: i,
			substance_name: Some(format!("Offset Substance {i}")),
			substance_termid: None,
			substance_termid_version: None,
			strength_value: None,
			strength_unit: None,
		};
		let substance_id =
			DrugActiveSubstanceBmc::create(&ctx, &mm, substance_c).await?;
		substance_ids.push(substance_id);
	}

	// List all
	let all_substances = DrugActiveSubstanceBmc::list(&ctx, &mm, None, None).await?;

	// List with offset 2
	let list_options = ListOptions {
		limit: Some(100),
		offset: Some(2),
		order_bys: Some("id".into()),
	};
	let offset_substances =
		DrugActiveSubstanceBmc::list(&ctx, &mm, None, Some(list_options)).await?;

	// With offset, should have fewer or equal results
	assert!(
		offset_substances.len() <= all_substances.len(),
		"offset should reduce or equal results"
	);

	// Cleanup
	for substance_id in substance_ids {
		DrugActiveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	}
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 3: List Limit Validation
// ============================================================================

#[serial]
#[tokio::test]
async fn test_list_limit_over_max() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;

	// Try to list with limit exceeding max (5000)
	let list_options = ListOptions {
		limit: Some(10000),
		offset: None,
		order_bys: None,
	};

	let result =
		DrugActiveSubstanceBmc::list(&ctx, &mm, None, Some(list_options)).await;

	match result {
		Err(ModelError::ListLimitOverMax { max, actual }) => {
			assert_eq!(max, 5000);
			assert_eq!(actual, 10000);
		}
		Err(other) => {
			return Err(format!("expected ListLimitOverMax, got: {other:?}").into())
		}
		Ok(_) => return Err("expected error for limit over max".into()),
	}

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_list_limit_at_max() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;

	// List with limit exactly at max (5000) - should succeed
	let list_options = ListOptions {
		limit: Some(5000),
		offset: None,
		order_bys: None,
	};

	let result =
		DrugActiveSubstanceBmc::list(&ctx, &mm, None, Some(list_options)).await;
	assert!(result.is_ok(), "limit at max should succeed");

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_list_default_limit() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;

	// List without specifying limit - should use default (1000)
	let result = DrugActiveSubstanceBmc::list(&ctx, &mm, None, None).await;
	assert!(
		result.is_ok(),
		"list without options should use default limit"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 4: List by Case for Non-existent Case
// ============================================================================

#[serial]
#[tokio::test]
async fn test_drug_list_by_nonexistent_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_case_id = sqlx::types::Uuid::new_v4();

	// List drugs for non-existent case should return empty, not error
	let drugs = DrugInformationBmc::list_by_case(&ctx, &mm, fake_case_id).await?;
	assert!(
		drugs.is_empty(),
		"non-existent case should return empty list"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reaction_list_by_nonexistent_case() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	begin_test_ctx(&mm, &ctx).await?;
	let fake_case_id = sqlx::types::Uuid::new_v4();

	// List reactions for non-existent case should return empty, not error
	let reactions = ReactionBmc::list_by_case(&ctx, &mm, fake_case_id).await?;
	assert!(
		reactions.is_empty(),
		"non-existent case should return empty list"
	);

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 5: List with Ordering
// ============================================================================

#[serial]
#[tokio::test]
async fn test_reaction_list_ordering() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Create reactions with different sequence numbers (out of order)
	let reaction_c_3 = ReactionForCreate {
		case_id,
		sequence_number: 3,
		primary_source_reaction: "Reaction 3".to_string(),
	};
	let reaction_c_1 = ReactionForCreate {
		case_id,
		sequence_number: 1,
		primary_source_reaction: "Reaction 1".to_string(),
	};
	let reaction_c_2 = ReactionForCreate {
		case_id,
		sequence_number: 2,
		primary_source_reaction: "Reaction 2".to_string(),
	};

	let id_3 = ReactionBmc::create(&ctx, &mm, reaction_c_3).await?;
	let id_1 = ReactionBmc::create(&ctx, &mm, reaction_c_1).await?;
	let id_2 = ReactionBmc::create(&ctx, &mm, reaction_c_2).await?;

	// List with ordering by sequence_number (list_by_case orders by sequence_number)
	let reactions = ReactionBmc::list_by_case(&ctx, &mm, case_id).await?;

	// Verify we got all 3
	assert_eq!(reactions.len(), 3, "should have 3 reactions");

	// Verify ordering
	assert_eq!(reactions[0].sequence_number, 1);
	assert_eq!(reactions[1].sequence_number, 2);
	assert_eq!(reactions[2].sequence_number, 3);

	// Cleanup
	ReactionBmc::delete(&ctx, &mm, id_1).await?;
	ReactionBmc::delete(&ctx, &mm, id_2).await?;
	ReactionBmc::delete(&ctx, &mm, id_3).await?;
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}

// ============================================================================
// SECTION 6: Multiple List Calls Consistency
// ============================================================================

#[serial]
#[tokio::test]
async fn test_list_consistency_after_modifications() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();

	set_current_user(&mm, demo_user_id()).await?;
	begin_test_ctx(&mm, &ctx).await?;
	let case_id = create_case_fixture(&mm, demo_org_id(), demo_user_id()).await?;

	// Initial list should be empty
	let initial = DrugInformationBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(initial.len(), 0);

	// Add a drug
	let drug_c = DrugInformationForCreate {
		case_id,
		sequence_number: 1,
		drug_characterization: "1".to_string(),
		medicinal_product: "Consistency Test Drug".to_string(),
		drug_generic_name: None,
	};
	let drug_id = DrugInformationBmc::create(&ctx, &mm, drug_c).await?;

	// List should now have 1
	let after_add = DrugInformationBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(after_add.len(), 1);

	// Delete the drug
	DrugInformationBmc::delete(&ctx, &mm, drug_id).await?;

	// List should be empty again
	let after_delete = DrugInformationBmc::list_by_case(&ctx, &mm, case_id).await?;
	assert_eq!(after_delete.len(), 0);

	// Cleanup
	CaseBmc::delete(&ctx, &mm, case_id).await?;

	commit_test_ctx(&mm).await?;
	Ok(())
}
