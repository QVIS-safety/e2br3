use crate::common::{
	cookie_header, init_test_mm, seed_org_with_admin_and_viewer, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use lib_core::model::store::{set_org_context, set_user_context};
use serial_test::serial;
use std::io::Write;
use tower::ServiceExt;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

async fn seed_terminology_admin(
	mm: &lib_core::model::ModelManager,
) -> Result<crate::common::SeedOrgUsers> {
	seed_org_with_admin_and_viewer(mm, "adminpwd", "viewpwd").await
}

async fn seed_active_terminology_rows(
	mm: &lib_core::model::ModelManager,
) -> Result<()> {
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;

	sqlx::query(
		"INSERT INTO meddra_terms (code, term, level, version, language, active)
		 VALUES ('90000001', 'Headache test term', 'LLT', '28.1', 'en', true)
		 ON CONFLICT (code, version, language)
		 DO UPDATE SET term = EXCLUDED.term, level = EXCLUDED.level, active = true",
	)
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO mfds_products
		 (item_seq, product_name_kr, product_name_en, manufacturer_name_kr, version, active)
		 VALUES
		 ('9000000001', '활성제품', 'Active MFDS Product', '활성제약', '2026-01', true),
		 ('9000000002', '비활성제품', 'Inactive MFDS Product', '비활성제약', '2026-02', false)
		 ON CONFLICT (item_seq, version)
		 DO UPDATE SET
		   product_name_kr = EXCLUDED.product_name_kr,
		   product_name_en = EXCLUDED.product_name_en,
		   manufacturer_name_kr = EXCLUDED.manufacturer_name_kr,
		   active = EXCLUDED.active",
	)
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO whodrug_products (code, drug_name, atc_code, version, language, active)
		 VALUES ('W90000001', 'Example Drug test term', 'A01AA01', '2026.03', 'en', true)
		 ON CONFLICT (code, version, language)
		 DO UPDATE SET drug_name = EXCLUDED.drug_name, atc_code = EXCLUDED.atc_code, active = true",
	)
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_access_terminology_endpoints() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for uri in [
		"/api/terminology/meddra?q=test&limit=5",
		"/api/terminology/whodrug?q=test&limit=5",
		"/api/terminology/mfds-products?q=9000000001&limit=5",
		"/api/terminology/countries",
		"/api/terminology/code-lists?list_name=report_type",
	] {
		let req = Request::builder()
			.method("GET")
			.uri(uri)
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		if res.status() != StatusCode::OK {
			let status = res.status();
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			return Err(format!(
				"terminology GET {uri} status {status} body {}",
				String::from_utf8_lossy(&body)
			)
			.into());
		}
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_searches_only_active_mfds_products_by_code_or_name(
) -> Result<()> {
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for (query, expected_count) in [
		("9000000001", 1),
		("Active%20MFDS", 1),
		("Inactive%20MFDS", 0),
	] {
		let req = Request::builder()
			.method("GET")
			.uri(format!("/api/terminology/mfds-products?q={query}&limit=5"))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		let payload: serde_json::Value = serde_json::from_slice(&body)?;
		assert_eq!(
			payload["data"].as_array().map(Vec::len),
			Some(expected_count)
		);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_mfds_product_activation_and_rollback_change_visible_search_release(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let suffix = Uuid::new_v4().simple().to_string();
	let version_v1 = format!("api-v1-{}", &suffix[..8]);
	let version_v2 = format!("api-v2-{}", &suffix[..8]);
	let item_v1 = format!("A{}", &suffix[..9]);
	let item_v2 = format!("B{}", &suffix[..9]);

	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO mfds_products
		 (item_seq, product_name_kr, version, active)
		 VALUES ($1, 'API V1 Product', $2, true), ($3, 'API V2 Product', $4, false)",
	)
	.bind(&item_v1)
	.bind(&version_v1)
	.bind(&item_v2)
	.bind(&version_v2)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO terminology_releases
		 (dictionary, version, language, status, loaded_rows, activated_at)
		 VALUES
		 ('mfds_product', $1, 'ko', 'active', 1, NOW()),
		 ('mfds_product', $2, 'ko', 'validated', 1, NULL)",
	)
	.bind(&version_v1)
	.bind(&version_v2)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let app = web_server::app(mm.clone());
	for uri in [
		format!(
			"/api/terminology/releases/mfds_product/{version_v2}/approve?language=ko"
		),
		format!(
			"/api/terminology/releases/mfds_product/{version_v2}/activate?language=ko"
		),
	] {
		let req = Request::builder()
			.method("POST")
			.uri(uri)
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
	}
	assert_mfds_search_count(&app, &cookie, &item_v2, 1).await?;
	assert_mfds_search_count(&app, &cookie, &item_v1, 0).await?;

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/mfds_product/{version_v1}/rollback?language=ko"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	assert_mfds_search_count(&app, &cookie, &item_v1, 1).await?;
	assert_mfds_search_count(&app, &cookie, &item_v2, 0).await?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query("DELETE FROM mfds_products WHERE version IN ($1, $2)")
		.bind(&version_v1)
		.bind(&version_v2)
		.execute(&mut *tx)
		.await?;
	sqlx::query(
		"DELETE FROM terminology_releases
		 WHERE dictionary = 'mfds_product' AND version IN ($1, $2) AND language = 'ko'",
	)
	.bind(&version_v1)
	.bind(&version_v2)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	Ok(())
}

async fn assert_mfds_search_count(
	app: &axum::Router,
	cookie: &str,
	query: &str,
	expected: usize,
) -> Result<()> {
	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/mfds-products?q={query}&limit=5"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"].as_array().map(Vec::len), Some(expected));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_sponsor_admin_can_search_active_meddra_and_whodrug_terms() -> Result<()>
{
	let mm = init_test_mm().await?;
	seed_active_terminology_rows(&mm).await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=Headache%20test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["term"], "Headache test term");

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/whodrug?q=Example%20Drug&limit=5")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"][0]["drug_name"], "Example Drug test term");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_access_terminology_endpoints() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/mfds-products?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en")
		.header("cookie", cookie)
		.header("content-type", "multipart/form-data; boundary=----boundary")
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_cannot_approve_activate_or_rollback_release() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	for uri in [
		"/api/terminology/releases/whodrug/2025.09/approve?language=en",
		"/api/terminology/releases/whodrug/2025.09/activate?language=en",
		"/api/terminology/releases/whodrug/2025.08/rollback?language=en",
	] {
		let req = Request::builder()
			.method("POST")
			.uri(uri)
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::FORBIDDEN);
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_dry_run_meddra_import() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let zip_bytes = make_meddra_zip_bytes()?;
	let (content_type, body_bytes) = make_multipart_file_body(
		"----safetydb-boundary-meddra",
		"meddra.zip",
		"application/zip",
		&zip_bytes,
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en&dry_run=true")
		.header("cookie", cookie)
		.header("content-type", content_type)
		.body(Body::from(body_bytes))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["dictionary"], "meddra");
	assert_eq!(payload["data"]["dry_run"], true);
	assert!(payload["data"]["loaded_rows"].as_i64().unwrap_or(0) > 0);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_dry_run_meddra_import_rejects_invalid_zip() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let (content_type, body_bytes) = make_multipart_file_body(
		"----safetydb-boundary-meddra-invalid",
		"meddra.zip",
		"application/zip",
		b"not-a-real-zip",
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/meddra?version=27.1&language=en&dry_run=true")
		.header("cookie", cookie)
		.header("content-type", content_type)
		.body(Body::from(body_bytes))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_dry_run_whodrug_import() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let csv = "code,drug_name,atc_code\nW001,Example Drug,A01AA01\n";
	let (content_type, body_bytes) = make_multipart_file_body(
		"----safetydb-boundary-whodrug",
		"whodrug.csv",
		"text/csv",
		csv.as_bytes(),
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/whodrug?version=2025.09&language=en&dry_run=true")
		.header("cookie", cookie)
		.header("content-type", content_type)
		.body(Body::from(body_bytes))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["dictionary"], "whodrug");
	assert_eq!(payload["data"]["dry_run"], true);
	assert_eq!(payload["data"]["loaded_rows"], 1);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_list_terminology_releases() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/releases?dictionary=whodrug&language=en")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert!(payload["data"].is_array());
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_dry_run_whodrug_import_rejects_missing_columns() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let csv = "foo,bar\nx,y\n";
	let (content_type, body_bytes) = make_multipart_file_body(
		"----safetydb-boundary-whodrug-invalid",
		"whodrug.csv",
		"text/csv",
		csv.as_bytes(),
	);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/whodrug?version=2025.09&language=en&dry_run=true")
		.header("cookie", cookie)
		.header("content-type", content_type)
		.body(Body::from(body_bytes))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_release_actions_validate_dictionary_and_target() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/notadict/1.0/approve?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2099.01/activate?language=en")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::BAD_REQUEST);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_whodrug_parser_accepts_zipped_delimited_and_alternate_headers(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let tag = short_tag();

	let tsv = "drug_code\tmedicinal product name\tatc1\nW101\tZip TSV Drug\tA01AA01\nW102\tZip TSV Drug 2\tA01AA02\n";
	let zip_tsv = make_single_file_zip_bytes("WHODRUG.TSV", tsv.as_bytes())?;
	let (ct_tsv, body_tsv) = make_multipart_file_body(
		"----whodrug-zip-tsv",
		"whodrug.tsv.zip",
		"application/zip",
		&zip_tsv,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/whodrug?version=WZ{tag}1&language=en&dry_run=true"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct_tsv)
		.body(Body::from(body_tsv))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["loaded_rows"], 2);

	let piped = "mpid|product_name|atc\nP201|Pipe Drug 1|B01AA01\nP202|Pipe Drug 2|B01AA02\n";
	let zip_pipe = make_single_file_zip_bytes("global.txt", piped.as_bytes())?;
	let (ct_pipe, body_pipe) = make_multipart_file_body(
		"----whodrug-zip-pipe",
		"whodrug.txt.zip",
		"application/zip",
		&zip_pipe,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/whodrug?version=WZ{tag}2&language=en&dry_run=true"
		))
		.header("cookie", cookie)
		.header("content-type", ct_pipe)
		.body(Body::from(body_pipe))?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["loaded_rows"], 2);
	Ok(())
}

