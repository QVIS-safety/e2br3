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
	ProductPresaveFdaCrossReportedIndBmc,
	ProductPresaveFdaCrossReportedIndForCreate,
	ProductPresaveFdaCrossReportedIndForUpdate, ProductPresaveForCreate,
	ProductPresaveForUpdate, ProductPresaveMfdsRegionalItemBmc,
	ProductPresaveMfdsRegionalItemForCreate,
	ProductPresaveMfdsRegionalItemForUpdate, ProductPresaveSubstanceBmc,
	ProductPresaveSubstanceForCreate, ProductPresaveSubstanceForUpdate,
	ReceiverPresaveBmc, ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsigneeForCreate, ReceiverPresaveConsigneeForUpdate,
	ReceiverPresaveForCreate, ReporterPresaveBmc, ReporterPresaveForCreate,
	ReporterPresaveForUpdate, SenderPresaveBmc, SenderPresaveForCreate,
	SenderPresaveForUpdate, SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate, SenderPresaveResponsiblePersonBmc,
	SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresaveBmc, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
};
use lib_core::model::store::{set_org_context, set_user_context};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
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

fn product_presave_create(
	authority: RegulatoryAuthority,
	name: String,
) -> ProductPresaveForCreate {
	ProductPresaveForCreate {
		authority,
		name,
		comments: None,
		sender_presave_id: None,
		drug_characterization: None,
		medicinal_product: Some("Authority Product".into()),
		medicinal_product_notation: None,
		preapproval_ip_name: None,
		brand_name: None,
		drug_generic_name: None,
		manufacturer_name: None,
		product_description: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		investigational_product_blinded: None,
		obtain_drug_country: None,
		drug_authorization_number: None,
		drug_authorization_country: None,
		drug_authorization_holder: None,
		holder_applicant_name_notation: None,
		fda_ind_number_occurred: None,
		fda_pre_anda_number_occurred: None,
		mfds_domestic_product_code: None,
		mfds_domestic_ingredient_code: None,
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
	}
}

fn reporter_presave_create(
	authority: RegulatoryAuthority,
	name: String,
) -> ReporterPresaveForCreate {
	ReporterPresaveForCreate {
		authority,
		name,
		comments: None,
		reporter_title: None,
		reporter_given_name: Some("Authority".into()),
		reporter_middle_name: None,
		reporter_family_name: Some("Reporter".into()),
		organization: None,
		department: None,
		street: None,
		city: None,
		state: None,
		postcode: None,
		telephone: None,
		country_code: Some("KR".into()),
		email: None,
		qualification: Some("1".into()),
		qualification_kr1: None,
		primary_source_regulatory: None,
	}
}

fn study_presave_create(
	authority: RegulatoryAuthority,
	name: String,
) -> StudyPresaveForCreate {
	StudyPresaveForCreate {
		authority,
		name,
		comments: None,
		product_presave_id: None,
		study_name: Some("Authority Study".into()),
		sponsor_study_number: None,
		study_type_reaction: Some("1".into()),
		study_type_reaction_kr1: None,
		edc_sync: None,
	}
}

const PRODUCT_MFDS_FIELDS: &[&str] = &[
	"mfds_domestic_product_code",
	"mfds_domestic_ingredient_code",
	"mfds_udl_product_code",
	"mfds_udl_ingredient_code",
	"mfds_udl_manufacturer_code",
	"mfds_udl_manufacturer_name",
	"mfds_foreign_ich_product_code",
	"mfds_foreign_ich_ingredient_code",
	"mfds_foreign_ich_holder_code",
	"mfds_foreign_ich_holder_name",
	"mfds_foreign_e2b_product_code",
	"mfds_foreign_e2b_ingredient_code",
	"mfds_foreign_e2b_holder_code",
	"mfds_foreign_e2b_holder_name",
];

