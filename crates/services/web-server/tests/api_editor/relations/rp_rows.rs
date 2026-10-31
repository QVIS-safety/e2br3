use super::{assert_malformed_row_id_rejected_without_mutation, patch_and_reload};
use crate::case_editor_contract_web::support::create_case_for_editor;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::json;

pub(crate) async fn reporter_row_delete_restore_and_resave_is_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "RP-LIFECYCLE", &["ich"]).await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let uri = format!("/api/cases/{case_id}/editor/pages/RP");
	let (status, body) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":{"primarySources":[{
			"sequenceNumber":1,"reporterGivenName":"Reporter"
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let row_id = body["rows"]["primarySources"][0]["id"].clone();
	let replay = json!({"primarySources":[{
		"id":row_id,"sequenceNumber":1,"reporterGivenName":"Reporter"
	}]});
	let audit_before_replay = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	for attempt in 0..2 {
		let (status, replayed) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":replay.clone()}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {replayed}");
		let rows = replayed["rows"]["primarySources"]
			.as_array()
			.ok_or("missing reporters")?;
		assert_eq!(rows.len(), 1, "{replayed}");
		assert_eq!(rows[0]["id"], row_id, "{replayed}");
		assert_eq!(rows[0]["reporterGivenName"], "Reporter", "{replayed}");
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &case_id).await?,
			audit_before_replay
		);
	}
	assert_malformed_row_id_rejected_without_mutation(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		&case_id,
		"ich",
		json!({"primarySources":[
			{"id":row_id,"reporterGivenName":"MUST ROLL BACK"},
			{"id":"not-a-uuid","reporterGivenName":"INVALID"}
		]}),
	)
	.await?;
	let delete = json!({"primarySources":[{"id":row_id,"deleted":true}]});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":delete}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "delete attempt {attempt}: {body}");
		assert_eq!(
			body["rows"]["primarySources"].as_array().map(Vec::len),
			Some(0),
			"{body}"
		);
	}
	for rows in [
		json!({}),
		json!({"primarySources":[]}),
		json!({"primarySources":[{"id":row_id}]}),
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
		assert_eq!(
			body["rows"]["primarySources"].as_array().map(Vec::len),
			Some(0),
			"{body}"
		);
	}
	let restore = json!({"primarySources":[{
		"id":row_id,"deleted":false,"reporterGivenName":"Restored reporter"
	}]});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":restore}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		let rows = body["rows"]["primarySources"]
			.as_array()
			.ok_or("missing restored reporters")?;
		assert_eq!(rows.len(), 1, "{body}");
		assert_eq!(rows[0]["id"], row_id, "{body}");
		assert_eq!(rows[0]["reporterGivenName"], "Restored reporter", "{body}");
	}
	super::assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "ich", 1)
		.await?;
	Ok(())
}
