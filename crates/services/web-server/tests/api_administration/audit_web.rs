use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_audit_list_requires_permission() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;

	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&admin_token.to_string()))
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri("/api/audit-logs")
		.header("cookie", cookie_header(&viewer_token.to_string()))
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_audit_chain_preserves_id_order_with_waiting_writer() -> Result<()> {
	use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
	use lib_core::model::store::set_full_context_dbx;
	let first = init_test_mm().await?;
	let seed = seed_org_with_users(&first, "adminpwd", "viewpwd").await?;
	let second = first.clone();
	for mm in [&first, &second] {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
	}
	let append = "SELECT append_audit_log('concurrency_regression', $1, $2, 'CREATE', NULL, '{}'::jsonb, NULL)";
	first
		.dbx()
		.execute(
			sqlx::query(append)
				.bind(uuid::Uuid::new_v4())
				.bind(seed.org_id),
		)
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
	let org = seed.org_id;
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
		.execute(
			sqlx::query(append)
				.bind(uuid::Uuid::new_v4())
				.bind(seed.org_id),
		)
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