fn set_product_mfds_create_field(data: &mut ProductPresaveForCreate, field: &str) {
	match field {
		"mfds_domestic_product_code" => {
			data.mfds_domestic_product_code = Some("MFDS-P".into())
		}
		"mfds_domestic_ingredient_code" => {
			data.mfds_domestic_ingredient_code = Some("MFDS-I".into())
		}
		"mfds_udl_product_code" => data.mfds_udl_product_code = Some("UDL-P".into()),
		"mfds_udl_ingredient_code" => {
			data.mfds_udl_ingredient_code = Some("UDL-I".into())
		}
		"mfds_udl_manufacturer_code" => {
			data.mfds_udl_manufacturer_code = Some("UDL-M".into())
		}
		"mfds_udl_manufacturer_name" => {
			data.mfds_udl_manufacturer_name = Some("UDL Manufacturer".into())
		}
		"mfds_foreign_ich_product_code" => {
			data.mfds_foreign_ich_product_code = Some("ICH-P".into())
		}
		"mfds_foreign_ich_ingredient_code" => {
			data.mfds_foreign_ich_ingredient_code = Some("ICH-I".into())
		}
		"mfds_foreign_ich_holder_code" => {
			data.mfds_foreign_ich_holder_code = Some("ICH-H".into())
		}
		"mfds_foreign_ich_holder_name" => {
			data.mfds_foreign_ich_holder_name = Some("ICH Holder".into())
		}
		"mfds_foreign_e2b_product_code" => {
			data.mfds_foreign_e2b_product_code = Some("E2B-P".into())
		}
		"mfds_foreign_e2b_ingredient_code" => {
			data.mfds_foreign_e2b_ingredient_code = Some("E2B-I".into())
		}
		"mfds_foreign_e2b_holder_code" => {
			data.mfds_foreign_e2b_holder_code = Some("E2B-H".into())
		}
		"mfds_foreign_e2b_holder_name" => {
			data.mfds_foreign_e2b_holder_name = Some("E2B Holder".into())
		}
		_ => panic!("unknown MFDS product field {field}"),
	}
}

