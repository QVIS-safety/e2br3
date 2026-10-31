use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::Method;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

struct ExpectedAudit {
	table_name: &'static str,
	record_id: Uuid,
	field: &'static str,
	old: Value,
	new: Value,
}

async fn assert_update_audits(
	mm: &lib_core::model::ModelManager,
	organization_id: Uuid,
	user_id: Uuid,
	reason: &str,
	category: &str,
	expected: &[ExpectedAudit],
) -> Result<()> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	for item in expected {
		let row = sqlx::query_as::<
			_,
			(
				Uuid,
				Uuid,
				String,
				Option<String>,
				Option<String>,
				Value,
				Value,
				Value,
			),
		>(
			r#"
			SELECT organization_id, user_id, action, reason_for_change,
			       change_category, changed_fields, old_values, new_values
			FROM audit_logs
			WHERE table_name = $1
			  AND record_id = $2
			  AND action = 'UPDATE'
			  AND changed_fields ? $3
			ORDER BY id DESC
			LIMIT 1
			"#,
		)
		.bind(item.table_name)
		.bind(item.record_id)
		.bind(item.field)
		.fetch_optional(&mut *tx)
		.await?
		.ok_or_else(|| {
			format!(
				"missing {}.{} UPDATE audit for {}",
				item.table_name, item.field, item.record_id
			)
		})?;
		let (
			recorded_org,
			recorded_user,
			action,
			recorded_reason,
			recorded_category,
			changed_fields,
			old_values,
			new_values,
		) = row;
		assert_eq!(recorded_org, organization_id);
		assert_eq!(recorded_user, user_id);
		assert_eq!(action, "UPDATE");
		assert_eq!(recorded_reason.as_deref(), Some(reason));
		assert_eq!(recorded_category.as_deref(), Some(category));
		assert_eq!(changed_fields[item.field]["old"], item.old);
		assert_eq!(changed_fields[item.field]["new"], item.new);
		assert_eq!(old_values[item.field], item.old);
		assert_eq!(new_values[item.field], item.new);
	}
	tx.rollback().await?;
	Ok(())
}

async fn audit_update_count(
	mm: &lib_core::model::ModelManager,
	records: &[(&str, Uuid)],
) -> Result<i64> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let mut count = 0;
	for (table_name, record_id) in records {
		count += sqlx::query_scalar::<_, i64>(
			"SELECT COUNT(*) FROM audit_logs
			 WHERE table_name=$1 AND record_id=$2 AND action='UPDATE'",
		)
		.bind(table_name)
		.bind(record_id)
		.fetch_one(&mut *tx)
		.await?;
	}
	tx.rollback().await?;
	Ok(count)
}

fn without_write_metadata(mut value: Value) -> Value {
	match &mut value {
		Value::Object(object) => {
			for key in ["createdAt", "updatedAt", "createdBy", "updatedBy"] {
				object.remove(key);
			}
			for child in object.values_mut() {
				*child = without_write_metadata(child.take());
			}
		}
		Value::Array(rows) => {
			for row in rows {
				*row = without_write_metadata(row.take());
			}
		}
		_ => {}
	}
	value
}

