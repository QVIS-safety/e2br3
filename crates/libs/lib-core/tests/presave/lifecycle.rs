use super::*;

#[serial]
#[tokio::test]
async fn inactive_presave_reference_returns_p2001_for_receiver_link() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("Trigger sender".into()),
	)
	.await?;
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Archived receiver {}", Uuid::new_v4())),
			day_count_rule: None,
			nsae_solicited_day_count: None,
			nsae_solicited_not_applicable: None,
			nsae_non_solicited_day_count: None,
			nsae_non_solicited_not_applicable: None,
			sae_solicited_day_count: None,
			sae_solicited_not_applicable: None,
			sae_non_solicited_day_count: None,
			sae_non_solicited_not_applicable: None,
			description: None,
		},
	)
	.await?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, demo_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query("UPDATE receiver_presaves SET deleted = true WHERE id = $1")
		.bind(receiver_id)
		.execute(&mut *tx)
		.await?;
	let result = sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, sender_presave_id, receiver_presave_id,
			created_by, updated_by
		) VALUES ($1, $2, $3, $4, $5, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(demo_org_id())
	.bind(sender_id)
	.bind(receiver_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await;
	assert!(
		matches!(result, Err(ref error) if error.as_database_error().and_then(|db| db.code()).as_deref() == Some("P2001")),
		"expected P2001, got {result:?}"
	);
	tx.rollback().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn presave_lifecycle_archives_unreferenced_and_blocks_used_receiver(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create(format!("Lifecycle sender {}", Uuid::new_v4())),
	)
	.await?;
	let available_receiver = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		receiver_presave_create(format!(
			"Lifecycle available receiver {}",
			Uuid::new_v4()
		)),
	)
	.await?;
	PresaveLifecycleService::archive(
		&ctx,
		&mm,
		PresaveKind::Receiver,
		available_receiver,
	)
	.await?;
	assert!(
		ReceiverPresaveBmc::get(&ctx, &mm, available_receiver)
			.await?
			.deleted
	);

	let used_receiver = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		receiver_presave_create(format!(
			"Lifecycle used receiver {}",
			Uuid::new_v4()
		)),
	)
	.await?;
	let mut product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Lifecycle receiver product {}", Uuid::new_v4()),
		sender_id,
	);
	product.receiver_presave_id = Some(used_receiver);
	ProductPresaveBmc::create(&ctx, &mm, product).await?;
	expect_conflict_error(
		PresaveLifecycleService::archive(
			&ctx,
			&mm,
			PresaveKind::Receiver,
			used_receiver,
		)
		.await,
		"receiver presave is in use",
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn presave_lifecycle_reference_race_preserves_active_reference_invariant(
) -> Result<()> {
	use tokio::time::{sleep, Duration};

	_dev_utils::init_dev().await;
	let setup_mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&setup_mm,
		sender_presave_create(format!("Race sender {}", Uuid::new_v4())),
	)
	.await?;
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&setup_mm,
		receiver_presave_create(format!("Race receiver {}", Uuid::new_v4())),
	)
	.await?;

	let lock_mm = ModelManager::new().await?;
	let mut archive_tx = lock_mm.dbx().db().begin().await?;
	set_user_context(&mut archive_tx, demo_user_id()).await?;
	set_org_context(&mut archive_tx, demo_org_id(), ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query("SELECT id FROM receiver_presaves WHERE id = $1 FOR UPDATE")
		.bind(receiver_id)
		.execute(&mut *archive_tx)
		.await?;
	sqlx::query("UPDATE receiver_presaves SET deleted = true WHERE id = $1")
		.bind(receiver_id)
		.execute(&mut *archive_tx)
		.await?;

	let reference_task = tokio::spawn(async move {
		let mm = ModelManager::new().await.expect("race model manager");
		let mut tx = mm.dbx().db().begin().await.expect("race transaction");
		set_user_context(&mut tx, demo_user_id())
			.await
			.expect("race user context");
		set_org_context(&mut tx, demo_org_id(), ROLE_SPONSOR_ADMIN_CRO)
			.await
			.expect("race org context");
		let result = sqlx::query(
			"INSERT INTO product_presaves (
				id, organization_id, sender_presave_id, receiver_presave_id,
				created_by, updated_by
			) VALUES ($1, $2, $3, $4, $5, $5)",
		)
		.bind(Uuid::new_v4())
		.bind(demo_org_id())
		.bind(sender_id)
		.bind(receiver_id)
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await;
		tx.rollback().await.expect("race rollback");
		result
	});
	sleep(Duration::from_millis(50)).await;
	archive_tx.commit().await?;
	let result = reference_task.await?;
	assert!(
		matches!(result, Err(ref error) if error.as_database_error().and_then(|db| db.code()).as_deref() == Some("P2001")),
		"reference created after archive lock must fail with P2001: {result:?}"
	);

	let mut verify_tx = setup_mm.dbx().db().begin().await?;
	set_user_context(&mut verify_tx, demo_user_id()).await?;
	set_org_context(&mut verify_tx, demo_org_id(), ROLE_SPONSOR_ADMIN_CRO).await?;
	let deleted: bool =
		sqlx::query_scalar("SELECT deleted FROM receiver_presaves WHERE id = $1")
			.bind(receiver_id)
			.fetch_one(&mut *verify_tx)
			.await?;
	let reference_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS (SELECT 1 FROM product_presaves WHERE receiver_presave_id = $1)",
	)
	.bind(receiver_id)
	.fetch_one(&mut *verify_tx)
	.await?;
	assert!(deleted && !reference_exists);
	verify_tx.rollback().await?;

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&setup_mm,
		receiver_presave_create(format!(
			"Race referenced receiver {}",
			Uuid::new_v4()
		)),
	)
	.await?;
	let reference_mm = ModelManager::new().await?;
	let mut reference_tx = reference_mm.dbx().db().begin().await?;
	set_user_context(&mut reference_tx, demo_user_id()).await?;
	set_org_context(&mut reference_tx, demo_org_id(), ROLE_SPONSOR_ADMIN_CRO)
		.await?;
	sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, sender_presave_id, receiver_presave_id,
			created_by, updated_by
		) VALUES ($1, $2, $3, $4, $5, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(demo_org_id())
	.bind(sender_id)
	.bind(receiver_id)
	.bind(demo_user_id())
	.execute(&mut *reference_tx)
	.await?;

	let archive_task = tokio::spawn(async move {
		let mm = ModelManager::new().await.expect("archive race manager");
		PresaveLifecycleService::archive(
			&demo_ctx(),
			&mm,
			PresaveKind::Receiver,
			receiver_id,
		)
		.await
	});
	sleep(Duration::from_millis(50)).await;
	reference_tx.commit().await?;
	expect_conflict_error(archive_task.await?, "receiver presave is in use");
	let receiver = ReceiverPresaveBmc::get(&ctx, &setup_mm, receiver_id).await?;
	assert!(!receiver.deleted);
	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_reject_delete_when_referenced() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create(format!("Referenced Sender {suffix}")),
	)
	.await?;
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Fda,
			format!("Referenced Product {suffix}"),
			sender_id,
		),
	)
	.await?;
	let _study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		study_presave_create_for_product(
			format!("Referenced Study {suffix}"),
			product_id,
		),
	)
	.await?;

	expect_validation_error(
		SenderPresaveBmc::update(
			&ctx,
			&mm,
			sender_id,
			SenderPresaveForUpdate {
				deleted: Some(true),
				..Default::default()
			},
		)
		.await,
		"presave deletion must use lifecycle service",
	);
	expect_conflict_error(
		SenderPresaveBmc::delete(&ctx, &mm, sender_id).await,
		"sender presave is in use",
	);

	expect_validation_error(
		ProductPresaveBmc::update(
			&ctx,
			&mm,
			product_id,
			ProductPresaveForUpdate {
				deleted: Some(true),
				..Default::default()
			},
		)
		.await,
		"presave deletion must use lifecycle service",
	);
	expect_conflict_error(
		ProductPresaveBmc::delete(&ctx, &mm, product_id).await,
		"product presave is in use",
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_receiver_delete_uses_receiver_name_not_template_name(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create(format!("Receiver Delete Sender {suffix}")),
	)
	.await?;
	let template_name = format!("Receiver Delete Template {suffix}");
	let receiver_org = format!("Receiver Delete Org {suffix}");
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(receiver_org.clone()),
			day_count_rule: None,
			nsae_solicited_day_count: None,
			nsae_solicited_not_applicable: None,
			nsae_non_solicited_day_count: None,
			nsae_non_solicited_not_applicable: None,
			sae_solicited_day_count: None,
			sae_solicited_not_applicable: None,
			sae_non_solicited_day_count: None,
			sae_non_solicited_not_applicable: None,
			description: None,
		},
	)
	.await?;

	let mut product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Template Name Manufacturer Product {suffix}"),
		sender_id,
	);
	product.original_manufacturer = Some(template_name);
	let product_id = ProductPresaveBmc::create(&ctx, &mm, product).await?;

	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;

	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}
