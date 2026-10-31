use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

async fn update_audit_count(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
) -> Result<i64> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let count = sqlx::query_scalar::<_, i64>(
		"SELECT count(*) FROM audit_logs WHERE table_name = $1 AND record_id = $2 AND action = 'UPDATE'",
	)
	.bind(table_name)
	.bind(record_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(count)
}

async fn audit_count(
	mm: &ModelManager,
	table_name: &str,
	record_id: Uuid,
) -> Result<i64> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let count = sqlx::query_scalar::<_, i64>(
		"SELECT count(*) FROM audit_logs WHERE table_name = $1 AND record_id = $2",
	)
	.bind(table_name)
	.bind(record_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(count)
}

fn list_row<'a>(value: &'a Value, id: Uuid) -> Result<&'a Value> {
	value["data"]
		.as_array()
		.ok_or("child list data is not an array")?
		.iter()
		.find(|row| row["id"].as_str() == Some(&id.to_string()))
		.ok_or_else(|| format!("child {id} missing from direct list").into())
}

#[serial]
#[tokio::test]
async fn parent_repeat_delete_emits_one_business_update_audit() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let receiver_id = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let product_id = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Repeat Delete Reporter {}", Uuid::new_v4()),
		"Repeat Delete Reporter Org",
	)
	.await?;
	let study_product = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&cookie,
		study_product,
		"fda",
	)
	.await?;
	let narrative = post_json_created(
		&app,
		&cookie,
		"/api/presaves/narratives".to_string(),
		json!({"data":{"rows":{"narrative":{
			"caseNarrative":"Repeat delete narrative"
		}}}}),
	)
	.await?;
	let narrative_id = data_rows_id(&narrative, "narrative")?;

	for (table_name, id, uri, deleted_pointer) in [
		(
			"sender_presaves",
			sender_id,
			format!("/api/presaves/senders/{sender_id}"),
			"/data/deleted",
		),
		(
			"receiver_presaves",
			receiver_id,
			format!("/api/presaves/receivers/{receiver_id}"),
			"/data/deleted",
		),
		(
			"product_presaves",
			product_id,
			format!("/api/presaves/products/{product_id}"),
			"/data/deleted",
		),
		(
			"reporter_presaves",
			reporter_id,
			format!("/api/presaves/reporters/{reporter_id}"),
			"/data/rows/reporter/deleted",
		),
		(
			"study_presaves",
			study_id,
			format!("/api/presaves/studies/{study_id}"),
			"/data/deleted",
		),
		(
			"narrative_presaves",
			narrative_id,
			format!("/api/presaves/narratives/{narrative_id}"),
			"/data/rows/narrative/deleted",
		),
	] {
		let before = update_audit_count(&mm, table_name, id).await?;
		for _ in 0..2 {
			let (status, value) =
				request_json(&app, &cookie, Method::DELETE, uri.clone(), None)
					.await?;
			assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
		}
		assert_eq!(update_audit_count(&mm, table_name, id).await?, before + 1);
		let deleted = get_json_ok(&app, &cookie, uri).await?;
		assert_eq!(deleted.pointer(deleted_pointer), Some(&json!(true)));
	}

	Ok(())
}

