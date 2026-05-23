mod common;

use crate::common::{
	demo_ctx, demo_org_id, demo_user_id, reset_role, set_auditor_role, Result,
};
use lib_core::_dev_utils;
use lib_core::model::presave::{
	NarrativePresaveBmc, NarrativePresaveForCreate, ProductPresaveBmc,
	ProductPresaveForCreate, ReceiverPresaveBmc, ReceiverPresaveForCreate,
	ReporterPresaveBmc, ReporterPresaveForCreate, SenderPresaveBmc,
	SenderPresaveForCreate, SenderPresaveForUpdate, StudyPresaveBmc,
	StudyPresaveForCreate,
};
use lib_core::model::store::{set_org_context, set_user_context};
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use serial_test::serial;
use std::collections::HashSet;

use sqlx::types::Uuid;
use sqlx::Error as SqlxError;

const SECTION_PRESAVE_TABLES: &[&str] = &[
	"sender_presaves",
	"sender_presave_gateways",
	"sender_presave_responsible_persons",
	"receiver_presaves",
	"receiver_presave_consignees",
	"product_presaves",
	"product_presave_substances",
	"product_presave_fda_cross_reported_inds",
	"product_presave_mfds_regional_items",
	"reporter_presaves",
	"study_presaves",
	"study_presave_registration_numbers",
	"narrative_presaves",
	"narrative_presave_sender_diagnoses",
	"narrative_presave_case_summaries",
];

fn is_foreign_key_violation(err: &SqlxError) -> bool {
	match err {
		SqlxError::Database(db_err) => db_err.code().as_deref() == Some("23503"),
		_ => false,
	}
}

