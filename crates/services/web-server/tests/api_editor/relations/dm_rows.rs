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
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":{
			"patientInformation":{"raceCodes":["1"]},
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
	let death_id = saved["deathInfo"]["id"].clone();
	let patient_id = saved["patientInformation"]["id"].clone();
	let replay = json!({
		"patientInformation":{"id":patient_id,"raceCodes":["1"]},
		"patientIdentifiers":[{"id":owners[0].1,"sequenceNumber":1,"identifierTypeCode":"1","identifierValue":"PATIENT-ID"}],
		"medicalHistoryEpisodes":[{"id":owners[1].1,"sequenceNumber":1,"comments":"HISTORY"}],
		"deathInfo":{"id":death_id,"dateOfDeath":"20261001"},
		"reportedCauses":[{"id":owners[2].1,"sequenceNumber":1,"causeText":"REPORTED"}],
		"autopsyCauses":[{"id":owners[3].1,"sequenceNumber":1,"causeText":"AUTOPSY"}],
		"parentInfo":{"id":parent_id,"parentIdentification":"PARENT"},
		"parentMedicalHistory":[{"id":owners[4].1,"sequenceNumber":1,"comments":"PARENT-HISTORY"}],
		"parentPastDrugs":[{"id":owners[5].1,"sequenceNumber":1,"drugName":"PARENT-DRUG"}]
	});
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
		for (owner, id) in &owners {
			let rows = after["rows"][*owner]
				.as_array()
				.ok_or("missing patient rows")?;
			assert_eq!(rows.len(), 1, "{owner}: {after}");
			assert_eq!(rows[0]["id"], id, "{owner}: {after}");
		}
		assert_eq!(
			after["rows"]["patientIdentifiers"][0]["identifier_value"],
			"PATIENT-ID"
		);
		assert_eq!(
			after["rows"]["medicalHistoryEpisodes"][0]["comments"],
			"HISTORY"
		);
		assert_eq!(after["rows"]["reportedCauses"][0]["cause_text"], "REPORTED");
		assert_eq!(after["rows"]["autopsyCauses"][0]["cause_text"], "AUTOPSY");
		assert_eq!(
			after["rows"]["parentMedicalHistory"][0]["comments"],
			"PARENT-HISTORY"
		);
		assert_eq!(
			after["rows"]["parentPastDrugs"][0]["drug_name"],
			"PARENT-DRUG"
		);
		assert_eq!(after["rows"]["parentInfo"]["id"], parent_id);
		assert_eq!(
			after["rows"]["parentInfo"]["parent_identification"],
			"PARENT"
		);
		assert_eq!(after["rows"]["deathInfo"]["id"], death_id);
		assert_eq!(after["rows"]["deathInfo"]["date_of_death"], "20261001");
		assert_eq!(
			after["rows"]["patientInformation"]["race_codes"],
			json!(["1"])
		);
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &case_id).await?,
			audit_before_replay
		);
	}
	let changed_patient = json!({
		"patientInformation":{"id":patient_id,"raceCodes":["2"]},
		"deathInfo":{"id":death_id,"dateOfDeath":"20261002"}
	});
	let (status, changed) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["ich"],"rows":changed_patient.clone()}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{changed}");
	assert_eq!(changed["rows"]["patientInformation"]["id"], patient_id);
	assert_eq!(
		changed["rows"]["patientInformation"]["race_codes"],
		json!(["2"])
	);
	assert_eq!(changed["rows"]["deathInfo"]["id"], death_id);
	assert_eq!(changed["rows"]["deathInfo"]["date_of_death"], "20261002");
	let audit_after_change = super::case_audit_ids(&mm, &ctx, &case_id).await?;
	for attempt in 0..2 {
		let (status, replayed) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["ich"],"rows":changed_patient.clone()}),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::OK,
			"updated replay {attempt}: {replayed}"
		);
		assert_eq!(replayed["rows"]["patientInformation"]["id"], patient_id);
		assert_eq!(
			replayed["rows"]["patientInformation"]["race_codes"],
			json!(["2"])
		);
		assert_eq!(replayed["rows"]["deathInfo"]["id"], death_id);
		assert_eq!(replayed["rows"]["deathInfo"]["date_of_death"], "20261002");
		assert_eq!(
			super::case_audit_ids(&mm, &ctx, &case_id).await?,
			audit_after_change
		);
	}
	let parent_children_before = [
		(
			"parentMedicalHistory",
			saved["parentMedicalHistory"][0].clone(),
			"comments",
		),
		(
			"parentPastDrugs",
			saved["parentPastDrugs"][0].clone(),
			"drug_name",
		),
	];
	for rows in [
		json!({"patientIdentifiers":[{"id":owners[0].1,"identifierValue":"MUST ROLL BACK"},{"id":"not-a-uuid","identifierValue":"INVALID"}]}),
		json!({"medicalHistoryEpisodes":[{"id":owners[1].1,"comments":"MUST ROLL BACK"},{"id":"not-a-uuid","comments":"INVALID"}]}),
		json!({"reportedCauses":[{"id":owners[2].1,"causeText":"MUST ROLL BACK"},{"id":"not-a-uuid","causeText":"INVALID"}]}),
		json!({"autopsyCauses":[{"id":owners[3].1,"causeText":"MUST ROLL BACK"},{"id":"not-a-uuid","causeText":"INVALID"}]}),
		json!({"parentMedicalHistory":[{"id":owners[4].1,"comments":"MUST ROLL BACK"},{"id":"not-a-uuid","comments":"INVALID"}]}),
		json!({"parentPastDrugs":[{"id":owners[5].1,"drugName":"MUST ROLL BACK"},{"id":"not-a-uuid","drugName":"INVALID"}]}),
		json!({"parentInfo":{"id":"not-a-uuid","parentIdentification":"INVALID"},"patientInformation":{"id":saved["patientInformation"]["id"],"patientInitials":"MUST ROLL BACK"}}),
		json!({"deathInfo":{"id":"not-a-uuid","dateOfDeath":"20261002"},"patientInformation":{"id":saved["patientInformation"]["id"],"patientInitials":"MUST ROLL BACK"}}),
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
			&mm,
			&ctx,
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
			&mm,
			&ctx,
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
			&mm,
			&ctx,
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
			&mm,
			&ctx,
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
			&mm,
			&ctx,
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
			&mm,
			&ctx,
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
		for (owner, before, value_field) in &parent_children_before {
			assert_eq!(
				body["rows"][*owner].as_array().map(Vec::len),
				Some(1),
				"{owner}: {body}"
			);
			assert_eq!(
				body["rows"][*owner][0]["id"], before["id"],
				"{owner}: {body}"
			);
			assert_eq!(
				body["rows"][*owner][0][*value_field], before[*value_field],
				"{owner}: child value changed across parent delete/restore"
			);
		}
	}
	super::assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "ich", 7)
		.await?;
	let (status, saved) = super::get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let death_id = saved["rows"]["deathInfo"]["id"]
		.as_str()
		.ok_or("missing death singleton ID")?;
	let guarded = [
		(
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"deathInfo":{"id":death_id,"dateOfDeath":"20261002"}}}),
		),
		(
			"PATCH",
			uri.clone(),
			json!({"authorities":["fda"],"rows":{"patientInformation":{"raceCodes":["1"]}}}),
		),
		(
			"PATCH",
			uri.clone(),
			json!({"authorities":["fda"],"rows":{"patientInformation":{"raceCodes":[]}}}),
		),
	];
	super::assert_editor_request_guards(&app, &cookie, &mm, &seed, &uri, &guarded)
		.await?;
	Ok(())
}