#[serial]
#[tokio::test]
async fn direct_soft_children_repeat_delete_restore_same_id_and_noop_audit(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let gateway_id =
		create_sender_gateway_via_api(&app, &cookie, sender_id, 1, "LIFECYCLE")
			.await?;
	let responsible_id = create_sender_responsible_person_via_api(
		&app,
		&cookie,
		sender_id,
		1,
		"Lifecycle",
	)
	.await?;
	let product_id = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let study_id =
		create_study_presave_for_product_via_api(&app, &cookie, product_id, "fda")
			.await?;
	let registration_id = create_study_registration_number_via_api(
		&app,
		&cookie,
		study_id,
		1,
		"LIFECYCLE-REG",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&cookie,
		study_id,
		1,
		product_id,
		"Lifecycle Product",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Lifecycle Reporter {}", Uuid::new_v4()),
		"Lifecycle Reporter Org",
	)
	.await?;
	let study_reporter_id = create_study_reporter_via_api(
		&app,
		&cookie,
		study_id,
		1,
		reporter_id,
		"Lifecycle Reporter Org",
	)
	.await?;

	for (table_name, collection_uri, item_uri, id, stable_field, stable_value) in [
		(
			"sender_presave_gateways",
			format!("/api/presaves/senders/{sender_id}/gateways"),
			format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"),
			gateway_id,
			"sender_identifier",
			"LIFECYCLE",
		),
		(
			"sender_presave_responsible_persons",
			format!("/api/presaves/senders/{sender_id}/responsible-persons"),
			format!(
				"/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"
			),
			responsible_id,
			"person_given_name",
			"Lifecycle",
		),
		(
			"study_presave_registration_numbers",
			format!("/api/presaves/studies/{study_id}/registration-numbers"),
			format!(
				"/api/presaves/studies/{study_id}/registration-numbers/{registration_id}"
			),
			registration_id,
			"registration_number",
			"LIFECYCLE-REG",
		),
		(
			"study_presave_products",
			format!("/api/presaves/studies/{study_id}/products"),
			format!("/api/presaves/studies/{study_id}/products/{study_product_id}"),
			study_product_id,
			"product_name",
			"Lifecycle Product",
		),
		(
			"study_presave_reporters",
			format!("/api/presaves/studies/{study_id}/reporters"),
			format!(
				"/api/presaves/studies/{study_id}/reporters/{study_reporter_id}"
			),
			study_reporter_id,
			"reporter_organization",
			"Lifecycle Reporter Org",
		),
	] {
		let baseline_audits = update_audit_count(&mm, table_name, id).await?;
		for _ in 0..2 {
			let (status, value) =
				request_json(&app, &cookie, Method::DELETE, item_uri.clone(), None)
					.await?;
			assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
		}
		let after_first_and_repeat = update_audit_count(&mm, table_name, id).await?;
		assert_eq!(after_first_and_repeat, baseline_audits + 1);

		let deleted = get_json_ok(&app, &cookie, item_uri.clone()).await?;
		assert_eq!(deleted["data"]["id"], id.to_string());
		assert_eq!(deleted["data"]["deleted"], true);
		assert_eq!(deleted["data"][stable_field], stable_value);
		let deleted_list =
			get_json_ok(&app, &cookie, collection_uri.clone()).await?;
		let deleted_row = list_row(&deleted_list, id)?;
		assert_eq!(deleted_row["deleted"], true);
		assert_eq!(deleted_row[stable_field], stable_value);

		for _ in 0..2 {
			let (status, restored) = request_json(
				&app,
				&cookie,
				Method::PATCH,
				item_uri.clone(),
				Some(json!({ "data": { "deleted": false } })),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{restored:?}");
			assert_eq!(restored["data"]["id"], id.to_string());
			assert_eq!(restored["data"]["deleted"], false);
			assert_eq!(
				update_audit_count(&mm, table_name, id).await?,
				after_first_and_repeat + 1,
				"restore should emit one business change; repeat should emit none"
			);
			let fresh = get_json_ok(&app, &cookie, item_uri.clone()).await?;
			assert_eq!(fresh["data"]["id"], id.to_string());
			assert_eq!(fresh["data"]["deleted"], false);
			assert_eq!(fresh["data"][stable_field], stable_value);
		}
		let restored = get_json_ok(&app, &cookie, item_uri.clone()).await?;
		assert_eq!(restored["data"]["id"], id.to_string());
		assert_eq!(restored["data"]["deleted"], false);
		assert_eq!(restored["data"][stable_field], stable_value);
		let restored_list = get_json_ok(&app, &cookie, collection_uri).await?;
		let matching = restored_list["data"]
			.as_array()
			.ok_or("child list data is not an array")?
			.iter()
			.filter(|row| row["id"].as_str() == Some(&id.to_string()))
			.collect::<Vec<_>>();
		assert_eq!(matching.len(), 1);
		assert_eq!(matching[0]["deleted"], false);
		assert_eq!(matching[0][stable_field], stable_value);
	}

	Ok(())
}

async fn assert_archived_parent_keeps_graph(
	app: &axum::Router,
	cookie: &str,
	parent_uri: String,
	details_uri: String,
	children: &[(&str, Uuid)],
) -> Result<()> {
	let before = get_json_ok(app, cookie, details_uri.clone()).await?;
	let mut before_counts = Vec::with_capacity(children.len());
	for (row_key, child_id) in children {
		let rows = before["data"]["rows"][*row_key]
			.as_array()
			.ok_or("missing graph child rows")?;
		assert!(rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&child_id.to_string())));
		before_counts.push((*row_key, rows.len(), *child_id));
	}

	let (status, value) =
		request_json(app, cookie, Method::DELETE, parent_uri.clone(), None).await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	let archived = get_json_ok(app, cookie, details_uri.clone()).await?;
	for (row_key, before_count, child_id) in &before_counts {
		let rows = archived["data"]["rows"][*row_key]
			.as_array()
			.ok_or("missing archived graph child rows")?;
		assert_eq!(rows.len(), *before_count);
		assert!(rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&child_id.to_string())));
	}

	for _ in 0..2 {
		let (status, restored) = request_json(
			app,
			cookie,
			Method::PATCH,
			parent_uri.clone(),
			Some(json!({ "data": { "deleted": false } })),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{restored:?}");
		assert_eq!(restored["data"]["deleted"], false);
	}
	let restored = get_json_ok(app, cookie, details_uri).await?;
	for (row_key, before_count, child_id) in before_counts {
		let rows = restored["data"]["rows"][row_key]
			.as_array()
			.ok_or("missing restored graph child rows")?;
		assert_eq!(rows.len(), before_count);
		assert!(rows
			.iter()
			.any(|row| row["id"].as_str() == Some(&child_id.to_string())));
	}
	Ok(())
}