#[serial]
#[tokio::test]
async fn section_presave_tables_exist() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;

	for table in SECTION_PRESAVE_TABLES {
		let exists: bool = sqlx::query_scalar(
			"SELECT EXISTS (
				SELECT 1 FROM information_schema.tables
				WHERE table_schema = 'public' AND table_name = $1
			)",
		)
		.bind(table)
		.fetch_one(mm.dbx().db())
		.await?;
		assert!(exists, "missing table {table}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_tables_have_rls_and_relationship_guards() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;

	let rls_rows: Vec<(String, bool, bool)> = sqlx::query_as(
		"SELECT relname::text, relrowsecurity, relforcerowsecurity
		 FROM pg_class
		 WHERE relnamespace = 'public'::regnamespace
		   AND relname = ANY($1)
		 ORDER BY relname",
	)
	.bind(SECTION_PRESAVE_TABLES)
	.fetch_all(mm.dbx().db())
	.await?;

	let rls_tables: HashSet<_> = rls_rows
		.iter()
		.map(|(table, _, _)| table.as_str())
		.collect();
	for table in SECTION_PRESAVE_TABLES {
		assert!(
			rls_tables.contains(table),
			"missing RLS catalog row for {table}"
		);
	}

	for (table, rls_enabled, rls_forced) in rls_rows {
		assert!(rls_enabled, "RLS must be enabled for {table}");
		assert!(rls_forced, "RLS must be forced for {table}");
	}

	let expected_policies = [
		("sender_presaves", "sender_presaves_org_isolation"),
		(
			"sender_presave_gateways",
			"sender_presave_gateways_via_parent",
		),
		(
			"sender_presave_responsible_persons",
			"sender_presave_responsible_persons_via_parent",
		),
		("receiver_presaves", "receiver_presaves_org_isolation"),
		(
			"receiver_presave_consignees",
			"receiver_presave_consignees_via_parent",
		),
		("product_presaves", "product_presaves_org_isolation"),
		(
			"product_presave_substances",
			"product_presave_substances_via_parent",
		),
		(
			"product_presave_fda_cross_reported_inds",
			"product_presave_fda_cross_reported_inds_via_parent",
		),
		(
			"product_presave_mfds_regional_items",
			"product_presave_mfds_regional_items_via_parent",
		),
		("reporter_presaves", "reporter_presaves_org_isolation"),
		("study_presaves", "study_presaves_org_isolation"),
		(
			"study_presave_registration_numbers",
			"study_presave_registration_numbers_via_parent",
		),
		("narrative_presaves", "narrative_presaves_org_isolation"),
		(
			"narrative_presave_sender_diagnoses",
			"narrative_presave_sender_diagnoses_via_parent",
		),
		(
			"narrative_presave_case_summaries",
			"narrative_presave_case_summaries_via_parent",
		),
	];

	let policy_rows: Vec<(String, String)> = sqlx::query_as(
		"SELECT tablename::text, policyname::text
		 FROM pg_policies
		 WHERE schemaname = 'public'
		   AND tablename = ANY($1)",
	)
	.bind(SECTION_PRESAVE_TABLES)
	.fetch_all(mm.dbx().db())
	.await?;
	let policies: HashSet<_> = policy_rows
		.iter()
		.map(|(table, policy)| (table.as_str(), policy.as_str()))
		.collect();

	for policy in expected_policies {
		assert!(
			policies.contains(&policy),
			"missing policy {} on {}",
			policy.1,
			policy.0
		);
	}

	let org_aware_fk_count: i64 = sqlx::query_scalar(
		"WITH fk_columns AS (
			SELECT
				c.conname::text AS constraint_name,
				c.conrelid::regclass::text AS table_name,
				c.confrelid::regclass::text AS foreign_table_name,
				c.confdeltype,
				array_agg(a.attname ORDER BY k.ord)::text[] AS columns,
				array_agg(fa.attname ORDER BY k.ord)::text[] AS foreign_columns
			FROM pg_constraint c
			JOIN LATERAL unnest(c.conkey, c.confkey) WITH ORDINALITY AS k(attnum, fattnum, ord) ON true
			JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum
			JOIN pg_attribute fa ON fa.attrelid = c.confrelid AND fa.attnum = k.fattnum
			WHERE c.contype = 'f'
			GROUP BY c.oid, c.conrelid, c.confrelid, c.confdeltype
		)
		SELECT COUNT(*)
		FROM fk_columns
		WHERE (
				table_name = 'product_presaves'
				AND foreign_table_name = 'sender_presaves'
				AND columns = ARRAY['sender_presave_id', 'organization_id']::text[]
				AND foreign_columns = ARRAY['id', 'organization_id']::text[]
				AND confdeltype = 'n'
			)
			OR (
				table_name = 'study_presaves'
				AND foreign_table_name = 'product_presaves'
				AND columns = ARRAY['product_presave_id', 'organization_id']::text[]
				AND foreign_columns = ARRAY['id', 'organization_id']::text[]
				AND confdeltype = 'n'
			)",
	)
	.fetch_one(mm.dbx().db())
	.await?;
	assert_eq!(
		org_aware_fk_count, 2,
		"missing org-aware composite FKs for product->sender and study->product"
	);

	let expected_constraints = [
		(
			"sender_presaves",
			"sender_presaves_id_organization_unique",
			"u",
		),
		(
			"product_presaves",
			"product_presaves_id_organization_unique",
			"u",
		),
		("product_presaves", "product_presaves_sender_org_fk", "f"),
		("study_presaves", "study_presaves_product_org_fk", "f"),
	];
	let constraint_rows: Vec<(String, String, String)> = sqlx::query_as(
		"SELECT conrelid::regclass::text, conname::text, contype::text
		 FROM pg_constraint
		 WHERE connamespace = 'public'::regnamespace
		   AND conname = ANY($1)",
	)
	.bind(
		expected_constraints
			.iter()
			.map(|(_, name, _)| *name)
			.collect::<Vec<_>>(),
	)
	.fetch_all(mm.dbx().db())
	.await?;
	let constraints: HashSet<_> = constraint_rows
		.iter()
		.map(|(table, name, contype)| {
			(table.as_str(), name.as_str(), contype.as_str())
		})
		.collect();
	for expected in expected_constraints {
		assert!(
			constraints.contains(&expected),
			"missing expected compatibility constraint {} on {}",
			expected.1,
			expected.0
		);
	}

	let legacy_fk_names = [
		"product_presaves_sender_presave_id_fkey",
		"study_presaves_product_presave_id_fkey",
	];
	let legacy_fk_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*)
		 FROM pg_constraint
		 WHERE connamespace = 'public'::regnamespace
		   AND contype = 'f'
		   AND conname = ANY($1)",
	)
	.bind(legacy_fk_names)
	.fetch_one(mm.dbx().db())
	.await?;
	assert_eq!(
		legacy_fk_count, 0,
		"legacy single-column section presave FKs must be removed"
	);

	let trigger_rows: Vec<(String, String)> = sqlx::query_as(
		"SELECT tgrelid::regclass::text, tgname::text
		 FROM pg_trigger
		 WHERE NOT tgisinternal
		   AND tgrelid = ANY($1::text[]::regclass[])",
	)
	.bind(SECTION_PRESAVE_TABLES)
	.fetch_all(mm.dbx().db())
	.await?;
	let triggers: HashSet<_> = trigger_rows
		.iter()
		.map(|(table, trigger)| (table.as_str(), trigger.as_str()))
		.collect();

	for table in SECTION_PRESAVE_TABLES {
		let audit_trigger = format!("audit_{table}");
		let update_trigger = format!("update_{table}_updated_at");
		assert!(
			triggers.contains(&(table, audit_trigger.as_str())),
			"missing audit trigger for {table}"
		);
		assert!(
			triggers.contains(&(table, update_trigger.as_str())),
			"missing updated_at trigger for {table}"
		);
	}

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
	set_user_context(&mut tx, demo_user_id()).await?;
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

	for (sender_id, org_id, label) in
		[(sender_a_id, org_a_id, "A"), (sender_b_id, org_b_id, "B")]
	{
		sqlx::query(
			"INSERT INTO sender_presaves (
				id, organization_id, authority, name, created_by, updated_by
			)
			VALUES ($1, $2, 'ich', $3, $4, $4)",
		)
		.bind(sender_id)
		.bind(org_id)
		.bind(format!("Presave FK Sender {label} {sender_id}"))
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await?;
	}

	sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, authority, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $5, $5)",
	)
	.bind(product_a_id)
	.bind(org_a_id)
	.bind(format!("Presave FK Product A {product_a_id}"))
	.bind(sender_a_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, authority, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $5, $5)",
	)
	.bind(product_b_id)
	.bind(org_b_id)
	.bind(format!("Presave FK Product B {product_b_id}"))
	.bind(sender_b_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO study_presaves (
			id, organization_id, authority, name, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $5, $5)",
	)
	.bind(study_a_id)
	.bind(org_a_id)
	.bind(format!("Presave FK Study A {study_a_id}"))
	.bind(product_a_id)
	.bind(demo_user_id())
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;

	let mut invalid_tx = mm.dbx().db().begin().await?;
	set_user_context(&mut invalid_tx, demo_user_id()).await?;
	set_org_context(&mut invalid_tx, org_a_id, "system_admin").await?;
	let cross_org_product = sqlx::query(
		"INSERT INTO product_presaves (
			id, organization_id, authority, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $5, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(org_a_id)
	.bind("Cross Org Product")
	.bind(sender_b_id)
	.bind(demo_user_id())
	.execute(&mut *invalid_tx)
	.await;
	assert!(
		matches!(cross_org_product, Err(ref err) if is_foreign_key_violation(err)),
		"cross-org product->sender link should fail composite FK: {cross_org_product:?}"
	);
	invalid_tx.rollback().await?;

	let mut invalid_tx = mm.dbx().db().begin().await?;
	set_user_context(&mut invalid_tx, demo_user_id()).await?;
	set_org_context(&mut invalid_tx, org_a_id, "system_admin").await?;
	let cross_org_study = sqlx::query(
		"INSERT INTO study_presaves (
			id, organization_id, authority, name, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $5, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(org_a_id)
	.bind("Cross Org Study")
	.bind(product_b_id)
	.bind(demo_user_id())
	.execute(&mut *invalid_tx)
	.await;
	assert!(
		matches!(cross_org_study, Err(ref err) if is_foreign_key_violation(err)),
		"cross-org study->product link should fail composite FK: {cross_org_study:?}"
	);
	invalid_tx.rollback().await?;

	Ok(())
}

#[test]
fn section_presave_compatibility_cleans_cross_org_links_before_composite_fks() {
	let schema = include_str!("../../../../db/bootstrap/01-safetydb-schema.sql");
	let product_cleanup = schema
		.find("UPDATE product_presaves p")
		.expect("compatibility block must null cross-org product->sender links");
	let product_fk = schema
		.find("ALTER TABLE product_presaves\n            ADD CONSTRAINT product_presaves_sender_org_fk")
		.expect("compatibility block must add product->sender composite FK");
	let study_cleanup = schema
		.find("UPDATE study_presaves s")
		.expect("compatibility block must null cross-org study->product links");
	let study_fk = schema
		.find("ALTER TABLE study_presaves\n            ADD CONSTRAINT study_presaves_product_org_fk")
		.expect("compatibility block must add study->product composite FK");

	assert!(
		product_cleanup < product_fk,
		"product->sender mismatch cleanup must run before adding the composite FK"
	);
	assert!(
		study_cleanup < study_fk,
		"study->product mismatch cleanup must run before adding the composite FK"
	);
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_crud_roundtrip() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			authority: RegulatoryAuthority::Ich,
			name: format!("Sender Presave {suffix}"),
			comments: Some("sender comment".into()),
			is_default: Some(true),
			sender_type: Some("1".into()),
			organization_name: Some("Sender Org Before".into()),
			department: Some("Safety".into()),
			street_address: Some("1 Sender St".into()),
			city: Some("Seoul".into()),
			state: Some("11".into()),
			postcode: Some("04524".into()),
			country_code: Some("KR".into()),
			telephone: Some("02-1111-2222".into()),
			fax: Some("02-1111-3333".into()),
			email: Some("sender@example.com".into()),
		},
	)
	.await?;
	let sender = SenderPresaveBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(sender.authority, RegulatoryAuthority::Ich);
	assert_eq!(sender.name, format!("Sender Presave {suffix}"));
	assert_eq!(
		sender.organization_name.as_deref(),
		Some("Sender Org Before")
	);

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			authority: RegulatoryAuthority::Fda,
			name: format!("Receiver Presave {suffix}"),
			comments: None,
			receiver_type: Some("agency".into()),
			organization_name: Some("Receiver Org".into()),
			receiver_identifier: Some("CDER".into()),
			day_count_rule: Some("calendar".into()),
			nsae_solicited_day_count: Some(15),
			nsae_solicited_not_applicable: Some(false),
			nsae_non_solicited_day_count: Some(15),
			nsae_non_solicited_not_applicable: Some(false),
			sae_solicited_day_count: Some(7),
			sae_solicited_not_applicable: Some(false),
			sae_non_solicited_day_count: Some(7),
			sae_non_solicited_not_applicable: Some(false),
			description: Some("FDA routing".into()),
		},
	)
	.await?;
	let receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
	assert_eq!(receiver.authority, RegulatoryAuthority::Fda);
	assert_eq!(receiver.name, format!("Receiver Presave {suffix}"));
	assert_eq!(receiver.receiver_identifier.as_deref(), Some("CDER"));

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			authority: RegulatoryAuthority::Mfds,
			name: format!("Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			drug_characterization: Some("1".into()),
			medicinal_product: Some("Medicinal Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: Some("Brand".into()),
			drug_generic_name: Some("Generic".into()),
			manufacturer_name: Some("Manufacturer".into()),
			product_description: Some("Product description".into()),
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: Some(false),
			obtain_drug_country: Some("KR".into()),
			drug_authorization_number: Some("AUTH-1".into()),
			drug_authorization_country: Some("KR".into()),
			drug_authorization_holder: Some("Holder".into()),
			holder_applicant_name_notation: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			mfds_domestic_product_code: Some("MFDS-P".into()),
			mfds_domestic_ingredient_code: Some("MFDS-I".into()),
			mfds_udl_product_code: None,
			mfds_udl_ingredient_code: None,
			mfds_udl_manufacturer_code: None,
			mfds_udl_manufacturer_name: None,
			mfds_foreign_ich_product_code: None,
			mfds_foreign_ich_ingredient_code: None,
			mfds_foreign_ich_holder_code: None,
			mfds_foreign_ich_holder_name: None,
			mfds_foreign_e2b_product_code: None,
			mfds_foreign_e2b_ingredient_code: None,
			mfds_foreign_e2b_holder_code: None,
			mfds_foreign_e2b_holder_name: None,
		},
	)
	.await?;
	let product = ProductPresaveBmc::get(&ctx, &mm, product_id).await?;
	assert_eq!(product.authority, RegulatoryAuthority::Mfds);
	assert_eq!(product.name, format!("Product Presave {suffix}"));
	assert_eq!(
		product.medicinal_product.as_deref(),
		Some("Medicinal Product")
	);

	let reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			authority: RegulatoryAuthority::Ich,
			name: format!("Reporter Presave {suffix}"),
			comments: None,
			reporter_title: Some("Dr".into()),
			reporter_given_name: Some("Casey".into()),
			reporter_middle_name: None,
			reporter_family_name: Some("Reporter".into()),
			organization: Some("Reporter Org".into()),
			department: Some("PV".into()),
			street: Some("2 Reporter St".into()),
			city: Some("Busan".into()),
			state: None,
			postcode: Some("48000".into()),
			telephone: Some("051-111-2222".into()),
			country_code: Some("KR".into()),
			email: Some("reporter@example.com".into()),
			qualification: Some("1".into()),
			qualification_kr1: None,
			primary_source_regulatory: Some("1".into()),
		},
	)
	.await?;
	let reporter = ReporterPresaveBmc::get(&ctx, &mm, reporter_id).await?;
	assert_eq!(reporter.authority, RegulatoryAuthority::Ich);
	assert_eq!(reporter.name, format!("Reporter Presave {suffix}"));
	assert_eq!(reporter.reporter_family_name.as_deref(), Some("Reporter"));

	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			authority: RegulatoryAuthority::Fda,
			name: format!("Study Presave {suffix}"),
			comments: None,
			product_presave_id: Some(product_id),
			study_name: Some("Study Name".into()),
			sponsor_study_number: Some("ST-001".into()),
			study_type_reaction: Some("1".into()),
			study_type_reaction_kr1: None,
			edc_sync: Some(true),
		},
	)
	.await?;
	let study = StudyPresaveBmc::get(&ctx, &mm, study_id).await?;
	assert_eq!(study.authority, RegulatoryAuthority::Fda);
	assert_eq!(study.name, format!("Study Presave {suffix}"));
	assert_eq!(study.sponsor_study_number.as_deref(), Some("ST-001"));

	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			authority: RegulatoryAuthority::Mfds,
			name: format!("Narrative Presave {suffix}"),
			comments: None,
			case_narrative: Some("Case narrative text".into()),
			reporter_comments: Some("Reporter comments".into()),
			sender_comments: Some("Sender comments".into()),
		},
	)
	.await?;
	let narrative = NarrativePresaveBmc::get(&ctx, &mm, narrative_id).await?;
	assert_eq!(narrative.authority, RegulatoryAuthority::Mfds);
	assert_eq!(narrative.name, format!("Narrative Presave {suffix}"));
	assert_eq!(
		narrative.case_narrative.as_deref(),
		Some("Case narrative text")
	);

	SenderPresaveBmc::update(
		&ctx,
		&mm,
		sender_id,
		SenderPresaveForUpdate {
			organization_name: Some("Sender Org After".into()),
			..Default::default()
		},
	)
	.await?;
	let updated_sender = SenderPresaveBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(
		updated_sender.organization_name.as_deref(),
		Some("Sender Org After")
	);
	let ich_senders =
		SenderPresaveBmc::list_by_authority(&ctx, &mm, RegulatoryAuthority::Ich)
			.await?;
	assert!(
		ich_senders.iter().any(|sender| sender.id == sender_id
			&& sender.organization_name.as_deref() == Some("Sender Org After")),
		"updated sender should appear in ICH list_by_authority results"
	);

	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ReporterPresaveBmc::delete(&ctx, &mm, reporter_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
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
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, demo_user_id()).await?;
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
			id, organization_id, authority, name, created_by, updated_by
		)
		VALUES ($1, $2, 'ich', $3, $4, $4)",
	)
	.bind(sender_id)
	.bind(parent_org_id)
	.bind(format!("Audit Sender {sender_id}"))
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

	Ok(())
}
