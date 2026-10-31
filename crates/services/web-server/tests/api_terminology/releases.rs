use super::support::*;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;

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
	let revision_v1 = CaseValidationSummaryBmc::validation_revision(
		&mm,
		validator::VALIDATOR_REVISION,
	)
	.await?;
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
	let revision_v2 = CaseValidationSummaryBmc::validation_revision(
		&mm,
		validator::VALIDATOR_REVISION,
	)
	.await?;
	assert_ne!(
		revision_v1, revision_v2,
		"activation must invalidate validation summaries"
	);

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
	let rollback_revision = CaseValidationSummaryBmc::validation_revision(
		&mm,
		validator::VALIDATOR_REVISION,
	)
	.await?;
	assert_ne!(rollback_revision, revision_v2);
	assert_ne!(
		rollback_revision, revision_v1,
		"rollback must not revive a pre-transition cache"
	);

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
async fn test_admin_can_meddra_approve_activate_and_rollback_release() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let tag = short_tag();
	let version_v1 = format!("M{}1", tag);
	let version_v2 = format!("M{}2", tag);
	let stem = format!("Lifecycle{tag}");

	let zip_v1 = make_meddra_zip_bytes_with_terms(
		&format!("{stem} V1 LLT"),
		&format!("{stem} V1 PT"),
		&format!("{stem} V1 HLT"),
		&format!("{stem} V1 HLGT"),
		&format!("{stem} V1 SOC"),
	)?;
	let (ct1, body1) = make_multipart_file_body(
		"----meddra-v1",
		"meddra_v1.zip",
		"application/zip",
		&zip_v1,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/meddra?version={version_v1}&language=en&dry_run=false"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct1)
		.body(Body::from(body1))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{version_v1}/approve?language=en"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{version_v1}/activate?language=en"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/meddra?q={stem}&limit=50"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let terms = payload["data"]
		.as_array()
		.cloned()
		.unwrap_or_default()
		.into_iter()
		.filter_map(|v| v["term"].as_str().map(|s| s.to_string()))
		.collect::<Vec<_>>();
	assert!(terms.iter().any(|t| t.contains("V1")));

	let zip_v2 = make_meddra_zip_bytes_with_terms(
		&format!("{stem} V2 LLT"),
		&format!("{stem} V2 PT"),
		&format!("{stem} V2 HLT"),
		&format!("{stem} V2 HLGT"),
		&format!("{stem} V2 SOC"),
	)?;
	let (ct2, body2) = make_multipart_file_body(
		"----meddra-v2",
		"meddra_v2.zip",
		"application/zip",
		&zip_v2,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/meddra?version={version_v2}&language=en&dry_run=false"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct2)
		.body(Body::from(body2))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{version_v2}/approve?language=en"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{version_v2}/activate?language=en"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/meddra?q={stem}&limit=50"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let terms = payload["data"]
		.as_array()
		.cloned()
		.unwrap_or_default()
		.into_iter()
		.filter_map(|v| v["term"].as_str().map(|s| s.to_string()))
		.collect::<Vec<_>>();
	assert!(terms.iter().any(|t| t.contains("V2")));
	assert!(!terms.iter().any(|t| t.contains("V1")));

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{version_v1}/rollback?language=en"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["status"], "active");
	assert_eq!(payload["data"]["version"], version_v1);
	assert_eq!(payload["data"]["rollback_from_version"], version_v2);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/meddra?q={stem}&limit=50"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let terms = payload["data"]
		.as_array()
		.cloned()
		.unwrap_or_default()
		.into_iter()
		.filter_map(|v| v["term"].as_str().map(|s| s.to_string()))
		.collect::<Vec<_>>();
	assert!(terms.iter().any(|t| t.contains("V1")));
	assert!(!terms.iter().any(|t| t.contains("V2")));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_reimport_same_version_is_idempotent_for_meddra_and_whodrug(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let tag = short_tag();
	let meddra_version = format!("MI{tag}");
	let whodrug_version = format!("WI{tag}");

	let zip_a = make_meddra_zip_bytes_with_terms(
		&format!("Idem{tag} A LLT"),
		&format!("Idem{tag} A PT"),
		&format!("Idem{tag} A HLT"),
		&format!("Idem{tag} A HLGT"),
		&format!("Idem{tag} A SOC"),
	)?;
	let zip_b = make_meddra_zip_bytes_with_terms(
		&format!("Idem{tag} B LLT"),
		&format!("Idem{tag} B PT"),
		&format!("Idem{tag} B HLT"),
		&format!("Idem{tag} B HLGT"),
		&format!("Idem{tag} B SOC"),
	)?;

	for (boundary, filename, bytes) in [
		("----idem-m-a", "m_a.zip", zip_a.as_slice()),
		("----idem-m-b", "m_b.zip", zip_b.as_slice()),
	] {
		let (ct, body) =
			make_multipart_file_body(boundary, filename, "application/zip", bytes);
		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/import/meddra?version={meddra_version}&language=en&dry_run=false"
			))
			.header("cookie", cookie.clone())
			.header("content-type", ct)
			.body(Body::from(body))?;
		let res = app.clone().oneshot(req).await?;
		if res.status() != StatusCode::OK {
			let status = res.status();
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			return Err(format!(
				"meddra import {meddra_version}/en status {status} body {}",
				String::from_utf8_lossy(&body)
			)
			.into());
		}
	}

	for (boundary, label) in [("----idem-w-a", "A"), ("----idem-w-b", "B")] {
		let csv = format!(
			"drug_code,medicinal product name,atc1\nW001,IdemDrug {label},A01AA01\n"
		);
		let (ct, body) =
			make_multipart_file_body(boundary, "w.csv", "text/csv", csv.as_bytes());
		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/import/whodrug?version={whodrug_version}&language=en&dry_run=false"
			))
			.header("cookie", cookie.clone())
			.header("content-type", ct)
			.body(Body::from(body))?;
		let res = app.clone().oneshot(req).await?;
		if res.status() != StatusCode::OK {
			let status = res.status();
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			return Err(format!(
				"whodrug import {whodrug_version}/en status {status} body {}",
				String::from_utf8_lossy(&body)
			)
			.into());
		}
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/releases?dictionary=meddra&language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let m_cnt = payload["data"]
		.as_array()
		.cloned()
		.unwrap_or_default()
		.into_iter()
		.filter(|v| v["version"] == meddra_version && v["language"] == "en")
		.count();
	assert_eq!(m_cnt, 1);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/releases?dictionary=whodrug&language=en")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let w_cnt = payload["data"]
		.as_array()
		.cloned()
		.unwrap_or_default()
		.into_iter()
		.filter(|v| v["version"] == whodrug_version && v["language"] == "en")
		.count();
	assert_eq!(w_cnt, 1);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_language_specific_activation_and_search_switching() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let tag = short_tag();
	let stem = format!("LangSwitch{tag}");
	let en_v1 = format!("E1{tag}");
	let en_v2 = format!("E2{tag}");
	let ko_v1 = format!("K1{tag}");
	let en_lang = "qa";
	let ko_lang = "qb";

	for (version, language, en_or_ko, boundary) in [
		(en_v1.clone(), en_lang, "EN1", "----lang-en1"),
		(ko_v1.clone(), ko_lang, "KO1", "----lang-ko1"),
	] {
		let zip = make_meddra_zip_bytes_with_terms(
			&format!("{stem} {en_or_ko} LLT"),
			&format!("{stem} {en_or_ko} PT"),
			&format!("{stem} {en_or_ko} HLT"),
			&format!("{stem} {en_or_ko} HLGT"),
			&format!("{stem} {en_or_ko} SOC"),
		)?;
		let (ct, body) = make_multipart_file_body(
			boundary,
			"meddra.zip",
			"application/zip",
			&zip,
		);
		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/import/meddra?version={version}&language={language}&dry_run=false"
			))
			.header("cookie", cookie.clone())
			.header("content-type", ct)
			.body(Body::from(body))?;
		let res = app.clone().oneshot(req).await?;
		if res.status() != StatusCode::OK {
			let status = res.status();
			let body = to_bytes(res.into_body(), usize::MAX).await?;
			return Err(format!(
				"meddra import {version}/{language} status {status} body {}",
				String::from_utf8_lossy(&body)
			)
			.into());
		}

		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/releases/meddra/{version}/approve?language={language}"
			))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);

		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/releases/meddra/{version}/activate?language={language}"
			))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
	}

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/meddra?q={stem}&limit=100"))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let rows = payload["data"].as_array().cloned().unwrap_or_default();
	assert!(rows.iter().any(|r| {
		r["language"] == en_lang
			&& r["version"] == en_v1
			&& r["term"].as_str().unwrap_or("").contains("EN1")
	}));
	assert!(rows.iter().any(|r| {
		r["language"] == ko_lang
			&& r["version"] == ko_v1
			&& r["term"].as_str().unwrap_or("").contains("KO1")
	}));

	let req = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/terminology/meddra?q={stem}&limit=100&language={ko_lang}"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let rows = payload["data"].as_array().cloned().unwrap_or_default();
	assert!(rows.iter().any(|r| {
		r["language"] == ko_lang
			&& r["version"] == ko_v1
			&& r["term"].as_str().unwrap_or("").contains("KO1")
	}));
	assert!(
		!rows.iter().any(|r| r["language"] == en_lang),
		"language={ko_lang} search must not return English MedDRA rows: {rows:?}"
	);

	let zip_en_v2 = make_meddra_zip_bytes_with_terms(
		&format!("{stem} EN2 LLT"),
		&format!("{stem} EN2 PT"),
		&format!("{stem} EN2 HLT"),
		&format!("{stem} EN2 HLGT"),
		&format!("{stem} EN2 SOC"),
	)?;
	let (ct, body) = make_multipart_file_body(
		"----lang-en2",
		"meddra_en2.zip",
		"application/zip",
		&zip_en_v2,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/meddra?version={en_v2}&language={en_lang}&dry_run=false"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct)
		.body(Body::from(body))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{en_v2}/approve?language={en_lang}"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/releases/meddra/{en_v2}/activate?language={en_lang}"
		))
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("GET")
		.uri(format!("/api/terminology/meddra?q={stem}&limit=100"))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	let rows = payload["data"].as_array().cloned().unwrap_or_default();
	assert!(rows.iter().any(|r| {
		r["language"] == en_lang
			&& r["version"] == en_v2
			&& r["term"].as_str().unwrap_or("").contains("EN2")
	}));
	assert!(!rows.iter().any(|r| {
		r["language"] == en_lang
			&& r["version"] == en_v1
			&& r["term"].as_str().unwrap_or("").contains("EN1")
	}));
	assert!(rows.iter().any(|r| {
		r["language"] == ko_lang
			&& r["version"] == ko_v1
			&& r["term"].as_str().unwrap_or("").contains("KO1")
	}));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_can_approve_activate_and_rollback_release() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let csv_v1 = "code,drug_name,atc_code\nW001,Example Drug v1,A01AA01\n";
	let csv_v2 = "code,drug_name,atc_code\nW001,Example Drug v2,A01AA01\n";

	let (ct1, body1) = make_multipart_file_body(
		"----whodrug-v1",
		"whodrug_v1.csv",
		"text/csv",
		csv_v1.as_bytes(),
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/whodrug?version=2025.09&language=en&dry_run=false")
		.header("cookie", cookie.clone())
		.header("content-type", ct1)
		.body(Body::from(body1))?;
	let res = app.clone().oneshot(req).await?;
	if res.status() != StatusCode::OK {
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"stage v1 status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.09/approve?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.09/activate?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let (ct2, body2) = make_multipart_file_body(
		"----whodrug-v2",
		"whodrug_v2.csv",
		"text/csv",
		csv_v2.as_bytes(),
	);
	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/import/whodrug?version=2025.10&language=en&dry_run=false")
		.header("cookie", cookie.clone())
		.header("content-type", ct2)
		.body(Body::from(body2))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.10/approve?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.10/activate?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.09/rollback?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	let body = to_bytes(res.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["status"], "active");
	assert_eq!(payload["data"]["version"], "2025.09");
	assert_eq!(payload["data"]["rollback_from_version"], "2025.10");

	Ok(())
}
