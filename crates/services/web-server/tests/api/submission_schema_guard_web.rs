use crate::common::{init_test_mm, Result};

fn ci_enforced() -> bool {
	match std::env::var("CI") {
		Ok(v) => {
			let v = v.trim().to_ascii_lowercase();
			v == "1" || v == "true" || v == "yes"
		}
		Err(_) => false,
	}
}

#[tokio::test]
async fn test_submission_dispatch_state_audit_schema_guard() -> Result<()> {
	let mm = init_test_mm().await?;

	let table_exists = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (bool,)>(
			"SELECT to_regclass('public.submission_dispatch_state') IS NOT NULL",
		))
		.await?
		.0;
	if !table_exists {
		return Ok(());
	}

	let audit_trigger_fn = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Option<String>,)>(
			"SELECT p.proname::text
			 FROM pg_trigger t
			 JOIN pg_class c ON c.oid = t.tgrelid
			 JOIN pg_namespace n ON n.oid = c.relnamespace
			 JOIN pg_proc p ON p.oid = t.tgfoid
			 WHERE n.nspname = 'public'
			   AND c.relname = 'submission_dispatch_state'
			   AND t.tgname = 'audit_submission_dispatch_state'
			   AND NOT t.tgisinternal
			 LIMIT 1",
		))
		.await?
		.0;
	if audit_trigger_fn.is_none() {
		return Ok(());
	}

	let has_id_column = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (bool,)>(
			"SELECT EXISTS (
				SELECT 1
				FROM information_schema.columns
				WHERE table_schema = 'public'
				  AND table_name = 'submission_dispatch_state'
				  AND column_name = 'id'
			)",
		))
		.await?
		.0;

	let has_incompatible_trigger =
		matches!(audit_trigger_fn.as_deref(), Some("audit_trigger_function"));
	if has_id_column || !has_incompatible_trigger {
		return Ok(());
	}

	let msg = "submission_dispatch_state uses audit_trigger_function but has no id column; switch trigger to audit_trigger_function_with_submission_id or add id column";
	if ci_enforced() {
		panic!("{msg}");
	}
	eprintln!("schema guard warning (non-CI): {msg}");
	Ok(())
}