fn make_meddra_zip_bytes() -> Result<Vec<u8>> {
	let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
	{
		let mut zip = ZipWriter::new(&mut cursor);
		let options = SimpleFileOptions::default()
			.compression_method(CompressionMethod::Deflated);
		zip.start_file("llt.asc", options)?;
		zip.write_all(b"11000001$Headache LLT$11000002$\n")?;
		zip.start_file("mdhier.asc", options)?;
		zip.write_all(b"12000001$13000001$14000001$15000001$Headache PT$Headache HLT$Headache HLGT$Nervous system disorders SOC$\n")?;
		zip.finish()?;
	}
	Ok(cursor.into_inner())
}

fn make_single_file_zip_bytes(filename: &str, content: &[u8]) -> Result<Vec<u8>> {
	let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
	{
		let mut zip = ZipWriter::new(&mut cursor);
		let options = SimpleFileOptions::default()
			.compression_method(CompressionMethod::Deflated);
		zip.start_file(filename, options)?;
		zip.write_all(content)?;
		zip.finish()?;
	}
	Ok(cursor.into_inner())
}

fn short_tag() -> String {
	let raw = Uuid::new_v4().simple().to_string();
	raw.chars().take(6).collect()
}

fn make_multipart_file_body(
	boundary: &str,
	filename: &str,
	content_type: &str,
	content: &[u8],
) -> (String, Vec<u8>) {
	let mut body = Vec::<u8>::new();
	body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
	body.extend_from_slice(
		format!(
			"Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n"
		)
		.as_bytes(),
	);
	body.extend_from_slice(
		format!("Content-Type: {content_type}\r\n\r\n").as_bytes(),
	);
	body.extend_from_slice(content);
	body.extend_from_slice(b"\r\n");
	body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
	(format!("multipart/form-data; boundary={boundary}"), body)
}
