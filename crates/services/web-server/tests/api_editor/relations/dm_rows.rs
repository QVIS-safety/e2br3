use super::{assert_malformed_row_id_rejected_without_mutation, patch_and_reload};
use crate::case_editor_contract_web::support::create_case_for_editor;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::{json, Value};

pub(crate) async fn patient_rows_delete_restore_and_resave_are_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "DM-LIFECYCLE", &["ich"]).await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DM");
	let (status, body) = patch_and_reload(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"patientIdentifiers":[{"sequenceNumber":1,"identifierTypeCode":"1","identifierValue":"PATIENT-ID"}],
			"medicalHistoryEpisodes":[{"sequenceNumber":1,"comments":"HISTORY"}],
			"deathInfo":{"dateOfDeath":"20261001"},
			"reportedCauses":[{"sequenceNumber":1,"causeText":"REPORTED"}],
			"autopsyCauses":[{"sequenceNumber":1,"causeText":"AUTOPSY"}],
			"parentInfo":{"parentIdentification":"PARENT"},
			"parentMedicalHistory":[{"sequenceNumber":1,"comments":"PARENT-HISTORY"}],
			"parentPastDrugs":[{"sequenceNumber":1,"drugName":"PARENT-DRUG"}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let saved = &body["rows"];
	let owners = [
		(
			"patientIdentifiers",
			saved["patientIdentifiers"][0]["id"].clone(),
		),
		(
			"medicalHistoryEpisodes",
			saved["medicalHistoryEpisodes"][0]["id"].clone(),
		),
		("reportedCauses", saved["reportedCauses"][0]["id"].clone()),
		("autopsyCauses", saved["autopsyCauses"][0]["id"].clone()),
		(
			"parentMedicalHistory",
			saved["parentMedicalHistory"][0]["id"].clone(),
		),
		("parentPastDrugs", saved["parentPastDrugs"][0]["id"].clone()),
	];
	let parent_id = saved["parentInfo"]["id"].clone();
	for rows in [
		json!({"patientIdentifiers":[{"id":"not-a-uuid","identifierValue":"INVALID"}]}),
		json!({"medicalHistoryEpisodes":[{"id":"not-a-uuid","comments":"INVALID"}]}),
		json!({"reportedCauses":[{"id":"not-a-uuid","causeText":"INVALID"}]}),
		json!({"autopsyCauses":[{"id":"not-a-uuid","causeText":"INVALID"}]}),
		json!({"parentMedicalHistory":[{"id":"not-a-uuid","comments":"INVALID"}]}),
		json!({"parentPastDrugs":[{"id":"not-a-uuid","drugName":"INVALID"}]}),
		json!({"parentInfo":{"id":"not-a-uuid","parentIdentification":"INVALID"}}),
		json!({"deathInfo":{"id":"not-a-uuid","dateOfDeath":"20261002"}}),
	] {
		assert_malformed_row_id_rejected_without_mutation(
			&app, &cookie, &mm, &ctx, &uri, &case_id, "ich", rows,
		)
		.await?;
	}
	let deleted_rows = owners
		.iter()
		.map(|(owner, id)| ((*owner).to_string(), json!([{"id":id,"deleted":true}])))
		.collect::<serde_json::Map<_, _>>();
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":deleted_rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "delete attempt {attempt}: {body}");
		for (owner, _) in &owners {
			assert_eq!(
				body["rows"][*owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	let empty_rows = owners
		.iter()
		.map(|(owner, _)| ((*owner).to_string(), json!([])))
		.collect::<serde_json::Map<_, _>>();
	let ordinary_rows = owners
		.iter()
		.map(|(owner, id)| ((*owner).to_string(), json!([{"id":id}])))
		.collect::<serde_json::Map<_, _>>();
	for rows in [
		json!({}),
		Value::Object(empty_rows),
		Value::Object(ordinary_rows),
	] {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		for (owner, _) in &owners {
			assert_eq!(
				body["rows"][*owner].as_array().map(Vec::len),
				Some(0),
				"{owner}: {body}"
			);
		}
	}
	let restored_rows = owners
		.iter()
		.map(|(owner, id)| {
			((*owner).to_string(), json!([{"id":id,"deleted":false}]))
		})
		.collect::<serde_json::Map<_, _>>();
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":restored_rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		for (owner, id) in &owners {
			let rows = body["rows"][*owner]
				.as_array()
				.ok_or("missing restored patient rows")?;
			assert_eq!(rows.len(), 1, "{owner}: {body}");
			assert_eq!(rows[0]["id"], id, "{owner}: {body}");
		}
	}
	let parent_delete = json!({"parentInfo":{"id":parent_id,"deleted":true}});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":parent_delete}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "parent delete {attempt}: {body}");
		assert!(body["rows"]["parentInfo"].is_null(), "{body}");
	}
	for rows in [
		json!({}),
		json!({"parentMedicalHistory":[],"parentPastDrugs":[]}),
		json!({"parentInfo":{"id":parent_id}}),
	] {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert!(body["rows"]["parentInfo"].is_null(), "{body}");
	}
	let parent_restore = json!({"parentInfo":{
		"id":parent_id,"deleted":false,"parentIdentification":"RESTORED-PARENT"
	}});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":parent_restore}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "parent restore {attempt}: {body}");
		assert_eq!(body["rows"]["parentInfo"]["id"], parent_id, "{body}");
		assert_eq!(
			body["rows"]["parentInfo"]["parent_identification"], "RESTORED-PARENT",
			"{body}"
		);
		for (owner, id) in &owners[4..] {
			assert_eq!(
				body["rows"][*owner].as_array().map(Vec::len),
				Some(1),
				"{owner}: {body}"
			);
			assert_eq!(body["rows"][*owner][0]["id"], id, "{owner}: {body}");
		}
	}
	Ok(())
}
