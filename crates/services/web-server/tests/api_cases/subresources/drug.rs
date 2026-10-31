use super::support::*;
use crate::validation_common::{get_json as get_value, post_json as post_value};

#[serial]
#[tokio::test]
async fn fda_device_replace_rolls_back_on_constraint_violation() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let uri = format!("/api/cases/{case_id}/drugs/{drug_id}/devices/replace");

	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		uri.clone(),
		json!({ "data": { "devices": [{ "device_brand_name": "Original" }] } }),
		"New Data",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, _) = put_json_with_audit_reason(
		&app,
		&cookie,
		uri,
		json!({
			"data": {
				"devices": [{
					"device_brand_name": "Replacement",
					"follow_up_types": [{ "value_code": "9" }]
				}]
			}
		}),
		"Edited Data",
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/devices"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let payload: Value = serde_json::from_slice(&body)?;
	assert_eq!(payload["data"].as_array().map(Vec::len), Some(1));
	assert_eq!(payload["data"][0]["device_brand_name"], "Original");
	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_device_replace_normalizes_blank_null_flavors() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let uri = format!("/api/cases/{case_id}/drugs/{drug_id}/devices/replace");

	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		uri.clone(),
		json!({
			"data": {
				"devices": [{
					"device_brand_name_null_flavor": "",
					"common_device_name_null_flavor": "  "
				}]
			}
		}),
		"New Data",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/devices"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let payload: Value = serde_json::from_slice(&body)?;
	let devices = payload["data"]
		.as_array()
		.ok_or("device data is not an array")?;
	assert_eq!(devices.len(), 1);
	let device = devices[0]
		.as_object()
		.ok_or("device row is not an object")?;
	let id = device
		.get("id")
		.and_then(Value::as_str)
		.ok_or("missing device id")?;
	assert!(!Uuid::parse_str(id)?.is_nil());
	assert_eq!(device.get("drug_id"), Some(&json!(drug_id)));
	assert!(device.contains_key("device_brand_name_null_flavor"));
	assert!(device.contains_key("common_device_name_null_flavor"));
	assert!(device["device_brand_name_null_flavor"].is_null());
	assert!(device["common_device_name_null_flavor"].is_null());

	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		uri,
		json!({
			"data": {
				"devices": [{
					"device_brand_name_null_flavor": "NI",
					"common_device_name_null_flavor": "NI"
				}]
			}
		}),
		"Edited Data",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/devices"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK);
	let payload: Value = serde_json::from_slice(&body)?;
	let devices = payload["data"]
		.as_array()
		.ok_or("device data is not an array")?;
	assert_eq!(devices.len(), 1);
	let device = devices[0]
		.as_object()
		.ok_or("device row is not an object")?;
	let id = device
		.get("id")
		.and_then(Value::as_str)
		.ok_or("missing device id")?;
	assert!(!Uuid::parse_str(id)?.is_nil());
	assert_eq!(device.get("drug_id"), Some(&json!(drug_id)));
	assert!(device.contains_key("device_brand_name_null_flavor"));
	assert!(device.contains_key("common_device_name_null_flavor"));
	assert_eq!(device["device_brand_name_null_flavor"], "NI");
	assert_eq!(device["common_device_name_null_flavor"], "NI");
	Ok(())
}

