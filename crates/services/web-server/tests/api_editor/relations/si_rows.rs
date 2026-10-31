use super::{assert_malformed_row_id_rejected_without_mutation, patch_and_reload};
use crate::case_editor_contract_web::support::create_case_for_editor;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use serde_json::json;

pub(crate) async fn study_rows_delete_restore_and_resave_are_stable(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id =
		create_case_for_editor(&app, &cookie, "SI-LIFECYCLE", &["fda"]).await?;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, body) = patch_and_reload(
		&app,
		&cookie,
		&mm,
		&ctx,
		&uri,
		json!({"authorities":["fda"],"rows":{
			"studyRegistrationNumbers":[{"sequenceNumber":1,"registrationNumber":"REG","countryCode":"US"}],
			"studyInformation":{"studyName":"Study","fdaCrossReportedIndNumbers":[{"sequenceNumber":1,"indNumber":"IND"}]}
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let registration_id = body["rows"]["studyRegistrationNumbers"][0]["id"].clone();
	let ind_id = body["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
		["id"]
		.clone();
	for rows in [
		json!({"studyRegistrationNumbers":[
			{"id":registration_id,"registrationNumber":"MUST ROLL BACK"},
			{"id":"not-a-uuid","registrationNumber":"INVALID"}
		]}),
		json!({"studyInformation":{"fdaCrossReportedIndNumbers":[
			{"id":ind_id,"indNumber":"MUST ROLL BACK"},
			{"id":"not-a-uuid","indNumber":"INVALID"}
		]}}),
	] {
		assert_malformed_row_id_rejected_without_mutation(
			&app, &cookie, &mm, &ctx, &uri, &case_id, "fda", rows,
		)
		.await?;
	}
	let deleted = json!({
		"studyRegistrationNumbers":[{"id":registration_id,"deleted":true}],
		"studyInformation":{"fdaCrossReportedIndNumbers":[{"id":ind_id,"deleted":true}]}
	});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["fda"],"rows":deleted}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "delete attempt {attempt}: {body}");
		assert_eq!(
			body["rows"]["studyRegistrationNumbers"]
				.as_array()
				.map(Vec::len),
			Some(0),
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"]
				.as_array()
				.map(Vec::len),
			Some(0),
			"{body}"
		);
	}
	for rows in [
		json!({}),
		json!({"studyRegistrationNumbers":[],"studyInformation":{"fdaCrossReportedIndNumbers":[]}}),
		json!({"studyRegistrationNumbers":[{"id":registration_id}],"studyInformation":{"fdaCrossReportedIndNumbers":[{"id":ind_id}]}}),
	] {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["fda"],"rows":rows}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(
			body["rows"]["studyRegistrationNumbers"]
				.as_array()
				.map(Vec::len),
			Some(0),
			"{body}"
		);
	}
	let restored = json!({
		"studyRegistrationNumbers":[{"id":registration_id,"deleted":false,"registrationNumber":"RESTORED","countryCode":"US"}],
		"studyInformation":{"fdaCrossReportedIndNumbers":[{"id":ind_id,"deleted":false,"indNumber":"RESTORED-IND"}]}
	});
	for attempt in 0..2 {
		let (status, body) = patch_and_reload(
			&app,
			&cookie,
			&mm,
			&ctx,
			&uri,
			json!({"authorities":["fda"],"rows":restored}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "restore attempt {attempt}: {body}");
		assert_eq!(
			body["rows"]["studyRegistrationNumbers"]
				.as_array()
				.map(Vec::len),
			Some(1),
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"]
				.as_array()
				.map(Vec::len),
			Some(1),
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyRegistrationNumbers"][0]["registration_number"],
			"RESTORED",
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]
				["ind_number"],
			"RESTORED-IND",
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyRegistrationNumbers"][0]["id"], registration_id,
			"{body}"
		);
		assert_eq!(
			body["rows"]["studyInformation"]["fdaCrossReportedIndNumbers"][0]["id"],
			ind_id,
			"{body}"
		);
	}
	super::assert_child_mutation_guards(&app, &cookie, &mm, &seed, &uri, "fda", 2)
		.await?;
	Ok(())
}
