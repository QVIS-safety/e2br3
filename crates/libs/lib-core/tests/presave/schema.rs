use super::*;

#[test]
fn inactive_presave_reference_migration_covers_every_uuid_link() {
	let migration = include_str!(
		"../../../../../db/migrations/20260714_presave_lifecycle_guards.sql"
	);
	for trigger in [
		"guard_sender_information_source_sender_presave",
		"guard_drug_information_source_product_presave",
		"guard_study_information_source_study_presave",
		"guard_primary_sources_source_reporter_presave",
		"guard_narrative_information_source_narrative_presave",
		"guard_product_presaves_sender_presave",
		"guard_product_presaves_receiver_presave",
		"guard_study_presaves_product_presave",
		"guard_study_presave_products_product_presave",
		"guard_study_presave_reporters_reporter_presave",
	] {
		assert!(migration.contains(trigger), "missing trigger {trigger}");
	}
	assert!(migration.contains("ERRCODE = 'P2001'"));
	assert!(migration.contains("FOR KEY SHARE"));
}

#[test]
fn schema_alignment_covers_auditor_access_and_reaction_language() {
	let bootstrap =
		include_str!("../../../../../db/bootstrap/01-safetydb-schema.sql");
	let reactions = include_str!("../../../../../db/bootstrap/05-reactions.sql");
	let migration =
		include_str!("../../../../../db/migrations/20260719_schema_alignment.sql");
	let grant = "GRANT USAGE ON SCHEMA public TO e2br3_auditor_role;";

	assert!(bootstrap.contains(grant));
	assert!(migration.contains(grant));
	assert!(reactions.contains("reaction_language VARCHAR(3)"));
	assert!(migration.contains("reaction_language TYPE VARCHAR(3)"));
}

#[serial]
#[tokio::test]
#[ignore = "requires receiver presave route schema"]
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
async fn reporter_presaves_do_not_store_case_only_reporter_fields() -> Result<()> {
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
	// qualification_kr1 (C.2.r.4.KR.1 Other Health Professional Type) IS a reusable
	// reporter attribute per the UI spec, so it is intentionally a presave column.
	assert!(
		columns.iter().any(|column| column == "qualification_kr1"),
		"reporter_presaves.qualification_kr1 (C.2.r.4.KR.1) must exist as a reporter presave column"
	);

	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "requires legacy submission receiver route schema assumptions"]
async fn submission_receiver_options_schema_exists_without_receiver_presave_routes(
) -> Result<()> {
	_dev_utils::init_dev().await;
	let mm = ModelManager::new().await?;

	let submission_options_exists: bool = sqlx::query_scalar(
		"SELECT to_regclass('public.submission_receiver_options') IS NOT NULL",
	)
	.fetch_one(mm.dbx().db())
	.await?;
	assert!(
		submission_options_exists,
		"submission_receiver_options must own submission routing options"
	);

	let receiver_presave_routes_exists: bool = sqlx::query_scalar(
		"SELECT to_regclass('public.receiver_presave_routes') IS NOT NULL",
	)
	.fetch_one(mm.dbx().db())
	.await?;
	assert!(
		!receiver_presave_routes_exists,
		"receiver_presave_routes must not exist; receiver presave is parent plus consignees only"
	);

	Ok(())
}

#[serial]
#[tokio::test]
#[ignore = "requires receiver presave route schema"]
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
		(
			"receiver_presave_routes",
			"receiver_presave_routes_via_parent",
		),
		("product_presaves", "product_presaves_org_isolation"),
		(
			"product_presave_active_substances",
			"product_presave_active_substances_via_parent",
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

#[test]
#[ignore = "requires legacy section presave compatibility block"]
fn section_presave_compatibility_cleans_cross_org_links_before_composite_fks() {
	let schema = include_str!("../../../../../db/bootstrap/01-safetydb-schema.sql");
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
