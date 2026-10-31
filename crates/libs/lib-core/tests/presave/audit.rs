use super::*;

#[serial]
#[tokio::test]
async fn section_presave_field_audit_records_changed_column() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Field Audit Sender Org {suffix}")),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Ich,
			format!("Field Audit Product {suffix}"),
			sender_id,
		),
	)
	.await?;

	ProductPresaveBmc::update(
		&ctx,
		&mm,
		product_id,
		ProductPresaveForUpdate {
			brand_name: Some(format!("Field Audit Brand {suffix}")),
			..Default::default()
		},
	)
	.await?;

	assert_audit_changed_field(
		&mm,
		"product_presaves",
		product_id,
		"brand_name",
		json!(null),
		json!(format!("Field Audit Brand {suffix}")),
	)
	.await?;

	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_child_audit_tracks_rows_separately() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create(format!("Multi Child Audit Sender {suffix}")),
	)
	.await?;

	let first_gateway_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 1,
			gateway_authority: "fda".into(),
			sender_identifier: Some("first-before".into()),
			routing_identifier: None,
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(false),
			deleted: None,
		},
	)
	.await?;
	let second_gateway_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 2,
			gateway_authority: "mfds".into(),
			sender_identifier: Some("second-before".into()),
			routing_identifier: None,
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(false),
			deleted: None,
		},
	)
	.await?;

	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		first_gateway_id,
		SenderPresaveGatewayForUpdate {
			sender_identifier: Some("first-after".into()),
			..Default::default()
		},
	)
	.await?;
	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		second_gateway_id,
		SenderPresaveGatewayForUpdate {
			sender_identifier: Some("second-after".into()),
			..Default::default()
		},
	)
	.await?;

	assert_audit_changed_field(
		&mm,
		"sender_presave_gateways",
		first_gateway_id,
		"sender_identifier",
		json!("first-before"),
		json!("first-after"),
	)
	.await?;
	assert_audit_changed_field(
		&mm,
		"sender_presave_gateways",
		second_gateway_id,
		"sender_identifier",
		json!("second-before"),
		json!("second-after"),
	)
	.await?;

	set_auditor_role(&mm).await?;
	let parent_update_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*)
		 FROM audit_logs
		 WHERE table_name = 'sender_presaves'
		   AND record_id = $1
		   AND action = 'UPDATE'",
	)
	.bind(sender_id)
	.fetch_one(mm.dbx().db())
	.await?;
	reset_role(&mm).await?;
	assert_eq!(
		parent_update_count, 0,
		"child row updates should not be collapsed into parent presave audit rows"
	);

	SenderPresaveGatewayBmc::delete(&ctx, &mm, second_gateway_id).await?;
	SenderPresaveGatewayBmc::delete(&ctx, &mm, first_gateway_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_child_audit_uses_parent_organization() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let parent_org_id = Uuid::new_v4();
	let sender_id = Uuid::new_v4();
	let gateway_id = Uuid::new_v4();
	let study_id = Uuid::new_v4();
	let study_product_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

	sqlx::query(
		"INSERT INTO organizations (
			id, name, org_type, address, city, state, postcode, country_code,
			contact_email, contact_phone, active, created_by, created_at, updated_at
		) VALUES (
			$1, $2, 'client', '1 Audit St', 'Seoul', '11', '00000',
			'KR', $3, '02-000-0000', true, $4, NOW(), NOW()
		)",
	)
	.bind(parent_org_id)
	.bind(format!("Audit Parent Org {parent_org_id}"))
	.bind(format!("audit-parent-{parent_org_id}@example.com"))
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO sender_presaves (
			id, organization_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $3)",
	)
	.bind(sender_id)
	.bind(parent_org_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO sender_presave_gateways (
			id, sender_presave_id, sequence_number, gateway_authority,
			sender_identifier, created_by, updated_by
		)
		VALUES ($1, $2, 1, 'fda', 'before-update', $3, $3)",
	)
	.bind(gateway_id)
	.bind(sender_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO study_presaves (
			id, organization_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $3)",
	)
	.bind(study_id)
	.bind(parent_org_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO study_presave_products (
			id, study_presave_id, sequence_number, product_name, created_by, updated_by
		)
		VALUES ($1, $2, 1, 'before-update', $3, $3)",
	)
	.bind(study_product_id)
	.bind(study_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"UPDATE study_presave_products
		 SET product_name = 'after-update',
		     updated_by = $1
		 WHERE id = $2",
	)
	.bind(demo_user_id())
	.bind(study_product_id)
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"UPDATE sender_presave_gateways
		 SET sender_identifier = 'after-update',
		     updated_by = $1
		 WHERE id = $2",
	)
	.bind(demo_user_id())
	.bind(gateway_id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	set_auditor_role(&mm).await?;
	let audited_org_id: Uuid = sqlx::query_scalar(
		"SELECT organization_id
		 FROM audit_logs
		 WHERE table_name = 'sender_presave_gateways'
		   AND record_id = $1
		   AND action = 'UPDATE'
		 ORDER BY created_at DESC, id DESC
		 LIMIT 1",
	)
	.bind(gateway_id)
	.fetch_one(mm.dbx().db())
	.await?;
	reset_role(&mm).await?;

	assert_eq!(
		audited_org_id, parent_org_id,
		"child presave audit log should inherit organization from parent"
	);

	set_auditor_role(&mm).await?;
	let audited_org_id: Uuid = sqlx::query_scalar(
		"SELECT organization_id
		 FROM audit_logs
		 WHERE table_name = 'study_presave_products'
		   AND record_id = $1
		   AND action = 'UPDATE'
		 ORDER BY created_at DESC, id DESC
		 LIMIT 1",
	)
	.bind(study_product_id)
	.fetch_one(mm.dbx().db())
	.await?;
	reset_role(&mm).await?;

	assert_eq!(
		audited_org_id, parent_org_id,
		"study product audit log should inherit organization from parent"
	);

	Ok(())
}
