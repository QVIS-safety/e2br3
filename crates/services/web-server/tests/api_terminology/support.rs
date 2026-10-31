pub(super) use crate::common::{
	cookie_header, init_test_mm, seed_org_with_admin_and_viewer,
	seed_org_with_all_roles, Result,
};
pub(super) use axum::body::{to_bytes, Body};
pub(super) use axum::http::{Request, StatusCode};
pub(super) use lib_auth::token::generate_web_token;
pub(super) use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
pub(super) use lib_core::model::store::{set_org_context, set_user_context};
pub(super) use serde_json::json;
pub(super) use serial_test::serial;
pub(super) use std::io::Write;
pub(super) use tower::ServiceExt;
pub(super) use uuid::Uuid;
pub(super) use zip::write::SimpleFileOptions;
pub(super) use zip::CompressionMethod;
pub(super) use zip::ZipWriter;

pub(super) async fn seed_terminology_admin(
	mm: &lib_core::model::ModelManager,
) -> Result<crate::common::SeedOrgUsers> {
	seed_org_with_admin_and_viewer(mm, "adminpwd", "viewpwd").await
}

pub(super) async fn seed_active_terminology_rows(
	mm: &lib_core::model::ModelManager,
) -> Result<()> {
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;

	sqlx::query(
		"INSERT INTO meddra_terms (code, term, level, version, language, active)
		 VALUES
		 ('90000001', 'Headache test term', 'LLT', '28.1', 'en', true),
		 ('10019211', 'Headache', 'LLT', '28.1', 'en', true)
		 ON CONFLICT (code, version, language)
		 DO UPDATE SET term = EXCLUDED.term, level = EXCLUDED.level, active = true",
	)
	.execute(&mut *tx)
	.await?;

	sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='meddra' AND language='en' AND status='active' AND version <> '28.1'").execute(&mut *tx).await?;

	sqlx::query(
		"INSERT INTO terminology_releases
		 (dictionary, version, language, status, loaded_rows, activated_at)
		 VALUES ('meddra', '28.1', 'en', 'active', 2, NOW())
		 ON CONFLICT (dictionary, version, language)
		 DO UPDATE SET status = 'active', loaded_rows = 2, activated_at = NOW()",
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

	sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='whodrug' AND language='en' AND status='active' AND version <> '2026.03'").execute(&mut *tx).await?;
	sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows) VALUES ('whodrug','2026.03','en','active',1) ON CONFLICT(dictionary,version,language) DO UPDATE SET status='active'").execute(&mut *tx).await?;

	sqlx::query(
		"INSERT INTO whodrug_products (code, drug_name, atc_code, version, language, active)
		 VALUES ('W90000001', 'Example Drug test term', 'A01AA01', '2026.03', 'en', true)
		 ON CONFLICT (code, version, language)
		 DO UPDATE SET drug_name = EXCLUDED.drug_name, atc_code = EXCLUDED.atc_code, active = true",
	)
	.execute(&mut *tx)
	.await?;

	sqlx::query(
		"INSERT INTO mfds_product_substances
		 (item_seq, substance_code, substance_name_kr, quantity, unit, version, active)
		 VALUES ('9000000001', 'M900001', '활성성분', '10', 'mg', '2026-01', true)
		 ON CONFLICT (item_seq, substance_code, material_sequence, total_amount_sequence, version)
		 DO UPDATE SET substance_name_kr=EXCLUDED.substance_name_kr,
		 quantity=EXCLUDED.quantity, unit=EXCLUDED.unit, active=true",
	)
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;
	Ok(())
}

