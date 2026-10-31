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
			"receiverLabel":"Audit Route Old"
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
	];
	for (method, uri, body) in operations {
		request_json_ok_with_audit_compliance(
			&app,
			&cookie,
			method,
			uri,
			body,
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

	Ok(())
}
