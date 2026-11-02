use super::{init_test_mm, Result};
use serial_test::serial;
use sqlx::postgres::{PgPool, PgPoolOptions};

async fn audit_admin_pool() -> Result<PgPool> {
	let pool = PgPoolOptions::new()
		.max_connections(1)
		.connect(&std::env::var("SERVICE_MIGRATION_DB_URL")?)
		.await?;
	let (database_name,): (String,) = sqlx::query_as("SELECT current_database()")
		.fetch_one(&pool)
		.await?;
	if !database_name.starts_with("e2br3_test_") {
		return Err(format!(
			"refusing destructive audit test outside isolated e2br3_test_ database: {database_name}"
		)
		.into());
	}
	Ok(pool)
}

#[serial]
#[tokio::test]
async fn test_audit_chain_preserves_id_order_with_waiting_writer() -> Result<()> {
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	use lib_core::model::store::set_full_context_dbx;
	let first = init_test_mm().await?;
	// The isolated database bootstrap owns this system user and organization.
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	let second = first.clone();
	for mm in [&first, &second] {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	}
	let append = "SELECT append_audit_log('concurrency_regression', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)";
	first
		.dbx()
		.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org_id))
		.await?;
	let start = first
		.dbx()
		.fetch_one(sqlx::query_as::<_, (i64,)>(
			"SELECT max(id) FROM audit_logs",
		))
		.await?
		.0;
	let pid = second
		.dbx()
		.fetch_one(sqlx::query_as::<_, (i32,)>("SELECT pg_backend_pid()"))
		.await?
		.0;
	let org = org_id;
	let writer = tokio::spawn(async move {
		second
			.dbx()
			.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org))
			.await?;
		second.dbx().commit_txn().await
	});
	tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND NOT granted)")
                .bind(pid).fetch_one(first.dbx().db()).await?;
            if waiting.0 { return Ok::<_, sqlx::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await??;
	first
		.dbx()
		.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org_id))
		.await?;
	first.dbx().commit_txn().await?;
	writer.await??;
	first.dbx().begin_txn().await?;
	set_full_context_dbx(first.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	first
		.dbx()
		.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org_id))
		.await?;
	first.dbx().commit_txn().await?;
	let pool = audit_admin_pool().await?;
	let rows: Vec<(i64, String, String)> = sqlx::query_as(
		"SELECT id, prev_hash::text, entry_hash::text FROM audit_logs WHERE id >= $1 ORDER BY id",
	)
	.bind(start)
	.fetch_all(&pool)
	.await?;
	assert_eq!(
		rows.len(),
		4,
		"the concurrency scenario must append four rows"
	);
	let verified: (i64,) =
		sqlx::query_as("SELECT broken_rows FROM verify_audit_log_hash_chain($1)")
			.bind(start)
			.fetch_one(first.dbx().db())
			.await?;
	assert_eq!(
		verified.0, 0,
		"waiting inserts must not allocate an ID before the chain lock"
	);
	for pair in rows.windows(2) {
		assert_eq!(pair[1].0, pair[0].0 + 1, "audit IDs must be consecutive");
		assert_eq!(
			pair[1].1, pair[0].2,
			"each row must reference the preceding ID's hash"
		);
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_jsonb_comparison_preserves_native_semantics() -> Result<()> {
	let mm = init_test_mm().await?;
	let (semantics_match,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT bool_and(actual IS NOT DISTINCT FROM expected)
				FROM (VALUES
					(audit_jsonb_is_distinct(NULL::jsonb, NULL::jsonb), false),
					(audit_jsonb_is_distinct(NULL::jsonb, 'null'::jsonb), true),
					(audit_jsonb_is_distinct('null'::jsonb, 'null'::jsonb), false),
					(audit_jsonb_is_distinct('1'::jsonb, '1.0'::jsonb), false),
					(audit_jsonb_is_distinct('{"a":1,"b":2}'::jsonb, '{"b":2,"a":1}'::jsonb), false),
					(audit_jsonb_is_distinct('[1,2]'::jsonb, '[2,1]'::jsonb), true),
					(audit_jsonb_is_distinct('[1,1]'::jsonb, '[1]'::jsonb), true),
					(audit_jsonb_is_distinct('{}'::jsonb -> 'missing', 'null'::jsonb), true),
					(audit_jsonb_is_distinct('{"a":{"b":"가"}}'::jsonb, '{"a":{"b":"나"}}'::jsonb), true),
					(audit_jsonb_is_distinct('{}'::jsonb, '[]'::jsonb), true),
					(pg_catalog.jsonb_hash_extended('{}'::jsonb, 0) =
					 pg_catalog.jsonb_hash_extended('[]'::jsonb, 0), true)
				) AS checks(actual, expected)
				"#,
			),
		)
		.await?;
	assert!(semantics_match, "guard changed native JSONB semantics");

	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_hash_v2_is_unambiguous_and_range_reports_continuation() -> Result<()>
{
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	use lib_core::model::store::set_full_context_dbx;
	let mm = init_test_mm().await?;
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;

	let (v1_collides, v2_distinguishes, null_distinguishes): (bool, bool, bool) = mm
		.dbx()
		.fetch_one(sqlx::query_as(
			r#"
			SELECT concat_ws('|', 'a|b', 'c') = concat_ws('|', 'a', 'b|c'),
			       jsonb_build_array('a|b', 'c') <> jsonb_build_array('a', 'b|c'),
			       jsonb_build_array(NULL, '') <> jsonb_build_array('', NULL)
			"#,
		))
		.await?;
	assert!(
		v1_collides,
		"fixture must demonstrate the v1 delimiter ambiguity"
	);
	assert!(v2_distinguishes, "v2 must preserve field boundaries");
	assert!(null_distinguishes, "v2 must preserve null positions");

	let append = "SELECT append_audit_log('range_regression', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)";
	for _ in 0..3 {
		mm.dbx()
			.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org_id))
			.await?;
	}
	let (first_id, head_id, head_hash, v2_rows): (i64, i64, String, i64) = mm
		.dbx()
		.fetch_one(sqlx::query_as(
			"SELECT min(id), max(id), (array_agg(entry_hash ORDER BY id DESC))[1]::text, count(*) FILTER (WHERE hash_version = 2) FROM audit_logs WHERE table_name = 'range_regression'",
		))
		.await?;
	assert_eq!(v2_rows, 3);
	mm.dbx().commit_txn().await?;

	let page: (i64, i64, i64, i64, String, i64, bool, i64) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT total_rows, broken_rows, checked_from_id, snapshot_head_id, snapshot_head_hash, next_since_id, has_more, legacy_rows FROM verify_audit_log_hash_chain_range($1, $2, 1)",
			)
			.bind(first_id)
			.bind(head_id),
		)
		.await?;
	assert_eq!(
		page,
		(1, 0, first_id, head_id, head_hash, first_id + 1, true, 0)
	);

	let missing_until = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (i64,)>(
			"SELECT total_rows FROM verify_audit_log_hash_chain_range(NULL, $1, 1)",
		).bind(head_id + 1000))
		.await;
	assert!(
		missing_until.is_err(),
		"a missing snapshot head must be rejected"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_logs_reject_app_role_mutation_permissions() -> Result<()> {
	let pool = audit_admin_pool().await?;
	for statement in [
		"UPDATE audit_logs SET action = action",
		"DELETE FROM audit_logs",
		"TRUNCATE audit_logs",
		"INSERT INTO audit_logs (organization_id, table_name, record_id, action, user_id, created_at, prev_hash, entry_hash) SELECT organization_id, 'forbidden', record_id, action, user_id, created_at, prev_hash, entry_hash FROM audit_logs LIMIT 1",
	] {
		let mut tx = pool.begin().await?;
		sqlx::query("SET LOCAL ROLE e2br3_app_role")
			.execute(&mut *tx)
			.await?;
		let error = sqlx::query(statement)
			.execute(&mut *tx)
			.await
			.expect_err("app role mutation must be denied by privileges");
		match error {
			sqlx::Error::Database(database) => {
				assert_eq!(database.code().as_deref(), Some("42501"));
			}
			other => panic!("unexpected app role mutation error: {other}"),
		}
		tx.rollback().await?;
	}

	let original: (i64, String, String) = sqlx::query_as(
		"SELECT id, action, entry_hash::text FROM audit_logs ORDER BY id LIMIT 1",
	)
	.fetch_one(&pool)
	.await?;
	for statement in [
		"UPDATE audit_logs SET action = action WHERE id = $1",
		"DELETE FROM audit_logs WHERE id = $1",
		"TRUNCATE audit_logs",
	] {
		let mut tx = pool.begin().await?;
		let result = if statement.contains("$1") {
			sqlx::query(statement)
				.bind(original.0)
				.execute(&mut *tx)
				.await
		} else {
			sqlx::query(statement).execute(&mut *tx).await
		};
		let error = result.expect_err(
			"owner mutation must be rejected by the append-only trigger",
		);
		match error {
			sqlx::Error::Database(database) => {
				assert_eq!(database.code().as_deref(), Some("55000"));
			}
			other => panic!("unexpected owner mutation error: {other}"),
		}
		tx.rollback().await?;
	}
	let after: (String, String) = sqlx::query_as(
		"SELECT action, entry_hash::text FROM audit_logs WHERE id = $1",
	)
	.bind(original.0)
	.fetch_one(&pool)
	.await?;
	assert_eq!(after, (original.1, original.2));
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_range_checks_predecessor_and_detects_middle_deletion() -> Result<()> {
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	use lib_core::model::store::set_full_context_dbx;
	let mm = init_test_mm().await?;
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	let append = "SELECT append_audit_log('deletion_regression', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)";
	for _ in 0..4 {
		mm.dbx()
			.execute(sqlx::query(append).bind(uuid::Uuid::new_v4()).bind(org_id))
			.await?;
	}
	let ids: Vec<(i64,)> = mm
		.dbx()
		.fetch_all(sqlx::query_as(
			"SELECT id FROM audit_logs WHERE table_name = 'deletion_regression' ORDER BY id",
		))
		.await?;
	assert_eq!(ids.len(), 4);
	mm.dbx().commit_txn().await?;

	let pool = audit_admin_pool().await?;
	let mut tx = pool.begin().await?;
	sqlx::query(
		"ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query("ALTER TABLE audit_logs ALTER COLUMN entry_hash DROP NOT NULL")
		.execute(&mut *tx)
		.await?;
	sqlx::query("SAVEPOINT corrupt_predecessor")
		.execute(&mut *tx)
		.await?;
	sqlx::query("UPDATE audit_logs SET entry_hash = NULL WHERE id = $1")
		.bind(ids[1].0)
		.execute(&mut *tx)
		.await?;
	let corrupt_predecessor: (i64, Option<String>) = sqlx::query_as(
		"SELECT broken_rows, first_broken_reason FROM verify_audit_log_hash_chain_range($1, $2, 10)",
	)
	.bind(ids[2].0)
	.bind(ids[3].0)
	.fetch_one(&mut *tx)
	.await?;
	assert_eq!(
		corrupt_predecessor,
		(
			1,
			Some("previous entry_hash is not a 64-char hex value".to_string())
		)
	);
	sqlx::query("ROLLBACK TO SAVEPOINT corrupt_predecessor")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DELETE FROM audit_logs WHERE id = $1")
		.bind(ids[1].0)
		.execute(&mut *tx)
		.await?;
	let broken: (i64, Option<i64>) = sqlx::query_as(
		"SELECT broken_rows, first_broken_id FROM verify_audit_log_hash_chain_range($1, $2, 10)",
	)
	.bind(ids[2].0)
	.bind(ids[3].0)
			.fetch_one(&mut *tx)
		.await?;
	assert_eq!(broken, (1, Some(ids[2].0)));
	tx.rollback().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_append_failure_rolls_back_business_write() -> Result<()> {
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	let pool = audit_admin_pool().await?;
	let original: (String,) =
		sqlx::query_as("SELECT name FROM organizations WHERE id = $1")
			.bind(org_id)
			.fetch_one(&pool)
			.await?;

	let mut tx = pool.begin().await?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(user_id)
		.execute(&mut *tx)
		.await?;
	sqlx::query("SELECT set_org_context($1, $2)")
		.bind(org_id)
		.bind(ROLE_SYSTEM_ADMIN)
		.execute(&mut *tx)
		.await?;
	sqlx::query(
		"ALTER TABLE audit_logs ADD CONSTRAINT audit_logs_test_reject_append CHECK (false) NOT VALID",
	)
	.execute(&mut *tx)
	.await?;
	let error = sqlx::query(
		"UPDATE organizations SET name = name || '-must-rollback' WHERE id = $1",
	)
	.bind(org_id)
	.execute(&mut *tx)
	.await
	.expect_err("audited business write must fail when audit storage fails");
	match error {
		sqlx::Error::Database(database) => {
			assert_eq!(database.code().as_deref(), Some("P0001"));
			assert!(database.message().contains("audit_logs_test_reject_append"));
			assert!(database.message().contains("violates check constraint"));
		}
		other => panic!("unexpected audit append failure: {other}"),
	}
	tx.rollback().await?;

	let after: (String,) =
		sqlx::query_as("SELECT name FROM organizations WHERE id = $1")
			.bind(org_id)
			.fetch_one(&pool)
			.await?;
	assert_eq!(
		after, original,
		"failed audit append must roll back business data"
	);
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_writer_rejects_null_timestamp_and_corrupt_tail() -> Result<()> {
	use lib_core::ctx::{SYSTEM_ORG_ID, SYSTEM_USER_ID};
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	let pool = audit_admin_pool().await?;
	let insert = "INSERT INTO audit_logs (organization_id, table_name, record_id, action, user_id, created_at, prev_hash, entry_hash) VALUES ($1, 'writer_guard_regression', $2, 'CREATE', $3, $4, repeat('0', 64), repeat('0', 64))";

	let mut null_timestamp = pool.begin().await?;
	let error = sqlx::query(insert)
		.bind(org_id)
		.bind(uuid::Uuid::new_v4())
		.bind(user_id)
		.bind(Option::<time::OffsetDateTime>::None)
		.execute(&mut *null_timestamp)
		.await
		.expect_err("explicit null created_at must be rejected");
	match error {
		sqlx::Error::Database(database) => {
			assert_eq!(database.code().as_deref(), Some("23502"));
		}
		other => panic!("unexpected null timestamp error: {other}"),
	}
	null_timestamp.rollback().await?;

	let mut corrupt_tail = pool.begin().await?;
	sqlx::query(
		"ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&mut *corrupt_tail)
	.await?;
	sqlx::query("ALTER TABLE audit_logs ALTER COLUMN entry_hash DROP NOT NULL")
		.execute(&mut *corrupt_tail)
		.await?;
	sqlx::query("UPDATE audit_logs SET entry_hash = NULL WHERE id = (SELECT max(id) FROM audit_logs)")
		.execute(&mut *corrupt_tail)
		.await?;
	let error = sqlx::query(insert)
		.bind(org_id)
		.bind(uuid::Uuid::new_v4())
		.bind(user_id)
		.bind(time::OffsetDateTime::now_utc())
		.execute(&mut *corrupt_tail)
		.await
		.expect_err("append after a corrupt chain head must be rejected");
	match error {
		sqlx::Error::Database(database) => {
			assert_eq!(database.code().as_deref(), Some("55000"));
		}
		other => panic!("unexpected corrupt tail error: {other}"),
	}
	corrupt_tail.rollback().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_verifier_detects_every_protected_field_and_unknown_versions(
) -> Result<()> {
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	use lib_core::model::store::set_full_context_dbx;
	let mm = init_test_mm().await?;
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	let record_id = uuid::Uuid::new_v4();
	mm.dbx()
		.execute(
			sqlx::query("SELECT append_audit_log('tamper_regression', $1, $2, 'CREATE', NULL, NULL, NULL)")
				.bind(record_id)
				.bind(org_id),
		)
		.await?;
	let audit_id: (i64,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT id FROM audit_logs WHERE table_name = 'tamper_regression' AND record_id = $1",
			)
			.bind(record_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	let pool = audit_admin_pool().await?;
	let baseline: (i64,) = sqlx::query_as(
			"SELECT broken_rows FROM verify_audit_log_hash_chain_range(NULL, NULL, 10000)",
		)
		.fetch_one(&pool)
		.await?;
	assert_eq!(
		baseline.0, 0,
		"isolated database must start with an intact chain"
	);

	let mut tx = pool.begin().await?;
	sqlx::query("ALTER TABLE audit_logs DISABLE TRIGGER ALL")
		.execute(&mut *tx)
		.await?;
	// Test-only: privileged malformed JSON must reach the verifier instead of
	// being rejected first by this generated-column expression. Rollback restores it.
	sqlx::query(
		"ALTER TABLE audit_logs ALTER COLUMN parent_record_ids DROP EXPRESSION",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query("SAVEPOINT invalid_version_enforcement")
		.execute(&mut *tx)
		.await?;
	let error = sqlx::query("UPDATE audit_logs SET hash_version = 99 WHERE id = $1")
		.bind(audit_id.0)
		.execute(&mut *tx)
		.await
		.expect_err("NOT VALID version check must still reject new invalid values");
	match error {
		sqlx::Error::Database(database) => {
			assert_eq!(database.code().as_deref(), Some("23514"));
			assert_eq!(database.constraint(), Some("audit_logs_hash_version_valid"));
		}
		other => panic!("unexpected invalid hash version error: {other}"),
	}
	sqlx::query("ROLLBACK TO SAVEPOINT invalid_version_enforcement")
		.execute(&mut *tx)
		.await?;
	sqlx::query(
		"ALTER TABLE audit_logs DROP CONSTRAINT audit_logs_hash_version_valid",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query("ALTER TABLE audit_logs ALTER COLUMN hash_version DROP NOT NULL")
		.execute(&mut *tx)
		.await?;
	sqlx::query(
			"ALTER TABLE audit_logs ALTER COLUMN prev_hash DROP NOT NULL, ALTER COLUMN entry_hash DROP NOT NULL",
		)
		.execute(&mut *tx)
		.await?;

	let mutations = [
		("id", "id = id + 1000000000"),
		("hash_version", "hash_version = 99"),
		("hash_version_null", "hash_version = NULL"),
		("prev_hash", "prev_hash = repeat('f', 64)"),
		("prev_hash_null", "prev_hash = NULL"),
		(
			"organization_id",
			"organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid",
		),
		("table_name", "table_name = table_name || '|tampered'"),
		(
			"record_id",
			"record_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'::uuid",
		),
		("action", "action = 'UPDATE'"),
		(
			"user_id",
			"user_id = 'cccccccc-cccc-cccc-cccc-cccccccccccc'::uuid",
		),
		("reason_for_change", "reason_for_change = '|tampered|'"),
		("change_category", "change_category = ''"),
		(
			"e_signature_id",
			"e_signature_id = 'dddddddd-dddd-dddd-dddd-dddddddddddd'::uuid",
		),
		(
			"old_values_sql_null_to_json_null",
			"old_values = 'null'::jsonb",
		),
		(
			"new_values_sql_null_to_json_null",
			"new_values = 'null'::jsonb",
		),
		("changed_fields", "changed_fields = 'null'::jsonb"),
		("ip_address", "ip_address = '192.0.2.10'::inet"),
		("user_agent", "user_agent = ''"),
		(
			"created_at",
			"created_at = created_at + interval '1 microsecond'",
		),
		("entry_hash", "entry_hash = repeat('e', 64)"),
		("entry_hash_null", "entry_hash = NULL"),
	];
	for (field, assignment) in mutations {
		sqlx::query("SAVEPOINT audit_tamper")
			.execute(&mut *tx)
			.await?;
		let statement = format!("UPDATE audit_logs SET {assignment} WHERE id = $1");
		sqlx::query(&statement)
			.bind(audit_id.0)
			.execute(&mut *tx)
			.await?;
		let report: (i64, Option<i64>, Option<String>) = sqlx::query_as(
				"SELECT broken_rows, first_broken_id, first_broken_reason FROM verify_audit_log_hash_chain_range(NULL, NULL, 10000)",
			)
			.fetch_one(&mut *tx)
			.await?;
		assert!(report.0 > 0, "tampering with {field} was not detected");
		if field.starts_with("hash_version") {
			assert_eq!(report.2.as_deref(), Some("unknown hash version"));
		}
		sqlx::query("ROLLBACK TO SAVEPOINT audit_tamper")
			.execute(&mut *tx)
			.await?;
	}
	tx.rollback().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_migration_preserves_legacy_hashes_across_v2_appends() -> Result<()> {
	use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
	use lib_core::model::store::set_full_context_dbx;
	let mm = init_test_mm().await?;
	let user_id = uuid::Uuid::parse_str(SYSTEM_USER_ID)?;
	let org_id = uuid::Uuid::parse_str(SYSTEM_ORG_ID)?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	let legacy_record_id = uuid::Uuid::new_v4();
	mm.dbx()
		.execute(
			sqlx::query("SELECT append_audit_log('legacy_fixture', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)")
				.bind(legacy_record_id)
				.bind(org_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	// Deterministic isolated fixture: convert the current head to a valid v1 row,
	// then prove later v2 appends neither rewrite nor invalidate it.
	let pool = audit_admin_pool().await?;
	let mut tx = pool.begin().await?;
	sqlx::query(
		"ALTER TABLE audit_logs DISABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
				r#"
				UPDATE audit_logs a
				   SET hash_version = 1,
				       entry_hash = encode(digest(concat_ws('|',
				           coalesce(a.id::text, ''), coalesce(a.prev_hash::text, ''),
				           a.table_name, a.record_id::text, a.action, a.user_id::text,
				           coalesce(a.reason_for_change, ''), coalesce(a.change_category, ''),
				           coalesce(a.e_signature_id::text, ''), coalesce(a.old_values::text, 'null'),
				           coalesce(a.new_values::text, 'null'), coalesce(a.changed_fields::text, 'null'),
				           coalesce(a.ip_address::text, ''), coalesce(a.user_agent, ''),
				           to_char(a.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
				       ), 'sha256'), 'hex')
				 WHERE a.table_name = 'legacy_fixture' AND a.record_id = $1
				"#,
			)
			.bind(legacy_record_id)
			.execute(&mut *tx)
		.await?;
	sqlx::query(
		"ALTER TABLE audit_logs ENABLE TRIGGER audit_logs_reject_update_delete",
	)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	let before: (Vec<String>,) = sqlx::query_as(
			"SELECT array_agg(entry_hash::text ORDER BY id) FROM audit_logs WHERE hash_version = 1",
		)
		.fetch_one(&pool)
		.await?;
	assert!(!before.0.is_empty());

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SYSTEM_ADMIN).await?;
	mm.dbx()
		.execute(
			sqlx::query("SELECT append_audit_log('mixed_version_regression', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)")
				.bind(uuid::Uuid::new_v4())
				.bind(org_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	let after: (Vec<String>, i64, i64) = sqlx::query_as(
			"SELECT array_agg(entry_hash::text ORDER BY id) FILTER (WHERE hash_version = 1), count(*) FILTER (WHERE hash_version = 1), count(*) FILTER (WHERE hash_version = 2) FROM audit_logs",
		)
		.fetch_one(&pool)
		.await?;
	assert_eq!(after.0, before.0, "v2 append changed a legacy digest");
	let report: (i64, i64) = sqlx::query_as(
			"SELECT broken_rows, legacy_rows FROM verify_audit_log_hash_chain_range(NULL, NULL, 10000)",
		)
		.fetch_one(&pool)
		.await?;
	assert_eq!(report, (0, after.1));
	assert!(after.2 > 0, "mixed history must contain v2 rows");
	Ok(())
}

#[serial]
#[tokio::test]
async fn audit_range_reports_empty_snapshot_without_claiming_rows() -> Result<()> {
	let pool = audit_admin_pool().await?;
	let mut tx = pool.begin().await?;
	sqlx::query("ALTER TABLE audit_logs DISABLE TRIGGER ALL")
		.execute(&mut *tx)
		.await?;
	sqlx::query("TRUNCATE audit_logs").execute(&mut *tx).await?;
	let report: (i64, i64, Option<i64>, Option<String>, Option<i64>, bool, i64) = sqlx::query_as(
			"SELECT total_rows, verified_ok_rows, snapshot_head_id, snapshot_head_hash, next_since_id, has_more, legacy_rows FROM verify_audit_log_hash_chain_range(NULL, NULL, 1000)",
		)
		.fetch_one(&mut *tx)
		.await?;
	assert_eq!(report, (0, 0, None, None, None, false, 0));
	tx.rollback().await?;
	Ok(())
}