pub(super) async fn assert_mfds_search_count(
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

pub(super) fn make_meddra_zip_bytes() -> Result<Vec<u8>> {
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

pub(super) fn make_single_file_zip_bytes(
	filename: &str,
	content: &[u8],
) -> Result<Vec<u8>> {
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

pub(super) fn short_tag() -> String {
	let raw = Uuid::new_v4().simple().to_string();
	raw.chars().take(6).collect()
}

pub(super) fn make_multipart_file_body(
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

pub(super) type C3PersistedState = (
	(i64, i64, i64),
	Option<String>,
	Option<i64>,
	Option<String>,
	Option<bool>,
	Option<String>,
	Option<bool>,
);

pub(super) fn make_meddra_zip_bytes_with_terms(
	llt_term: &str,
	pt_term: &str,
	hlt_term: &str,
	hlgt_term: &str,
	soc_term: &str,
) -> Result<Vec<u8>> {
	let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
	{
		let mut zip = ZipWriter::new(&mut cursor);
		let options = SimpleFileOptions::default()
			.compression_method(CompressionMethod::Deflated);
		zip.start_file("llt.asc", options)?;
		zip.write_all(format!("11000001${llt_term}$11000002$\n").as_bytes())?;
		zip.start_file("mdhier.asc", options)?;
		zip.write_all(
			format!(
				"12000001$13000001$14000001$15000001${pt_term}${hlt_term}${hlgt_term}${soc_term}$\n"
			)
			.as_bytes(),
		)?;
		zip.finish()?;
	}
	Ok(cursor.into_inner())
}

pub(super) fn make_c3_zip_bytes(
	version: &str,
	rows: usize,
	last_record_id: Option<&str>,
) -> Result<Vec<u8>> {
	let mut mp = String::new();
	for index in 0..rows {
		let record_id = if index + 1 == rows {
			last_record_id
				.map(str::to_string)
				.unwrap_or_else(|| format!("R{index:09}"))
		} else {
			format!("R{index:09}")
		};
		mp.push_str(&format!(
			"{record_id},,000001,01,001,0000000001,0000000001,Y,Drug {index},,,,,N/A,,0,001,N/A,,001,19851231,20250930\n"
		));
	}
	let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
	{
		let mut zip = ZipWriter::new(&mut cursor);
		let options = SimpleFileOptions::default()
			.compression_method(CompressionMethod::Deflated);
		zip.start_file("Version.csv", options)?;
		zip.write_all(format!("Global C3,{version}\n").as_bytes())?;
		zip.start_file("C3/MP.csv", options)?;
		zip.write_all(mp.as_bytes())?;
		zip.start_file("C3/SUN.csv", options)?;
		zip.write_all(b"1,1234567890,Substance,0,0,0\n")?;
		zip.finish()?;
	}
	Ok(cursor.into_inner())
}

pub(super) async fn c3_persisted_counts(
	mm: &lib_core::model::ModelManager,
	version: &str,
) -> Result<(i64, i64, i64)> {
	Ok(mm
		.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT
				   (SELECT COUNT(*) FROM terminology_releases
				     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
				   (SELECT COUNT(*) FROM whodrug_products
				     WHERE version=$1 AND language='en'),
				   (SELECT COUNT(*) FROM controlled_terminology_terms
				     WHERE dictionary='whodrug' AND version=$1 AND language='en')",
			)
			.bind(version),
		)
		.await?)
}

pub(super) async fn c3_admin_persisted_state(
	mm: &lib_core::model::ModelManager,
	version: &str,
) -> Result<C3PersistedState> {
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;
	let state: (
		i64,
		i64,
		i64,
		Option<String>,
		Option<i64>,
		Option<String>,
		Option<bool>,
		Option<String>,
		Option<bool>,
	) = sqlx::query_as(
		"SELECT
		   (SELECT COUNT(*) FROM terminology_releases
		     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
		   (SELECT COUNT(*) FROM whodrug_products
		     WHERE version=$1 AND language='en'),
		   (SELECT COUNT(*) FROM controlled_terminology_terms
		     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
		   (SELECT status FROM terminology_releases
		     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
		   (SELECT loaded_rows FROM terminology_releases
		     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
		   (SELECT source_checksum FROM terminology_releases
		     WHERE dictionary='whodrug' AND version=$1 AND language='en'),
		   (SELECT bool_and(NOT active) FROM whodrug_products
		     WHERE version=$1 AND language='en'),
		   (SELECT MIN(drug_name) FROM whodrug_products
		     WHERE version=$1 AND language='en'),
		   (SELECT bool_and(NOT active) FROM controlled_terminology_terms
		     WHERE dictionary='whodrug' AND version=$1 AND language='en')",
	)
	.bind(version)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok((
		(state.0, state.1, state.2),
		state.3,
		state.4,
		state.5,
		state.6,
		state.7,
		state.8,
	))
}

pub(super) async fn post_json_value(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: serde_json::Value,
) -> Result<(StatusCode, serde_json::Value)> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	Ok((status, payload))
}

pub(super) async fn put_json_value(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: serde_json::Value,
) -> Result<(StatusCode, serde_json::Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	Ok((status, payload))
}

pub(super) async fn get_json_value(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, serde_json::Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	Ok((status, payload))
}
