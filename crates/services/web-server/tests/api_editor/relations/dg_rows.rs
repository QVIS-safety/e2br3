use super::{assert_malformed_row_id_rejected_without_mutation, patch_and_reload};
use crate::case_editor_contract_web::support::create_case_for_editor;
use crate::case_editor_contract_web::support::post_json;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::{json, Value};

pub(crate) async fn drug_child_rows_delete_restore_and_resave_are_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "DG-CHILD-LIFECYCLE", &["ich"])
			.await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Child lifecycle drug"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let (status, body) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":{"drug":{
			"activeSubstances":[{"sequenceNumber":1,"substanceName":"Substance"}],
			"dosageInformation":[{"sequenceNumber":1,"dosageText":"Dosage"}],
			"indications":[{"sequenceNumber":1,"indicationText":"Indication"}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let owners = [
		(
			"activeSubstances",
			body["data"]["drug"]["activeSubstances"][0]["id"].clone(),
		),
		(
			"dosageInformation",
			body["data"]["drug"]["dosageInformation"][0]["id"].clone(),
		),
		(
			"indications",
			body["data"]["drug"]["indications"][0]["id"].clone(),
		),
	];
	for drug in [
		json!({"activeSubstances":[{"id":"not-a-uuid","substanceName":"INVALID"}]}),
		json!({"dosageInformation":[{"id":"not-a-uuid","dosageText":"INVALID"}]}),
		json!({"indications":[{"id":"not-a-uuid","indicationText":"INVALID"}]}),
	] {
		assert_malformed_row_id_rejected_without_mutation(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			&case_id,
			"ich",
			json!({"drug":drug}),
		)
		.await?;
	}
	let deleted = owners
		.iter()
		.map(|(owner, id)| ((*owner).to_string(), json!([{"id":id,"deleted":true}])))
		.collect::<serde_json::Map<_, _>>();
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":deleted}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "delete attempt {attempt}: {body}");
		for (owner, _) in &owners {
			assert_eq!(
				body["data"]["drug"][*owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	let ordinary_rows = owners
		.iter()
		.map(|(owner, id)| ((*owner).to_string(), json!([{"id":id}])))
		.collect::<serde_json::Map<_, _>>();
	for drug in [
		json!({}),
		json!({"activeSubstances":[],"dosageInformation":[],"indications":[]}),
		Value::Object(ordinary_rows),
	] {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":drug}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		for (owner, _) in &owners {
			assert_eq!(
				body["data"]["drug"][*owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	let restored = owners
		.iter()
		.map(|(owner, id)| {
			((*owner).to_string(), json!([{"id":id,"deleted":false}]))
		})
		.collect::<serde_json::Map<_, _>>();
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":restored}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		for (owner, id) in &owners {
			let rows = body["data"]["drug"][*owner]
				.as_array()
				.ok_or("missing restored drug child rows")?;
			assert_eq!(rows.len(), 1, "{owner}: {body}");
			assert_eq!(rows[0]["id"], id, "{owner}: {body}");
		}
	}
	super::assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "ich", 3)
		.await?;
	Ok(())
}
