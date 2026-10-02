use crate::common::{
	cookie_header, init_test_mm, seed_org_with_admin_and_viewer, Result,
};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::json;
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

#[serial]
#[tokio::test]
async fn test_admin_can_access_terminology_endpoints() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());

	let app = web_server::app(mm);

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/meddra?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	if res.status() != StatusCode::OK {
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"meddra status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/whodrug?q=test&limit=5")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	if res.status() != StatusCode::OK {
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"whodrug status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/countries")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	if res.status() != StatusCode::OK {
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"countries status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	let req = Request::builder()
		.method("GET")
		.uri("/api/terminology/code-lists?list_name=report_type")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	if res.status() != StatusCode::OK {
		let status = res.status();
		let body = to_bytes(res.into_body(), usize::MAX).await?;
		return Err(format!(
			"code-lists status {} body {}",
			status,
			String::from_utf8_lossy(&body)
		)
		.into());
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_viewer_can_read_terminology_but_cannot_import() -> Result<()> {
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
	assert_eq!(res.status(), StatusCode::OK);

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

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.09/approve?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.09/activate?language=en")
		.header("cookie", cookie.clone())
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

	let req = Request::builder()
		.method("POST")
		.uri("/api/terminology/releases/whodrug/2025.08/rollback?language=en")
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::FORBIDDEN);

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
	let boundary = "----safetydb-boundary-meddra";
	let (content_type, body_bytes) = make_multipart_file_body(
		boundary,
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
	assert_eq!(
		payload["data"]["loaded_rows"].as_i64().unwrap_or(0) > 0,
		true
	);

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

	let boundary = "----safetydb-boundary-meddra-invalid";
	let (content_type, body_bytes) = make_multipart_file_body(
		boundary,
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
	let boundary = "----safetydb-boundary-whodrug";
	let (content_type, body_bytes) = make_multipart_file_body(
		boundary,
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
async fn test_admin_c3_dry_run_streams_batches_and_rejects_late_invalid_data(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let version = format!("C3{}", short_tag());

	let valid = make_c3_zip_bytes(&version, 1001, None)?;
	let boundary = "----whodrug-c3-batches";
	let (content_type, body) = make_multipart_file_body(
		boundary,
		"global-c3.zip",
		"application/zip",
		&valid,
	);
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/api/terminology/import/whodrug?version={version}&language=en&dry_run=true"))
				.header("cookie", cookie.clone())
				.header("content-type", content_type)
				.body(Body::from(body))?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	let payload: serde_json::Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"]["loaded_rows"], 1001);
	assert_eq!(c3_admin_persisted_state(&mm, &version).await?.0, (0, 0, 0));

	for (suffix, package_version, bad_row, expected) in [
		(
			"duplicate",
			version.as_str(),
			Some("R000000000"),
			"duplicate WHODrug C3 Record_Id",
		),
		(
			"late-invalid",
			version.as_str(),
			Some("RID-INVALID"),
			"invalid Record_Id",
		),
		(
			"version",
			"DIFFERENT",
			None,
			"does not match package short version",
		),
	] {
		let zip = make_c3_zip_bytes(package_version, 1001, bad_row)?;
		let boundary = format!("----whodrug-c3-{suffix}");
		let (content_type, body) = make_multipart_file_body(
			&boundary,
			"global-c3.zip",
			"application/zip",
			&zip,
		);
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri(format!("/api/terminology/import/whodrug?version={version}&language=en&dry_run=true"))
					.header("cookie", cookie.clone())
					.header("content-type", content_type)
					.body(Body::from(body))?,
			)
			.await?;
		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
		let body = to_bytes(response.into_body(), usize::MAX).await?;
		assert!(String::from_utf8_lossy(&body).contains(expected));
		assert_eq!(c3_admin_persisted_state(&mm, &version).await?.0, (0, 0, 0));
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_admin_c3_stage_persists_thirteen_character_short_once() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let version = format!("C3SHORT{}", short_tag());
	assert_eq!(version.len(), 13);
	let zip = make_c3_zip_bytes(&version, 2, None)?;
	let expected_checksum = lib_core::model::terminology_import::sha256_hex(&zip);

	let (content_type, body) = make_multipart_file_body(
		"----whodrug-c3-stage",
		"global-c3.zip",
		"application/zip",
		&zip,
	);
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/api/terminology/import/whodrug?version={version}&language=en&dry_run=false"))
				.header("cookie", cookie.clone())
				.header("content-type", content_type)
				.body(Body::from(body))?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(c3_persisted_counts(&mm, &version).await?, (1, 0, 0));
	let stored = c3_admin_persisted_state(&mm, &version).await?;
	assert_eq!(stored.0, (1, 2, 1));
	assert_eq!(stored.1.as_deref(), Some("validated"));
	assert_eq!(stored.2, Some(2));
	assert_eq!(stored.3.as_deref(), Some(expected_checksum.as_str()));
	assert_eq!(stored.4, Some(true));
	assert_eq!(stored.5.as_deref(), Some("Drug 0"));
	assert_eq!(stored.6, Some(true));

	let replacement = make_c3_zip_bytes(&version, 1, None)?;
	let (content_type, body) = make_multipart_file_body(
		"----whodrug-c3-reimport",
		"global-c3.zip",
		"application/zip",
		&replacement,
	);
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/api/terminology/import/whodrug?version={version}&language=en&dry_run=false"))
				.header("cookie", cookie)
				.header("content-type", content_type)
				.body(Body::from(body))?,
		)
		.await?;
	assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	let preserved = c3_admin_persisted_state(&mm, &version).await?;
	assert_eq!(preserved.0, (1, 2, 1));
	assert_eq!(preserved.1.as_deref(), Some("validated"));
	assert_eq!(preserved.2, Some(2));
	assert_eq!(preserved.3.as_deref(), Some(expected_checksum.as_str()));
	assert_eq!(preserved.5.as_deref(), Some("Drug 0"));

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
	let boundary = "----safetydb-boundary-whodrug-invalid";
	let (content_type, body_bytes) = make_multipart_file_body(
		boundary,
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

#[serial]
#[tokio::test]
async fn test_imported_terminology_can_be_used_in_case_generation_flow() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_terminology_admin(&mm).await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let tag = short_tag();
	let stem = format!("CaseGen{tag}");
	let meddra_version = format!("CGM{tag}");
	let whodrug_version = format!("CGW{tag}");

	let meddra_zip = make_meddra_zip_bytes_with_terms(
		&format!("{stem} LLT"),
		&format!("{stem} PT"),
		&format!("{stem} HLT"),
		&format!("{stem} HLGT"),
		&format!("{stem} SOC"),
	)?;
	let (ct_m, body_m) = make_multipart_file_body(
		"----casegen-meddra",
		"meddra.zip",
		"application/zip",
		&meddra_zip,
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/meddra?version={meddra_version}&language=en&dry_run=false"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct_m)
		.body(Body::from(body_m))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	for action in ["approve", "activate"] {
		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/releases/meddra/{meddra_version}/{action}?language=en"
			))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
	}

	let who_csv = format!(
		"code,drug_name,atc_code\nW900,{stem} Drug,A01AA01\nW901,{stem} Drug 2,A01AA02\n"
	);
	let (ct_w, body_w) = make_multipart_file_body(
		"----casegen-whodrug",
		"whodrug.csv",
		"text/csv",
		who_csv.as_bytes(),
	);
	let req = Request::builder()
		.method("POST")
		.uri(format!(
			"/api/terminology/import/whodrug?version={whodrug_version}&language=en&dry_run=false"
		))
		.header("cookie", cookie.clone())
		.header("content-type", ct_w)
		.body(Body::from(body_w))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::OK);

	for action in ["approve", "activate"] {
		let req = Request::builder()
			.method("POST")
			.uri(format!(
				"/api/terminology/releases/whodrug/{whodrug_version}/{action}?language=en"
			))
			.header("cookie", cookie.clone())
			.body(Body::empty())?;
		let res = app.clone().oneshot(req).await?;
		assert_eq!(res.status(), StatusCode::OK);
	}

	let meddra_search_uri = format!("/api/terminology/meddra?q={stem}&limit=10");
	let (status, payload) =
		get_json_value(&app, &cookie, &meddra_search_uri).await?;
	assert_eq!(status, StatusCode::OK);
	let meddra_term = payload["data"]
		.as_array()
		.and_then(|arr| arr.first())
		.cloned()
		.ok_or("missing meddra term from search")?;
	let meddra_code = meddra_term["code"]
		.as_str()
		.ok_or("missing meddra code")?
		.to_string();

	let whodrug_search_uri = format!("/api/terminology/whodrug?q={stem}&limit=10");
	let (status, payload) =
		get_json_value(&app, &cookie, &whodrug_search_uri).await?;
	assert_eq!(status, StatusCode::OK);
	let whodrug_term = payload["data"]
		.as_array()
		.and_then(|arr| arr.first())
		.cloned()
		.ok_or("missing whodrug term from search")?;
	let drug_name = whodrug_term["drug_name"]
		.as_str()
		.ok_or("missing whodrug drug_name")?
		.to_string();

	let (status, payload) = post_json_value(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": format!("SR-{tag}")
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let case_id = payload["data"]["id"]
		.as_str()
		.ok_or("missing case id")?
		.to_string();

	let (status, payload) = post_json_value(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": stem
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let reaction_id = payload["data"]["id"]
		.as_str()
		.ok_or("missing reaction id")?
		.to_string();

	let (status, _) = put_json_value(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
		json!({
			"data": {
				"reaction_meddra_version": meddra_version,
				"reaction_meddra_code": meddra_code
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);

	let (status, payload) = post_json_value(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": drug_name
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let drug_id = payload["data"]["id"]
		.as_str()
		.ok_or("missing drug id")?
		.to_string();

	let (status, payload) = get_json_value(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions/{reaction_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(payload["data"]["reaction_meddra_code"], meddra_code);
	assert_eq!(payload["data"]["reaction_meddra_version"], meddra_version);

	let (status, payload) = get_json_value(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(payload["data"]["medicinal_product"], drug_name);

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

fn make_meddra_zip_bytes() -> Result<Vec<u8>> {
	make_meddra_zip_bytes_with_terms(
		"Headache LLT",
		"Headache PT",
		"Headache HLT",
		"Headache HLGT",
		"Nervous system disorders SOC",
	)
}

fn make_meddra_zip_bytes_with_terms(
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

fn make_c3_zip_bytes(
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

async fn c3_persisted_counts(
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

type C3PersistedState = (
	(i64, i64, i64),
	Option<String>,
	Option<i64>,
	Option<String>,
	Option<bool>,
	Option<String>,
	Option<bool>,
);

async fn c3_admin_persisted_state(
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

fn short_tag() -> String {
	let raw = Uuid::new_v4().simple().to_string();
	raw.chars().take(6).collect()
}

async fn post_json_value(
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

async fn put_json_value(
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

async fn get_json_value(
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
