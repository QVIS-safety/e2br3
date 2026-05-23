mod common;

use crate::common::{
	demo_ctx, demo_org_id, demo_user_id, init_test_mm, Result, DEMO_ROLE,
};
use lib_core::ctx::Ctx;
use lib_core::model::presave_template::{
	PresaveEntityType, PresaveTemplateAuditBmc, PresaveTemplateBmc,
	PresaveTemplateForCreate, PresaveTemplateForUpdate, PresaveUsagePhase,
};
use lib_core::model::store::{
	set_full_context_dbx, set_org_context, set_user_context,
};
use serde_json::json;
use serial_test::serial;
use sqlx::types::Uuid;

async fn seed_alt_org_user(
	mm: &lib_core::model::ModelManager,
) -> Result<(Uuid, Uuid)> {
	let org_id = Uuid::new_v4();
	let user_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;

	set_user_context(&mut tx, demo_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

	sqlx::query(
		"INSERT INTO organizations (
			id, name, org_type, address, city, state, postcode, country_code,
			contact_email, contact_phone, active, created_by, created_at, updated_at
		) VALUES (
			$1, 'Alt Presave Org', 'client', '1 Alt St', 'Seoul', '11', '00000',
			'KR', 'alt-presave@example.com', '02-000-0000', true, $2, NOW(), NOW()
		)",
	)
	.bind(org_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO users (
			id, organization_id, email, username, pwd, pwd_salt, token_salt,
			role, active, must_change_password, created_by, created_at, updated_at
		) VALUES (
			$1, $2, $3, $4,
			'#02#$argon2id$v=19$m=19456,t=2,p=1$B0RCYSuiRr6tIIJVTVqABA$lhortXyud6bAy7oSK7NOVqR72TCmhVOcP9nG6bB+qXw',
			$5, $6, 'user', true, false, $7, NOW(), NOW()
		)",
	)
	.bind(user_id)
	.bind(org_id)
	.bind(format!("alt-presave-{user_id}@example.com"))
	.bind(format!("alt_presave_{}", &user_id.to_string()[..8]))
	.bind(Uuid::new_v4())
	.bind(Uuid::new_v4())
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;

	Ok((org_id, user_id))
}

async fn seed_user_in_org(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	role: &str,
) -> Result<Uuid> {
	let user_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;

	set_user_context(&mut tx, demo_user_id()).await?;
	set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

	sqlx::query(
		"INSERT INTO users (
			id, organization_id, email, username, pwd, pwd_salt, token_salt,
			role, active, must_change_password, created_by, created_at, updated_at
		) VALUES (
			$1, $2, $3, $4,
			'#02#$argon2id$v=19$m=19456,t=2,p=1$B0RCYSuiRr6tIIJVTVqABA$lhortXyud6bAy7oSK7NOVqR72TCmhVOcP9nG6bB+qXw',
			$5, $6, $7, true, false, $8, NOW(), NOW()
		)",
	)
	.bind(user_id)
	.bind(org_id)
	.bind(format!("presave-{role}-{user_id}@example.com"))
	.bind(format!("presave_{}_{}", role, &user_id.to_string()[..8]))
	.bind(Uuid::new_v4())
	.bind(Uuid::new_v4())
	.bind(role)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;
	Ok(user_id)
}

fn presave_payload(entity_type: PresaveEntityType) -> serde_json::Value {
	match entity_type {
		PresaveEntityType::Sender => {
			json!({"sender_type": "1", "organization_name": "Sender Org"})
		}
		PresaveEntityType::Receiver => json!({
			"receiver_type": "2",
			"organization_name": "Receiver Org",
			"receiver_id": "ZZFDA",
			"batch_receiver_id": "ZZFDA",
			"routing_rules": [
				{
					"authority": "fda",
					"report_type": "1",
					"batch_receiver_identifier": "ZZFDA",
					"message_receiver_identifier": "CDER"
				}
			]
		}),
		PresaveEntityType::Product => {
			json!({"drug_characterization": "1", "medicinal_product": "Product"})
		}
		PresaveEntityType::Reporter => {
			json!({"qualification": "1", "email": "reporter@example.com"})
		}
		PresaveEntityType::Study => {
			json!({"study_name": "Study", "sponsor_study_number": "STUDY-001"})
		}
		PresaveEntityType::Narrative => json!({"case_narrative": "Narrative"}),
	}
}

#[serial]
#[tokio::test]
async fn presave_entity_types_roundtrip_and_phase_classification() -> Result<()> {
	let sender: PresaveEntityType = serde_json::from_value(json!("sender"))?;
	let receiver: PresaveEntityType = serde_json::from_value(json!("receiver"))?;

	assert_eq!(sender, PresaveEntityType::Sender);
	assert_eq!(receiver, PresaveEntityType::Receiver);
	assert_eq!(sender.usage_phase(), PresaveUsagePhase::CaseAuthoring);
	assert_eq!(receiver.usage_phase(), PresaveUsagePhase::SubmissionRouting);

	let invalid = serde_json::from_value::<PresaveEntityType>(json!("bogus"));
	assert!(invalid.is_err(), "invalid entity type must be rejected");

	Ok(())
}

