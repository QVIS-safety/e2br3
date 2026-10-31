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
		deleted_update_status: StatusCode::OK,
		restore: Some(format!("{item}/restore")),
		restore_status: StatusCode::OK,
	}
}

#[allow(clippy::too_many_arguments)]
fn hard_contract(
	name: &'static str,
	table: &'static str,
	id: Uuid,
	collection: String,
	wrong_collection: String,
	update: Value,
	changed_field: &'static str,
	changed_value: Value,
) -> RowLifecycleContract {
	RowLifecycleContract {
		name,
		table,
		id,
		item: format!("{collection}/{id}"),
		item_readable: true,
		collection,
		wrong_item: format!("{wrong_collection}/{id}"),
		update,
		changed_field,
		changed_value,
		delete_status: StatusCode::NO_CONTENT,
		repeat_delete_status: StatusCode::NOT_FOUND,
		deleted_get_status: StatusCode::NOT_FOUND,
		deleted_update_status: StatusCode::NOT_FOUND,
		restore: None,
		restore_status: StatusCode::OK,
	}
}

#[serial]
#[tokio::test]
async fn report_rows_enforce_scope_permission_lock_lifecycle_and_audit() -> Result<()>
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

	let message_path = format!("/api/cases/{case_id}/message-header");
	let message_create = json!({"data":{
		"case_id":case_id,
		"message_number":"LC-MESSAGE-1",
		"message_sender_identifier":"LC-SENDER",
		"message_receiver_identifier":"LC-RECEIVER",
		"message_date":"20261031120000"
	}});
	let (message_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&message_path,
		message_create.clone(),
		"message header",
	)
	.await?;
	let message_contract = SingletonLifecycleContract {
		name: "message header",
		table: "message_headers",
		id: message_id,
		path: message_path,
		update: json!({"data":{"message_number":"LC-MESSAGE-2"}}),
		changed_field: "message_number",
		changed_value: json!("LC-MESSAGE-2"),
		create: message_create,
		delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::NOT_FOUND,
	};

	let receiver_path = format!("/api/cases/{case_id}/receiver");
	let receiver_create =
		json!({"data":{"case_id":case_id,"organization_name":"Lifecycle receiver"}});
	let (receiver_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&receiver_path,
		receiver_create.clone(),
		"receiver information",
	)
	.await?;
	let receiver_contract = SingletonLifecycleContract {
		name: "receiver information",
		table: "receiver_information",
		id: receiver_id,
		path: receiver_path,
		update: json!({"data":{"organization_name":"Updated lifecycle receiver"}}),
		changed_field: "organization_name",
		changed_value: json!("Updated lifecycle receiver"),
		create: receiver_create,
		delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::OK,
	};

	let report_path = format!("/api/cases/{case_id}/safety-report");
	let report_create = json!({"data":{
		"case_id":case_id,
		"safety_report_id":"LC-SR-1",
		"transmission_date":"20261031120000",
		"report_type":"1",
		"date_first_received_from_source":"20261031",
		"date_of_most_recent_information":"20261031"
	}});
	let (report_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&report_path,
		report_create.clone(),
		"safety report identification",
	)
	.await?;
	let report_contract = SingletonLifecycleContract {
		name: "safety report identification",
		table: "safety_report_identification",
		id: report_id,
		path: report_path.clone(),
		update: json!({"data":{"receiver_organization":"Lifecycle authority"}}),
		changed_field: "receiver_organization",
		changed_value: json!("Lifecycle authority"),
		create: report_create,
		delete_status: StatusCode::NO_CONTENT,
		deleted_get_status: StatusCode::NOT_FOUND,
	};

	let soft_specs = [
		(
			"other case identifier",
			"other_case_identifiers",
			format!("/api/cases/{case_id}/other-identifiers"),
			format!("/api/cases/{wrong_case_id}/other-identifiers"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"source_of_identifier":"Lifecycle source","case_identifier":"LC-CASE-1"}}),
			json!({"data":{"case_identifier":"LC-CASE-2"}}),
			"case_identifier",
			json!("LC-CASE-2"),
		),
		(
			"documents held by sender",
			"documents_held_by_sender",
			format!("{report_path}/documents"),
			format!("/api/cases/{wrong_case_id}/safety-report/documents"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"title":"Lifecycle document"}}),
			json!({"data":{"title":"Updated lifecycle document"}}),
			"title",
			json!("Updated lifecycle document"),
		),
		(
			"literature reference",
			"literature_references",
			format!("{report_path}/literature"),
			format!("/api/cases/{wrong_case_id}/safety-report/literature"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"reference_text":"Lifecycle reference"}}),
			json!({"data":{"reference_text":"Updated lifecycle reference"}}),
			"reference_text",
			json!("Updated lifecycle reference"),
		),
		(
			"primary source",
			"primary_sources",
			format!("{report_path}/primary-sources"),
			format!("/api/cases/{wrong_case_id}/safety-report/primary-sources"),
			json!({"data":{"case_id":case_id,"sequence_number":1,"organization":"Lifecycle source"}}),
			json!({"data":{"organization":"Updated lifecycle source"}}),
			"organization",
			json!("Updated lifecycle source"),
		),
	];
	let mut soft_contracts = Vec::new();
	for (name, table, collection, wrong, create, update, field, value) in soft_specs
	{
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		soft_contracts.push(soft_contract(
			name, table, id, collection, wrong, update, field, value,
		));
	}

	let sender_collection = format!("{report_path}/senders");
	let sender_create =
		json!({"data":{"case_id":case_id,"organization_name":"Lifecycle sender"}});
	let (sender_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&sender_collection,
		sender_create.clone(),
		"sender information",
	)
	.await?;
	let sender_contract = hard_contract(
		"sender information",
		"sender_information",
		sender_id,
		sender_collection,
		format!("/api/cases/{wrong_case_id}/safety-report/senders"),
		json!({"data":{"organization_name":"Updated lifecycle sender"}}),
		"organization_name",
		json!("Updated lifecycle sender"),
	);

	let study_collection = format!("{report_path}/studies");
	let study_create =
		json!({"data":{"case_id":case_id,"study_name":"Lifecycle study"}});
	let (study_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&study_collection,
		study_create.clone(),
		"study information",
	)
	.await?;
	let study_contract = hard_contract(
		"study information",
		"study_information",
		study_id,
		study_collection.clone(),
		format!("/api/cases/{wrong_case_id}/safety-report/studies"),
		json!({"data":{"study_name":"Updated lifecycle study"}}),
		"study_name",
		json!("Updated lifecycle study"),
	);
	for (name, table, suffix, create, update, field, value) in [
		(
			"study registration number",
			"study_registration_numbers",
			"registrations",
			json!({"data":{"study_information_id":study_id,"sequence_number":1,"registration_number":"LC-REG-1","country_code":"KR"}}),
			json!({"data":{"registration_number":"LC-REG-2"}}),
			"registration_number",
			json!("LC-REG-2"),
		),
		(
			"study FDA cross-reported IND",
			"study_fda_cross_reported_inds",
			"fda-cross-reported-inds",
			json!({"data":{"study_information_id":study_id,"sequence_number":1,"ind_number":"LC-IND-1"}}),
			json!({"data":{"ind_number":"LC-IND-2"}}),
			"ind_number",
			json!("LC-IND-2"),
		),
	] {
		let collection = format!("{study_collection}/{study_id}/{suffix}");
		let (id, _) =
			create_lifecycle_row(&app, &cookie, &collection, create, name).await?;
		soft_contracts.insert(
			0,
			soft_contract(
				name,
				table,
				id,
				collection,
				format!("/api/cases/{wrong_case_id}/safety-report/studies/{study_id}/{suffix}"),
				update,
				field,
				value,
			),
		);
	}

	let source_collection = format!("/api/cases/{case_id}/source-documents");
	let source_create = json!({"data":{
		"case_id":case_id,"sequence_number":1,"source_document_name":"lifecycle.txt"
	}});
	let (source_id, _) = create_lifecycle_row(
		&app,
		&cookie,
		&source_collection,
		source_create.clone(),
		"source document",
	)
	.await?;
	let mut source_contract = hard_contract(
		"source document",
		"source_documents",
		source_id,
		source_collection,
		format!("/api/cases/{wrong_case_id}/source-documents"),
		json!({"data":{"source_document_name":"updated-lifecycle.txt"}}),
		"source_document_name",
		json!("updated-lifecycle.txt"),
	);
	source_contract.item_readable = false;

	for contract in soft_contracts.iter().chain([
		&sender_contract,
		&study_contract,
		&source_contract,
	]) {
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
	for contract in [&message_contract, &receiver_contract, &report_contract] {
		assert_singleton_policy(
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
	for contract in soft_contracts.iter().chain([
		&sender_contract,
		&study_contract,
		&source_contract,
	]) {
		assert_locked_row_mutation(&app, &cookie, &mm, &ctx, contract).await?;
	}
	for contract in [&message_contract, &receiver_contract, &report_contract] {
		assert_locked_singleton(&app, &cookie, &mm, &ctx, contract).await?;
	}
	toggle_case_lock(&app, &cookie, case_id).await?;

	for contract in &soft_contracts {
		assert_soft_row_lifecycle(&app, &cookie, &mm, &ctx, contract).await?;
	}
	assert_hard_row_lifecycle(
		&app,
		&cookie,
		&mm,
		&ctx,
		&sender_contract,
		sender_create,
	)
	.await?;
	assert_hard_row_lifecycle(
		&app,
		&cookie,
		&mm,
		&ctx,
		&study_contract,
		study_create,
	)
	.await?;
	for contract in &soft_contracts {
		if matches!(
			contract.table,
			"study_registration_numbers" | "study_fda_cross_reported_inds"
		) {
			let (status, response) = request_json_with_reason(
				&app,
				&cookie,
				"GET",
				contract.item.clone(),
				None,
				"read study child after hard parent delete",
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
				"delete study information lifecycle",
			)
			.await?;
		}
	}
	assert_hard_row_lifecycle(
		&app,
		&cookie,
		&mm,
		&ctx,
		&source_contract,
		source_create,
	)
	.await?;
	for contract in [&message_contract, &receiver_contract, &report_contract] {
		assert_hard_singleton_lifecycle(&app, &cookie, &mm, &ctx, contract).await?;
	}
	Ok(())
}
