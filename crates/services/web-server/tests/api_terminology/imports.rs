use super::support::*;

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
