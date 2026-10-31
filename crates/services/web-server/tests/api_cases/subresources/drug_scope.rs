use super::support::*;

async fn request_json_method(
	app: &Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Vec<u8>)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if matches!(method, "POST" | "PUT" | "DELETE") {
		builder = builder.header("x-e2br3-reason-for-change", "scope regression");
	}
	let body = if let Some(body) = body {
		builder = builder.header("content-type", "application/json");
		Body::from(body.to_string())
	} else {
		Body::empty()
	};
	let response = app.clone().oneshot(builder.body(body)?).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?.to_vec();
	Ok((status, body))
}

#[serial]
#[tokio::test]
async fn generated_drug_child_routes_bind_drug_to_case_for_every_resource(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let foreign_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let owning_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, owning_case_id).await?;

	let resources = [
		(
			"devices",
			json!({"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"device_brand_name": "Scoped Device"
			}}),
			json!({"data": {"device_brand_name": "must not change"}}),
		),
		(
			"active-substances",
			json!({"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"substance_name": "Scoped Substance"
			}}),
			json!({"data": {"substance_name": "must not change"}}),
		),
		(
			"dosages",
			json!({"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"dose_value": 10,
				"dose_unit": "mg"
			}}),
			json!({"data": {"dose_unit": "must not change"}}),
		),
		(
			"indications",
			json!({"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"indication_text": "Scoped Indication"
			}}),
			json!({"data": {"indication_text": "must not change"}}),
		),
		(
			"device-characteristics",
			json!({"data": {
				"drug_id": drug_id,
				"sequence_number": 1,
				"code": "SCOPE-CODE",
				"value_code": "SCOPE-VALUE"
			}}),
			json!({"data": {"value_code": "must not change"}}),
		),
	];

	for (resource, create_body, update_body) in resources {
		let correct_collection =
			format!("/api/cases/{owning_case_id}/drugs/{drug_id}/{resource}");
		let (status, created) = post_json(
			&app,
			&cookie,
			correct_collection.clone(),
			create_body.clone(),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::CREATED,
			"{resource}: {}",
			String::from_utf8_lossy(&created)
		);
		let id = extract_id(&created)?;
		let correct_item = format!("{correct_collection}/{id}");
		let foreign_collection =
			format!("/api/cases/{foreign_case_id}/drugs/{drug_id}/{resource}");
		let foreign_item = format!("{foreign_collection}/{id}");

		for (method, uri, body) in [
			(
				"POST",
				foreign_collection.clone(),
				Some(create_body.clone()),
			),
			("GET", foreign_collection.clone(), None),
			("GET", foreign_item.clone(), None),
			("PUT", foreign_item.clone(), Some(update_body)),
			("DELETE", foreign_item.clone(), None),
		] {
			let (status, body) =
				request_json_method(&app, &cookie, method, uri, body).await?;
			assert_eq!(
				status,
				StatusCode::NOT_FOUND,
				"{resource} {method} crossed the case/drug boundary: {}",
				String::from_utf8_lossy(&body)
			);
		}

		let (status, body) = get_json(&app, &cookie, correct_item.clone()).await?;
		assert_eq!(status, StatusCode::OK, "{resource}");
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(value["data"]["id"], id.to_string(), "{resource}");
		assert_eq!(value["data"]["deleted"], false, "{resource}");
		let (status, body) =
			get_json(&app, &cookie, correct_collection.clone()).await?;
		assert_eq!(status, StatusCode::OK, "{resource}");
		let value: Value = serde_json::from_slice(&body)?;
		let rows = value["data"].as_array().ok_or("data is not an array")?;
		assert_eq!(rows.len(), 1, "{resource}: {rows:?}");
		assert_eq!(rows[0]["id"], id.to_string(), "{resource}");

		let (status, body) =
			delete_json(&app, &cookie, correct_item.clone()).await?;
		assert_eq!(
			status,
			StatusCode::NO_CONTENT,
			"{resource}: {}",
			String::from_utf8_lossy(&body)
		);
		let (status, body) =
			post_json(&app, &cookie, format!("{foreign_item}/restore"), json!({}))
				.await?;
		assert_eq!(
			status,
			StatusCode::NOT_FOUND,
			"{resource}: {}",
			String::from_utf8_lossy(&body)
		);
		let (status, body) = get_json(&app, &cookie, correct_item).await?;
		assert_eq!(status, StatusCode::OK, "{resource}");
		let value: Value = serde_json::from_slice(&body)?;
		assert_eq!(value["data"]["id"], id.to_string(), "{resource}");
		assert_eq!(value["data"]["deleted"], true, "{resource}");
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn deleted_drug_hides_children_and_restore_preserves_child_identity(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug(&app, &cookie, case_id).await?;
	let collection =
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances");
	let (status, body) = post_json(
		&app,
		&cookie,
		collection.clone(),
		json!({"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"substance_name": "Preserved Child"
		}}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let child_id = extract_id(&body)?;
	let item = format!("{collection}/{child_id}");

	let (status, body) = request_json_method(
		&app,
		&cookie,
		"DELETE",
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
		None,
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);
	for uri in [collection.clone(), item.clone()] {
		let (status, body) = get_json(&app, &cookie, uri).await?;
		assert_eq!(
			status,
			StatusCode::NOT_FOUND,
			"{}",
			String::from_utf8_lossy(&body)
		);
	}

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let restored_drug: Value = serde_json::from_slice(&body)?;
	assert_eq!(restored_drug["data"]["id"], drug_id.to_string());
	assert_eq!(restored_drug["data"]["deleted"], false);
	let (status, body) = get_json(&app, &cookie, item).await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], child_id.to_string());
	assert_eq!(value["data"]["substance_name"], "Preserved Child");
	assert_eq!(value["data"]["deleted"], false);
	let (status, body) = get_json(&app, &cookie, collection).await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let value: Value = serde_json::from_slice(&body)?;
	let rows = value["data"].as_array().ok_or("data is not an array")?;
	assert_eq!(rows.len(), 1, "{rows:?}");
	assert_eq!(rows[0]["id"], child_id.to_string());

	Ok(())
}