#[serial]
#[tokio::test]
async fn presave_crud_and_audit_cover_all_entity_types() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let entity_types = [
		PresaveEntityType::Sender,
		PresaveEntityType::Receiver,
		PresaveEntityType::Product,
		PresaveEntityType::Reporter,
		PresaveEntityType::Study,
		PresaveEntityType::Narrative,
	];

	for entity_type in entity_types {
		let name = format!("presave-{}-{}", entity_type.as_str(), Uuid::new_v4());
		let payload = presave_payload(entity_type);

		let template_id = PresaveTemplateBmc::create(
			&ctx,
			&mm,
			PresaveTemplateForCreate {
				entity_type,
				authority: None,
				name: name.clone(),
				description: Some(format!(
					"description for {}",
					entity_type.as_str()
				)),
				data: payload.clone(),
			},
		)
		.await?;

		set_full_context_dbx(mm.dbx(), demo_user_id(), demo_org_id(), DEMO_ROLE)
			.await?;

		let saved = PresaveTemplateBmc::get(&ctx, &mm, template_id).await?;
		assert_eq!(saved.entity_type, entity_type);
		assert_eq!(saved.name, name);
		assert_eq!(saved.data, payload);

		let filtered =
			PresaveTemplateBmc::list_by_entity_type(&ctx, &mm, entity_type).await?;
		assert!(
			filtered.iter().any(|row| row.id == template_id),
			"template {template_id} missing from filtered list"
		);

		PresaveTemplateBmc::update(
			&ctx,
			&mm,
			template_id,
			PresaveTemplateForUpdate {
				entity_type: None,
				authority: None,
				name: Some(format!("{name}-updated")),
				description: Some("updated description".to_string()),
				data: Some(payload.clone()),
			},
		)
		.await?;

		set_full_context_dbx(mm.dbx(), demo_user_id(), demo_org_id(), DEMO_ROLE)
			.await?;
		let updated = PresaveTemplateBmc::get(&ctx, &mm, template_id).await?;
		assert_eq!(updated.name, format!("{name}-updated"));
		assert_eq!(updated.description.as_deref(), Some("updated description"));

		let audits =
			PresaveTemplateAuditBmc::list_by_template(&ctx, &mm, template_id)
				.await?;
		assert!(
			audits.iter().any(|row| row.action == "CREATE"),
			"missing CREATE audit for {template_id}"
		);
		assert!(
			audits.iter().any(|row| row.action == "UPDATE"),
			"missing UPDATE audit for {template_id}"
		);

		if entity_type == PresaveEntityType::Receiver {
			let rules = updated.data["routing_rules"]
				.as_array()
				.ok_or("receiver routing_rules must be an array")?;
			assert_eq!(rules.len(), 1);
			assert_eq!(
				rules[0]["batch_receiver_identifier"].as_str(),
				Some("ZZFDA")
			);
			assert_eq!(
				rules[0]["message_receiver_identifier"].as_str(),
				Some("CDER")
			);
		}

		PresaveTemplateBmc::delete(&ctx, &mm, template_id).await?;
		let audits =
			PresaveTemplateAuditBmc::list_by_template(&ctx, &mm, template_id)
				.await?;
		assert!(
			audits.iter().any(|row| row.action == "DELETE"),
			"missing DELETE audit for {template_id}"
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn presave_templates_are_scoped_by_organization() -> Result<()> {
	let mm = init_test_mm().await;
	let (alt_org_id, alt_user_id) = seed_alt_org_user(&mm).await?;
	let demo_scoped_user_id = seed_user_in_org(&mm, demo_org_id(), "user").await?;
	let alt_ctx = Ctx::new(alt_user_id, alt_org_id, "user".to_string())?;
	let demo = Ctx::new(demo_scoped_user_id, demo_org_id(), "user".to_string())?;

	let demo_template_id = PresaveTemplateBmc::create(
		&demo,
		&mm,
		PresaveTemplateForCreate {
			entity_type: PresaveEntityType::Sender,
			authority: None,
			name: format!("demo-sender-{}", Uuid::new_v4()),
			description: None,
			data: presave_payload(PresaveEntityType::Sender),
		},
	)
	.await?;

	let alt_template_id = PresaveTemplateBmc::create(
		&alt_ctx,
		&mm,
		PresaveTemplateForCreate {
			entity_type: PresaveEntityType::Receiver,
			authority: None,
			name: format!("alt-receiver-{}", Uuid::new_v4()),
			description: None,
			data: presave_payload(PresaveEntityType::Receiver),
		},
	)
	.await?;

	let demo_saved = PresaveTemplateBmc::get(&demo, &mm, demo_template_id).await?;
	let alt_saved = PresaveTemplateBmc::get(&alt_ctx, &mm, alt_template_id).await?;
	assert_eq!(demo_saved.organization_id, demo_org_id());
	assert_eq!(alt_saved.organization_id, alt_org_id);

	set_full_context_dbx(mm.dbx(), demo_user_id(), demo_org_id(), "user").await?;
	let demo_visible = PresaveTemplateBmc::list(&demo, &mm).await?;
	assert!(demo_visible.iter().any(|row| row.id == demo_template_id));
	assert!(!demo_visible.iter().any(|row| row.id == alt_template_id));

	set_full_context_dbx(mm.dbx(), alt_user_id, alt_org_id, "user").await?;
	let alt_visible = PresaveTemplateBmc::list(&alt_ctx, &mm).await?;
	assert!(alt_visible.iter().any(|row| row.id == alt_template_id));
	assert!(!alt_visible.iter().any(|row| row.id == demo_template_id));

	let cross_demo = PresaveTemplateBmc::get(&demo, &mm, alt_template_id).await;
	assert!(cross_demo.is_err(), "demo org must not read alt template");
	let cross_alt = PresaveTemplateBmc::get(&alt_ctx, &mm, demo_template_id).await;
	assert!(cross_alt.is_err(), "alt org must not read demo template");

	Ok(())
}
