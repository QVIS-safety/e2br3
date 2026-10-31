use super::super::audit::case_audit_ids;
use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::ModelManager;
use serde_json::json;
use serde_json::Value;

// Compare both cases and audit streams after each rejected mutation.
#[allow(clippy::too_many_arguments)]
async fn assert_foreign_patch_rejected_without_mutation(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	target_uri: &str,
	foreign_uri: &str,
	target_case_id: &str,
	foreign_case_id: &str,
	target_before: &Value,
	foreign_before: &Value,
	target_audit_before: &[i64],
	foreign_audit_before: &[i64],
	rows: Value,
) -> Result<()> {
	let (status, body) = patch_json(
		app,
		cookie,
		target_uri,
		json!({"authorities": ["ich"], "rows": rows}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, target_after) = get_json(app, cookie, target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_after}");
	assert_eq!(&target_after, target_before, "target case mutated");
	let (status, foreign_after) = get_json(app, cookie, foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_after}");
	assert_eq!(&foreign_after, foreign_before, "foreign case mutated");
	assert_eq!(
		case_audit_ids(mm, ctx, target_case_id).await?,
		target_audit_before,
		"rejected patch audited the target case"
	);
	assert_eq!(
		case_audit_ids(mm, ctx, foreign_case_id).await?,
		foreign_audit_before,
		"rejected patch audited the foreign case"
	);
	Ok(())
}

pub(crate) async fn isolates_study_children_and_rejects_foreign_ids(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let first_case =
		create_case_for_editor(&app, &cookie, "SI-ISOLATION-FIRST", &["fda"])
			.await?;
	let second_case =
		create_case_for_editor(&app, &cookie, "SI-ISOLATION-SECOND", &["fda"])
			.await?;

	for (case_id, suffix) in [(&first_case, "FIRST"), (&second_case, "SECOND")] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/editor/pages/SI"),
			json!({
				"authorities": ["fda"],
				"rows": {
					"studyInformation": {
						"studyName": format!("Study {suffix}"),
						"fdaCrossReportedIndNumbers": [{
							"indNumber": format!("IND-{suffix}"),
							"sequenceNumber": 1
						}]
					},
					"studyRegistrationNumbers": [{
						"registrationNumber": format!("REG-{suffix}"),
						"countryCode": "US",
						"sequenceNumber": 1
					}]
				}
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}

	let first_uri = format!("/api/cases/{first_case}/editor/pages/SI");
	let second_uri = format!("/api/cases/{second_case}/editor/pages/SI");
	let (status, first) = get_json(&app, &cookie, &first_uri).await?;
	assert_eq!(status, StatusCode::OK, "{first}");
	assert_eq!(
		first["rows"]["studyRegistrationNumbers"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		first["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"REG-FIRST"
	);
	assert_eq!(
		first["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"]
			.as_array()
			.unwrap()
			.len(),
		1
	);
	assert_eq!(
		first["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
			["ind_number"],
		"IND-FIRST"
	);

	let (status, second) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{second}");
	let foreign_study_id = second["rows"]["studyInformation"]["id"]
		.as_str()
		.ok_or("missing foreign study id")?;
	let foreign_registration_id = second["rows"]["studyRegistrationNumbers"][0]
		["id"]
		.as_str()
		.ok_or("missing registration id")?;
	let foreign_ind_id = second["rows"]["studyInformation"]
		["fdaCrossReportedIndNumbers"][0]["id"]
		.as_str()
		.ok_or("missing IND id")?;
	let (status, target_before) = get_json(&app, &cookie, &first_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_before}");
	let (status, foreign_before) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before}");
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let target_audit_before = case_audit_ids(&mm, &ctx, &first_case).await?;
	let foreign_audit_before = case_audit_ids(&mm, &ctx, &second_case).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&first_uri,
		json!({"authorities":["fda"],"rows":{
			"studyInformation":{"id":foreign_study_id,"studyName":"TAMPERED"},
			"studyRegistrationNumbers":[{
				"id":foreign_registration_id,"registrationNumber":"TAMPERED"
			}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, target_after) = get_json(&app, &cookie, &first_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_after}");
	assert_eq!(target_after, target_before, "target study mutated");
	let (status, foreign_after) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_after}");
	assert_eq!(foreign_after, foreign_before, "foreign study mutated");
	assert_eq!(
		case_audit_ids(&mm, &ctx, &first_case).await?,
		target_audit_before,
		"rejected study patch audited the target case"
	);
	assert_eq!(
		case_audit_ids(&mm, &ctx, &second_case).await?,
		foreign_audit_before,
		"rejected study patch audited the foreign case"
	);

	for rows in [
		json!({
			"studyRegistrationNumbers": [{
				"id": foreign_registration_id,
				"registrationNumber": "REG-TAMPERED",
				"sequenceNumber": 1
			}]
		}),
		json!({
			"studyRegistrationNumbers": [{
				"id": foreign_registration_id,
				"deleted": true
			}]
		}),
		json!({
			"studyInformation": {
				"fdaCrossReportedIndNumbers": [{
					"id": foreign_ind_id,
					"indNumber": "BAD-IND",
					"sequenceNumber": 1
				}]
			}
		}),
		json!({
			"studyInformation": {
				"fdaCrossReportedIndNumbers": [{
					"id": foreign_ind_id,
					"deleted": true
				}]
			}
		}),
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			&first_uri,
			json!({"authorities": ["fda"], "rows": rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	}

	let (status, second) = get_json(&app, &cookie, &second_uri).await?;
	assert_eq!(status, StatusCode::OK, "{second}");
	assert_eq!(
		second["rows"]["studyRegistrationNumbers"][0]["registration_number"],
		"REG-SECOND"
	);
	assert_eq!(
		second["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
			["ind_number"],
		"IND-SECOND"
	);
	Ok(())
}

pub(crate) async fn rejects_foreign_reporter_rows_without_mutation(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let target_case =
		create_case_for_editor(&app, &cookie, "RP-SCOPE-TARGET", &["ich"]).await?;
	let foreign_case =
		create_case_for_editor(&app, &cookie, "RP-SCOPE-FOREIGN", &["ich"]).await?;
	let target_uri = format!("/api/cases/{target_case}/editor/pages/RP");
	let foreign_uri = format!("/api/cases/{foreign_case}/editor/pages/RP");
	for (uri, prefix) in [(&target_uri, "TARGET"), (&foreign_uri, "FOREIGN")] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			uri,
			json!({"authorities":["ich"],"rows":{"primarySources":[
				{"sequenceNumber":1,"reporterGivenName":format!("{prefix}-ACTIVE")},
				{"sequenceNumber":2,"reporterGivenName":format!("{prefix}-DELETED")}
			]}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}
	let (status, foreign_created) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_created}");
	let foreign_active_id = foreign_created["rows"]["primarySources"][0]["id"]
		.as_str()
		.ok_or("missing foreign active reporter id")?;
	let foreign_deleted_id = foreign_created["rows"]["primarySources"][1]["id"]
		.as_str()
		.ok_or("missing foreign deleted reporter id")?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&foreign_uri,
		json!({"authorities":["ich"],"rows":{"primarySources":[{
			"id":foreign_deleted_id,"deleted":true
		}]}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, target_before) = get_json(&app, &cookie, &target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_before}");
	let (status, foreign_before) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before}");
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let target_audit_before = case_audit_ids(&mm, &ctx, &target_case).await?;
	let foreign_audit_before = case_audit_ids(&mm, &ctx, &foreign_case).await?;
	for rows in [
		json!({"primarySources":[{"id":foreign_active_id,"reporterGivenName":"TAMPERED"}]}),
		json!({"primarySources":[{"id":foreign_active_id,"deleted":true}]}),
		json!({"primarySources":[{"id":foreign_deleted_id,"deleted":false,"reporterGivenName":"RESTORED"}]}),
	] {
		assert_foreign_patch_rejected_without_mutation(
			&app,
			&cookie,
			&mm,
			&ctx,
			&target_uri,
			&foreign_uri,
			&target_case,
			&foreign_case,
			&target_before,
			&foreign_before,
			&target_audit_before,
			&foreign_audit_before,
			rows,
		)
		.await?;
	}
	Ok(())
}

pub(crate) async fn rejects_foreign_patient_rows_without_mutation(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let target_case =
		create_case_for_editor(&app, &cookie, "DM-SCOPE-TARGET", &["ich"]).await?;
	let foreign_case =
		create_case_for_editor(&app, &cookie, "DM-SCOPE-FOREIGN", &["ich"]).await?;
	let target_uri = format!("/api/cases/{target_case}/editor/pages/DM");
	let foreign_uri = format!("/api/cases/{foreign_case}/editor/pages/DM");
	for (uri, prefix) in [(&target_uri, "TARGET"), (&foreign_uri, "FOREIGN")] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			uri,
			json!({"authorities":["ich"],"rows":{
				"patientInformation":{"patientInitials":prefix},
				"patientIdentifiers":[
					{"sequenceNumber":1,"identifierTypeCode":"1","identifierValue":format!("{prefix}-ID-A")},
					{"sequenceNumber":2,"identifierTypeCode":"2","identifierValue":format!("{prefix}-ID-D")}
				],
				"medicalHistoryEpisodes":[
					{"sequenceNumber":1,"comments":format!("{prefix}-MH-A")},
					{"sequenceNumber":2,"comments":format!("{prefix}-MH-D")}
				],
				"deathInfo":{"dateOfDeath":"20261001"},
				"reportedCauses":[
					{"sequenceNumber":1,"causeText":format!("{prefix}-RC-A")},
					{"sequenceNumber":2,"causeText":format!("{prefix}-RC-D")}
				],
				"autopsyCauses":[
					{"sequenceNumber":1,"causeText":format!("{prefix}-AC-A")},
					{"sequenceNumber":2,"causeText":format!("{prefix}-AC-D")}
				],
				"parentInfo":{"parentIdentification":format!("{prefix}-PARENT")},
				"parentMedicalHistory":[
					{"sequenceNumber":1,"comments":format!("{prefix}-PM-A")},
					{"sequenceNumber":2,"comments":format!("{prefix}-PM-D")}
				],
				"parentPastDrugs":[
					{"sequenceNumber":1,"drugName":format!("{prefix}-PD-A")},
					{"sequenceNumber":2,"drugName":format!("{prefix}-PD-D")}
				]
			}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}
	let (status, foreign_created) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_created}");
	let rows = &foreign_created["rows"];
	let active_ids = [
		(
			"patientIdentifiers",
			rows["patientIdentifiers"][0]["id"]
				.as_str()
				.ok_or("missing active patient identifier id")?
				.to_string(),
			json!({"identifierValue":"TAMPERED"}),
		),
		(
			"medicalHistoryEpisodes",
			rows["medicalHistoryEpisodes"][0]["id"]
				.as_str()
				.ok_or("missing active medical history id")?
				.to_string(),
			json!({"comments":"TAMPERED"}),
		),
		(
			"reportedCauses",
			rows["reportedCauses"][0]["id"]
				.as_str()
				.ok_or("missing active reported cause id")?
				.to_string(),
			json!({"causeText":"TAMPERED"}),
		),
		(
			"autopsyCauses",
			rows["autopsyCauses"][0]["id"]
				.as_str()
				.ok_or("missing active autopsy cause id")?
				.to_string(),
			json!({"causeText":"TAMPERED"}),
		),
		(
			"parentMedicalHistory",
			rows["parentMedicalHistory"][0]["id"]
				.as_str()
				.ok_or("missing active parent medical history id")?
				.to_string(),
			json!({"comments":"TAMPERED"}),
		),
		(
			"parentPastDrugs",
			rows["parentPastDrugs"][0]["id"]
				.as_str()
				.ok_or("missing active parent past drug id")?
				.to_string(),
			json!({"drugName":"TAMPERED"}),
		),
	];
	let deleted_ids = [
		(
			"patientIdentifiers",
			rows["patientIdentifiers"][1]["id"]
				.as_str()
				.ok_or("missing deleted patient identifier id")?
				.to_string(),
		),
		(
			"medicalHistoryEpisodes",
			rows["medicalHistoryEpisodes"][1]["id"]
				.as_str()
				.ok_or("missing deleted medical history id")?
				.to_string(),
		),
		(
			"reportedCauses",
			rows["reportedCauses"][1]["id"]
				.as_str()
				.ok_or("missing deleted reported cause id")?
				.to_string(),
		),
		(
			"autopsyCauses",
			rows["autopsyCauses"][1]["id"]
				.as_str()
				.ok_or("missing deleted autopsy cause id")?
				.to_string(),
		),
		(
			"parentMedicalHistory",
			rows["parentMedicalHistory"][1]["id"]
				.as_str()
				.ok_or("missing deleted parent medical history id")?
				.to_string(),
		),
		(
			"parentPastDrugs",
			rows["parentPastDrugs"][1]["id"]
				.as_str()
				.ok_or("missing deleted parent past drug id")?
				.to_string(),
		),
	];
	let death_info_id = rows["deathInfo"]["id"]
		.as_str()
		.ok_or("missing death info id")?
		.to_string();
	let parent_info_id = rows["parentInfo"]["id"]
		.as_str()
		.ok_or("missing parent info id")?
		.to_string();
	let deleted_rows = deleted_ids
		.iter()
		.map(|(owner, id)| ((*owner).to_string(), json!([{"id":id,"deleted":true}])))
		.collect::<serde_json::Map<_, _>>();
	let (status, body) = patch_json(
		&app,
		&cookie,
		&foreign_uri,
		json!({"authorities":["ich"],"rows":deleted_rows}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let (status, target_before_parent_delete) =
		get_json(&app, &cookie, &target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_before_parent_delete}");
	let (status, foreign_before_parent_delete) =
		get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before_parent_delete}");
	let target_audit_before = case_audit_ids(&mm, &ctx, &target_case).await?;
	let foreign_audit_before = case_audit_ids(&mm, &ctx, &foreign_case).await?;
	for rows in [
		json!({"deathInfo":{"id":death_info_id,"dateOfDeath":"20260101"}}),
		json!({"parentInfo":{"id":parent_info_id,"parentIdentification":"TAMPERED"}}),
		json!({"parentInfo":{"id":parent_info_id,"deleted":true}}),
	] {
		assert_foreign_patch_rejected_without_mutation(
			&app,
			&cookie,
			&mm,
			&ctx,
			&target_uri,
			&foreign_uri,
			&target_case,
			&foreign_case,
			&target_before_parent_delete,
			&foreign_before_parent_delete,
			&target_audit_before,
			&foreign_audit_before,
			rows,
		)
		.await?;
	}
	for (owner, id, update) in &active_ids {
		let mut update = update.clone();
		update["id"] = json!(id);
		for row in [update, json!({"id":id,"deleted":true})] {
			assert_foreign_patch_rejected_without_mutation(
				&app,
				&cookie,
				&mm,
				&ctx,
				&target_uri,
				&foreign_uri,
				&target_case,
				&foreign_case,
				&target_before_parent_delete,
				&foreign_before_parent_delete,
				&target_audit_before,
				&foreign_audit_before,
				json!({(*owner):[row]}),
			)
			.await?;
		}
	}
	for (owner, id) in &deleted_ids {
		assert_foreign_patch_rejected_without_mutation(
			&app,
			&cookie,
			&mm,
			&ctx,
			&target_uri,
			&foreign_uri,
			&target_case,
			&foreign_case,
			&target_before_parent_delete,
			&foreign_before_parent_delete,
			&target_audit_before,
			&foreign_audit_before,
			json!({(*owner):[{"id":id,"deleted":false}]}),
		)
		.await?;
	}

	let (status, body) = patch_json(
		&app,
		&cookie,
		&foreign_uri,
		json!({"authorities":["ich"],"rows":{"parentInfo":{"id":parent_info_id,"deleted":true}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, target_before) = get_json(&app, &cookie, &target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_before}");
	let (status, foreign_before) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before}");
	let target_audit_before = case_audit_ids(&mm, &ctx, &target_case).await?;
	let foreign_audit_before = case_audit_ids(&mm, &ctx, &foreign_case).await?;
	assert_foreign_patch_rejected_without_mutation(
		&app,
		&cookie,
		&mm,
		&ctx,
		&target_uri,
		&foreign_uri,
		&target_case,
		&foreign_case,
		&target_before,
		&foreign_before,
		&target_audit_before,
		&foreign_audit_before,
		json!({"parentInfo":{"id":parent_info_id,"deleted":false}}),
	)
	.await?;
	Ok(())
}

pub(crate) async fn rejects_foreign_sender_without_mutation(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let target_case =
		create_case_for_editor(&app, &cookie, "SD-SCOPE-TARGET", &["ich"]).await?;
	let foreign_case =
		create_case_for_editor(&app, &cookie, "SD-SCOPE-FOREIGN", &["ich"]).await?;
	let target_uri = format!("/api/cases/{target_case}/editor/pages/SD");
	let foreign_uri = format!("/api/cases/{foreign_case}/editor/pages/SD");
	for (uri, organization_name) in [
		(&target_uri, "Target sender"),
		(&foreign_uri, "Foreign sender"),
	] {
		let (status, body) = patch_json(
			&app,
			&cookie,
			uri,
			json!({"authorities":["ich"],"rows":{"senderInformation":{
				"senderType":"3","organizationName":organization_name
			}}}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
	}
	let (status, target_before) = get_json(&app, &cookie, &target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_before}");
	let (status, foreign_before) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_before}");
	let foreign_sender_id = foreign_before["rows"]["senderInformation"]["id"]
		.as_str()
		.ok_or("missing foreign sender id")?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let target_audit_before = case_audit_ids(&mm, &ctx, &target_case).await?;
	let foreign_audit_before = case_audit_ids(&mm, &ctx, &foreign_case).await?;
	let (status, body) = patch_json(
		&app,
		&cookie,
		&target_uri,
		json!({"authorities":["ich"],"rows":{"senderInformation":{
			"id":foreign_sender_id,"organizationName":"TAMPERED"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
	let (status, target_after) = get_json(&app, &cookie, &target_uri).await?;
	assert_eq!(status, StatusCode::OK, "{target_after}");
	assert_eq!(target_after, target_before, "target sender mutated");
	let (status, foreign_after) = get_json(&app, &cookie, &foreign_uri).await?;
	assert_eq!(status, StatusCode::OK, "{foreign_after}");
	assert_eq!(foreign_after, foreign_before, "foreign sender mutated");
	assert_eq!(
		case_audit_ids(&mm, &ctx, &target_case).await?,
		target_audit_before,
		"rejected sender patch audited the target case"
	);
	assert_eq!(
		case_audit_ids(&mm, &ctx, &foreign_case).await?,
		foreign_audit_before,
		"rejected sender patch audited the foreign case"
	);
	Ok(())
}
