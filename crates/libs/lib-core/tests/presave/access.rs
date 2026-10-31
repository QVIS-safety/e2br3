use super::*;

#[serial]
#[tokio::test]
async fn sponsor_company_sender_presave_limited_to_one_active_record() -> Result<()>
{
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let org_id = create_acl_test_org(&mm, "company-sender-limit").await?;
	let ctx = sponsor_ctx(org_id, ROLE_SPONSOR_ADMIN_COMPANY);

	let first_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("Company Sender First".into()),
	)
	.await?;

	expect_conflict_error(
		SenderPresaveBmc::create(
			&ctx,
			&mm,
			sender_presave_create("Company Sender Second".into()),
		)
		.await,
		"pharmaceutical company sponsor administrators can register only one active sender presave",
	);

	PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Sender, first_id)
		.await?;

	SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("Company Sender Replacement".into()),
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn sponsor_cro_sender_presave_allows_multiple_active_records() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let org_id = create_acl_test_org(&mm, "cro-sender-limit").await?;
	let ctx = sponsor_ctx(org_id, ROLE_SPONSOR_ADMIN_CRO);

	SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("CRO Sender First".into()),
	)
	.await?;
	SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create("CRO Sender Second".into()),
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn sponsor_company_can_set_product_presave_sender_in_own_org() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let org_id = create_acl_test_org(&mm, "company-product-sender").await?;
	let cro_ctx = sponsor_ctx(org_id, ROLE_SPONSOR_ADMIN_CRO);
	let company_ctx = sponsor_ctx(org_id, ROLE_SPONSOR_ADMIN_COMPANY);

	let first_sender_id = SenderPresaveBmc::create(
		&cro_ctx,
		&mm,
		sender_presave_create("Product Sender First".into()),
	)
	.await?;
	let second_sender_id = SenderPresaveBmc::create(
		&cro_ctx,
		&mm,
		sender_presave_create("Product Sender Second".into()),
	)
	.await?;

	let company_product_id = ProductPresaveBmc::create(
		&company_ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Fda,
			"Company Product Create".into(),
			first_sender_id,
		),
	)
	.await?;

	let _cro_product_id = ProductPresaveBmc::create(
		&cro_ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Fda,
			"CRO Product Create".into(),
			first_sender_id,
		),
	)
	.await?;

	ProductPresaveBmc::update(
		&company_ctx,
		&mm,
		company_product_id,
		ProductPresaveForUpdate {
			sender_presave_id: Some(second_sender_id),
			..Default::default()
		},
	)
	.await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_relationships_reject_cross_org_links() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let org_a_id = Uuid::new_v4();
	let org_b_id = Uuid::new_v4();
	let sender_a_id = Uuid::new_v4();
	let sender_b_id = Uuid::new_v4();
	let product_a_id = Uuid::new_v4();
	let product_b_id = Uuid::new_v4();
	let study_a_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

	for (org_id, label) in [(org_a_id, "A"), (org_b_id, "B")] {
		sqlx::query(
			"INSERT INTO organizations (
				id, name, org_type, address, city, state, postcode, country_code,
				contact_email, contact_phone, active, created_by, created_at, updated_at
			) VALUES (
				$1, $2, 'client', '1 Presave St', 'Seoul', '11', '00000',
				'KR', $3, '02-000-0000', true, $4, NOW(), NOW()
			)",
		)
		.bind(org_id)
		.bind(format!("Presave FK Org {label} {org_id}"))
		.bind(format!("presave-fk-{label}-{org_id}@example.com"))
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await?;
	}

	for (sender_id, org_id, _label) in
		[(sender_a_id, org_a_id, "A"), (sender_b_id, org_b_id, "B")]
	{
		sqlx::query(
			"INSERT INTO sender_presaves (
				id, organization_id, created_by, updated_by
			)
			VALUES ($1, $2, $3, $3)",
		)
		.bind(sender_id)
		.bind(org_id)
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await?;
	}

	sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(product_a_id)
	.bind(org_a_id)
	.bind(sender_a_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(product_b_id)
	.bind(org_b_id)
	.bind(sender_b_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO study_presaves (
			id, organization_id, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(study_a_id)
	.bind(org_a_id)
	.bind(product_a_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;

	let mut invalid_tx = mm.dbx().db().begin().await?;
	set_user_context(&mut invalid_tx, system_user_id()).await?;
	set_org_context(&mut invalid_tx, org_a_id, "system_admin").await?;
	let cross_org_product = sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(Uuid::new_v4())
	.bind(org_a_id)
	.bind(sender_b_id)
	.bind(demo_user_id())
	.execute(&mut *invalid_tx)
	.await;
	assert!(
		matches!(cross_org_product, Err(ref err) if is_inactive_presave_reference(err)),
		"cross-org product->sender link should fail the active-reference boundary: {cross_org_product:?}"
	);
	invalid_tx.rollback().await?;

	let mut invalid_tx = mm.dbx().db().begin().await?;
	set_user_context(&mut invalid_tx, system_user_id()).await?;
	set_org_context(&mut invalid_tx, org_a_id, "system_admin").await?;
	let cross_org_study = sqlx::query(
		"INSERT INTO study_presaves (
			id, organization_id, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(Uuid::new_v4())
	.bind(org_a_id)
	.bind(product_b_id)
	.bind(demo_user_id())
	.execute(&mut *invalid_tx)
	.await;
	assert!(
		matches!(cross_org_study, Err(ref err) if is_inactive_presave_reference(err)),
		"cross-org study->product link should fail the active-reference boundary: {cross_org_study:?}"
	);
	invalid_tx.rollback().await?;

	Ok(())
}
