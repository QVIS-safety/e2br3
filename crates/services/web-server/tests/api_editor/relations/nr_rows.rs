use super::{assert_malformed_row_id_rejected_without_mutation, patch_and_reload};
use crate::case_editor_contract_web::support::create_case_for_editor;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::json;

pub(crate) async fn narrative_rows_delete_restore_and_resave_are_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "NR-LIFECYCLE", &["ich"]).await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let uri = format!("/api/cases/{case_id}/editor/pages/NR");
	let (status, body) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"senderDiagnoses":[{"sequenceNumber":1,"diagnosisMeddraVersion":"28.1","diagnosisMeddraCode":"10019211"}],
			"caseSummaryInformation":[{"sequenceNumber":1,"languageCode":"en","summaryText":"Summary"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let diagnosis_id = body["rows"]["senderDiagnoses"][0]["id"].clone();
	let summary_id = body["rows"]["caseSummaryInformation"][0]["id"].clone();
	let replay = json!({"senderDiagnoses":[{"id":diagnosis_id,"sequenceNumber":1,"diagnosisMeddraVersion":"28.1","diagnosisMeddraCode":"10019211"}],"caseSummaryInformation":[{"id":summary_id,"sequenceNumber":1,"languageCode":"en","summaryText":"Summary"}]});
	let audit_before_replay = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	for attempt in 0..2 {
		let (status, after) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":replay.clone()}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {after}");
		assert_eq!(
			after["rows"]["senderDiagnoses"].as_array().map(Vec::len),
			Some(1)
		);
		assert_eq!(after["rows"]["senderDiagnoses"][0]["id"], diagnosis_id);
		assert_eq!(
			after["rows"]["senderDiagnoses"][0]["diagnosis_meddra_code"],
			"10019211"
		);
		assert_eq!(
			after["rows"]["caseSummaryInformation"]
				.as_array()
				.map(Vec::len),
			Some(1)
		);
		assert_eq!(after["rows"]["caseSummaryInformation"][0]["id"], summary_id);
		assert_eq!(
			after["rows"]["caseSummaryInformation"][0]["summary_text"],
			"Summary"
		);
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &case_id).await?,
			audit_before_replay
		);
	}
	for rows in [
		json!({"senderDiagnoses":[
			{"id":diagnosis_id,"diagnosisMeddraCode":"10000000"},
			{"id":"not-a-uuid","diagnosisMeddraCode":"10000000"}
		]}),
		json!({"caseSummaryInformation":[
			{"id":summary_id,"summaryText":"MUST ROLL BACK"},
			{"id":"not-a-uuid","summaryText":"INVALID"}
		]}),
	] {
		assert_malformed_row_id_rejected_without_mutation(
			&app, &cookie, &mm, &ctx, &uri, &case_id, "ich", rows,
		)
		.await?;
	}
	let deleted = json!({
		"senderDiagnoses":[{"id":diagnosis_id,"deleted":true}],
		"caseSummaryInformation":[{"id":summary_id,"deleted":true}]
	});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":deleted}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "delete attempt {attempt}: {body}");
		for owner in ["senderDiagnoses", "caseSummaryInformation"] {
			assert_eq!(
				body["rows"][owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	for rows in [
		json!({}),
		json!({"senderDiagnoses":[],"caseSummaryInformation":[]}),
		json!({"senderDiagnoses":[{"id":diagnosis_id}],"caseSummaryInformation":[{"id":summary_id}]}),
	] {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		for owner in ["senderDiagnoses", "caseSummaryInformation"] {
			assert_eq!(
				body["rows"][owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	let restored = json!({
		"senderDiagnoses":[{"id":diagnosis_id,"deleted":false,"diagnosisMeddraVersion":"28.1","diagnosisMeddraCode":"10019212"}],
		"caseSummaryInformation":[{"id":summary_id,"deleted":false,"languageCode":"en","summaryText":"Restored summary"}]
	});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":restored}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		for owner in ["senderDiagnoses", "caseSummaryInformation"] {
			assert_eq!(
				body["rows"][owner].as_array().map(Vec::len),
				Some(1),
				"{owner}: {body}"
			);
		}
		assert_eq!(
			body["rows"]["senderDiagnoses"][0]["diagnosis_meddra_code"], "10019212",
			"{body}"
		);
		assert_eq!(
			body["rows"]["caseSummaryInformation"][0]["summary_text"],
			"Restored summary",
			"{body}"
		);
		assert_eq!(
			body["rows"]["senderDiagnoses"][0]["id"], diagnosis_id,
			"{body}"
		);
		assert_eq!(
			body["rows"]["caseSummaryInformation"][0]["id"], summary_id,
			"{body}"
		);
	}
	super::assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "ich", 2)
		.await?;
	Ok(())
}
