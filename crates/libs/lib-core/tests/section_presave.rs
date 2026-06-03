mod common;

use crate::common::{
	demo_ctx, demo_org_id, demo_user_id, reset_role, set_auditor_role, Result,
};
use lib_core::_dev_utils;
use lib_core::model::presave::{
	NarrativePresaveBmc, NarrativePresaveCaseSummaryBmc,
	NarrativePresaveCaseSummaryForCreate, NarrativePresaveCaseSummaryForUpdate,
	NarrativePresaveForCreate, NarrativePresaveSenderDiagnosisBmc,
	NarrativePresaveSenderDiagnosisForCreate,
	NarrativePresaveSenderDiagnosisForUpdate, ProductPresaveBmc,
	ProductPresaveForCreate, ProductPresaveForUpdate,
	ProductPresaveMfdsDeviceItemBmc, ProductPresaveMfdsDeviceItemForCreate,
	ProductPresaveMfdsDeviceItemForUpdate, ProductPresaveSubstanceBmc,
	ProductPresaveSubstanceForCreate, ProductPresaveSubstanceForUpdate,
	ReceiverPresaveBmc, ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsigneeForCreate, ReceiverPresaveConsigneeForUpdate,
	ReceiverPresaveForCreate, ReceiverPresaveForUpdate, ReporterPresaveBmc,
	ReporterPresaveForCreate, ReporterPresaveForUpdate, SenderPresaveBmc,
	SenderPresaveForCreate, SenderPresaveForUpdate, SenderPresaveGatewayBmc,
	SenderPresaveGatewayForCreate, SenderPresaveGatewayForUpdate,
	SenderPresaveResponsiblePersonBmc, SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresaveBmc, StudyPresaveForCreate,
	StudyPresaveProductBmc, StudyPresaveProductForCreate,
	StudyPresaveProductForUpdate, StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
};
use lib_core::model::store::{set_org_context, set_user_context};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use serde_json::json;
use serial_test::serial;
use std::collections::HashSet;

use rust_decimal::Decimal;
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
	"product_presave_mfds_device_items",
	"reporter_presaves",
	"study_presaves",
	"study_presave_registration_numbers",
	"study_presave_products",
	"study_presave_reporters",
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

fn expect_store_error<T>(result: lib_core::model::Result<T>, expected: &str) {
	match result {
		Err(ModelError::Store(message)) => assert!(
			message.contains(expected),
			"expected Store error containing {expected:?}, got {message:?}"
		),
		Err(err) => {
			panic!("expected Store error containing {expected:?}, got {err:?}")
		}
		Ok(_) => panic!("expected Store error containing {expected:?}, got Ok"),
	}
}

fn expect_validation_error<T>(result: lib_core::model::Result<T>, expected: &str) {
	match result {
		Err(ModelError::Validation { message }) => assert!(
			message.contains(expected),
			"expected Validation error containing {expected:?}, got {message:?}"
		),
		Err(err) => {
			panic!("expected Validation error containing {expected:?}, got {err:?}")
		}
		Ok(_) => panic!("expected Validation error containing {expected:?}, got Ok"),
	}
}

fn expect_conflict_error<T>(result: lib_core::model::Result<T>, expected: &str) {
	match result {
		Err(ModelError::Conflict { message }) => assert!(
			message.contains(expected),
			"expected Conflict error containing {expected:?}, got {message:?}"
		),
		Err(err) => {
			panic!("expected Conflict error containing {expected:?}, got {err:?}")
		}
		Ok(_) => panic!("expected Conflict error containing {expected:?}, got Ok"),
	}
}

async fn latest_audit_changed_fields(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
) -> Result<serde_json::Value> {
	set_auditor_role(mm).await?;
	let changed_fields_result = sqlx::query_scalar(
		"SELECT changed_fields
		 FROM audit_logs
		 WHERE table_name = $1
		   AND record_id = $2
		   AND action = 'UPDATE'
		 ORDER BY created_at DESC, id DESC
		 LIMIT 1",
	)
	.bind(table_name)
	.bind(record_id)
	.fetch_one(mm.dbx().db())
	.await;
	reset_role(mm).await?;

	Ok(changed_fields_result?)
}

async fn assert_audit_changed_field(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
	field_name: &str,
	expected_old: serde_json::Value,
	expected_new: serde_json::Value,
) -> Result<()> {
	let changed_fields =
		latest_audit_changed_fields(mm, table_name, record_id).await?;
	assert_eq!(
		changed_fields.get(field_name),
		Some(&json!({
			"old": expected_old,
			"new": expected_new
		})),
		"expected {table_name}.{field_name} audit diff for {record_id}, got {changed_fields}"
	);
	Ok(())
}

fn product_presave_create(
	_authority: RegulatoryAuthority,
	name: String,
	sender_presave_id: Uuid,
) -> ProductPresaveForCreate {
	ProductPresaveForCreate {
		name,
		comments: None,
		sender_presave_id: Some(sender_presave_id),
		product_id: Some(format!("PRODUCT-{}", Uuid::new_v4())),
		medicinal_product: Some("Authority Product".into()),
		medicinal_product_notation: None,
		preapproval_ip_name: None,
		brand_name: None,
		original_manufacturer: None,
		product_description: None,
		mpid: None,
		mpid_version: None,
		mfds_mpid: None,
		mfds_mpid_version: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		drug_authorization_country: None,
		drug_authorization_holder: None,
		holder_applicant_name_notation: None,
	}
}

fn sender_presave_create(name: String) -> SenderPresaveForCreate {
	SenderPresaveForCreate {
		name,
		comments: None,
		is_default: None,
		sender_type: Some("1".into()),
		organization_name: Some(format!("Sender Org {}", Uuid::new_v4())),
		organization_name_notation: None,
		person_given_name: Some("Sender Given".into()),
		department: None,
		street_address: None,
		city: None,
		state: None,
		postcode: None,
		country_code: None,
		telephone: None,
		fax: None,
		email: None,
	}
}