#[serial]
#[tokio::test]
async fn parent_archive_restore_preserves_hard_soft_and_graph_only_children(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let gateway_id =
		create_sender_gateway_via_api(&app, &cookie, sender_id, 1, "PARENT").await?;
	let responsible_id = create_sender_responsible_person_via_api(
		&app, &cookie, sender_id, 1, "Parent",
	)
	.await?;
	assert_archived_parent_keeps_graph(
		&app,
		&cookie,
		format!("/api/presaves/senders/{sender_id}"),
		format!("/api/presaves/senders/{sender_id}/details"),
		&[
			("gateways", gateway_id),
			("responsiblePersons", responsible_id),
		],
	)
	.await?;

	let receiver_id = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let consignee_id = create_receiver_consignee_via_api(
		&app,
		&cookie,
		receiver_id,
		1,
		"Parent Lifecycle Consignee",
	)
	.await?;
	let route_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_id}/details"),
		json!({ "data": { "rows": { "routes": [{
			"sequenceNumber": 1,
			"authority": "fda",
			"receiverLabel": "Parent lifecycle route",
			"messageReceiverIdentifier": "CBER_IND",
			"conditionPage": "CI",
			"conditionFieldCode": "FDA_REPORT_TYPE",
			"conditionOperator": "Equal",
			"conditionValueCode": "3",
			"conditionValueLabel": "CBER IND"
		}] } } }),
	)
	.await?;
	let route_id = Uuid::parse_str(
		route_details["data"]["rows"]["routes"][0]["id"]
			.as_str()
			.ok_or("missing receiver route id")?,
	)?;
	assert_archived_parent_keeps_graph(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_id}"),
		format!("/api/presaves/receivers/{receiver_id}/details"),
		&[("consignees", consignee_id), ("routes", route_id)],
	)
	.await?;

	let product_id = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let substance_id = create_product_active_substance_via_api(
		&app,
		&cookie,
		product_id,
		1,
		"Parent lifecycle substance",
	)
	.await?;
	assert_archived_parent_keeps_graph(
		&app,
		&cookie,
		format!("/api/presaves/products/{product_id}"),
		format!("/api/presaves/products/{product_id}/details"),
		&[("activeSubstances", substance_id)],
	)
	.await?;

	let study_product_parent =
		create_product_presave_via_api(&app, &cookie, "fda").await?;
	let study_id = create_study_presave_for_product_via_api(
		&app,
		&cookie,
		study_product_parent,
		"fda",
	)
	.await?;
	let registration_id = create_study_registration_number_via_api(
		&app,
		&cookie,
		study_id,
		1,
		"PARENT-LIFECYCLE-REG",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&cookie,
		study_id,
		1,
		study_product_parent,
		"Parent Lifecycle Product",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Parent Lifecycle Reporter {}", Uuid::new_v4()),
		"Parent Lifecycle Reporter Org",
	)
	.await?;
	let study_reporter_id = create_study_reporter_via_api(
		&app,
		&cookie,
		study_id,
		1,
		reporter_id,
		"Parent Lifecycle Reporter Org",
	)
	.await?;
	let fda_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({ "data": { "rows": { "fdaCrossReportedInds": [{
			"sequenceNumber": 1,
			"indNumber": "PARENT-LIFECYCLE-IND"
		}] } } }),
	)
	.await?;
	let fda_id = Uuid::parse_str(
		fda_details["data"]["rows"]["fdaCrossReportedInds"][0]["id"]
			.as_str()
			.ok_or("missing FDA IND id")?,
	)?;
	assert_archived_parent_keeps_graph(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_id}"),
		format!("/api/presaves/studies/{study_id}/details"),
		&[
			("registrationNumbers", registration_id),
			("products", study_product_id),
			("reporters", study_reporter_id),
			("fdaCrossReportedInds", fda_id),
		],
	)
	.await?;

	let (status, value) = request_json(
		&app,
		&cookie,
		Method::DELETE,
		format!("/api/presaves/reporters/{reporter_id}"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::CONFLICT, "{value:?}");
	assert!(value.to_string().contains("reporter presave is in use"));
	let reporter = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/reporters/{reporter_id}"),
	)
	.await?;
	assert_eq!(reporter["data"]["rows"]["reporter"]["deleted"], false);

	let child_uri =
		format!("/api/presaves/studies/{study_id}/reporters/{study_reporter_id}");
	let (status, value) =
		request_json(&app, &cookie, Method::DELETE, child_uri.clone(), None).await?;
	assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	for _ in 0..2 {
		let (status, value) = request_json(
			&app,
			&cookie,
			Method::DELETE,
			format!("/api/presaves/reporters/{reporter_id}"),
			None,
		)
		.await?;
		assert_eq!(status, StatusCode::NO_CONTENT, "{value:?}");
	}
	for _ in 0..2 {
		let (status, restored) = request_json(
			&app,
			&cookie,
			Method::PATCH,
			format!("/api/presaves/reporters/{reporter_id}"),
			Some(json!({"data":{"rows":{"reporter":{"deleted":false}}}})),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{restored:?}");
		assert_eq!(
			restored["data"]["rows"]["reporter"]["id"],
			reporter_id.to_string()
		);
		assert_eq!(restored["data"]["rows"]["reporter"]["deleted"], false);
	}
	for _ in 0..2 {
		let (status, restored_child) = request_json(
			&app,
			&cookie,
			Method::PATCH,
			child_uri.clone(),
			Some(json!({"data":{"deleted":false}})),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{restored_child:?}");
		assert_eq!(restored_child["data"]["id"], study_reporter_id.to_string());
		assert_eq!(restored_child["data"]["deleted"], false);
	}
	let restored_child = get_json_ok(&app, &cookie, child_uri).await?;
	assert_eq!(restored_child["data"]["id"], study_reporter_id.to_string());
	assert_eq!(restored_child["data"]["deleted"], false);

	Ok(())
}

#[serial]
#[tokio::test]
async fn direct_and_graph_children_reject_wrong_parent_without_state_change(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let sender_a = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let sender_b = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let gateway_b =
		create_sender_gateway_via_api(&app, &cookie, sender_b, 1, "WRONG-PARENT")
			.await?;
	let responsible_b = create_sender_responsible_person_via_api(
		&app,
		&cookie,
		sender_b,
		1,
		"WrongParent",
	)
	.await?;
	let receiver_a = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let receiver_b = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let consignee_b = create_receiver_consignee_via_api(
		&app,
		&cookie,
		receiver_b,
		1,
		"Wrong Parent Consignee",
	)
	.await?;
	let receiver_b_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_b}/details"),
		json!({"data":{"rows":{"routes":[{
			"sequenceNumber":1,
			"authority":"fda",
			"receiverLabel":"Wrong Parent Route"
		}]}}}),
	)
	.await?;
	let route_b = Uuid::parse_str(
		receiver_b_details["data"]["rows"]["routes"][0]["id"]
			.as_str()
			.ok_or("missing wrong-parent route id")?,
	)?;
	let product_a = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let product_b = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let substance_b = create_product_active_substance_via_api(
		&app,
		&cookie,
		product_b,
		1,
		"Wrong Parent Substance",
	)
	.await?;
	let study_a =
		create_study_presave_for_product_via_api(&app, &cookie, product_a, "fda")
			.await?;
	let study_b =
		create_study_presave_for_product_via_api(&app, &cookie, product_b, "fda")
			.await?;
	let registration_b = create_study_registration_number_via_api(
		&app,
		&cookie,
		study_b,
		1,
		"WRONG-PARENT-REG",
	)
	.await?;
	let study_product_b = create_study_product_via_api(
		&app,
		&cookie,
		study_b,
		1,
		product_b,
		"Wrong Parent Study Product",
	)
	.await?;
	let reporter_b = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Wrong Parent Reporter {}", Uuid::new_v4()),
		"Wrong Parent Reporter Org",
	)
	.await?;
	let study_reporter_b = create_study_reporter_via_api(
		&app,
		&cookie,
		study_b,
		1,
		reporter_b,
		"Wrong Parent Reporter Org",
	)
	.await?;
	let study_b_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_b}/details"),
		json!({"data":{"rows":{"fdaCrossReportedInds":[{
			"sequenceNumber":1,
			"indNumber":"WRONG-PARENT-IND"
		}]}}}),
	)
	.await?;
	let fda_b = Uuid::parse_str(
		study_b_details["data"]["rows"]["fdaCrossReportedInds"][0]["id"]
			.as_str()
			.ok_or("missing wrong-parent FDA IND id")?,
	)?;
	let audit_targets = [
		("sender_presave_gateways", gateway_b),
		("sender_presave_responsible_persons", responsible_b),
		("receiver_presave_consignees", consignee_b),
		("receiver_presave_routes", route_b),
		("receiver_presaves", receiver_a),
		("product_presave_active_substances", substance_b),
		("study_presave_registration_numbers", registration_b),
		("study_presave_products", study_product_b),
		("study_presave_reporters", study_reporter_b),
		("study_presave_fda_cross_reported_ind_numbers", fda_b),
		("study_presaves", study_a),
	];
	let mut audit_before = Vec::with_capacity(audit_targets.len());
	for (table_name, id) in audit_targets {
		audit_before.push((table_name, id, audit_count(&mm, table_name, id).await?));
	}

	for (wrong_uri, owner_uri) in [
		(
			format!("/api/presaves/senders/{sender_a}/gateways/{gateway_b}"),
			format!("/api/presaves/senders/{sender_b}/gateways/{gateway_b}"),
		),
		(
			format!(
				"/api/presaves/senders/{sender_a}/responsible-persons/{responsible_b}"
			),
			format!(
				"/api/presaves/senders/{sender_b}/responsible-persons/{responsible_b}"
			),
		),
		(
			format!("/api/presaves/receivers/{receiver_a}/consignees/{consignee_b}"),
			format!("/api/presaves/receivers/{receiver_b}/consignees/{consignee_b}"),
		),
		(
			format!(
				"/api/presaves/products/{product_a}/active-substances/{substance_b}"
			),
			format!(
				"/api/presaves/products/{product_b}/active-substances/{substance_b}"
			),
		),
		(
			format!(
				"/api/presaves/studies/{study_a}/registration-numbers/{registration_b}"
			),
			format!(
				"/api/presaves/studies/{study_b}/registration-numbers/{registration_b}"
			),
		),
		(
			format!("/api/presaves/studies/{study_a}/products/{study_product_b}"),
			format!("/api/presaves/studies/{study_b}/products/{study_product_b}"),
		),
		(
			format!("/api/presaves/studies/{study_a}/reporters/{study_reporter_b}"),
			format!("/api/presaves/studies/{study_b}/reporters/{study_reporter_b}"),
		),
	] {
		let baseline = get_json_ok(&app, &cookie, owner_uri.clone()).await?;
		for (method, body) in [
			(Method::GET, None),
			(Method::PATCH, Some(json!({"data":{}}))),
			(Method::DELETE, None),
		] {
			let (status, value) =
				request_json(&app, &cookie, method, wrong_uri.clone(), body).await?;
			assert_eq!(status, StatusCode::NOT_FOUND, "{wrong_uri}: {value:?}");
		}
		let after = get_json_ok(&app, &cookie, owner_uri).await?;
		assert_eq!(after["data"], baseline["data"]);
	}

	let receiver_a_before = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
	)
	.await?;
	let receiver_b_before = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_b}/details"),
	)
	.await?;
	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PUT,
		format!("/api/presaves/receivers/{receiver_a}/details"),
		Some(json!({"data":{"rows":{
			"receiver":{"description":"must roll back"},
			"routes":[{"id":route_b,"receiverLabel":"WRONG"}]
		}}})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	let receiver_a_after = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_a}/details"),
	)
	.await?;
	let receiver_b_after = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_b}/details"),
	)
	.await?;
	assert_eq!(receiver_a_after["data"], receiver_a_before["data"]);
	assert_eq!(receiver_b_after["data"], receiver_b_before["data"]);

	let study_a_before = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_a}/details"),
	)
	.await?;
	let study_b_before = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_b}/details"),
	)
	.await?;
	let (status, value) = request_json(
		&app,
		&cookie,
		Method::PUT,
		format!("/api/presaves/studies/{study_a}/details"),
		Some(json!({"data":{"rows":{
			"study":{"studyName":"must roll back"},
			"fdaCrossReportedInds":[{"id":fda_b,"indNumber":"WRONG"}]
		}}})),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
	let study_a_after = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_a}/details"),
	)
	.await?;
	let study_b_after = get_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_b}/details"),
	)
	.await?;
	assert_eq!(study_a_after["data"], study_a_before["data"]);
	assert_eq!(study_b_after["data"], study_b_before["data"]);
	for (table_name, id, before) in audit_before {
		assert_eq!(audit_count(&mm, table_name, id).await?, before);
	}

	Ok(())
}
