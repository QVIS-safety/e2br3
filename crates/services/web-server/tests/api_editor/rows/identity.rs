use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn device_subresources_survive_dg_row_reload(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-DEVICE-{}", Uuid::new_v4()),
		&["fda", "mfds"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Device drug"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/devices"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"malfunction": true,
				"device_brand_name": "Imported Brand",
				"common_device_name_null_flavor": "NI"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let device_id = body["data"]["id"].as_str().ok_or("missing device id")?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes"),
		json!({
			"data": {
				"device_id": device_id,
				"element": "device_problem",
				"sequence_number": 1,
				"value_code": "1234567"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"code": "KR_DVC_MFR",
				"value_type": "ST",
				"value_value": "Imported Maker"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["device_brand_name"],
		json!("Imported Brand")
	);
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["common_device_name_null_flavor"],
		json!("NI")
	);
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["deviceProblemCodes"][0]["value_code"],
		json!("1234567")
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"][0]["code"],
		json!("KR_DVC_MFR")
	);
	assert_eq!(
		body["data"]["drug"]["deviceCharacteristics"][0]["value_value"],
		json!("Imported Maker")
	);
	crate::case_editor_contract_web::support::assert_batched_rows(
		&app, &cookie, &case_id, "DG",
	)
	.await?;
	Ok(())
}

pub(crate) async fn fda_device_and_code_ids_survive_replacement(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let id = create_case_for_editor(&app, &cookie, "FDA-DEVICE-IDENTITY", &["fda"])
		.await?;
	let base = format!("/api/cases/{id}/editor/pages");
	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("{base}/DG/rows"),
		json!({"authorities":["fda"],"rows":{"drug":{
			"drugCharacterization":"1", "medicinalProduct":"DEVICE TEST",
			"fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}],"remedialActions":[{"valueCode":"10"}],"deviceProblemCodes":[{"valueCode":"1234567"}]}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (status, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "1", "{saved}");
	let original_device = &drug["fdaDevices"][0];
	let original_device_id = original_device["id"].clone();
	let original_created_at = original_device["created_at"].clone();
	let original_code_id = original_device["followUpTypes"][0]["id"].clone();
	let original_code_created_at =
		original_device["followUpTypes"][0]["created_at"].clone();
	let remedial_id = original_device["remedialActions"][0]["id"].clone();
	let problem_id = original_device["deviceProblemCodes"][0]["id"].clone();
	let full_replay = json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
		"id":original_device_id,"deviceBrandName":"DEVICE-1",
		"followUpTypes":[{"id":original_code_id,"valueCode":"1"}],
		"remedialActions":[{"id":remedial_id,"valueCode":"10"}],
		"deviceProblemCodes":[{"id":problem_id,"valueCode":"1234567"}]
	}]}}});
	let audit_before_replay =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?;
	for attempt in 0..2 {
		let (status, replayed) =
			patch_json(&app, &cookie, &uri, full_replay.clone()).await?;
		assert_eq!(status, StatusCode::OK, "replay {attempt}: {replayed}");
		let (status, fresh) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{fresh}");
		let devices = fresh["data"]["drug"]["fdaDevices"]
			.as_array()
			.ok_or("missing devices")?;
		assert_eq!(devices.len(), 1);
		let device = &devices[0];
		assert_eq!(device["id"], original_device_id);
		assert_eq!(device["device_brand_name"], "DEVICE-1");
		for (owner, child_id, value) in [
			("followUpTypes", &original_code_id, "1"),
			("remedialActions", &remedial_id, "10"),
			("deviceProblemCodes", &problem_id, "1234567"),
		] {
			let rows = device[owner].as_array().ok_or("missing device code rows")?;
			assert_eq!(rows.len(), 1);
			assert_eq!(rows[0]["id"], *child_id);
			assert_eq!(rows[0]["value_code"], value);
		}
		assert_eq!(
			crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
				.await?,
			audit_before_replay
		);
	}
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}],
			"remedialActions":[{"id":remedial_id,"valueCode":"10"}],
			"deviceProblemCodes":[{"id":problem_id,"valueCode":"1234567"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let edited = &saved["data"]["drug"]["fdaDevices"][0];
	assert_eq!(edited["id"], original_device_id, "{saved}");
	assert_eq!(edited["created_at"], original_created_at, "{saved}");
	assert_eq!(edited["device_brand_name"], json!(null), "{saved}");
	assert_eq!(
		edited["device_brand_name_null_flavor"],
		json!(null),
		"{saved}"
	);
	assert_eq!(
		edited["followUpTypes"][0]["id"], original_code_id,
		"{saved}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"2"}],
			"remedialActions":[{"id":remedial_id,"valueCode":"10"}],
			"deviceProblemCodes":[{"id":problem_id,"valueCode":"1234567"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, code_edited) = get_json(&app, &cookie, &uri).await?;
	let edited_code =
		&code_edited["data"]["drug"]["fdaDevices"][0]["followUpTypes"][0];
	assert_eq!(edited_code["id"], original_code_id, "{code_edited}");
	assert_eq!(
		edited_code["created_at"], original_code_created_at,
		"{code_edited}"
	);
	assert_eq!(edited_code["value_code"], "2", "{code_edited}");
	let updated_replay = json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
		"id":original_device_id,"deviceBrandName":null,"deviceBrandNameNullFlavor":null,
		"followUpTypes":[{"id":original_code_id,"valueCode":"2"}],
		"remedialActions":[{"id":remedial_id,"valueCode":"10"}],
		"deviceProblemCodes":[{"id":problem_id,"valueCode":"1234567"}]
	}]}}});
	let updated_audits =
		crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
			.await?;
	for attempt in 0..2 {
		let (status, replayed) =
			patch_json(&app, &cookie, &uri, updated_replay.clone()).await?;
		assert_eq!(
			status,
			StatusCode::OK,
			"updated replay {attempt}: {replayed}"
		);
		let (status, fresh) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{fresh}");
		let device = &fresh["data"]["drug"]["fdaDevices"][0];
		assert_eq!(
			fresh["data"]["drug"]["fdaDevices"].as_array().map(Vec::len),
			Some(1)
		);
		assert_eq!(device["id"], original_device_id);
		assert_eq!(
			device.get("device_brand_name"),
			Some(&serde_json::Value::Null)
		);
		assert_eq!(
			device.get("device_brand_name_null_flavor"),
			Some(&serde_json::Value::Null)
		);
		for (owner, child_id, value) in [
			("followUpTypes", &original_code_id, "2"),
			("remedialActions", &remedial_id, "10"),
			("deviceProblemCodes", &problem_id, "1234567"),
		] {
			let rows = device[owner].as_array().ok_or("missing device code rows")?;
			assert_eq!(rows.len(), 1);
			assert_eq!(rows[0]["id"], *child_id);
			assert_eq!(rows[0]["value_code"], value);
		}
		assert_eq!(
			crate::case_editor_contract_web::audit::case_audit_ids(&mm, &ctx, &id)
				.await?,
			updated_audits
		);
	}
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"followUpTypes":[
				{"id":original_code_id,"valueCode":"2"},
				{"id":original_code_id,"valueCode":"2"}
			]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, duplicate_code_unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(duplicate_code_unchanged, code_edited);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,
			"deviceBrandName":null,
			"deviceBrandNameNullFlavor":null,
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}]
		},{"deviceBrandName":"DEVICE-2"}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let devices = saved["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing FDA devices")?;
	assert_eq!(devices.len(), 2, "{saved}");
	assert_eq!(devices[0]["id"], original_device_id, "{saved}");
	let second_id = devices[1]["id"].clone();
	let second_created_at = devices[1]["created_at"].clone();
	let second_before_sibling_edit = devices[1].clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":original_device_id,"deviceBrandName":"DEVICE-1-EDITED",
			"followUpTypes":[{"id":original_code_id,"valueCode":"1"}]
		},{"id":second_id,"deviceBrandName":"DEVICE-2"}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, after_sibling_edit) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		after_sibling_edit["data"]["drug"]["fdaDevices"][1],
		second_before_sibling_edit,
		"{after_sibling_edit}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,
			"deviceBrandName":"DEVICE-2"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let devices = saved["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing FDA devices")?;
	assert_eq!(devices.len(), 1, "{saved}");
	assert_eq!(devices[0]["id"], second_id, "{saved}");
	assert_eq!(devices[0]["created_at"], second_created_at, "{saved}");
	let retained_sequence = devices[0]["sequence_number"].clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[
			{"id":second_id,"deviceBrandName":"DEVICE-2"},
			{"deviceBrandName":"DEVICE-3"}
		]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, appended) = get_json(&app, &cookie, &uri).await?;
	let appended_devices = appended["data"]["drug"]["fdaDevices"]
		.as_array()
		.ok_or("missing appended FDA devices")?;
	assert_eq!(appended_devices.len(), 2, "{appended}");
	assert_eq!(appended_devices[0]["id"], second_id, "{appended}");
	assert_eq!(
		appended_devices[0]["sequence_number"], retained_sequence,
		"{appended}"
	);
	assert_ne!(
		appended_devices[1]["sequence_number"], retained_sequence,
		"{appended}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,"deviceBrandName":"DEVICE-2"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[
			{"id":second_id,"deviceBrandName":"DEVICE-2"},
			{"id":second_id,"deviceBrandName":"DEVICE-2"}
		]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
	);
	let (status, other) = post_json(
		&app,
		&cookie,
		&format!("{base}/DG/rows"),
		json!({"authorities":["fda"],"rows":{"drug":{
			"drugCharacterization":"1", "medicinalProduct":"OTHER", "sequenceNumber":2,
			"fdaDevices":[{"deviceBrandName":"FOREIGN","followUpTypes":[{"valueCode":"2"}]}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{other}");
	let other_id = other["rowId"].as_str().ok_or("missing other drug id")?;
	let (_, other_saved) =
		get_json(&app, &cookie, &format!("{base}/DG/rows/{other_id}")).await?;
	let foreign_device_id =
		other_saved["data"]["drug"]["fdaDevices"][0]["id"].clone();
	let foreign_code_id = other_saved["data"]["drug"]["fdaDevices"][0]
		["followUpTypes"][0]["id"]
		.clone();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":foreign_device_id,"deviceBrandName":"FOREIGN"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["fda"],"rows":{"drug":{"fdaDevices":[{
			"id":second_id,"deviceBrandName":"DEVICE-2",
			"followUpTypes":[{"id":foreign_code_id,"valueCode":"2"}]
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (_, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		unchanged["data"]["drug"]["fdaDevices"],
		saved["data"]["drug"]["fdaDevices"]
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		saved["data"]["drug"]["fdaDevices"].as_array().map(Vec::len),
		Some(0),
		"{saved}"
	);
	Ok(())
}