fn reporter_presave_create(
	_authority: RegulatoryAuthority,
	name: String,
) -> ReporterPresaveForCreate {
	ReporterPresaveForCreate {
		name,
		comments: None,
		reporter_title: None,
		reporter_given_name: Some("Authority".into()),
		reporter_middle_name: None,
		reporter_family_name: Some("Reporter".into()),
		organization: Some(format!("Authority Reporter Org {}", Uuid::new_v4())),
		department: None,
		street: None,
		city: None,
		state: None,
		postcode: None,
		telephone: None,
		country_code: Some("KR".into()),
		qualification: Some("1".into()),
		qualification_kr1: None,
		primary_source_regulatory: None,
	}
}

fn study_presave_create(
	_authority: RegulatoryAuthority,
	name: String,
) -> StudyPresaveForCreate {
	StudyPresaveForCreate {
		name,
		comments: None,
		product_presave_id: None,
		study_name: Some("Authority Study".into()),
		study_name_notation: None,
		sponsor_study_number: Some("AUTH-STUDY".into()),
		sponsor_study_number_kind: None,
		study_type_reaction: Some("1".into()),
		edc_sync: None,
		exclude_case_key_from_sync: None,
	}
}

fn study_presave_create_for_product(
	name: String,
	product_presave_id: Uuid,
) -> StudyPresaveForCreate {
	StudyPresaveForCreate {
		name,
		comments: None,
		product_presave_id: Some(product_presave_id),
		study_name: Some("Relationship Study".into()),
		study_name_notation: None,
		sponsor_study_number: Some(format!("REL-STUDY-{}", Uuid::new_v4())),
		sponsor_study_number_kind: None,
		study_type_reaction: Some("1".into()),
		edc_sync: None,
		exclude_case_key_from_sync: None,
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
async fn reporter_presaves_store_mfds_qualification_detail_but_not_case_only_email(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;

	let columns: Vec<String> = sqlx::query_scalar(
		"SELECT column_name::text
		 FROM information_schema.columns
		 WHERE table_schema = 'public' AND table_name = 'reporter_presaves'",
	)
	.fetch_all(mm.dbx().db())
	.await?;

	assert!(
		!columns.iter().any(|column| column == "email"),
		"reporter_presaves.email is case-only and must not be a reporter presave column"
	);
	assert!(
		columns.iter().any(|column| column == "qualification_kr1"),
		"reporter_presaves.qualification_kr1 stores MFDS C.2.r.4.KR.1 for reporter presave"
	);

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
			"product_presave_mfds_device_items",
			"product_presave_mfds_device_items_via_parent",
		),
		("reporter_presaves", "reporter_presaves_org_isolation"),
		("study_presaves", "study_presaves_org_isolation"),
		(
			"study_presave_registration_numbers",
			"study_presave_registration_numbers_via_parent",
		),
		(
			"study_presave_products",
			"study_presave_products_via_parent",
		),
		(
			"study_presave_reporters",
			"study_presave_reporters_via_parent",
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
				id, organization_id, name, created_by, updated_by
			)
			VALUES ($1, $2, $3, $4, $4)",
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
			id, organization_id, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $5, $5)",
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
			id, organization_id, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $5, $5)",
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
			id, organization_id, name, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $5, $5)",
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
			id, organization_id, name, sender_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $5, $5)",
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
			id, organization_id, name, product_presave_id, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $5, $5)",
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
			name: format!("Sender Presave {suffix}"),
			comments: Some("sender comment".into()),
			is_default: Some(true),
			sender_type: Some("1".into()),
			organization_name: Some(format!("Sender Org Before {suffix}")),
			organization_name_notation: None,
			person_given_name: Some("Sender Given".into()),
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
	assert_eq!(sender.name, format!("Sender Presave {suffix}"));
	assert_eq!(
		sender.organization_name.as_deref(),
		Some(format!("Sender Org Before {suffix}").as_str())
	);

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			name: format!("Receiver Presave {suffix}"),
			comments: None,
			receiver_type: Some("agency".into()),
			organization_name: Some(format!("Receiver Org {suffix}")),
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
	assert_eq!(receiver.name, format!("Receiver Presave {suffix}"));
	assert_eq!(receiver.receiver_identifier.as_deref(), Some("CDER"));

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			name: format!("Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("PRODUCT-{suffix}")),
			medicinal_product: Some(format!("Medicinal Product {suffix}")),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: Some("Brand".into()),
			original_manufacturer: None,
			product_description: Some("Product description".into()),
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: Some(false),
			obtain_drug_country: Some("KR".into()),
			drug_authorization_number: Some("AUTH-1".into()),
			drug_authorization_country: Some("KR".into()),
			drug_authorization_holder: Some("Holder".into()),
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	let product = ProductPresaveBmc::get(&ctx, &mm, product_id).await?;
	assert_eq!(product.name, format!("Product Presave {suffix}"));
	assert_eq!(
		product.medicinal_product.as_deref(),
		Some(format!("Medicinal Product {suffix}").as_str())
	);

	let reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			name: format!("Reporter Presave {suffix}"),
			comments: None,
			reporter_title: Some("Dr".into()),
			reporter_given_name: Some("Casey".into()),
			reporter_middle_name: None,
			reporter_family_name: Some("Reporter".into()),
			organization: Some(format!("Reporter Org {suffix}")),
			department: Some("PV".into()),
			street: Some("2 Reporter St".into()),
			city: Some("Busan".into()),
			state: None,
			postcode: Some("48000".into()),
			telephone: Some("051-111-2222".into()),
			country_code: Some("KR".into()),
			qualification: Some("1".into()),
			qualification_kr1: None,
			primary_source_regulatory: Some("1".into()),
		},
	)
	.await?;
	let reporter = ReporterPresaveBmc::get(&ctx, &mm, reporter_id).await?;
	assert_eq!(reporter.name, format!("Reporter Presave {suffix}"));
	assert_eq!(reporter.reporter_family_name.as_deref(), Some("Reporter"));

	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			name: format!("Study Presave {suffix}"),
			comments: None,
			product_presave_id: Some(product_id),
			study_name: Some(format!("Study Name {suffix}")),
			study_name_notation: Some("Study Name Notation".into()),
			sponsor_study_number: Some(format!("ST-001-{suffix}")),
			sponsor_study_number_kind: Some("PROTOCOL_NO".into()),
			study_type_reaction: Some("1".into()),
			edc_sync: Some(true),
			exclude_case_key_from_sync: Some(true),
		},
	)
	.await?;
	let study = StudyPresaveBmc::get(&ctx, &mm, study_id).await?;
	assert_eq!(study.name, format!("Study Presave {suffix}"));
	assert_eq!(
		study.sponsor_study_number.as_deref(),
		Some(format!("ST-001-{suffix}").as_str())
	);
	assert_eq!(
		study.study_name_notation.as_deref(),
		Some("Study Name Notation")
	);
	assert_eq!(
		study.sponsor_study_number_kind.as_deref(),
		Some("PROTOCOL_NO")
	);
	assert_eq!(study.exclude_case_key_from_sync, Some(true));

	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			name: format!("Narrative Presave {suffix}"),
			comments: None,
			case_narrative: Some("Case narrative text".into()),
			case_narrative_notation: Some("Case narrative notation".into()),
			additional_information: Some("Sponsor additional information".into()),
			reporter_comments: Some("Reporter comments".into()),
			sender_comments: Some("Sender comments".into()),
		},
	)
	.await?;
	let narrative = NarrativePresaveBmc::get(&ctx, &mm, narrative_id).await?;
	assert_eq!(narrative.name, format!("Narrative Presave {suffix}"));
	assert_eq!(
		narrative.case_narrative.as_deref(),
		Some("Case narrative text")
	);
	assert_eq!(
		narrative.case_narrative_notation.as_deref(),
		Some("Case narrative notation")
	);
	assert_eq!(
		narrative.additional_information.as_deref(),
		Some("Sponsor additional information")
	);

	SenderPresaveBmc::update(
		&ctx,
		&mm,
		sender_id,
		SenderPresaveForUpdate {
			organization_name: Some(format!("Sender Org After {suffix}")),
			..Default::default()
		},
	)
	.await?;
	let updated_sender = SenderPresaveBmc::get(&ctx, &mm, sender_id).await?;
	assert_eq!(
		updated_sender.organization_name.as_deref(),
		Some(format!("Sender Org After {suffix}").as_str())
	);
	let ich_senders = SenderPresaveBmc::list(&ctx, &mm, None).await?;
	assert!(
		ich_senders.iter().any(|sender| sender.id == sender_id
			&& sender.organization_name.as_deref()
				== Some(format!("Sender Org After {suffix}").as_str())),
		"updated sender should appear in presave list results"
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
async fn authorityless_union_fields_are_allowed() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			name: format!("Authorityless Sender {suffix}"),
			comments: None,
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Authorityless Sender Org {suffix}")),
			organization_name_notation: None,
			person_given_name: Some("Authorityless".into()),
			department: None,
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

	let product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authorityless Union Product {suffix}"),
		sender_id,
	);
	let product_id = ProductPresaveBmc::create(&ctx, &mm, product).await?;
	ProductPresaveBmc::update(
		&ctx,
		&mm,
		product_id,
		ProductPresaveForUpdate {
			medicinal_product: Some("Authorityless Union Product Updated".into()),
			..Default::default()
		},
	)
	.await?;

	let mut reporter = reporter_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authorityless Reporter {suffix}"),
	);
	reporter.primary_source_regulatory = Some("1".into());
	let reporter_id = ReporterPresaveBmc::create(&ctx, &mm, reporter).await?;
	ReporterPresaveBmc::update(
		&ctx,
		&mm,
		reporter_id,
		ReporterPresaveForUpdate {
			primary_source_regulatory: Some("2".into()),
			..Default::default()
		},
	)
	.await?;

	let mut study = study_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authorityless Study {suffix}"),
	);
	study.product_presave_id = Some(product_id);
	study.exclude_case_key_from_sync = Some(true);
	let study_id = StudyPresaveBmc::create(&ctx, &mm, study).await?;
	StudyPresaveProductBmc::create(
		&ctx,
		&mm,
		StudyPresaveProductForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			product_presave_id: Some(product_id),
			product_name: Some("Study Product Child".into()),
			deleted: Some(false),
		},
	)
	.await?;

	let mut invalid_kind_study = study_presave_create(
		RegulatoryAuthority::Ich,
		format!("Authorityless Study Invalid Kind {suffix}"),
	);
	invalid_kind_study.product_presave_id = Some(product_id);
	invalid_kind_study.sponsor_study_number_kind = Some("other_no".into());
	expect_store_error(
		StudyPresaveBmc::create(&ctx, &mm, invalid_kind_study).await,
		"sponsor_study_number_kind",
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_enforce_minimal_identity_requirements(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	expect_validation_error(
		SenderPresaveBmc::create(
			&ctx,
			&mm,
			SenderPresaveForCreate {
				name: format!("Invalid Sender Presave {suffix}"),
				comments: None,
				is_default: None,
				sender_type: Some("1".into()),
				organization_name: Some("Invalid Sender Org".into()),
				organization_name_notation: None,
				person_given_name: None,
				department: None,
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
		.await,
		"sender_type, organization_name, and person_given_name",
	);
	expect_validation_error(
		ReceiverPresaveBmc::create(
			&ctx,
			&mm,
			ReceiverPresaveForCreate {
				name: format!("Invalid Receiver Presave {suffix}"),
				comments: None,
				receiver_type: None,
				organization_name: Some("Invalid Receiver Org".into()),
				receiver_identifier: None,
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
		.await,
		"receiver_type and organization_name",
	);
	expect_validation_error(
		ProductPresaveBmc::create(
			&ctx,
			&mm,
			ProductPresaveForCreate {
				name: format!("Invalid Product Presave {suffix}"),
				comments: None,
				sender_presave_id: None,
				product_id: None,
				medicinal_product: None,
				medicinal_product_notation: None,
				preapproval_ip_name: None,
				brand_name: None,
				original_manufacturer: None,
				product_description: None,
				mpid: None,
				mpid_version: None,
				mfds_mpid: None,
				mfds_mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				drug_authorization_number: None,
				drug_authorization_country: None,
				drug_authorization_holder: None,
				holder_applicant_name_notation: None,
			},
		)
		.await,
		"sender_presave_id and product_id or preapproval_ip_name",
	);
	for (label, reporter_given_name, organization, qualification) in [
		("given name", None, Some("Invalid Reporter Org"), Some("1")),
		("organization", Some("Invalid"), None, Some("1")),
		(
			"qualification",
			Some("Invalid"),
			Some("Invalid Reporter Org"),
			None,
		),
	] {
		expect_validation_error(
			ReporterPresaveBmc::create(
				&ctx,
				&mm,
				ReporterPresaveForCreate {
					name: format!("Invalid Reporter Presave {label} {suffix}"),
					comments: None,
					reporter_title: None,
					reporter_given_name: reporter_given_name.map(str::to_string),
					reporter_middle_name: None,
					reporter_family_name: None,
					organization: organization.map(str::to_string),
					department: None,
					street: None,
					city: None,
					state: None,
					postcode: None,
					telephone: None,
					country_code: None,
					qualification: qualification.map(str::to_string),
					qualification_kr1: None,
					primary_source_regulatory: None,
				},
			)
			.await,
			"reporter_given_name, organization, and qualification",
		);
	}
	expect_validation_error(
		StudyPresaveBmc::create(
			&ctx,
			&mm,
			StudyPresaveForCreate {
				name: format!("Invalid Study Presave {suffix}"),
				comments: None,
				product_presave_id: None,
				study_name: Some("Invalid Study".into()),
				study_name_notation: None,
				sponsor_study_number: Some("INVALID-STUDY".into()),
				sponsor_study_number_kind: None,
				study_type_reaction: None,
				edc_sync: None,
				exclude_case_key_from_sync: None,
			},
		)
		.await,
		"product_presave_id, sponsor_study_number, study_name, and study_type_reaction",
	);
	expect_validation_error(
		NarrativePresaveBmc::create(
			&ctx,
			&mm,
			NarrativePresaveForCreate {
				name: "   ".into(),
				comments: None,
				case_narrative: None,
				case_narrative_notation: None,
				additional_information: None,
				reporter_comments: None,
				sender_comments: None,
			},
		)
		.await,
		"name",
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_parent_bmcs_reject_duplicate_identity_within_org(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			name: format!("Duplicate Sender Presave {suffix}"),
			comments: None,
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Duplicate Sender Org {suffix}")),
			organization_name_notation: None,
			person_given_name: Some("Alice".into()),
			department: None,
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
	expect_conflict_error(
		SenderPresaveBmc::create(
			&ctx,
			&mm,
			SenderPresaveForCreate {
				name: format!("Duplicate Sender Presave Copy {suffix}"),
				comments: None,
				is_default: None,
				sender_type: Some("1".into()),
				organization_name: Some(format!(" duplicate sender org {suffix} ")),
				organization_name_notation: None,
				person_given_name: Some("Bob".into()),
				department: None,
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
		.await,
		"sender presave duplicate",
	);

	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			name: format!("Duplicate Receiver Presave {suffix}"),
			comments: None,
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(format!("Duplicate Receiver Org {suffix}")),
			receiver_identifier: None,
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
	expect_conflict_error(
		ReceiverPresaveBmc::create(
			&ctx,
			&mm,
			ReceiverPresaveForCreate {
				name: format!("Duplicate Receiver Presave Copy {suffix}"),
				comments: None,
				receiver_type: Some("Original Manufacturer".into()),
				organization_name: Some(format!(
					" duplicate receiver org {suffix} "
				)),
				receiver_identifier: None,
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
		.await,
		"receiver presave duplicate",
	);

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			name: format!("Duplicate Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("DUP-PRODUCT-{suffix}")),
			medicinal_product: None,
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	expect_conflict_error(
		ProductPresaveBmc::create(
			&ctx,
			&mm,
			ProductPresaveForCreate {
				name: format!("Duplicate Product Presave Copy {suffix}"),
				comments: None,
				sender_presave_id: Some(sender_id),
				product_id: Some(format!(" dup-product-{suffix} ")),
				medicinal_product: None,
				medicinal_product_notation: None,
				preapproval_ip_name: None,
				brand_name: None,
				original_manufacturer: None,
				product_description: None,
				mpid: None,
				mpid_version: None,
				mfds_mpid: None,
				mfds_mpid_version: None,
				phpid: None,
				phpid_version: None,
				investigational_product_blinded: None,
				obtain_drug_country: None,
				drug_authorization_number: None,
				drug_authorization_country: None,
				drug_authorization_holder: None,
				holder_applicant_name_notation: None,
			},
		)
		.await,
		"product presave duplicate",
	);

	let reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		ReporterPresaveForCreate {
			name: format!("Duplicate Reporter Presave {suffix}"),
			comments: None,
			reporter_title: None,
			reporter_given_name: Some("Robin".into()),
			reporter_middle_name: None,
			reporter_family_name: None,
			organization: Some(format!("Duplicate Reporter Org {suffix}")),
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			telephone: None,
			country_code: None,
			qualification: Some("1".into()),
			qualification_kr1: None,
			primary_source_regulatory: None,
		},
	)
	.await?;
	expect_conflict_error(
		ReporterPresaveBmc::create(
			&ctx,
			&mm,
			ReporterPresaveForCreate {
				name: format!("Duplicate Reporter Presave Copy {suffix}"),
				comments: None,
				reporter_title: None,
				reporter_given_name: Some(" robin ".into()),
				reporter_middle_name: None,
				reporter_family_name: None,
				organization: Some(format!(" duplicate reporter org {suffix} ")),
				department: None,
				street: None,
				city: None,
				state: None,
				postcode: None,
				telephone: None,
				country_code: None,
				qualification: Some("1".into()),
				qualification_kr1: None,
				primary_source_regulatory: None,
			},
		)
		.await,
		"reporter presave duplicate",
	);

	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			name: format!("Duplicate Study Presave {suffix}"),
			comments: None,
			product_presave_id: Some(product_id),
			study_name: Some("Duplicate Study".into()),
			study_name_notation: None,
			sponsor_study_number: Some(format!("DUP-STUDY-{suffix}")),
			sponsor_study_number_kind: None,
			study_type_reaction: Some("1".into()),
			edc_sync: None,
			exclude_case_key_from_sync: None,
		},
	)
	.await?;
	expect_conflict_error(
		StudyPresaveBmc::create(
			&ctx,
			&mm,
			StudyPresaveForCreate {
				name: format!("Duplicate Study Presave Copy {suffix}"),
				comments: None,
				product_presave_id: Some(product_id),
				study_name: Some("Different Study".into()),
				study_name_notation: None,
				sponsor_study_number: Some(format!(" dup-study-{suffix} ")),
				sponsor_study_number_kind: None,
				study_type_reaction: Some("2".into()),
				edc_sync: None,
				exclude_case_key_from_sync: None,
			},
		)
		.await,
		"study presave duplicate",
	);

	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			name: format!("Duplicate Narrative Presave {suffix}"),
			comments: None,
			case_narrative: None,
			case_narrative_notation: None,
			additional_information: None,
			reporter_comments: None,
			sender_comments: None,
		},
	)
	.await?;
	expect_conflict_error(
		NarrativePresaveBmc::create(
			&ctx,
			&mm,
			NarrativePresaveForCreate {
				name: format!(" duplicate narrative presave {suffix} "),
				comments: None,
				case_narrative: Some("Body is not part of identity".into()),
				case_narrative_notation: None,
				additional_information: None,
				reporter_comments: None,
				sender_comments: None,
			},
		)
		.await,
		"narrative presave duplicate",
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

	expect_conflict_error(
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
		"sender presave is used by product presaves",
	);
	expect_conflict_error(
		SenderPresaveBmc::delete(&ctx, &mm, sender_id).await,
		"sender presave is used by product presaves",
	);

	expect_conflict_error(
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
		"product presave is used by study presaves",
	);
	expect_conflict_error(
		ProductPresaveBmc::delete(&ctx, &mm, product_id).await,
		"product presave is used by study presaves",
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_receiver_allows_legacy_type_update() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	for legacy_type in ["1", "2", "3", "4", "5", "6"] {
		let receiver_id = Uuid::new_v4();
		let mut tx = mm.dbx().db().begin().await?;
		set_user_context(&mut tx, demo_user_id()).await?;
		set_org_context(&mut tx, demo_org_id(), "system_admin").await?;

		sqlx::query(
			"INSERT INTO receiver_presaves (
				id, organization_id, name, receiver_type, organization_name,
				created_by, updated_by
			)
			VALUES ($1, $2, $3, $4, $5, $6, $6)",
		)
		.bind(receiver_id)
		.bind(demo_org_id())
		.bind(format!("Legacy Receiver Template {legacy_type} {suffix}"))
		.bind(legacy_type)
		.bind(format!("Legacy Receiver Org {legacy_type} {suffix}"))
		.bind(demo_user_id())
		.execute(&mut *tx)
		.await?;
		tx.commit().await?;

		ReceiverPresaveBmc::update(
			&ctx,
			&mm,
			receiver_id,
			ReceiverPresaveForUpdate {
				description: Some("legacy receiver still editable".into()),
				..Default::default()
			},
		)
		.await?;
		let receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
		assert_eq!(receiver.receiver_type.as_deref(), Some(legacy_type));
		assert_eq!(
			receiver.description.as_deref(),
			Some("legacy receiver still editable")
		);

		ReceiverPresaveBmc::update(
			&ctx,
			&mm,
			receiver_id,
			ReceiverPresaveForUpdate {
				receiver_type: Some(legacy_type.into()),
				description: Some("legacy receiver round-tripped".into()),
				..Default::default()
			},
		)
		.await?;
		let receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
		assert_eq!(receiver.receiver_type.as_deref(), Some(legacy_type));
		assert_eq!(
			receiver.description.as_deref(),
			Some("legacy receiver round-tripped")
		);

		ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	}

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
			name: template_name.clone(),
			comments: None,
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some(receiver_org.clone()),
			receiver_identifier: None,
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
	let deleted_receiver = ReceiverPresaveBmc::get(&ctx, &mm, receiver_id).await?;
	assert!(deleted_receiver.deleted);

	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn section_presave_child_bmcs_crud_roundtrip() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		SenderPresaveForCreate {
			name: format!("Child Sender Presave {suffix}"),
			comments: None,
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some("Child Sender Org".into()),
			organization_name_notation: None,
			person_given_name: Some("Child Sender Given".into()),
			department: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: Some("KR".into()),
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	let receiver_id = ReceiverPresaveBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveForCreate {
			name: format!("Child Receiver Presave {suffix}"),
			comments: None,
			receiver_type: Some("Regulatory Authority".into()),
			organization_name: Some("Child Receiver Org".into()),
			receiver_identifier: None,
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
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			name: format!("Child Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("CHILD-PRODUCT-{suffix}")),
			medicinal_product: Some("Child Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	let fda_product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		ProductPresaveForCreate {
			name: format!("Child FDA Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("CHILD-FDA-PRODUCT-{suffix}")),
			medicinal_product: Some("Child FDA Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?;
	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			name: format!("Child Study Presave {suffix}"),
			comments: None,
			product_presave_id: Some(product_id),
			study_name: Some("Child Study".into()),
			study_name_notation: None,
			sponsor_study_number: Some(format!("CHILD-STUDY-{suffix}")),
			sponsor_study_number_kind: None,
			study_type_reaction: Some("1".into()),
			edc_sync: None,
			exclude_case_key_from_sync: None,
		},
	)
	.await?;
	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			name: format!("Child Narrative Presave {suffix}"),
			comments: None,
			case_narrative: Some("Child narrative".into()),
			case_narrative_notation: None,
			additional_information: None,
			reporter_comments: None,
			sender_comments: None,
		},
	)
	.await?;

	let gateway_late_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 20,
			gateway_authority: "fda".into(),
			sender_identifier: Some("gateway-late".into()),
			routing_identifier: None,
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(false),
			deleted: None,
		},
	)
	.await?;
	let gateway_first_id = SenderPresaveGatewayBmc::create(
		&ctx,
		&mm,
		SenderPresaveGatewayForCreate {
			sender_presave_id: sender_id,
			sequence_number: 10,
			gateway_authority: "mfds".into(),
			sender_identifier: Some("gateway-first".into()),
			routing_identifier: Some("route-before".into()),
			cde_sender_identifier: None,
			cdr_sender_identifier: None,
			is_default_for_authority: Some(true),
			deleted: None,
		},
	)
	.await?;
	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		gateway_first_id,
		SenderPresaveGatewayForUpdate {
			routing_identifier: Some("route-after".into()),
			..Default::default()
		},
	)
	.await?;
	let gateway_first =
		SenderPresaveGatewayBmc::get(&ctx, &mm, gateway_first_id).await?;
	assert_eq!(gateway_first.sender_presave_id, sender_id);
	assert_eq!(gateway_first.sequence_number, 10);
	assert_eq!(
		gateway_first.routing_identifier.as_deref(),
		Some("route-after")
	);
	assert_audit_changed_field(
		&mm,
		"sender_presave_gateways",
		gateway_first_id,
		"routing_identifier",
		json!("route-before"),
		json!("route-after"),
	)
	.await?;
	let gateways =
		SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, sender_id).await?;
	assert_eq!(gateways[0].id, gateway_first_id);
	assert_eq!(gateways[1].id, gateway_late_id);
	SenderPresaveGatewayBmc::delete(&ctx, &mm, gateway_late_id).await?;
	assert!(
		SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, sender_id)
			.await?
			.iter()
			.all(|gateway| gateway.id != gateway_late_id)
	);

	let responsible_id = SenderPresaveResponsiblePersonBmc::create(
		&ctx,
		&mm,
		SenderPresaveResponsiblePersonForCreate {
			sender_presave_id: sender_id,
			sequence_number: 1,
			department: Some("PV".into()),
			person_title: Some("Dr".into()),
			person_given_name: Some("Before".into()),
			person_middle_name: None,
			person_family_name: Some("Person".into()),
			is_default: Some(false),
			deleted: None,
		},
	)
	.await?;
	SenderPresaveResponsiblePersonBmc::update(
		&ctx,
		&mm,
		responsible_id,
		SenderPresaveResponsiblePersonForUpdate {
			person_given_name: Some("After".into()),
			is_default: Some(true),
			..Default::default()
		},
	)
	.await?;
	let responsible =
		SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, responsible_id).await?;
	assert_eq!(responsible.sender_presave_id, sender_id);
	assert_eq!(responsible.person_given_name.as_deref(), Some("After"));
	assert_audit_changed_field(
		&mm,
		"sender_presave_responsible_persons",
		responsible_id,
		"person_given_name",
		json!("Before"),
		json!("After"),
	)
	.await?;
	assert_eq!(
		SenderPresaveResponsiblePersonBmc::list_by_parent(&ctx, &mm, sender_id)
			.await?[0]
			.id,
		responsible_id
	);

	let consignee_id = ReceiverPresaveConsigneeBmc::create(
		&ctx,
		&mm,
		ReceiverPresaveConsigneeForCreate {
			receiver_presave_id: receiver_id,
			sequence_number: 1,
			name: Some("Consignee Before".into()),
			phone: None,
			email: Some("before@example.com".into()),
		},
	)
	.await?;
	ReceiverPresaveConsigneeBmc::update(
		&ctx,
		&mm,
		consignee_id,
		ReceiverPresaveConsigneeForUpdate {
			name: Some("Consignee After".into()),
			..Default::default()
		},
	)
	.await?;
	let consignee =
		ReceiverPresaveConsigneeBmc::get(&ctx, &mm, consignee_id).await?;
	assert_eq!(consignee.receiver_presave_id, receiver_id);
	assert_eq!(consignee.name.as_deref(), Some("Consignee After"));
	assert_audit_changed_field(
		&mm,
		"receiver_presave_consignees",
		consignee_id,
		"name",
		json!("Consignee Before"),
		json!("Consignee After"),
	)
	.await?;
	assert_eq!(
		ReceiverPresaveConsigneeBmc::list_by_parent(&ctx, &mm, receiver_id).await?
			[0]
		.id,
		consignee_id
	);

	let substance_id = ProductPresaveSubstanceBmc::create(
		&ctx,
		&mm,
		ProductPresaveSubstanceForCreate {
			product_presave_id: product_id,
			sequence_number: 1,
			substance_name: Some("Substance Before".into()),
			substance_termid_version: None,
			substance_termid: Some("SUB-1".into()),
			mfds_version: None,
			mfds_id: None,
			strength_value: Some(Decimal::new(125, 2)),
			strength_unit: Some("mg".into()),
		},
	)
	.await?;
	ProductPresaveSubstanceBmc::update(
		&ctx,
		&mm,
		substance_id,
		ProductPresaveSubstanceForUpdate {
			substance_name: Some("Substance After".into()),
			..Default::default()
		},
	)
	.await?;
	let substance = ProductPresaveSubstanceBmc::get(&ctx, &mm, substance_id).await?;
	assert_eq!(substance.product_presave_id, product_id);
	assert_eq!(substance.substance_name.as_deref(), Some("Substance After"));
	assert_audit_changed_field(
		&mm,
		"product_presave_substances",
		substance_id,
		"substance_name",
		json!("Substance Before"),
		json!("Substance After"),
	)
	.await?;
	assert_eq!(
		ProductPresaveSubstanceBmc::list_by_parent(&ctx, &mm, product_id).await?[0]
			.id,
		substance_id
	);

	let maker_id = ProductPresaveMfdsDeviceItemBmc::create(
		&ctx,
		&mm,
		ProductPresaveMfdsDeviceItemForCreate {
			product_presave_id: product_id,
			sequence_number: 1,
			code: Some("KR_DVC_MFR".into()),
			value_code: None,
			value_value: Some("KR Maker".into()),
		},
	)
	.await?;
	let problem_id = ProductPresaveMfdsDeviceItemBmc::create(
		&ctx,
		&mm,
		ProductPresaveMfdsDeviceItemForCreate {
			product_presave_id: product_id,
			sequence_number: 2,
			code: Some("KR_DVC_PROBC".into()),
			value_code: Some("PROB-1".into()),
			value_value: None,
		},
	)
	.await?;
	ProductPresaveMfdsDeviceItemBmc::update(
		&ctx,
		&mm,
		problem_id,
		ProductPresaveMfdsDeviceItemForUpdate {
			sequence_number: Some(3),
			code: Some("KR_DVC_PROBC".into()),
			value_code: Some("PROB-2".into()),
			value_value: None,
		},
	)
	.await?;
	let device_items =
		ProductPresaveMfdsDeviceItemBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	assert_eq!(device_items.len(), 2);
	assert_eq!(device_items[0].id, maker_id);
	assert_eq!(device_items[0].code.as_deref(), Some("KR_DVC_MFR"));
	assert_eq!(device_items[0].value_value.as_deref(), Some("KR Maker"));
	assert_eq!(device_items[1].id, problem_id);
	assert_eq!(device_items[1].sequence_number, 3);
	assert_eq!(device_items[1].value_code.as_deref(), Some("PROB-2"));
	assert_audit_changed_field(
		&mm,
		"product_presave_mfds_device_items",
		problem_id,
		"value_code",
		json!("PROB-1"),
		json!("PROB-2"),
	)
	.await?;
	ProductPresaveMfdsDeviceItemBmc::delete(&ctx, &mm, maker_id).await?;
	let device_items =
		ProductPresaveMfdsDeviceItemBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	assert_eq!(device_items.len(), 1);
	assert_eq!(device_items[0].id, problem_id);

	let registration_id = StudyPresaveRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyPresaveRegistrationNumberForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			registration_number: Some("REG-before".into()),
			country_code: Some("KR".into()),
			deleted: Some(false),
		},
	)
	.await?;
	StudyPresaveRegistrationNumberBmc::update(
		&ctx,
		&mm,
		registration_id,
		StudyPresaveRegistrationNumberForUpdate {
			registration_number: Some("REG-after".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let registration =
		StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, registration_id).await?;
	assert_eq!(registration.study_presave_id, study_id);
	assert_eq!(
		registration.registration_number.as_deref(),
		Some("REG-after")
	);
	assert!(registration.deleted);
	assert_audit_changed_field(
		&mm,
		"study_presave_registration_numbers",
		registration_id,
		"registration_number",
		json!("REG-before"),
		json!("REG-after"),
	)
	.await?;
	assert_eq!(
		StudyPresaveRegistrationNumberBmc::list_by_parent(&ctx, &mm, study_id)
			.await?[0]
			.id,
		registration_id
	);

	let study_product_id = StudyPresaveProductBmc::create(
		&ctx,
		&mm,
		StudyPresaveProductForCreate {
			study_presave_id: study_id,
			sequence_number: 2,
			product_presave_id: Some(product_id),
			product_name: Some("STUDY-PRODUCT-before".into()),
			deleted: Some(false),
		},
	)
	.await?;
	StudyPresaveProductBmc::update(
		&ctx,
		&mm,
		study_product_id,
		StudyPresaveProductForUpdate {
			product_name: Some("STUDY-PRODUCT-after".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let study_product =
		StudyPresaveProductBmc::get(&ctx, &mm, study_product_id).await?;
	assert_eq!(study_product.study_presave_id, study_id);
	assert_eq!(
		study_product.product_name.as_deref(),
		Some("STUDY-PRODUCT-after")
	);
	assert!(study_product.deleted);
	assert_audit_changed_field(
		&mm,
		"study_presave_products",
		study_product_id,
		"product_name",
		json!("STUDY-PRODUCT-before"),
		json!("STUDY-PRODUCT-after"),
	)
	.await?;
	assert_eq!(
		StudyPresaveProductBmc::list_by_parent(&ctx, &mm, study_id).await?[0].id,
		study_product_id
	);

	let diagnosis_id = NarrativePresaveSenderDiagnosisBmc::create(
		&ctx,
		&mm,
		NarrativePresaveSenderDiagnosisForCreate {
			narrative_presave_id: narrative_id,
			sequence_number: 1,
			diagnosis_meddra_version: Some("26.1".into()),
			diagnosis_meddra_code: Some("10000001".into()),
			deleted: Some(false),
		},
	)
	.await?;
	NarrativePresaveSenderDiagnosisBmc::update(
		&ctx,
		&mm,
		diagnosis_id,
		NarrativePresaveSenderDiagnosisForUpdate {
			diagnosis_meddra_code: Some("10000002".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let diagnosis =
		NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, diagnosis_id).await?;
	assert_eq!(diagnosis.narrative_presave_id, narrative_id);
	assert_eq!(diagnosis.diagnosis_meddra_code.as_deref(), Some("10000002"));
	assert!(diagnosis.deleted);
	assert_audit_changed_field(
		&mm,
		"narrative_presave_sender_diagnoses",
		diagnosis_id,
		"diagnosis_meddra_code",
		json!("10000001"),
		json!("10000002"),
	)
	.await?;
	assert_eq!(
		NarrativePresaveSenderDiagnosisBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?[0]
			.id,
		diagnosis_id
	);

	let summary_id = NarrativePresaveCaseSummaryBmc::create(
		&ctx,
		&mm,
		NarrativePresaveCaseSummaryForCreate {
			narrative_presave_id: narrative_id,
			sequence_number: 2,
			summary_type: Some("sender".into()),
			language_code: Some("en".into()),
			summary_text: Some("summary before".into()),
			deleted: Some(false),
		},
	)
	.await?;
	NarrativePresaveCaseSummaryBmc::update(
		&ctx,
		&mm,
		summary_id,
		NarrativePresaveCaseSummaryForUpdate {
			summary_text: Some("summary after".into()),
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	let summary = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, summary_id).await?;
	assert_eq!(summary.narrative_presave_id, narrative_id);
	assert_eq!(summary.summary_text.as_deref(), Some("summary after"));
	assert!(summary.deleted);
	assert_audit_changed_field(
		&mm,
		"narrative_presave_case_summaries",
		summary_id,
		"summary_text",
		json!("summary before"),
		json!("summary after"),
	)
	.await?;
	assert_eq!(
		NarrativePresaveCaseSummaryBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?[0]
			.id,
		summary_id
	);

	NarrativePresaveCaseSummaryBmc::delete(&ctx, &mm, summary_id).await?;
	NarrativePresaveSenderDiagnosisBmc::delete(&ctx, &mm, diagnosis_id).await?;
	StudyPresaveProductBmc::delete(&ctx, &mm, study_product_id).await?;
	StudyPresaveRegistrationNumberBmc::delete(&ctx, &mm, registration_id).await?;
	ProductPresaveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	ReceiverPresaveConsigneeBmc::delete(&ctx, &mm, consignee_id).await?;
	SenderPresaveResponsiblePersonBmc::delete(&ctx, &mm, responsible_id).await?;
	SenderPresaveGatewayBmc::delete(&ctx, &mm, gateway_first_id).await?;
	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, fda_product_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;

	Ok(())
}

#[serial]
#[tokio::test]
async fn product_presave_mfds_device_items_round_trip() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		&mm,
		sender_presave_create(format!("Device Item Sender {}", Uuid::new_v4())),
	)
	.await?;
	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Mfds,
			format!("Device Item Product {}", Uuid::new_v4()),
			sender_id,
		),
	)
	.await?;

	let maker_id = ProductPresaveMfdsDeviceItemBmc::create(
		&ctx,
		&mm,
		ProductPresaveMfdsDeviceItemForCreate {
			product_presave_id: product_id,
			sequence_number: 1,
			code: Some("KR_DVC_MFR".into()),
			value_code: None,
			value_value: Some("KR Maker".into()),
		},
	)
	.await?;
	let problem_id = ProductPresaveMfdsDeviceItemBmc::create(
		&ctx,
		&mm,
		ProductPresaveMfdsDeviceItemForCreate {
			product_presave_id: product_id,
			sequence_number: 2,
			code: Some("KR_DVC_PROBC".into()),
			value_code: Some("PROB-1".into()),
			value_value: None,
		},
	)
	.await?;

	ProductPresaveMfdsDeviceItemBmc::update(
		&ctx,
		&mm,
		problem_id,
		ProductPresaveMfdsDeviceItemForUpdate {
			sequence_number: Some(3),
			code: Some("KR_DVC_PROBC".into()),
			value_code: Some("PROB-2".into()),
			value_value: None,
		},
	)
	.await?;

	let rows =
		ProductPresaveMfdsDeviceItemBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].id, maker_id);
	assert_eq!(rows[0].code.as_deref(), Some("KR_DVC_MFR"));
	assert_eq!(rows[0].value_value.as_deref(), Some("KR Maker"));
	assert_eq!(rows[1].id, problem_id);
	assert_eq!(rows[1].sequence_number, 3);
	assert_eq!(rows[1].value_code.as_deref(), Some("PROB-2"));

	ProductPresaveMfdsDeviceItemBmc::delete(&ctx, &mm, maker_id).await?;
	let rows =
		ProductPresaveMfdsDeviceItemBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].id, problem_id);

	ProductPresaveMfdsDeviceItemBmc::delete(&ctx, &mm, problem_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	SenderPresaveBmc::delete(&ctx, &mm, sender_id).await?;
	Ok(())
}

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
			name: format!("Field Audit Sender {suffix}"),
			comments: None,
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("Field Audit Sender Org {suffix}")),
			organization_name_notation: None,
			person_given_name: Some("Audit".into()),
			department: None,
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

	set_auditor_role(&mm).await?;
	let changed_fields_result = sqlx::query_scalar(
		"SELECT changed_fields
		 FROM audit_logs
		 WHERE table_name = 'product_presaves'
		   AND record_id = $1
		   AND action = 'UPDATE'
		 ORDER BY created_at DESC
		 LIMIT 1",
	)
	.bind(product_id)
	.fetch_one(mm.dbx().db())
	.await;
	reset_role(&mm).await?;
	let changed_fields: serde_json::Value = changed_fields_result?;

	assert!(
		changed_fields.get("brand_name").is_some(),
		"expected changed_fields to contain brand_name, got {changed_fields}"
	);

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
			id, organization_id, name, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
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
		"INSERT INTO study_presaves (
			id, organization_id, name, created_by, updated_by
		)
		VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(study_id)
	.bind(parent_org_id)
	.bind(format!("Audit Study {study_id}"))
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
