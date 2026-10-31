use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_row_with_reason as patch_json,
	post_json, ROW_CHANGE_REASON,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn device_list_replacement_removes_children(
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
	let id =
		create_case_for_editor(&app, &cookie, "UI-REGRESSION", &["ich"]).await?;
	let base = format!("/api/cases/{id}/editor/pages");
	let (status, body) = post_json(&app, &cookie, &format!("{base}/DG/rows"), json!({"authorities":["ich"],"rows":{"drug":{
		"drugCharacterization":"1", "medicinalProduct":"DEVICE TEST",
        "mfdsDeviceInfo":{"serialNumber":"SN-1","udiManufacturingDate":"20260901","models":[{"value":"MODEL-1"}]},
        "fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}],
            "remedialActions":[{"valueCode":"1"}],"deviceProblemCodes":[{"valueCode":"C133496"}]}]
    }}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "1", "{saved}");
	assert_eq!(
		drug["fdaDevices"].as_array().map(Vec::len),
		Some(1),
		"{saved}"
	);
	assert!(
		!drug["deviceCharacteristics"]
			.as_array()
			.unwrap()
			.iter()
			.any(|r| r["value_value"] == "SN-1"),
		"{saved}"
	);
	let original_device = drug["fdaDevices"][0].clone();
	let case_uuid = uuid::Uuid::parse_str(&id)?;
	let device_id = original_device["id"].as_str().ok_or("missing device ID")?;
	let groups = ["followUpTypes", "remedialActions", "deviceProblemCodes"];
	let mut guarded = vec![
		(
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[]}}}),
		),
		(
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[original_device]}}}),
		),
	];
	for group in groups {
		let mut replacement = original_device.clone();
		replacement[group] = json!([]);
		guarded.push((
			"PATCH",
			uri.clone(),
			json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[replacement]}}}),
		));
	}
	super::assert_editor_request_guards(&app, &cookie, &mm, &seed, &uri, &guarded)
		.await?;
	let mut current_device = original_device.clone();
	for group in groups {
		let removed = current_device[group][0].clone();
		let projection = json!({"id":removed["id"]});
		let spec = json!({"table":"fda_device_codes","column":"deleted","record":{"source":"projection","path":"id"}});
		current_device[group] = json!([]);
		for _ in 0..2 {
			let before = crate::case_editor_contract_web::audit::capture(
				&mm,
				&ctx,
				case_uuid,
				&spec,
				&projection,
			)
			.await?;
			let (status, body) = patch_json(
				&app,
				&cookie,
				&uri,
				json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[current_device]}}}),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{body}");
			let (status, persisted) = get_json(&app, &cookie, &uri).await?;
			assert_eq!(status, StatusCode::OK, "{persisted}");
			assert_eq!(body["data"], persisted["data"]);
			assert_eq!(
				persisted["data"]["drug"]["fdaDevices"],
				json!([current_device]),
				"replacement changed another group or device ID"
			);
			crate::case_editor_contract_web::audit::assert_mutation(
				&mm,
				&ctx,
				case_uuid,
				&spec,
				before,
				&projection,
				ROW_CHANGE_REASON,
			)
			.await?;
		}
		let (status, before) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{before}");
		let audits = super::case_audit_ids(&mm, &ctx, &id).await?;
		let mut stale = current_device.clone();
		stale[group] = json!([removed]);
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[stale]}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::BAD_REQUEST, "stale {group} ID: {body}");
		assert_eq!(
			get_json(&app, &cookie, &uri).await?,
			(StatusCode::OK, before)
		);
		assert_eq!(super::case_audit_ids(&mm, &ctx, &id).await?, audits);
	}
	let device_spec = json!({"table":"fda_device_information","column":"deleted","record":{"source":"projection","path":"id"}});
	let device_projection = json!({"id":device_id});
	for rows in [
		json!({"drug":{"fdaDevices":[]}}),
		json!({"drug":{"fdaDevices":[]}}),
		json!({"drug":{}}),
	] {
		let before = crate::case_editor_contract_web::audit::capture(
			&mm,
			&ctx,
			case_uuid,
			&device_spec,
			&device_projection,
		)
		.await?;
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({"authorities":["ich"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let (status, persisted) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{persisted}");
		assert_eq!(body["data"], persisted["data"]);
		assert_eq!(persisted["data"]["drug"]["fdaDevices"], json!([]));
		crate::case_editor_contract_web::audit::assert_mutation(
			&mm,
			&ctx,
			case_uuid,
			&device_spec,
			before,
			&device_projection,
			ROW_CHANGE_REASON,
		)
		.await?;
	}
	let (status, before) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{before}");
	let audits = super::case_audit_ids(&mm, &ctx, &id).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[original_device]}}}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NOT_FOUND,
		"deleted device ID cannot be implicitly restored: {body}"
	);
	assert_eq!(
		get_json(&app, &cookie, &uri).await?,
		(StatusCode::OK, before)
	);
	assert_eq!(super::case_audit_ids(&mm, &ctx, &id).await?, audits);

	Ok(())
}