fn product_mfds_update(field: &str) -> ProductPresaveForUpdate {
	let mut update = ProductPresaveForUpdate::default();
	match field {
		"mfds_domestic_product_code" => {
			update.mfds_domestic_product_code = Some("MFDS-P".into())
		}
		"mfds_domestic_ingredient_code" => {
			update.mfds_domestic_ingredient_code = Some("MFDS-I".into())
		}
		"mfds_udl_product_code" => {
			update.mfds_udl_product_code = Some("UDL-P".into())
		}
		"mfds_udl_ingredient_code" => {
			update.mfds_udl_ingredient_code = Some("UDL-I".into())
		}
		"mfds_udl_manufacturer_code" => {
			update.mfds_udl_manufacturer_code = Some("UDL-M".into())
		}
		"mfds_udl_manufacturer_name" => {
			update.mfds_udl_manufacturer_name = Some("UDL Manufacturer".into())
		}
		"mfds_foreign_ich_product_code" => {
			update.mfds_foreign_ich_product_code = Some("ICH-P".into())
		}
		"mfds_foreign_ich_ingredient_code" => {
			update.mfds_foreign_ich_ingredient_code = Some("ICH-I".into())
		}
		"mfds_foreign_ich_holder_code" => {
			update.mfds_foreign_ich_holder_code = Some("ICH-H".into())
		}
		"mfds_foreign_ich_holder_name" => {
			update.mfds_foreign_ich_holder_name = Some("ICH Holder".into())
		}
		"mfds_foreign_e2b_product_code" => {
			update.mfds_foreign_e2b_product_code = Some("E2B-P".into())
		}
		"mfds_foreign_e2b_ingredient_code" => {
			update.mfds_foreign_e2b_ingredient_code = Some("E2B-I".into())
		}
		"mfds_foreign_e2b_holder_code" => {
			update.mfds_foreign_e2b_holder_code = Some("E2B-H".into())
		}
		"mfds_foreign_e2b_holder_name" => {
			update.mfds_foreign_e2b_holder_name = Some("E2B Holder".into())
		}
		_ => panic!("unknown MFDS product field {field}"),
	}
	update
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
async fn authority_specific_fields_are_enforced() -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;
	let ctx = demo_ctx();
	let suffix = Uuid::new_v4();

	let mut ich_product = product_presave_create(
		RegulatoryAuthority::Ich,
		format!("Authority ICH Product {suffix}"),
	);
	ich_product.fda_ind_number_occurred = Some("IND-ICH".into());
	expect_store_error(
		ProductPresaveBmc::create(&ctx, &mm, ich_product).await,
		"fda_ind_number_occurred",
	);

	let mut ich_product_pre_anda = product_presave_create(
		RegulatoryAuthority::Ich,
		format!("Authority ICH Product Pre-ANDA {suffix}"),
	);
	ich_product_pre_anda.fda_pre_anda_number_occurred = Some("ANDA-ICH".into());
	expect_store_error(
		ProductPresaveBmc::create(&ctx, &mm, ich_product_pre_anda).await,
		"fda_pre_anda_number_occurred",
	);

	let mut fda_product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authority FDA Product {suffix}"),
	);
	fda_product.mfds_domestic_product_code = Some("MFDS-FDA".into());
	expect_store_error(
		ProductPresaveBmc::create(&ctx, &mm, fda_product).await,
		"mfds_domestic_product_code",
	);

	for mfds_field in PRODUCT_MFDS_FIELDS {
		let mut fda_product = product_presave_create(
			RegulatoryAuthority::Fda,
			format!("Authority FDA Product {mfds_field} {suffix}"),
		);
		set_product_mfds_create_field(&mut fda_product, mfds_field);
		expect_store_error(
			ProductPresaveBmc::create(&ctx, &mm, fda_product).await,
			mfds_field,
		);
	}

	let mut valid_fda_product = product_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authority Valid FDA Product {suffix}"),
	);
	valid_fda_product.fda_ind_number_occurred = Some("IND-FDA".into());
	valid_fda_product.fda_pre_anda_number_occurred = Some("ANDA-FDA".into());
	let valid_fda_product_id =
		ProductPresaveBmc::create(&ctx, &mm, valid_fda_product).await?;
	ProductPresaveBmc::update(
		&ctx,
		&mm,
		valid_fda_product_id,
		ProductPresaveForUpdate {
			fda_ind_number_occurred: Some("IND-FDA-UPDATE".into()),
			fda_pre_anda_number_occurred: Some("ANDA-FDA-UPDATE".into()),
			..Default::default()
		},
	)
	.await?;

	let mut valid_mfds_product = product_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authority Valid MFDS Product {suffix}"),
	);
	for mfds_field in PRODUCT_MFDS_FIELDS {
		set_product_mfds_create_field(&mut valid_mfds_product, mfds_field);
	}
	let valid_mfds_product_id =
		ProductPresaveBmc::create(&ctx, &mm, valid_mfds_product).await?;
	for mfds_field in PRODUCT_MFDS_FIELDS {
		ProductPresaveBmc::update(
			&ctx,
			&mm,
			valid_mfds_product_id,
			product_mfds_update(mfds_field),
		)
		.await?;
	}

	let mut mfds_product = product_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authority MFDS Product {suffix}"),
	);
	mfds_product.fda_ind_number_occurred = Some("IND-MFDS".into());
	expect_store_error(
		ProductPresaveBmc::create(&ctx, &mm, mfds_product).await,
		"fda_ind_number_occurred",
	);

	let mut mfds_product_pre_anda = product_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authority MFDS Product Pre-ANDA {suffix}"),
	);
	mfds_product_pre_anda.fda_pre_anda_number_occurred = Some("ANDA-MFDS".into());
	expect_store_error(
		ProductPresaveBmc::create(&ctx, &mm, mfds_product_pre_anda).await,
		"fda_pre_anda_number_occurred",
	);

	let ich_product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Ich,
			format!("Authority ICH Product Update {suffix}"),
		),
	)
	.await?;
	expect_store_error(
		ProductPresaveBmc::update(
			&ctx,
			&mm,
			ich_product_id,
			ProductPresaveForUpdate {
				fda_ind_number_occurred: Some("IND-UPDATE".into()),
				..Default::default()
			},
		)
		.await,
		"fda_ind_number_occurred",
	);
	expect_store_error(
		ProductPresaveBmc::update(
			&ctx,
			&mm,
			ich_product_id,
			ProductPresaveForUpdate {
				fda_pre_anda_number_occurred: Some("ANDA-UPDATE".into()),
				..Default::default()
			},
		)
		.await,
		"fda_pre_anda_number_occurred",
	);

	let fda_product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Fda,
			format!("Authority FDA Product Update {suffix}"),
		),
	)
	.await?;
	for mfds_field in PRODUCT_MFDS_FIELDS {
		expect_store_error(
			ProductPresaveBmc::update(
				&ctx,
				&mm,
				fda_product_id,
				product_mfds_update(mfds_field),
			)
			.await,
			mfds_field,
		);
	}

	let mut valid_mfds_reporter = reporter_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authority MFDS Reporter {suffix}"),
	);
	valid_mfds_reporter.qualification_kr1 = Some("KR-QUAL".into());
	let mfds_reporter_id =
		ReporterPresaveBmc::create(&ctx, &mm, valid_mfds_reporter).await?;

	let mut invalid_fda_reporter = reporter_presave_create(
		RegulatoryAuthority::Fda,
		format!("Authority FDA Reporter {suffix}"),
	);
	invalid_fda_reporter.qualification_kr1 = Some("KR-QUAL".into());
	expect_store_error(
		ReporterPresaveBmc::create(&ctx, &mm, invalid_fda_reporter).await,
		"qualification_kr1",
	);

	let fda_reporter_id = ReporterPresaveBmc::create(
		&ctx,
		&mm,
		reporter_presave_create(
			RegulatoryAuthority::Fda,
			format!("Authority FDA Reporter Update {suffix}"),
		),
	)
	.await?;
	expect_store_error(
		ReporterPresaveBmc::update(
			&ctx,
			&mm,
			fda_reporter_id,
			ReporterPresaveForUpdate {
				qualification_kr1: Some("KR-QUAL".into()),
				..Default::default()
			},
		)
		.await,
		"qualification_kr1",
	);

	let mut valid_mfds_study = study_presave_create(
		RegulatoryAuthority::Mfds,
		format!("Authority MFDS Study {suffix}"),
	);
	valid_mfds_study.study_type_reaction_kr1 = Some("KR-STUDY".into());
	let mfds_study_id = StudyPresaveBmc::create(&ctx, &mm, valid_mfds_study).await?;

	let mut invalid_ich_study = study_presave_create(
		RegulatoryAuthority::Ich,
		format!("Authority ICH Study {suffix}"),
	);
	invalid_ich_study.study_type_reaction_kr1 = Some("KR-STUDY".into());
	expect_store_error(
		StudyPresaveBmc::create(&ctx, &mm, invalid_ich_study).await,
		"study_type_reaction_kr1",
	);

	let ich_study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		study_presave_create(
			RegulatoryAuthority::Ich,
			format!("Authority ICH Study Update {suffix}"),
		),
	)
	.await?;
	expect_store_error(
		StudyPresaveBmc::update(
			&ctx,
			&mm,
			ich_study_id,
			StudyPresaveForUpdate {
				study_type_reaction_kr1: Some("KR-STUDY".into()),
				..Default::default()
			},
		)
		.await,
		"study_type_reaction_kr1",
	);

	StudyPresaveBmc::delete(&ctx, &mm, ich_study_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, mfds_study_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, fda_product_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, ich_product_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, valid_mfds_product_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, valid_fda_product_id).await?;
	ReporterPresaveBmc::delete(&ctx, &mm, fda_reporter_id).await?;
	ReporterPresaveBmc::delete(&ctx, &mm, mfds_reporter_id).await?;

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
			authority: RegulatoryAuthority::Ich,
			name: format!("Child Sender Presave {suffix}"),
			comments: None,
			is_default: None,
			sender_type: None,
			organization_name: Some("Child Sender Org".into()),
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
			authority: RegulatoryAuthority::Fda,
			name: format!("Child Receiver Presave {suffix}"),
			comments: None,
			receiver_type: None,
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
			authority: RegulatoryAuthority::Mfds,
			name: format!("Child Product Presave {suffix}"),
			comments: None,
			sender_presave_id: Some(sender_id),
			drug_characterization: None,
			medicinal_product: Some("Child Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			drug_generic_name: None,
			manufacturer_name: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
			fda_ind_number_occurred: None,
			fda_pre_anda_number_occurred: None,
			mfds_domestic_product_code: None,
			mfds_domestic_ingredient_code: None,
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
	let study_id = StudyPresaveBmc::create(
		&ctx,
		&mm,
		StudyPresaveForCreate {
			authority: RegulatoryAuthority::Fda,
			name: format!("Child Study Presave {suffix}"),
			comments: None,
			product_presave_id: Some(product_id),
			study_name: Some("Child Study".into()),
			sponsor_study_number: None,
			study_type_reaction: None,
			study_type_reaction_kr1: None,
			edc_sync: None,
		},
	)
	.await?;
	let narrative_id = NarrativePresaveBmc::create(
		&ctx,
		&mm,
		NarrativePresaveForCreate {
			authority: RegulatoryAuthority::Mfds,
			name: format!("Child Narrative Presave {suffix}"),
			comments: None,
			case_narrative: Some("Child narrative".into()),
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
			ema_sender_identifier: None,
			is_default_for_authority: Some(false),
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
			ema_sender_identifier: None,
			is_default_for_authority: Some(true),
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
	assert_eq!(
		ProductPresaveSubstanceBmc::list_by_parent(&ctx, &mm, product_id).await?[0]
			.id,
		substance_id
	);

	let ind_id = ProductPresaveFdaCrossReportedIndBmc::create(
		&ctx,
		&mm,
		ProductPresaveFdaCrossReportedIndForCreate {
			product_presave_id: product_id,
			sequence_number: 2,
			ind_number: Some("IND-before".into()),
		},
	)
	.await?;
	ProductPresaveFdaCrossReportedIndBmc::update(
		&ctx,
		&mm,
		ind_id,
		ProductPresaveFdaCrossReportedIndForUpdate {
			ind_number: Some("IND-after".into()),
			..Default::default()
		},
	)
	.await?;
	let ind = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, ind_id).await?;
	assert_eq!(ind.product_presave_id, product_id);
	assert_eq!(ind.ind_number.as_deref(), Some("IND-after"));
	assert!(ProductPresaveFdaCrossReportedIndBmc::list_by_parent(
		&ctx, &mm, product_id
	)
	.await?
	.iter()
	.any(|item| item.id == ind_id));

	let regional_id = ProductPresaveMfdsRegionalItemBmc::create(
		&ctx,
		&mm,
		ProductPresaveMfdsRegionalItemForCreate {
			product_presave_id: product_id,
			sequence_number: 3,
			item_type: Some("domestic".into()),
			item_value: Some("before".into()),
		},
	)
	.await?;
	ProductPresaveMfdsRegionalItemBmc::update(
		&ctx,
		&mm,
		regional_id,
		ProductPresaveMfdsRegionalItemForUpdate {
			item_value: Some("after".into()),
			..Default::default()
		},
	)
	.await?;
	let regional =
		ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, regional_id).await?;
	assert_eq!(regional.product_presave_id, product_id);
	assert_eq!(regional.item_value.as_deref(), Some("after"));
	assert!(ProductPresaveMfdsRegionalItemBmc::list_by_parent(
		&ctx, &mm, product_id
	)
	.await?
	.iter()
	.any(|item| item.id == regional_id));

	let registration_id = StudyPresaveRegistrationNumberBmc::create(
		&ctx,
		&mm,
		StudyPresaveRegistrationNumberForCreate {
			study_presave_id: study_id,
			sequence_number: 1,
			registration_number: Some("REG-before".into()),
			country_code: Some("KR".into()),
		},
	)
	.await?;
	StudyPresaveRegistrationNumberBmc::update(
		&ctx,
		&mm,
		registration_id,
		StudyPresaveRegistrationNumberForUpdate {
			registration_number: Some("REG-after".into()),
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
	assert_eq!(
		StudyPresaveRegistrationNumberBmc::list_by_parent(&ctx, &mm, study_id)
			.await?[0]
			.id,
		registration_id
	);

	let diagnosis_id = NarrativePresaveSenderDiagnosisBmc::create(
		&ctx,
		&mm,
		NarrativePresaveSenderDiagnosisForCreate {
			narrative_presave_id: narrative_id,
			sequence_number: 1,
			diagnosis_meddra_version: Some("26.1".into()),
			diagnosis_meddra_code: Some("10000001".into()),
		},
	)
	.await?;
	NarrativePresaveSenderDiagnosisBmc::update(
		&ctx,
		&mm,
		diagnosis_id,
		NarrativePresaveSenderDiagnosisForUpdate {
			diagnosis_meddra_code: Some("10000002".into()),
			..Default::default()
		},
	)
	.await?;
	let diagnosis =
		NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, diagnosis_id).await?;
	assert_eq!(diagnosis.narrative_presave_id, narrative_id);
	assert_eq!(diagnosis.diagnosis_meddra_code.as_deref(), Some("10000002"));
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
		},
	)
	.await?;
	NarrativePresaveCaseSummaryBmc::update(
		&ctx,
		&mm,
		summary_id,
		NarrativePresaveCaseSummaryForUpdate {
			summary_text: Some("summary after".into()),
			..Default::default()
		},
	)
	.await?;
	let summary = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, summary_id).await?;
	assert_eq!(summary.narrative_presave_id, narrative_id);
	assert_eq!(summary.summary_text.as_deref(), Some("summary after"));
	assert_eq!(
		NarrativePresaveCaseSummaryBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?[0]
			.id,
		summary_id
	);

	NarrativePresaveCaseSummaryBmc::delete(&ctx, &mm, summary_id).await?;
	NarrativePresaveSenderDiagnosisBmc::delete(&ctx, &mm, diagnosis_id).await?;
	StudyPresaveRegistrationNumberBmc::delete(&ctx, &mm, registration_id).await?;
	ProductPresaveMfdsRegionalItemBmc::delete(&ctx, &mm, regional_id).await?;
	ProductPresaveFdaCrossReportedIndBmc::delete(&ctx, &mm, ind_id).await?;
	ProductPresaveSubstanceBmc::delete(&ctx, &mm, substance_id).await?;
	ReceiverPresaveConsigneeBmc::delete(&ctx, &mm, consignee_id).await?;
	SenderPresaveResponsiblePersonBmc::delete(&ctx, &mm, responsible_id).await?;
	SenderPresaveGatewayBmc::delete(&ctx, &mm, gateway_first_id).await?;
	NarrativePresaveBmc::delete(&ctx, &mm, narrative_id).await?;
	StudyPresaveBmc::delete(&ctx, &mm, study_id).await?;
	ProductPresaveBmc::delete(&ctx, &mm, product_id).await?;
	ReceiverPresaveBmc::delete(&ctx, &mm, receiver_id).await?;
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

	let product_id = ProductPresaveBmc::create(
		&ctx,
		&mm,
		product_presave_create(
			RegulatoryAuthority::Ich,
			format!("Field Audit Product {suffix}"),
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