#[serial]
#[tokio::test]
async fn presave_parent_and_child_http_updates_record_complete_audit_rows(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let reason = "Edited Data: presave lifecycle audit matrix";
	let category = "Edited Data";

	let sender_id = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let gateway_id =
		create_sender_gateway_via_api(&app, &cookie, sender_id, 1, "AUDIT-OLD")
			.await?;
	let responsible_id = create_sender_responsible_person_via_api(
		&app, &cookie, sender_id, 1, "Audit",
	)
	.await?;
	let receiver_id = create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let consignee_id = create_receiver_consignee_via_api(
		&app,
		&cookie,
		receiver_id,
		1,
		"Audit Consignee",
	)
	.await?;
	let receiver_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/receivers/{receiver_id}/details"),
		json!({"data":{"rows":{"routes":[{
			"sequenceNumber":1,
			"authority":"fda",
			"receiverLabel":"Audit Route Old",
			"messageReceiverIdentifier":"CBER_IND",
			"conditionPage":"CI",
			"conditionFieldCode":"FDA_REPORT_TYPE",
			"conditionOperator":"Equal",
			"conditionValueCode":"3",
			"conditionValueLabel":"CBER IND"
		}]}}}),
	)
	.await?;
	let route_id = Uuid::parse_str(
		receiver_details["data"]["rows"]["routes"][0]["id"]
			.as_str()
			.ok_or("missing receiver route id")?,
	)?;
	let product_id = create_named_product_presave_for_sender_via_api(
		&app,
		&cookie,
		sender_id,
		format!("Audit Product {}", Uuid::new_v4()),
		"Audit Product Old",
	)
	.await?;
	let substance_id = create_product_active_substance_via_api(
		&app,
		&cookie,
		product_id,
		1,
		"Audit Substance Old",
	)
	.await?;
	let reporter_id = create_named_reporter_presave_via_api(
		&app,
		&cookie,
		format!("Audit Reporter {}", Uuid::new_v4()),
		"Audit Reporter Old",
	)
	.await?;
	let study_id =
		create_study_presave_for_product_via_api(&app, &cookie, product_id, "fda")
			.await?;
	let study_before =
		get_json_ok(&app, &cookie, format!("/api/presaves/studies/{study_id}"))
			.await?;
	let old_study_name = study_before["data"]["studyName"]
		.as_str()
		.ok_or("missing study_name")?
		.to_string();
	let registration_id = create_study_registration_number_via_api(
		&app,
		&cookie,
		study_id,
		1,
		"AUDIT-REG-OLD",
	)
	.await?;
	let study_product_id = create_study_product_via_api(
		&app,
		&cookie,
		study_id,
		1,
		product_id,
		"Audit Study Product Old",
	)
	.await?;
	let study_reporter_id = create_study_reporter_via_api(
		&app,
		&cookie,
		study_id,
		1,
		reporter_id,
		"Audit Study Reporter Old",
	)
	.await?;
	let study_details = put_json_ok(
		&app,
		&cookie,
		format!("/api/presaves/studies/{study_id}/details"),
		json!({"data":{"rows":{"fdaCrossReportedInds":[{
			"sequenceNumber":1,
			"indNumber":"AUDIT-IND-OLD"
		}]}}}),
	)
	.await?;
	let fda_ind_id = Uuid::parse_str(
		study_details["data"]["rows"]["fdaCrossReportedInds"][0]["id"]
			.as_str()
			.ok_or("missing FDA IND id")?,
	)?;
	let narrative = post_json_created(
		&app,
		&cookie,
		"/api/presaves/narratives".to_string(),
		json!({"data":{"rows":{"narrative":{
			"caseNarrative":"Audit Narrative Old"
		}}}}),
	)
	.await?;
	let narrative_id = data_rows_id(&narrative, "narrative")?;

	let operations = [
		(Method::PATCH, format!("/api/presaves/senders/{sender_id}"), json!({"data":{"city":"Audit City New"}})),
		(Method::PATCH, format!("/api/presaves/senders/{sender_id}/gateways/{gateway_id}"), json!({"data":{"sender_identifier":"AUDIT-NEW"}})),
		(Method::PATCH, format!("/api/presaves/senders/{sender_id}/responsible-persons/{responsible_id}"), json!({"data":{"department":"Audit Department New"}})),
		(Method::PATCH, format!("/api/presaves/receivers/{receiver_id}"), json!({"data":{"description":"Audit Receiver New"}})),
		(Method::PATCH, format!("/api/presaves/receivers/{receiver_id}/consignees/{consignee_id}"), json!({"data":{"name":"Audit Consignee New"}})),
		(Method::PUT, format!("/api/presaves/receivers/{receiver_id}/details"), json!({"data":{"rows":{"routes":[{"id":route_id,"receiverLabel":"Audit Route New"}]}}})),
		(Method::PATCH, format!("/api/presaves/products/{product_id}"), json!({"data":{"medicinalProduct":"Audit Product New"}})),
		(Method::PATCH, format!("/api/presaves/products/{product_id}/active-substances/{substance_id}"), json!({"data":{"substance_name":"Audit Substance New"}})),
		(Method::PATCH, format!("/api/presaves/reporters/{reporter_id}"), json!({"data":{"rows":{"reporter":{"organization":"Audit Reporter New"}}}})),
		(Method::PATCH, format!("/api/presaves/studies/{study_id}"), json!({"data":{"studyName":"Audit Study New"}})),
		(Method::PATCH, format!("/api/presaves/studies/{study_id}/registration-numbers/{registration_id}"), json!({"data":{"registration_number":"AUDIT-REG-NEW"}})),
		(Method::PATCH, format!("/api/presaves/studies/{study_id}/products/{study_product_id}"), json!({"data":{"product_name":"Audit Study Product New"}})),
		(Method::PATCH, format!("/api/presaves/studies/{study_id}/reporters/{study_reporter_id}"), json!({"data":{"reporter_organization":"Audit Study Reporter New"}})),
		(Method::PUT, format!("/api/presaves/studies/{study_id}/details"), json!({"data":{"rows":{"fdaCrossReportedInds":[{"id":fda_ind_id,"indNumber":"AUDIT-IND-NEW"}]}}})),
		(Method::PATCH, format!("/api/presaves/narratives/{narrative_id}"), json!({"data":{"rows":{"narrative":{"caseNarrative":"Audit Narrative New"}}}})),
		(Method::PUT, format!("/api/presaves/senders/{sender_id}/details"), json!({"data":{"rows":{
			"sender":{"city":"Audit City New"},
			"gateways":[{"id":gateway_id,"senderIdentifier":"AUDIT-NEW"}],
			"responsiblePersons":[{"id":responsible_id,"department":"Audit Department New"}]
		}}})),
		(Method::PUT, format!("/api/presaves/receivers/{receiver_id}/details"), json!({"data":{"rows":{
			"receiver":{"description":"Audit Receiver New"},
			"consignees":[{"id":consignee_id,"name":"Audit Consignee New"}],
			"routes":[{"id":route_id,"receiverLabel":"Audit Route New"}]
		}}})),
		(Method::PUT, format!("/api/presaves/products/{product_id}/details"), json!({"data":{"rows":{
			"product":{"medicinalProduct":"Audit Product New"},
			"activeSubstances":[{"id":substance_id,"substanceName":"Audit Substance New"}]
		}}})),
		(Method::PUT, format!("/api/presaves/studies/{study_id}/details"), json!({"data":{"rows":{
			"study":{"studyName":"Audit Study New"},
			"registrationNumbers":[{"id":registration_id,"registrationNumber":"AUDIT-REG-NEW"}],
			"products":[{"id":study_product_id,"productName":"Audit Study Product New"}],
			"reporters":[{"id":study_reporter_id,"reporterOrganization":"Audit Study Reporter New"}],
			"fdaCrossReportedInds":[{"id":fda_ind_id,"indNumber":"AUDIT-IND-NEW"}]
		}}})),
	];
	for (method, uri, body) in &operations {
		request_json_ok_with_audit_compliance(
			&app,
			&cookie,
			method.clone(),
			uri.clone(),
			body.clone(),
			reason,
			Some(category),
		)
		.await?;
	}

	assert_update_audits(
		&mm,
		seed.org_id,
		seed.admin.id,
		reason,
		category,
		&[
			ExpectedAudit {
				table_name: "sender_presaves",
				record_id: sender_id,
				field: "city",
				old: Value::Null,
				new: json!("Audit City New"),
			},
			ExpectedAudit {
				table_name: "sender_presave_gateways",
				record_id: gateway_id,
				field: "sender_identifier",
				old: json!("AUDIT-OLD"),
				new: json!("AUDIT-NEW"),
			},
			ExpectedAudit {
				table_name: "sender_presave_responsible_persons",
				record_id: responsible_id,
				field: "department",
				old: json!("Safety"),
				new: json!("Audit Department New"),
			},
			ExpectedAudit {
				table_name: "receiver_presaves",
				record_id: receiver_id,
				field: "description",
				old: Value::Null,
				new: json!("Audit Receiver New"),
			},
			ExpectedAudit {
				table_name: "receiver_presave_consignees",
				record_id: consignee_id,
				field: "name",
				old: json!("Audit Consignee"),
				new: json!("Audit Consignee New"),
			},
			ExpectedAudit {
				table_name: "receiver_presave_routes",
				record_id: route_id,
				field: "receiver_label",
				old: json!("Audit Route Old"),
				new: json!("Audit Route New"),
			},
			ExpectedAudit {
				table_name: "product_presaves",
				record_id: product_id,
				field: "medicinal_product",
				old: json!("Audit Product Old"),
				new: json!("Audit Product New"),
			},
			ExpectedAudit {
				table_name: "product_presave_active_substances",
				record_id: substance_id,
				field: "substance_name",
				old: json!("Audit Substance Old"),
				new: json!("Audit Substance New"),
			},
			ExpectedAudit {
				table_name: "reporter_presaves",
				record_id: reporter_id,
				field: "organization",
				old: json!("Audit Reporter Old"),
				new: json!("Audit Reporter New"),
			},
			ExpectedAudit {
				table_name: "study_presaves",
				record_id: study_id,
				field: "study_name",
				old: json!(old_study_name),
				new: json!("Audit Study New"),
			},
			ExpectedAudit {
				table_name: "study_presave_registration_numbers",
				record_id: registration_id,
				field: "registration_number",
				old: json!("AUDIT-REG-OLD"),
				new: json!("AUDIT-REG-NEW"),
			},
			ExpectedAudit {
				table_name: "study_presave_products",
				record_id: study_product_id,
				field: "product_name",
				old: json!("Audit Study Product Old"),
				new: json!("Audit Study Product New"),
			},
			ExpectedAudit {
				table_name: "study_presave_reporters",
				record_id: study_reporter_id,
				field: "reporter_organization",
				old: json!("Audit Study Reporter Old"),
				new: json!("Audit Study Reporter New"),
			},
			ExpectedAudit {
				table_name: "study_presave_fda_cross_reported_ind_numbers",
				record_id: fda_ind_id,
				field: "ind_number",
				old: json!("AUDIT-IND-OLD"),
				new: json!("AUDIT-IND-NEW"),
			},
			ExpectedAudit {
				table_name: "narrative_presaves",
				record_id: narrative_id,
				field: "case_narrative",
				old: json!("Audit Narrative Old"),
				new: json!("Audit Narrative New"),
			},
		],
	)
	.await?;

	let graph_uris = [
		format!("/api/presaves/senders/{sender_id}/details"),
		format!("/api/presaves/receivers/{receiver_id}/details"),
		format!("/api/presaves/products/{product_id}/details"),
		format!("/api/presaves/reporters/{reporter_id}"),
		format!("/api/presaves/studies/{study_id}/details"),
		format!("/api/presaves/narratives/{narrative_id}"),
	];
	let replay_records = [
		("sender_presaves", sender_id),
		("sender_presave_gateways", gateway_id),
		("sender_presave_responsible_persons", responsible_id),
		("receiver_presaves", receiver_id),
		("receiver_presave_consignees", consignee_id),
		("receiver_presave_routes", route_id),
		("product_presaves", product_id),
		("product_presave_active_substances", substance_id),
		("reporter_presaves", reporter_id),
		("study_presaves", study_id),
		("study_presave_registration_numbers", registration_id),
		("study_presave_products", study_product_id),
		("study_presave_reporters", study_reporter_id),
		("study_presave_fda_cross_reported_ind_numbers", fda_ind_id),
		("narrative_presaves", narrative_id),
	];
	let mut before_replay = Vec::new();
	for uri in &graph_uris {
		before_replay.push(get_json_ok(&app, &cookie, uri.clone()).await?);
	}
	let audit_count = audit_update_count(&mm, &replay_records).await?;
	for _ in 0..2 {
		for (method, uri, body) in &operations {
			request_json_ok_with_audit_compliance(
				&app,
				&cookie,
				method.clone(),
				uri.clone(),
				body.clone(),
				reason,
				Some(category),
			)
			.await?;
		}
	}
	for (uri, expected) in graph_uris.iter().zip(&before_replay) {
		let replayed = get_json_ok(&app, &cookie, uri.clone()).await?;
		assert_eq!(
			without_write_metadata(replayed["data"].clone()),
			without_write_metadata(expected["data"].clone()),
			"{uri}"
		);
	}
	assert_eq!(
		audit_update_count(&mm, &replay_records).await?,
		audit_count,
		"same-value graph replay must not emit UPDATE audits"
	);

	let other_sender = create_sender_presave_via_api(&app, &cookie, "fda").await?;
	let other_gateway =
		create_sender_gateway_via_api(&app, &cookie, other_sender, 1, "OTHER")
			.await?;
	let other_receiver =
		create_receiver_presave_via_api(&app, &cookie, "fda").await?;
	let other_consignee = create_receiver_consignee_via_api(
		&app,
		&cookie,
		other_receiver,
		1,
		"Other Consignee",
	)
	.await?;
	let other_product = create_product_presave_via_api(&app, &cookie, "fda").await?;
	let other_substance = create_product_active_substance_via_api(
		&app,
		&cookie,
		other_product,
		1,
		"Other Substance",
	)
	.await?;
	let other_study = create_study_presave_for_product_via_api(
		&app,
		&cookie,
		other_product,
		"fda",
	)
	.await?;
	let other_registration = create_study_registration_number_via_api(
		&app,
		&cookie,
		other_study,
		1,
		"OTHER-REG",
	)
	.await?;
	let rollback_records = [
		("sender_presaves", sender_id),
		("sender_presave_gateways", other_gateway),
		("receiver_presaves", receiver_id),
		("receiver_presave_consignees", other_consignee),
		("product_presaves", product_id),
		("product_presave_active_substances", other_substance),
		("study_presaves", study_id),
		("study_presave_registration_numbers", other_registration),
		("sender_presaves", other_sender),
		("receiver_presaves", other_receiver),
		("product_presaves", other_product),
		("study_presaves", other_study),
	];
	let rollback_audit_count = audit_update_count(&mm, &rollback_records).await?;
	let mut rollback_expected = Vec::new();
	for index in [0, 1, 2, 4] {
		rollback_expected
			.push(get_json_ok(&app, &cookie, graph_uris[index].clone()).await?);
	}
	let foreign_parent_uris = [
		format!("/api/presaves/senders/{other_sender}/details"),
		format!("/api/presaves/receivers/{other_receiver}/details"),
		format!("/api/presaves/products/{other_product}/details"),
		format!("/api/presaves/studies/{other_study}/details"),
	];
	let mut foreign_parent_expected = Vec::new();
	for uri in &foreign_parent_uris {
		foreign_parent_expected.push(get_json_ok(&app, &cookie, uri.clone()).await?);
	}
	let rollback_cases = [
		(
			format!("/api/presaves/senders/{sender_id}/details"),
			json!({"data":{"rows":{
				"sender":{"city":"must roll back"},
				"gateways":[{"id":other_gateway,"senderIdentifier":"must not move"}]
			}}}),
		),
		(
			format!("/api/presaves/receivers/{receiver_id}/details"),
			json!({"data":{"rows":{
				"receiver":{"description":"must roll back"},
				"consignees":[{"id":other_consignee,"name":"must not move"}]
			}}}),
		),
		(
			format!("/api/presaves/products/{product_id}/details"),
			json!({"data":{"rows":{
				"product":{"medicinalProduct":"must roll back"},
				"activeSubstances":[{"id":other_substance,"substanceName":"must not move"}]
			}}}),
		),
		(
			format!("/api/presaves/studies/{study_id}/details"),
			json!({"data":{"rows":{
				"study":{"studyName":"must roll back"},
				"registrationNumbers":[{"id":other_registration,"registrationNumber":"must not move"}]
			}}}),
		),
	];
	for ((uri, body), expected) in rollback_cases.iter().zip(&rollback_expected) {
		let (status, value) = request_json(
			&app,
			&cookie,
			Method::PUT,
			uri.clone(),
			Some(body.clone()),
		)
		.await?;
		assert_eq!(status, axum::http::StatusCode::BAD_REQUEST, "{value:?}");
		let after = get_json_ok(&app, &cookie, uri.clone()).await?;
		assert_eq!(after["data"], expected["data"], "{uri}");
	}
	for (uri, expected) in foreign_parent_uris.iter().zip(&foreign_parent_expected) {
		let after = get_json_ok(&app, &cookie, uri.clone()).await?;
		assert_eq!(after["data"], expected["data"], "{uri}");
	}
	assert_eq!(
		audit_update_count(&mm, &rollback_records).await?,
		rollback_audit_count,
		"wrong-parent graph requests must not emit UPDATE audits"
	);

	Ok(())
}
