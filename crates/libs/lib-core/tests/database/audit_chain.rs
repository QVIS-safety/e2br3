use super::{init_test_mm, Result};
use serial_test::serial;

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
	let verified: (i64,) =
		sqlx::query_as("SELECT broken_rows FROM verify_audit_log_hash_chain($1)")
			.bind(start)
			.fetch_one(first.dbx().db())
			.await?;
	assert_eq!(
		verified.0, 0,
		"waiting inserts must not allocate an ID before the chain lock"
	);
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
