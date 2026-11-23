use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json, repeatable_row_id,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};

fn direct(page: &str, rows: Value) -> Value {
	json!({"pageId":page,"request":{"authorities":["ich"],"rows":rows}})
}
fn batch(page: &str, operations: Value) -> Value {
	json!({"pageId":page,"request":{"authorities":["ich"],"operations":operations}})
}
fn create(index: usize, token: &str, rows: Value) -> Value {
	json!({"operation":"create","clientRowIndex":index,"clientRowId":token,"rows":rows})
}

#[tokio::test]
async fn pages_save_rolls_back_other_pages_rows_and_audit_then_commits_retry(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let id = create_case_for_editor(&app, &cookie, "PAGES-ATOMIC", &["ich"]).await?;
	let row = repeatable_row_id("LB", &app, &cookie, &id)
		.await?
		.ok_or("missing LB")?;
	let sd_uri = format!("/api/cases/{id}/editor/pages/SD");
	let lb_uri = format!("/api/cases/{id}/editor/pages/LB?include_rows=true");
	let uri = format!("/api/cases/{id}/editor/pages");
	let (_, sd_before) = get_json(&app, &cookie, &sd_uri).await?;
	let (_, lb_before) = get_json(&app, &cookie, &lb_uri).await?;
	let audit_before =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?;
	let mut payload = json!({"pages":[
		direct("SD", json!({"senderInformation":{"organizationName":"Atomic sender"}})),
		batch("LB", json!([
			{"operation":"update","rowId":row,"clientRowIndex":0,"rows":{"testResult":{"testName":"Atomic update"}}},
			create(1,"index-new",json!({"testResult":{"testName":12,"sequenceNumber":2}}))
		]))
	]});
	let (status, body) = patch_json(&app, &cookie, &uri, payload.clone()).await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
	assert_eq!(
		body["error"]["data"]["detail"]["path"],
		"testResults.1.testName"
	);
	assert_eq!(get_json(&app, &cookie, &sd_uri).await?.1, sd_before);
	assert_eq!(get_json(&app, &cookie, &lb_uri).await?.1, lb_before);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?,
		audit_before
	);
	payload["pages"][1]["request"]["operations"][1]["rows"]["testResult"]
		["testName"] = json!("Atomic create");
	let (status, body) = patch_json(&app, &cookie, &uri, payload).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(body["caseId"], id);
	assert_eq!(body["pages"][0]["pageId"], "SD");
	assert_eq!(body["pages"][1]["results"][0]["rowId"], row);
	let created = body["pages"][1]["results"][1]["rowId"]
		.as_str()
		.ok_or("missing created identity")?;
	assert_ne!(created, row);
	let (_, stored) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{id}/editor/pages/LB/rows/{created}"),
	)
	.await?;
	assert_eq!(stored["data"]["testResult"]["test_name"], "Atomic create");
	let (_, updated) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{id}/editor/pages/LB/rows/{row}"),
	)
	.await?;
	assert_eq!(updated["data"]["testResult"]["test_name"], "Atomic update");
	assert_eq!(
		get_json(&app, &cookie, &sd_uri).await?.1["rows"]["senderInformation"]
			["organizationName"],
		"Atomic sender"
	);
	let (_, rows) = get_json(&app, &cookie, &lb_uri).await?;
	assert_eq!(
		rows["rows"]["rows"]
			.as_array()
			.ok_or("missing LB rows")?
			.len(),
		2
	);
	Ok(())
}

