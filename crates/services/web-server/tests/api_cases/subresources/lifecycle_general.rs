use super::lifecycle_audit::*;
use super::lifecycle_policy::*;
use super::lifecycle_singleton::*;
use super::lifecycle_support::*;
use super::support::*;

#[allow(clippy::too_many_arguments)]
fn soft_contract(
	name: &'static str,
	table: &'static str,
	id: Uuid,
	collection: String,
	wrong_collection: String,
	update: Value,
	changed_field: &'static str,
	changed_value: Value,
	deleted_update_status: StatusCode,
) -> RowLifecycleContract {
	let item = format!("{collection}/{id}");
	RowLifecycleContract {
		name,
		table,
		id,
		item: item.clone(),
		item_readable: true,
		collection,
		wrong_item: format!("{wrong_collection}/{id}"),
		update,
		changed_field,
		changed_value,
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: deleted_update_status,
		deleted_update_status,
		restore: Some(format!("{item}/restore")),
		restore_status: StatusCode::OK,
	}
}

#[serial]
#[tokio::test]
async fn general_case_rows_enforce_scope_permission_lock_lifecycle_and_audit(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other = seed_org_with_users(&mm, "otheradmin", "otherviewer").await?;
	let cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let viewer_cookie = cookie_header(
		&generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?.to_string(),
	);
	let other_cookie = cookie_header(
		&generate_web_token(&other.admin.email, other.admin.token_salt)?.to_string(),
	);
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let wrong_case_id = create_case(&app, &cookie, seed.org_id).await?;

	let narrative_path = format!("/api/cases/{case_id}/narrative");
	let narrative_create =
		json!({"data":{"case_id":case_id,"case_narrative":"Lifecycle narrative"}});
	let (narrative_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&narrative_path,
		narrative_create.clone(),
		"narrative information",
	)
	.await?;
	let narrative_contract = SingletonLifecycleContract {
		name: "narrative information",
		table: "narrative_information",
		id: narrative_id,
		path: narrative_path.clone(),
		update: json!({"data":{"case_narrative":"Updated lifecycle narrative"}}),
		changed_field: "case_narrative",
		changed_value: json!("Updated lifecycle narrative"),
		create: narrative_create,
		delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::OK,
	};

	let specs = [
		(
			"linked report number",
			"linked_report_numbers",
			format!("/api/cases/{case_id}/linked-reports"),
			format!("/api/cases/{wrong_case_id}/linked-reports"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"linked_report_number":"LC-1"}}),
			json!({"data":{"linked_report_number":"LC-2"}}),
			"linked_report_number",
			json!("LC-2"),
			StatusCode::OK,
		),
		(
			"sender diagnosis",
			"sender_diagnoses",
			format!("{narrative_path}/sender-diagnoses"),
			format!("/api/cases/{wrong_case_id}/narrative/sender-diagnoses"),
			json!({"data":{"narrative_id":narrative_id,"sequence_number":1,"diagnosis_meddra_code":"10012345"}}),
			json!({"data":{"diagnosis_meddra_code":"10054321"}}),
			"diagnosis_meddra_code",
			json!("10054321"),
			StatusCode::OK,
		),
		(
			"case summary information",
			"case_summary_information",
			format!("{narrative_path}/summaries"),
			format!("/api/cases/{wrong_case_id}/narrative/summaries"),
			json!({"data":{"narrative_id":narrative_id,"sequence_number":1,"language_code":"eng","summary_text":"Lifecycle summary"}}),
			json!({"data":{"summary_text":"Updated lifecycle summary"}}),
			"summary_text",
			json!("Updated lifecycle summary"),
			StatusCode::OK,
		),
		(
			"reaction",
			"reactions",
			format!("/api/cases/{case_id}/reactions"),
			format!("/api/cases/{wrong_case_id}/reactions"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"primary_source_reaction":"Lifecycle reaction"}}),
			json!({"data":{"primary_source_reaction":"Updated lifecycle reaction"}}),
			"primary_source_reaction",
			json!("Updated lifecycle reaction"),
			StatusCode::NOT_FOUND,
		),
		(
			"test result",
			"test_results",
			format!("/api/cases/{case_id}/test-results"),
			format!("/api/cases/{wrong_case_id}/test-results"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"test_name":"Lifecycle test"}}),
			json!({"data":{"test_name":"Updated lifecycle test"}}),
			"test_name",
			json!("Updated lifecycle test"),
			StatusCode::NOT_FOUND,
		),
	];
	let mut contracts = Vec::new();
	for (
		name,
		table,
		collection,
		wrong,
		create,
		update,
		field,
		value,
		deleted_update,
	) in specs
	{
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		contracts.push(soft_contract(
			name,
			table,
			id,
			collection,
			wrong,
			update,
			field,
			value,
			deleted_update,
		));
	}

	for contract in &contracts {
		assert_row_mutation_policy(
			&app,
			&cookie,
			&viewer_cookie,
			&other_cookie,
			&mm,
			&ctx,
			contract,
		)
		.await?;
	}
	let reaction_contract = contracts
		.iter()
		.find(|contract| contract.table == "reactions")
		.ok_or("reaction lifecycle contract is missing")?;
	let reaction_before =
		assert_row_visible(&app, &cookie, reaction_contract).await?;
	let reaction_value =
		reaction_before["data"][reaction_contract.changed_field].clone();
	let delete_via_put_reason = "soft delete reaction through legacy PUT";
	let audit_before =
		audit_rows(&mm, &ctx, reaction_contract.table, reaction_contract.id).await?;
	let (status, deleted) = request_json_with_reason(
		&app,
		&cookie,
		"PUT",
		reaction_contract.item.clone(),
		Some(json!({"data":{"deleted":true}})),
		delete_via_put_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{deleted}");
	assert_eq!(deleted["data"]["id"], reaction_contract.id.to_string());
	assert_eq!(deleted["data"]["deleted"], true);
	assert_row_hidden(&app, &cookie, reaction_contract).await?;
	let audit = assert_audit_appended(
		&mm,
		&ctx,
		reaction_contract.table,
		reaction_contract.id,
		&audit_before,
		"UPDATE",
		delete_via_put_reason,
		Some((false, true)),
		None,
	)
	.await?;
	let restore_reason = "restore reaction through legacy PUT false";
	let restored_value = json!("Legacy PUT restored reaction");
	let restore_body = json!({"data":{
		"deleted":false,
		"primary_source_reaction":restored_value.clone()
	}});
	let (status, restored) = request_json_with_reason(
		&app,
		&cookie,
		"PUT",
		reaction_contract.item.clone(),
		Some(restore_body.clone()),
		restore_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{restored}");
	assert_eq!(restored["data"]["id"], reaction_contract.id.to_string());
	let reaction_after =
		assert_row_visible(&app, &cookie, reaction_contract).await?;
	assert_eq!(
		reaction_after["data"][reaction_contract.changed_field],
		restored_value
	);
	let restored_audit = assert_audit_appended(
		&mm,
		&ctx,
		reaction_contract.table,
		reaction_contract.id,
		&audit,
		"UPDATE",
		restore_reason,
		Some((true, false)),
		Some((
			reaction_contract.changed_field,
			reaction_value,
			restored_value,
		)),
	)
	.await?;
	let (status, repeated) = request_json_with_reason(
		&app,
		&cookie,
		"PUT",
		reaction_contract.item.clone(),
		Some(restore_body),
		"repeat reaction legacy PUT false restore",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{repeated}");
	assert_eq!(repeated["data"]["id"], reaction_contract.id.to_string());
	assert_eq!(repeated["data"]["deleted"], false);
	assert_eq!(
		repeated["data"][reaction_contract.changed_field],
		json!("Legacy PUT restored reaction")
	);
	assert_eq!(
		audit_rows(&mm, &ctx, reaction_contract.table, reaction_contract.id).await?,
		restored_audit,
		"identical reaction legacy PUT false wrote an audit row"
	);
	assert_singleton_policy(
		&app,
		&cookie,
		&viewer_cookie,
		&other_cookie,
		&mm,
		&ctx,
		&narrative_contract,
	)
	.await?;
	toggle_case_lock(&app, &cookie, case_id).await?;
	for contract in &contracts {
		assert_locked_row_mutation(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_locked_singleton(&app, &cookie, &mm, &ctx, &narrative_contract).await?;
	toggle_case_lock(&app, &cookie, case_id).await?;

	for contract in &contracts {
		assert_soft_row_lifecycle(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_hard_singleton_lifecycle(&app, &cookie, &mm, &ctx, &narrative_contract)
		.await?;
	for contract in &contracts {
		if matches!(
			contract.table,
			"sender_diagnoses" | "case_summary_information"
		) {
			let (status, response) = request_json_with_reason(
				&app,
				&cookie,
				"GET",
				contract.item.clone(),
				None,
				"read narrative child after hard parent delete",
			)
			.await?;
			assert_eq!(
				status,
				StatusCode::NOT_FOUND,
				"{}: {response}",
				contract.name
			);
			assert_cascade_delete_audit(
				&mm,
				&ctx,
				contract.table,
				contract.id,
				"delete narrative information lifecycle",
			)
			.await?;
		}
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn field_notation_repeated_clear_and_lock_preserve_state_and_audit(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let other = seed_org_with_users(&mm, "otheradmin", "otherviewer").await?;
	let cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let viewer_cookie = cookie_header(
		&generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?.to_string(),
	);
	let other_cookie = cookie_header(
		&generate_web_token(&other.admin.email, other.admin.token_salt)?.to_string(),
	);
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let endpoint = format!("/api/cases/{case_id}/field-notation");
	let query = format!("{endpoint}?e2bCode=C.1.1");
	let payload = json!({"e2bCode":"C.1.1","notation":"lifecycle notation"});
	let (status, created) = request_json_with_reason(
		&app,
		&cookie,
		"PUT",
		endpoint.clone(),
		Some(payload.clone()),
		"create field notation lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{created}");
	let notation_id = Uuid::parse_str(
		created["id"]
			.as_str()
			.ok_or("field notation response is missing id")?,
	)?;
	let initial_audit =
		audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id).await?;
	assert!(initial_audit.iter().any(|row| {
		row.action == "CREATE"
			&& row.reason.as_deref() == Some("create field notation lifecycle")
			&& row.user_id == ctx.user_id()
			&& row.organization_id == ctx.organization_id()
			&& row.category.is_none()
			&& row
				.new_values
				.as_ref()
				.and_then(|value| value.get("notation"))
				== Some(&json!("lifecycle notation"))
			&& row
				.changed_fields
				.as_ref()
				.and_then(|value| value.get("notation"))
				.and_then(|value| value.get("new"))
				== Some(&json!("lifecycle notation"))
	}));

	for (actor, uri, expected) in [
		(&viewer_cookie, endpoint.clone(), StatusCode::FORBIDDEN),
		(&other_cookie, endpoint.clone(), StatusCode::NOT_FOUND),
	] {
		let before =
			read_json_state(&app, &cookie, std::slice::from_ref(&query)).await?;
		let audit =
			audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id).await?;
		for (method, operation_uri, body) in [
			("PUT", uri.clone(), Some(payload.clone())),
			("DELETE", format!("{uri}?e2bCode=C.1.1"), None),
		] {
			let (status, response) = request_json_with_reason(
				&app,
				actor,
				method,
				operation_uri,
				body,
				"rejected field notation lifecycle",
			)
			.await?;
			assert_eq!(status, expected, "{response}");
			assert_eq!(
				read_json_state(&app, &cookie, std::slice::from_ref(&query)).await?,
				before
			);
			assert_eq!(
				audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id)
					.await?,
				audit
			);
		}
	}

	toggle_case_lock(&app, &cookie, case_id).await?;
	let before =
		read_json_state(&app, &cookie, std::slice::from_ref(&query)).await?;
	let audit =
		audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id).await?;
	for (method, uri, body) in [
		("PUT", endpoint.clone(), Some(payload.clone())),
		("DELETE", query.clone(), None),
	] {
		let (status, response) = request_json_with_reason(
			&app,
			&cookie,
			method,
			uri,
			body,
			"locked field notation lifecycle",
		)
		.await?;
		assert_eq!(status, StatusCode::FORBIDDEN, "{response}");
		assert_eq!(
			read_json_state(&app, &cookie, std::slice::from_ref(&query)).await?,
			before
		);
		assert_eq!(
			audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id).await?,
			audit
		);
	}
	toggle_case_lock(&app, &cookie, case_id).await?;

	let delete_reason = "clear field notation lifecycle";
	let (status, deleted) = request_json_with_reason(
		&app,
		&cookie,
		"DELETE",
		query.clone(),
		None,
		delete_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{deleted}");
	assert!(deleted["id"].is_null());
	let deleted_audit = assert_audit_appended(
		&mm,
		&ctx,
		"case_e2b_field_notations",
		notation_id,
		&audit,
		"DELETE",
		delete_reason,
		None,
		None,
	)
	.await?;
	let (status, missing) = request_json_with_reason(
		&app,
		&cookie,
		"GET",
		query.clone(),
		None,
		"read cleared field notation",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{missing}");
	assert!(missing["id"].is_null());
	let (status, repeated) = request_json_with_reason(
		&app,
		&cookie,
		"DELETE",
		query.clone(),
		None,
		"repeat clear field notation lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{repeated}");
	assert!(repeated["id"].is_null());
	assert_eq!(
		audit_rows(&mm, &ctx, "case_e2b_field_notations", notation_id).await?,
		deleted_audit
	);

	let (status, recreated) = request_json_with_reason(
		&app,
		&cookie,
		"PUT",
		endpoint,
		Some(payload),
		"recreate field notation lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{recreated}");
	let recreated_id = Uuid::parse_str(
		recreated["id"]
			.as_str()
			.ok_or("recreated field notation is missing id")?,
	)?;
	assert_ne!(recreated_id, notation_id);
	let (status, fresh) = request_json_with_reason(
		&app,
		&cookie,
		"GET",
		query,
		None,
		"read recreated field notation",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{fresh}");
	assert_eq!(fresh["id"], recreated_id.to_string());
	Ok(())
}