#[serial]
#[tokio::test]
async fn dg_kr_substance_fields_create_read_and_update_independently() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let substance_payload = json!({
		"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"substance_name": "Substance A",
			"substance_termid": "BASE-SUB",
			"substance_termid_version": "BASE-SV1",
			"mfds_id": "KR-SUB",
			"mfds_version": "KR-SV1"
		}
	});

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		substance_payload,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let created: Value = serde_json::from_slice(&body)?;
	let substance_id = extract_id(&body)?;
	assert_eq!(created["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(created["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(created["data"]["mfds_id"], "KR-SUB");
	assert_eq!(created["data"]["mfds_version"], "KR-SV1");

	let update_payload = json!({
		"data": {
			"mfds_id": "KR-SUB-2",
			"mfds_version": "KR-SV2"
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
		update_payload,
		"update KR substance fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(updated["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(updated["data"]["mfds_id"], "KR-SUB-2");
	assert_eq!(updated["data"]["mfds_version"], "KR-SV2");

	let (status, body) = get_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
		),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let fetched: Value = serde_json::from_slice(&body)?;
	assert_eq!(fetched["data"]["substance_termid"], "BASE-SUB");
	assert_eq!(fetched["data"]["substance_termid_version"], "BASE-SV1");
	assert_eq!(fetched["data"]["mfds_id"], "KR-SUB-2");
	assert_eq!(fetched["data"]["mfds_version"], "KR-SV2");

	Ok(())
}

#[serial]
#[tokio::test]
async fn dg_kr_product_fields_create_read_and_update_independently() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let create_payload = json!({
		"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Base Product",
			"mpid": "BASE-MPID",
			"mpid_version": "BASE-V1",
			"mfds_mpid": "KR-MPID",
			"mfds_mpid_version": "KR-V1"
		}
	});

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs"),
		create_payload,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let created: Value = serde_json::from_slice(&body)?;
	assert_eq!(created["data"]["mpid"], "BASE-MPID");
	assert_eq!(created["data"]["mpid_version"], "BASE-V1");
	assert_eq!(created["data"]["mfds_mpid"], "KR-MPID");
	assert_eq!(created["data"]["mfds_mpid_version"], "KR-V1");
	let drug_id = created["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let fetched: Value = serde_json::from_slice(&body)?;
	assert_eq!(fetched["data"]["mpid"], "BASE-MPID");
	assert_eq!(fetched["data"]["mpid_version"], "BASE-V1");
	assert_eq!(fetched["data"]["mfds_mpid"], "KR-MPID");
	assert_eq!(fetched["data"]["mfds_mpid_version"], "KR-V1");

	let update_payload = json!({
		"data": {
			"mfds_mpid": "KR-MPID-2",
			"mfds_mpid_version": "KR-V2"
		}
	});
	let (status, body) = put_json_with_audit_reason(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
		update_payload,
		"update KR product fields",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let updated: Value = serde_json::from_slice(&body)?;
	assert_eq!(updated["data"]["mpid"], "BASE-MPID");
	assert_eq!(updated["data"]["mpid_version"], "BASE-V1");
	assert_eq!(updated["data"]["mfds_mpid"], "KR-MPID-2");
	assert_eq!(updated["data"]["mfds_mpid_version"], "KR-V2");

	Ok(())
}

#[serial]
#[tokio::test]
async fn drug_child_id_is_hidden_under_a_different_drug() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let first_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let second_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let first_drug_id = create_drug(&app, &cookie, first_case_id).await?;
	let second_drug_id = create_drug(&app, &cookie, second_case_id).await?;
	let (status, created) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{first_case_id}/drugs/{first_drug_id}/active-substances"
		),
		json!({"data": {
			"drug_id": first_drug_id,
			"sequence_number": 1,
			"substance_name": "contract",
			"substance_termid": "TERM-CONTRACT",
			"substance_termid_version": "2026-01",
			"strength_value": 1.0,
			"strength_unit": "mg"
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let id = extract_id(&created)?;
	let (status, _) = get_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{second_case_id}/drugs/{second_drug_id}/active-substances/{id}"
		),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND);
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_active_substance_soft_delete_restore() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"substance_name": "Soft delete substance"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let row_id = extract_id(&body)?;
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances/{row_id}"),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	let (status, body) = get_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("data is not an array")?;
	assert!(
		rows.iter()
			.all(|row| row["id"].as_str() != Some(&row_id.to_string())),
		"soft-deleted row should be excluded from default list"
	);

	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{row_id}/restore"
		),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], row_id.to_string());
	assert_eq!(value["data"]["deleted"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_drug_subresources_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"substance_name": "Substance",
		"substance_termid": "TERM-1",
		"substance_termid_version": "2026-01",
		"strength_value": 12.5,
		"strength_unit": "mg"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["substance_name"], "Substance");
	assert_eq!(value["data"]["substance_termid"], "TERM-1");
	assert_eq!(value["data"]["substance_termid_version"], "2026-01");
	assert_eq!(value["data"]["strength_unit"], "mg");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"dose_value": 10,
		"dose_unit": "mg",
		"number_of_units": 0.5,
		"frequency_unit": "d",
		"duration_value": 3,
		"duration_unit": "800",
		"batch_lot_number": "LOT-1",
		"dosage_text": "10 mg daily",
		"dose_form": "tablet",
		"dose_form_termid": "DF-1",
		"dose_form_termid_version": "2026-01",
		"route_of_administration": "001",
		"route_termid": "ROUTE-1",
		"route_termid_version": "2026-02",
		"parent_route": "parent oral",
		"parent_route_termid": "PROUTE-1",
		"parent_route_termid_version": "2026-03"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		body,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["dose_unit"], "mg");
	assert_eq!(value["data"]["number_of_units"], "0.50000");
	assert_eq!(value["data"]["frequency_unit"], "d");
	assert!(value["data"].get("frequency_value").is_none());
	assert_eq!(value["data"]["duration_unit"], "800");
	assert_eq!(value["data"]["batch_lot_number"], "LOT-1");
	assert_eq!(value["data"]["dosage_text"], "10 mg daily");
	assert_eq!(value["data"]["dose_form_termid"], "DF-1");
	assert_eq!(value["data"]["route_termid"], "ROUTE-1");
	assert_eq!(value["data"]["route_of_administration"], "001");
	assert_eq!(value["data"]["parent_route_termid"], "PROUTE-1");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"indication_text": "test",
		"indication_meddra_version": "27.1",
		"indication_meddra_code": "10000001"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["indication_text"], "test");
	assert_eq!(value["data"]["indication_meddra_version"], "27.1");
	assert_eq!(value["data"]["indication_meddra_code"], "10000001");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 2,
		"indication_text_null_flavor": "NASK"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["indication_text"], Value::Null);
	assert_eq!(value["data"]["indication_text_null_flavor"], "NASK");

	let body = json!({"data": {
		"drug_id": drug_id,
		"sequence_number": 1,
		"code": "FDA.G.k.12.r.3",
		"code_system": "2.16.840.1.113883.3.989.2.1.1.19",
		"code_display_name": "Problem Code",
		"value_type": "CE",
		"value_code": "1",
		"value_code_system": "FDA",
		"value_display_name": "Problem"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["code"], "FDA.G.k.12.r.3");
	assert_eq!(
		value["data"]["code_system"],
		"2.16.840.1.113883.3.989.2.1.1.19"
	);
	assert_eq!(value["data"]["value_type"], "CE");
	assert_eq!(value["data"]["value_code"], "1");
	assert_eq!(value["data"]["value_code_system"], "FDA");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_kr_device_characteristics_round_trip_via_api() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("KR-DVC-{suffix}");

	let (status, raw_body) = post_json(
		&app,
		&cookie,
		"/api/cases".to_string(),
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?;

	let (status, body) = post_value(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "KR Device Product"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;

	let (status, body) = post_value(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"code": "KR_DVC_PROBC",
				"value_code": "PROB-1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = post_value(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({
			"data": {
				"drug_id": drug_id,
				"sequence_number": 2,
				"code": "KR_DVC_MFR",
				"value_value": "KR Maker"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = get_value(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let rows = body["data"].as_array().ok_or("missing rows")?;
	assert_eq!(rows.len(), 2, "{rows:?}");
	assert!(
		rows.iter().any(|row| {
			row["code"].as_str() == Some("KR_DVC_PROBC")
				&& row["value_code"].as_str() == Some("PROB-1")
		}),
		"{rows:?}"
	);
	assert!(
		rows.iter().any(|row| {
			row["code"].as_str() == Some("KR_DVC_MFR")
				&& row["value_value"].as_str() == Some("KR Maker")
		}),
		"{rows:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_relatedness_assessment_endpoints_ok() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let reaction_id = create_reaction(&app, &cookie, case_id).await?;
	let assessment_id =
		create_reaction_assessment(&app, &cookie, case_id, drug_id, reaction_id)
			.await?;

	let body = json!({"data": {
		"drug_reaction_assessment_id": assessment_id,
		"sequence_number": 1,
		"source_of_assessment": "Sponsor",
		"method_of_assessment": "Algorithm",
		"result_of_assessment": "1",
		"result_of_assessment_kr2": "KR result"
	}});
	let (status, body) = post_json(
		&app,
		&cookie,
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		body,
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(
		value["data"]["drug_reaction_assessment_id"],
		assessment_id.to_string()
	);
	assert_eq!(value["data"]["source_of_assessment"], "Sponsor");
	assert_eq!(value["data"]["method_of_assessment"], "Algorithm");
	assert_eq!(value["data"]["result_of_assessment"], "1");
	assert_eq!(value["data"]["result_of_assessment_kr2"], "KR result");

	Ok(())
}