#[tokio::test]
async fn pages_save_all_eleven_pages_resolves_new_parent_and_reaction_ids(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm).expect("web server configuration");
	let id = create_case_for_editor(&app, &cookie, "PAGES-ALL", &["ich"]).await?;
	let mut pages = vec![
		direct(
			"CI",
			json!({"safetyReportIdentification":{"reportType":"1"}}),
		),
		direct(
			"RP",
			json!({"primarySources":[{"reporterGivenName":"Isolated reporter","sequenceNumber":1}]}),
		),
		direct(
			"SD",
			json!({"senderInformation":{"organizationName":"Isolated sender"}}),
		),
		direct(
			"SI",
			json!({"studyInformation":{"studyName":"Isolated study"}}),
		),
		direct(
			"DM",
			json!({"patientInformation":{"patientInitials":"ISOLATED"}}),
		),
		direct(
			"NR",
			json!({"narrative":{"caseNarrative":"Isolated narrative"}}),
		),
		batch(
			"LR",
			json!([create(
				0,
				"index-lr",
				json!({"literatureReference":{"referenceText":"Isolated reference","sequenceNumber":1}})
			)]),
		),
		batch(
			"DH",
			json!([create(
				0,
				"index-dh",
				json!({"pastDrugHistory":{"drugName":"Isolated history","sequenceNumber":1}})
			)]),
		),
		batch(
			"AE",
			json!([create(
				0,
				"index-ae",
				json!({"reaction":{"primarySourceReaction":"Isolated reaction","sequenceNumber":1}})
			)]),
		),
		batch(
			"LB",
			json!([create(
				0,
				"index-lb",
				json!({"testResult":{"testName":"Isolated test","sequenceNumber":1}})
			)]),
		),
		batch(
			"DG",
			json!([create(
				0,
				"index-dg",
				json!({"drug":{"medicinalProduct":"Isolated drug","drugCharacterization":"1","sequenceNumber":1,"drugReactionAssessments":[{"reactionId":"index-ae","sourceOfAssessment":"Isolated reporter","sequenceNumber":1}]}})
			)]),
		),
	];
	pages.reverse();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&format!("/api/cases/{id}/editor/pages"),
		json!({"pages":pages}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let results = body["pages"].as_array().ok_or("missing page results")?;
	assert_eq!(results.len(), 11);
	let mut ids = std::collections::HashMap::new();
	for page in results {
		let page_id = page["pageId"].as_str().ok_or("missing page id")?;
		let (status, projection) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{id}/editor/pages/{page_id}?include_rows=true"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{projection}");
		let direct_value = match page_id {
			"CI" => Some(("/rows/safetyReportIdentification/reportType", "1")),
			"RP" => Some((
				"/rows/primarySources/0/reporterGivenName",
				"Isolated reporter",
			)),
			"SD" => Some((
				"/rows/senderInformation/organizationName",
				"Isolated sender",
			)),
			"SI" => Some(("/rows/studyInformation/study_name", "Isolated study")),
			"DM" => Some(("/rows/patientInformation/patient_initials", "ISOLATED")),
			"NR" => Some(("/rows/narrative/case_narrative", "Isolated narrative")),
			_ => None,
		};
		if let Some((path, expected)) = direct_value {
			assert_eq!(
				projection.pointer(path),
				Some(&json!(expected)),
				"{page_id}"
			);
		}
		if matches!(page_id, "LR" | "DH" | "AE" | "LB" | "DG") {
			let row_id = page["results"][0]["rowId"]
				.as_str()
				.ok_or("missing row id")?;
			assert_eq!(projection["rows"]["rows"][0]["id"], row_id);
			ids.insert(page_id, row_id);
			let (path, expected) = match page_id {
				"LR" => (
					"/data/literatureReference/reference_text",
					"Isolated reference",
				),
				"DH" => ("/data/pastDrugHistory/drug_name", "Isolated history"),
				"AE" => (
					"/data/reaction/primary_source_reaction",
					"Isolated reaction",
				),
				"LB" => ("/data/testResult/test_name", "Isolated test"),
				"DG" => ("/data/drug/medicinal_product", "Isolated drug"),
				_ => return Err("unexpected repeated page".into()),
			};
			let (status, detail) = get_json(
				&app,
				&cookie,
				&format!("/api/cases/{id}/editor/pages/{page_id}/rows/{row_id}"),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{detail}");
			assert_eq!(detail.pointer(path), Some(&json!(expected)), "{page_id}");
		}
	}
	let dg = ids.get("DG").ok_or("missing DG")?;
	let (_, stored) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{id}/editor/pages/DG/rows/{dg}"),
	)
	.await?;
	assert_eq!(
		stored["data"]["drug"]["drugReactionAssessments"][0]["reactionId"],
		*ids.get("AE").ok_or("missing AE")?
	);
	Ok(())
}

#[tokio::test]
async fn pages_save_rejects_permission_scope_lifecycle_and_ambiguous_requests(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let viewer = cookie_header(
		&generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?.to_string(),
	);
	let app = web_server::app(mm.clone()).expect("web server configuration");
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let id = create_case_for_editor(&app, &cookie, "PAGES-GUARDS", &["ich"]).await?;
	let other =
		create_case_for_editor(&app, &cookie, "PAGES-OTHER", &["ich"]).await?;
	let foreign = repeatable_row_id("LB", &app, &cookie, &other)
		.await?
		.ok_or("missing other row")?;
	let uri = format!("/api/cases/{id}/editor/pages");
	let sd_uri = format!("/api/cases/{id}/editor/pages/SD");
	let (_, before) = get_json(&app, &cookie, &sd_uri).await?;
	let audit =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?;
	let sd = direct(
		"SD",
		json!({"senderInformation":{"organizationName":"Must not persist"}}),
	);
	let (status, _) =
		patch_json(&app, &viewer, &uri, json!({"pages":[sd.clone()]})).await?;
	assert_eq!(status, StatusCode::FORBIDDEN);
	for payload in [
		json!({"pages":[{"pageId":"SD","request":{"rows":{"senderInformation":{"organizationName":"Must not persist"}}}}]}),
		json!({"pages":[{"pageId":"SD","request":{"authorities":[],"rows":{"senderInformation":{"organizationName":"Must not persist"}}}}]}),
		json!({"pages":[]}),
		json!({"pages":[sd.clone(),sd.clone()]}),
		json!({"pages":[sd.clone(),{"pageId":"CI","request":{"authorities":["fda"],"rows":{"safetyReportIdentification":{"reportType":"1"}}}}]}),
		json!({"pages":[sd.clone(),batch("LB",json!([create(0,"index-x",json!({"testResult":{"testName":"One"}})),create(0,"index-y",json!({"testResult":{"testName":"Two"}}))]))]}),
		json!({"pages":[sd.clone(),batch("LB",json!([create(0,"index-x",json!({"testResult":{"testName":"One"}})),create(1,"index-x",json!({"testResult":{"testName":"Two"}}))]))]}),
	] {
		let (status, body) = patch_json(&app, &cookie, &uri, payload).await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
		assert_eq!(get_json(&app, &cookie, &sd_uri).await?.1, before);
		assert_eq!(
			crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
				.await?,
			audit
		);
	}
	let (status, body) = patch_json(&app, &cookie, &uri, json!({"pages":[sd.clone(),batch("LB",json!([{"operation":"delete","clientRowIndex":0,"rowId":foreign}]))]})).await?;
	assert!(
		matches!(status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN),
		"{status}: {body}"
	);
	assert_eq!(get_json(&app, &cookie, &sd_uri).await?.1, before);
	assert_eq!(
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?,
		audit
	);
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{id}/lock/toggle"),
		json!({"data":{"expected_status":"draft"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, body) =
		patch_json(&app, &cookie, &uri, json!({"pages":[sd]})).await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
	assert_eq!(get_json(&app, &cookie, &sd_uri).await?.1, before);
	Ok(())
}
