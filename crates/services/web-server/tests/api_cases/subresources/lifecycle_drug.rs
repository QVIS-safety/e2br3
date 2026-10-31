use super::lifecycle_audit::*;
use super::lifecycle_parents::*;
use super::lifecycle_policy::*;
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
		deleted_get_status: StatusCode::OK,
		deleted_update_status,
		restore: Some(format!("{item}/restore")),
		restore_status: StatusCode::OK,
	}
}

#[serial]
#[tokio::test]
async fn drug_rows_enforce_scope_permission_lock_lifecycle_and_audit() -> Result<()>
{
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

	let drug_collection = format!("/api/cases/{case_id}/drugs");
	let (drug_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&drug_collection,
		json!({"data":{"case_id":case_id,"sequence_number":1,"drug_characterization":"1","medicinal_product":"Lifecycle drug"}}),
		"drug",
	)
	.await?;
	let drug_item = format!("{drug_collection}/{drug_id}");
	let drug_contract = RowLifecycleContract {
		name: "drug",
		table: "drug_information",
		id: drug_id,
		item: drug_item.clone(),
		item_readable: true,
		collection: drug_collection.clone(),
		wrong_item: format!("/api/cases/{wrong_case_id}/drugs/{drug_id}"),
		update: json!({"data":{"medicinal_product":"Updated lifecycle drug"}}),
		changed_field: "medicinal_product",
		changed_value: json!("Updated lifecycle drug"),
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::NOT_FOUND,
		deleted_update_status: StatusCode::NOT_FOUND,
		restore: Some(format!("{drug_item}/restore")),
		restore_status: StatusCode::OK,
	};
	let mut contracts = Vec::new();

	let child_specs = [
		(
			"drug active substance",
			"drug_active_substances",
			"active-substances",
			json!({"data":{"drug_id":drug_id,"sequence_number":1,"substance_name":"Lifecycle substance"}}),
			json!({"data":{"substance_name":"Updated lifecycle substance"}}),
			"substance_name",
			json!("Updated lifecycle substance"),
		),
		(
			"drug dosage",
			"dosage_information",
			"dosages",
			json!({"data":{"drug_id":drug_id,"sequence_number":1,"dose_value":10,"dose_unit":"mg"}}),
			json!({"data":{"dose_unit":"mL"}}),
			"dose_unit",
			json!("mL"),
		),
		(
			"drug indication",
			"drug_indications",
			"indications",
			json!({"data":{"drug_id":drug_id,"sequence_number":1,"indication_text":"Lifecycle indication"}}),
			json!({"data":{"indication_text":"Updated lifecycle indication"}}),
			"indication_text",
			json!("Updated lifecycle indication"),
		),
		(
			"drug device characteristic",
			"drug_device_characteristics",
			"device-characteristics",
			json!({"data":{"drug_id":drug_id,"sequence_number":1,"code":"LIFECYCLE","value_code":"ORIGINAL"}}),
			json!({"data":{"value_code":"UPDATED"}}),
			"value_code",
			json!("UPDATED"),
		),
		(
			"FDA device",
			"fda_device_information",
			"devices",
			json!({"data":{"drug_id":drug_id,"sequence_number":1,"device_brand_name":"Lifecycle device"}}),
			json!({"data":{"device_brand_name":"Updated lifecycle device"}}),
			"device_brand_name",
			json!("Updated lifecycle device"),
		),
	];
	let mut device_id = None;
	for (name, table, suffix, create, update, field, value) in child_specs {
		let collection = format!("{drug_item}/{suffix}");
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		if suffix == "devices" {
			device_id = Some(id);
		}
		let wrong = format!("/api/cases/{wrong_case_id}/drugs/{drug_id}/{suffix}");
		let contract = soft_contract(
			name,
			table,
			id,
			collection,
			wrong,
			update,
			field,
			value,
			StatusCode::OK,
		);
		contracts.push(contract);
	}

	let device_id = device_id.ok_or("FDA device setup did not return an ID")?;
	let code_collection = format!("{drug_item}/devices/{device_id}/codes");
	let (code_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&code_collection,
		json!({"data":{"device_id":device_id,"sequence_number":1,"element":"FDA.G.k.2.3.r.1","value_code":"LIFECYCLE"}}),
		"FDA device code",
	)
	.await?;
	contracts.push(RowLifecycleContract {
		name: "FDA device code",
		table: "fda_device_codes",
		id: code_id,
		item: format!("{code_collection}/{code_id}"),
		item_readable: true,
		collection: code_collection,
		wrong_item: format!("/api/cases/{wrong_case_id}/drugs/{drug_id}/devices/{device_id}/codes/{code_id}"),
		update: json!({"data":{"value_code":"UPDATED"}}),
		changed_field: "value_code",
		changed_value: json!("UPDATED"),
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::OK,
		deleted_update_status: StatusCode::OK,
		restore: Some(format!("{drug_item}/devices/{device_id}/codes/{code_id}/restore")),
		restore_status: StatusCode::NO_CONTENT,
	});

	let reaction_id = create_reaction(&app, &cookie, case_id).await?;
	let assessment_collection = format!("{drug_item}/reaction-assessments");
	let assessment_create = json!({"data":{
		"drug_id":drug_id,"reaction_id":reaction_id,"expectedness":"1"
	}});
	let (assessment_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&assessment_collection,
		assessment_create.clone(),
		"drug reaction assessment",
	)
	.await?;
	let assessment_contract = RowLifecycleContract {
		name: "drug reaction assessment",
		table: "drug_reaction_assessments",
		id: assessment_id,
		item: format!("{assessment_collection}/{assessment_id}"),
		item_readable: true,
		collection: assessment_collection.clone(),
		wrong_item: format!("/api/cases/{wrong_case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}"),
		update: json!({"data":{"expectedness":"2"}}),
		changed_field: "expectedness",
		changed_value: json!("2"),
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NOT_FOUND,
		deleted_get_status: StatusCode::NOT_FOUND,
		deleted_update_status: StatusCode::NOT_FOUND,
		restore: None,
		restore_status: StatusCode::OK,
	};

	let related_collection =
		format!("{assessment_collection}/{assessment_id}/relatedness");
	let (related_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&related_collection,
		json!({"data":{"drug_reaction_assessment_id":assessment_id,"sequence_number":1,"source_of_assessment":"Sponsor","result_of_assessment":"1"}}),
		"relatedness assessment",
	)
	.await?;
	contracts.push(RowLifecycleContract {
		name: "relatedness assessment",
		table: "relatedness_assessments",
		id: related_id,
		item: format!("{related_collection}/{related_id}"),
		item_readable: true,
		collection: related_collection,
		wrong_item: format!("/api/cases/{wrong_case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{related_id}"),
		update: json!({"data":{"result_of_assessment":"2"}}),
		changed_field: "result_of_assessment",
		changed_value: json!("2"),
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::OK,
		deleted_update_status: StatusCode::OK,
		restore: Some(format!("{assessment_collection}/{assessment_id}/relatedness/{related_id}/restore")),
		restore_status: StatusCode::OK,
	});
	let case_rows = contracts
		.iter()
		.chain([&assessment_contract, &drug_contract])
		.collect::<Vec<_>>();
	assert_case_soft_parent_preserves_rows(
		&app,
		&cookie,
		"adminpwd",
		&mm,
		&ctx,
		case_id,
		&case_rows,
		&[],
	)
	.await?;
	assert_nested_parent_contract(
		&app,
		&cookie,
		&mm,
		&ctx,
		&drug_contract,
		&case_rows[..case_rows.len() - 1],
	)
	.await?;
	let device_contract = contracts
		.iter()
		.find(|contract| contract.table == "fda_device_information")
		.ok_or("FDA device lifecycle contract is missing")?;
	let code_contract = contracts
		.iter()
		.find(|contract| contract.table == "fda_device_codes")
		.ok_or("FDA device code lifecycle contract is missing")?;
	assert_nested_parent_contract(
		&app,
		&cookie,
		&mm,
		&ctx,
		device_contract,
		&[code_contract],
	)
	.await?;

	for contract in contracts
		.iter()
		.chain([&assessment_contract, &drug_contract])
	{
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
	toggle_case_lock(&app, &cookie, case_id).await?;
	for contract in contracts
		.iter()
		.chain([&assessment_contract, &drug_contract])
	{
		assert_locked_row_mutation(&app, &cookie, &mm, &ctx, contract).await?;
	}
	toggle_case_lock(&app, &cookie, case_id).await?;

	for contract in &contracts {
		assert_soft_row_lifecycle(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_hard_row_lifecycle(
		&app,
		&cookie,
		&mm,
		&ctx,
		&assessment_contract,
		assessment_create,
	)
	.await?;
	let (status, response) = request_json_with_reason(
		&app,
		&cookie,
		"GET",
		format!("{assessment_collection}/{assessment_id}/relatedness/{related_id}"),
		None,
		"read relatedness after hard parent delete",
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{response}");
	assert_cascade_delete_audit(
		&mm,
		&ctx,
		"relatedness_assessments",
		related_id,
		"delete drug reaction assessment lifecycle",
	)
	.await?;
	assert_soft_row_lifecycle(&app, &cookie, &mm, &ctx, &drug_contract).await?;
	Ok(())
}
