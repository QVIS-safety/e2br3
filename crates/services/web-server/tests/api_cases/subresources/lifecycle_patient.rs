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
	delete_status: StatusCode,
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
		delete_status,
		repeat_delete_status: delete_status,
		deleted_get_status: StatusCode::OK,
		deleted_update_status: StatusCode::OK,
		restore: Some(format!("{item}/restore")),
		restore_status: StatusCode::OK,
	}
}

#[serial]
#[tokio::test]
async fn patient_rows_enforce_scope_permission_lock_lifecycle_and_audit(
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
	let patient_path = format!("/api/cases/{case_id}/patient");
	let patient_create = json!({"data":{"case_id":case_id,"patient_initials":"LC"}});
	let (patient_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&patient_path,
		patient_create.clone(),
		"patient information",
	)
	.await?;
	let patient_contract = SingletonLifecycleContract {
		name: "patient information",
		table: "patient_information",
		id: patient_id,
		path: patient_path.clone(),
		update: json!({"data":{"patient_initials":"UP"}}),
		changed_field: "patient_initials",
		changed_value: json!("UP"),
		create: patient_create,
		delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::NOT_FOUND,
	};

	let direct_specs = [
		(
			"patient identifier",
			"patient_identifiers",
			"identifiers",
			json!({"data":{"patient_id":patient_id,"sequence_number":1,"identifier_type_code":"1","identifier_value":"LC-ID"}}),
			json!({"data":{"identifier_value":"UPDATED-ID"}}),
			"identifier_value",
			json!("UPDATED-ID"),
			StatusCode::OK,
		),
		(
			"medical history episode",
			"medical_history_episodes",
			"medical-history",
			json!({"data":{"patient_id":patient_id,"sequence_number":1,"meddra_code":"10012345"}}),
			json!({"data":{"comments":"updated medical history"}}),
			"comments",
			json!("updated medical history"),
			StatusCode::NO_CONTENT,
		),
		(
			"past drug history",
			"past_drug_history",
			"past-drugs",
			json!({"data":{"patient_id":patient_id,"sequence_number":1,"drug_name":"Lifecycle drug"}}),
			json!({"data":{"drug_name":"Updated lifecycle drug"}}),
			"drug_name",
			json!("Updated lifecycle drug"),
			StatusCode::NO_CONTENT,
		),
		(
			"parent information",
			"parent_information",
			"parents",
			json!({"data":{"patient_id":patient_id,"parent_identification":"Lifecycle parent"}}),
			json!({"data":{"parent_identification":"Updated lifecycle parent"}}),
			"parent_identification",
			json!("Updated lifecycle parent"),
			StatusCode::NO_CONTENT,
		),
	];
	let mut contracts = Vec::new();
	let mut parent_id = None;
	for (name, table, suffix, create, update, field, value, delete_status) in
		direct_specs
	{
		let collection = format!("{patient_path}/{suffix}");
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		if suffix == "parents" {
			parent_id = Some(id);
		}
		contracts.push(soft_contract(
			name,
			table,
			id,
			collection,
			format!("/api/cases/{wrong_case_id}/patient/{suffix}"),
			update,
			field,
			value,
			delete_status,
		));
	}
	let parent_id = parent_id.ok_or("parent setup did not return an ID")?;

	for (name, table, suffix, create, update, field, value) in [
		(
			"parent medical history",
			"parent_medical_history",
			"medical-history",
			json!({"data":{"parent_id":parent_id,"sequence_number":1,"meddra_code":"10012345"}}),
			json!({"data":{"comments":"updated parent history"}}),
			"comments",
			json!("updated parent history"),
		),
		(
			"parent past drug history",
			"parent_past_drug_history",
			"past-drugs",
			json!({"data":{"parent_id":parent_id,"sequence_number":1,"drug_name":"Parent lifecycle drug"}}),
			json!({"data":{"drug_name":"Updated parent lifecycle drug"}}),
			"drug_name",
			json!("Updated parent lifecycle drug"),
		),
	] {
		let collection = format!("{patient_path}/parent/{parent_id}/{suffix}");
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		contracts.insert(
			0,
			soft_contract(
				name,
				table,
				id,
				collection,
				format!(
					"/api/cases/{wrong_case_id}/patient/parent/{parent_id}/{suffix}"
				),
				update,
				field,
				value,
				StatusCode::NO_CONTENT,
			),
		);
	}

	let death_collection = format!("{patient_path}/death-info");
	let death_create =
		json!({"data":{"patient_id":patient_id,"autopsy_performed":false}});
	let (death_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&death_collection,
		death_create.clone(),
		"patient death information",
	)
	.await?;
	let death_contract = RowLifecycleContract {
		name: "patient death information",
		table: "patient_death_information",
		id: death_id,
		item: format!("{death_collection}/{death_id}"),
		item_readable: true,
		collection: death_collection.clone(),
		wrong_item: format!(
			"/api/cases/{wrong_case_id}/patient/death-info/{death_id}"
		),
		update: json!({"data":{"autopsy_performed":true}}),
		changed_field: "autopsy_performed",
		changed_value: json!(true),
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NOT_FOUND,
		deleted_get_status: StatusCode::NOT_FOUND,
		deleted_update_status: StatusCode::NOT_FOUND,
		restore: None,
		restore_status: StatusCode::OK,
	};
	for (name, table, suffix) in [
		(
			"reported cause of death",
			"reported_causes_of_death",
			"reported-causes",
		),
		(
			"autopsy cause of death",
			"autopsy_causes_of_death",
			"autopsy-causes",
		),
	] {
		let collection = format!("{death_collection}/{death_id}/{suffix}");
		let (id, _) = create_lifecycle_row(
			&app,
			&cookie,
			&collection,
			json!({"data":{"death_info_id":death_id,"sequence_number":1,"meddra_code":"10012345","comments":"lifecycle"}}),
			name,
		)
		.await?;
		contracts.insert(
			0,
			soft_contract(
				name,
				table,
				id,
				collection,
				format!("/api/cases/{wrong_case_id}/patient/death-info/{death_id}/{suffix}"),
				json!({"data":{"comments":"updated lifecycle"}}),
				"comments",
				json!("updated lifecycle"),
				StatusCode::NO_CONTENT,
			),
		);
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
	assert_row_mutation_policy(
		&app,
		&cookie,
		&viewer_cookie,
		&other_cookie,
		&mm,
		&ctx,
		&death_contract,
	)
	.await?;
	assert_singleton_policy(
		&app,
		&cookie,
		&viewer_cookie,
		&other_cookie,
		&mm,
		&ctx,
		&patient_contract,
	)
	.await?;

	toggle_case_lock(&app, &cookie, case_id).await?;
	for contract in &contracts {
		assert_locked_row_mutation(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_locked_row_mutation(&app, &cookie, &mm, &ctx, &death_contract).await?;
	assert_locked_singleton(&app, &cookie, &mm, &ctx, &patient_contract).await?;
	toggle_case_lock(&app, &cookie, case_id).await?;

	for contract in &contracts {
		assert_soft_row_lifecycle(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_hard_row_lifecycle(
		&app,
		&cookie,
		&mm,
		&ctx,
		&death_contract,
		death_create,
	)
	.await?;
	for contract in &contracts {
		if matches!(
			contract.table,
			"reported_causes_of_death" | "autopsy_causes_of_death"
		) {
			let (status, response) = request_json_with_reason(
				&app,
				&cookie,
				"GET",
				contract.item.clone(),
				None,
				"read death cause after hard parent delete",
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
				"delete patient death information lifecycle",
			)
			.await?;
		}
	}
	assert_hard_singleton_lifecycle(&app, &cookie, &mm, &ctx, &patient_contract)
		.await?;
	for contract in &contracts {
		if !matches!(
			contract.table,
			"reported_causes_of_death" | "autopsy_causes_of_death"
		) {
			let (status, response) = request_json_with_reason(
				&app,
				&cookie,
				"GET",
				contract.item.clone(),
				None,
				"read patient child after hard patient delete",
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
				"delete patient information lifecycle",
			)
			.await?;
		}
	}
	Ok(())
}