#[serial]
#[tokio::test]
async fn fda_device_code_routes_reject_a_matching_drug_from_a_different_case(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let first_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let second_case_id = create_case(&app, &cookie, seed.org_id).await?;
	let _first_drug_id = create_drug(&app, &cookie, first_case_id).await?;
	let second_drug_id = create_drug(&app, &cookie, second_case_id).await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("/api/cases/{second_case_id}/drugs/{second_drug_id}/devices"),
		json!({"data": {
			"drug_id": second_drug_id,
			"sequence_number": 1,
			"device_brand_name": "Original Device"
		}}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let device_id = extract_id(&body)?;
	let correct_collection = format!(
		"/api/cases/{second_case_id}/drugs/{second_drug_id}/devices/{device_id}/codes"
	);
	let (status, body) = post_json(
		&app,
		&cookie,
		correct_collection.clone(),
		json!({"data": {
			"device_id": device_id,
			"element": "device_problem",
			"sequence_number": 1,
			"value_code": "ORIGINAL"
		}}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let code_id = extract_id(&body)?;

	let foreign_collection = format!(
		"/api/cases/{first_case_id}/drugs/{second_drug_id}/devices/{device_id}/codes"
	);
	let foreign_item = format!("{foreign_collection}/{code_id}");
	for (method, uri, body) in [
		(
			"POST",
			foreign_collection.clone(),
			Some(json!({"data": {
				"device_id": device_id,
				"element": "device_problem",
				"sequence_number": 2,
				"value_code": "MUST-NOT-CREATE"
			}})),
		),
		("GET", foreign_collection, None),
		("GET", foreign_item.clone(), None),
		(
			"PUT",
			foreign_item.clone(),
			Some(json!({"data": {"value_code": "MUST-NOT-CHANGE"}})),
		),
		("DELETE", foreign_item.clone(), None),
	] {
		let (status, body) =
			request_json_method(&app, &cookie, method, uri, body).await?;
		assert_eq!(
			status,
			StatusCode::NOT_FOUND,
			"{method} unexpectedly crossed the case/drug boundary: {}",
			String::from_utf8_lossy(&body)
		);
	}

	let correct_item = format!("{correct_collection}/{code_id}");
	let (status, body) = get_json(&app, &cookie, correct_item.clone()).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["value_code"], "ORIGINAL");
	assert_eq!(value["data"]["deleted"], false);

	let (status, body) = delete_json(&app, &cookie, correct_item.clone()).await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let (status, body) =
		post_json(&app, &cookie, format!("{foreign_item}/restore"), json!({}))
			.await?;
	assert_eq!(
		status,
		StatusCode::NOT_FOUND,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let (status, body) = get_json(&app, &cookie, correct_item.clone()).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["deleted"], true);

	let (status, body) =
		post_json(&app, &cookie, format!("{correct_item}/restore"), json!({}))
			.await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);
	let correct_device = format!(
		"/api/cases/{second_case_id}/drugs/{second_drug_id}/devices/{device_id}"
	);
	let (status, body) = delete_json(&app, &cookie, correct_device.clone()).await?;
	assert_eq!(
		status,
		StatusCode::NO_CONTENT,
		"{}",
		String::from_utf8_lossy(&body)
	);

	for (method, uri, body) in [
		(
			"POST",
			correct_collection.clone(),
			Some(json!({"data": {
				"device_id": device_id,
				"element": "device_problem",
				"sequence_number": 2,
				"value_code": "MUST-NOT-CREATE-WHILE-DELETED"
			}})),
		),
		("GET", correct_collection.clone(), None),
		("GET", correct_item.clone(), None),
		(
			"PUT",
			correct_item.clone(),
			Some(json!({"data": {"value_code": "MUST-NOT-CHANGE-WHILE-DELETED"}})),
		),
		("DELETE", correct_item.clone(), None),
		("POST", format!("{correct_item}/restore"), Some(json!({}))),
	] {
		let (status, body) =
			request_json_method(&app, &cookie, method, uri, body).await?;
		assert_eq!(
			status,
			StatusCode::NOT_FOUND,
			"{method} accepted a code operation under a deleted device: {}",
			String::from_utf8_lossy(&body)
		);
	}

	let (status, body) = post_json(
		&app,
		&cookie,
		format!("{correct_device}/restore"),
		json!({}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
	let (status, body) = get_json(&app, &cookie, correct_item).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"]["id"], code_id.to_string());
	assert_eq!(value["data"]["value_code"], "ORIGINAL");
	assert_eq!(value["data"]["deleted"], false);
	let (status, body) = get_json(&app, &cookie, correct_collection).await?;
	assert_eq!(status, StatusCode::OK);
	let value: Value = serde_json::from_slice(&body)?;
	assert_eq!(value["data"].as_array().map(Vec::len), Some(1));
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
