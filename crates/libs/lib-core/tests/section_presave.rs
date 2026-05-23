mod common;

use crate::common::Result;
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use serial_test::serial;
use std::collections::HashSet;

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
