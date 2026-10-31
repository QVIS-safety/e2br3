use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::Map;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum DmParentField {
	Identification,
	Sex,
}

async fn verify_dm_parent_transition(field: DmParentField) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DM-PARENT-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;
	let (
		payload_key,
		null_flavor_key,
		projection_key,
		projection_null_flavor_key,
		values,
		invalid,
	) = match field {
		DmParentField::Identification => (
			"parentIdentification",
			"parentIdentificationNullFlavor",
			"parent_identification",
			"parent_identification_null_flavor",
			[
				(Some("MOTHER-01"), None),
				(None, Some("UNK")),
				(Some("MOTHER-02"), None),
			],
			None,
		),
		DmParentField::Sex => (
			"parentSex",
			"parentSexNullFlavor",
			"sex",
			"sex_null_flavor",
			[(Some("2"), None), (None, Some("NASK")), (Some("1"), None)],
			Some("NI"),
		),
	};
	let uri = format!("/api/cases/{case_id}/editor/pages/DM");

	for (value, null_flavor) in values {
		let mut parent = Map::new();
		parent.insert(payload_key.to_string(), json!(value));
		parent.insert(null_flavor_key.to_string(), json!(null_flavor));
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({
				"authorities": ["ich"],
				"rows": {
					"patientInformation": {"patientInitials": "PT-FIXTURE"},
					"parentInfo": parent
				}
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");

		mm.dbx().begin_txn().await?;
		set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		let pair = match field {
			DmParentField::Identification => mm
				.dbx()
				.fetch_one(
					sqlx::query_as::<_, (Option<String>, Option<String>)>(
						"SELECT p.parent_identification, p.parent_identification_null_flavor
						   FROM parent_information p
						   JOIN patient_information patient ON patient.id = p.patient_id
						  WHERE patient.case_id = $1",
					)
					.bind(Uuid::parse_str(&case_id)?),
				)
				.await?,
			DmParentField::Sex => mm
				.dbx()
				.fetch_one(
					sqlx::query_as::<_, (Option<String>, Option<String>)>(
						"SELECT p.sex, p.sex_null_flavor
						   FROM parent_information p
						   JOIN patient_information patient ON patient.id = p.patient_id
						  WHERE patient.case_id = $1",
					)
					.bind(Uuid::parse_str(&case_id)?),
				)
				.await?,
		};
		mm.dbx().commit_txn().await?;
		assert_eq!(
			pair,
			(value.map(str::to_string), null_flavor.map(str::to_string))
		);

		let (status, body) = get_json(&app, &cookie, &uri).await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		assert_eq!(body["rows"]["parentInfo"][projection_key], json!(value));
		assert_eq!(
			body["rows"]["parentInfo"][projection_null_flavor_key],
			json!(null_flavor)
		);
	}

	if let Some(invalid) = invalid {
		let mut parent = Map::new();
		parent.insert(payload_key.to_string(), json!(invalid));
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({
				"authorities": ["ich"],
				"rows": {"parentInfo": parent}
			}),
		)
		.await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
		assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
	}
	Ok(())
}

pub(crate) async fn dosage_value_null_flavor_pairs_reject_simultaneous_values(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DB-PAIR-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({"data": {"case_id": case_id, "sequence_number": 1, "drug_characterization": "1", "medicinal_product": "Pair drug"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let dg_uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&dg_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {"dosageInformation": [{
			"sequenceNumber": 1,
			"doseForm": null, "doseFormNullFlavor": "UNK",
			"routeOfAdministration": null, "routeOfAdministrationNullFlavor": "ASKU",
			"parentRouteOfAdministration": null, "parentRouteOfAdministrationNullFlavor": "NASK"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (dosage_id, dose_form, dose_form_nf, route, route_nf, parent_route, parent_route_nf): (Uuid, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = mm.dbx().fetch_one(
		sqlx::query_as("SELECT id, dose_form, dose_form_null_flavor, route_of_administration, route_of_administration_null_flavor, parent_route, parent_route_null_flavor FROM dosage_information WHERE drug_id = $1")
			.bind(Uuid::parse_str(drug_id)?),
	).await?;
	assert_eq!(
		(
			dose_form,
			dose_form_nf,
			route,
			route_nf,
			parent_route,
			parent_route_nf
		),
		(
			None,
			Some("UNK".into()),
			None,
			Some("ASKU".into()),
			None,
			Some("NASK".into())
		)
	);
	mm.dbx().commit_txn().await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&dg_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {"dosageInformation": [{
			"id": dosage_id,
			"doseForm": "Tablet", "doseFormNullFlavor": null,
			"routeOfAdministration": "048", "routeOfAdministrationNullFlavor": null,
			"parentRouteOfAdministration": "Oral", "parentRouteOfAdministrationNullFlavor": null
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	for (field, null_flavor, value, flavor) in [
		("doseForm", "doseFormNullFlavor", "Tablet", "UNK"),
		(
			"routeOfAdministration",
			"routeOfAdministrationNullFlavor",
			"048",
			"ASKU",
		),
		(
			"parentRouteOfAdministration",
			"parentRouteOfAdministrationNullFlavor",
			"Oral",
			"NASK",
		),
	] {
		let (status, body) = patch_json(&app, &cookie, &dg_uri, json!({"authorities":["ich"],"rows":{"drug":{"dosageInformation":[{"id":dosage_id,field:value,null_flavor:flavor}]}}})).await?;
		assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	}
	let (status, body) = get_json(&app, &cookie, &dg_uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let dosage = &body["data"]["drug"]["dosageInformation"][0];
	assert_eq!(dosage["id"], dosage_id.to_string());
	for (column, value) in [
		("dose_form", "Tablet"),
		("route_of_administration", "048"),
		("parent_route", "Oral"),
	] {
		assert_eq!(dosage[column], value, "{body}");
		assert_eq!(
			dosage[format!("{column}_null_flavor")],
			Value::Null,
			"{body}"
		);
	}

	Ok(())
}

pub(crate) async fn split_null_flavor_http_contract(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-AE-SPLIT-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;
	let reaction_id =
		create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/AE/rows/{reaction_id}");

	let request = |start_date: Value, null_flavor: Value| {
		json!({
			"authorities": ["ich"],
			"rows": {
				"reaction": {
					"reactionStartDate": start_date,
					"reactionStartDateNullFlavor": null_flavor
				}
			}
		})
	};
	let (status, body) =
		patch_json(&app, &cookie, &uri, request(Value::Null, json!("MSK"))).await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) =
		patch_json(&app, &cookie, &uri, request(json!("MSK"), Value::Null)).await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"]["path"],
		"reactions.0.reactionStartDate"
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		request(json!("20260715"), json!("MSK")),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
	assert_eq!(
		body["error"]["data"]["detail"]["path"],
		"reactions.0.reactionStartDateNullFlavor"
	);

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let pair = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Option<sqlx::types::time::Date>, Option<String>)>(
				"SELECT start_date, start_date_null_flavor FROM reactions WHERE id = $1",
			)
			.bind(Uuid::parse_str(&reaction_id)?),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	assert_eq!(pair, (None, Some("MSK".to_string())));
	Ok(())
}

pub(crate) async fn d_10_1_value_nullflavor_value(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	verify_dm_parent_transition(DmParentField::Identification).await
}

pub(crate) async fn d_10_6_value_nullflavor_value(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	verify_dm_parent_transition(DmParentField::Sex).await
}

pub(crate) async fn patient_identity_can_be_cleared_and_set_to_null_flavor(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	use crate::case_editor_contract_web::support::{
		create_case_for_editor, get_json, patch_json,
	};
	use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
	use axum::http::StatusCode;
	use lib_auth::token::generate_web_token;
	use serde_json::{json, Value};

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-DM-CLEAR", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/DM");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"patientInformation": {
			"patientInitials": "PT-CLEAR"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"patientInformation": {
			"patientInitials": null,
			"patientInitialsNullFlavor": null
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["patientInformation"]["patient_initials"],
		Value::Null
	);
	assert_eq!(
		body["rows"]["patientInformation"]["patient_initials_null_flavor"],
		Value::Null
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {"patientInformation": {
			"patientInitials": null,
			"patientInitialsNullFlavor": "UNK"
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["rows"]["patientInformation"]["patient_initials"],
		Value::Null
	);
	assert_eq!(
		body["rows"]["patientInformation"]["patient_initials_null_flavor"],
		"UNK"
	);
	Ok(())
}

pub(crate) async fn registration_blank_values_can_be_replaced_by_null_flavors(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id =
		create_case_for_editor(&app, &cookie, "EDITOR-SI-NULL-FLAVOR", &["ich"])
			.await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/SI");
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"studyInformation": {"studyName": "Null flavor study"},
			"studyRegistrationNumbers": [
				{"registrationNumber": "REG-1", "countryCode": "KR", "sequenceNumber": 1},
				{"registrationNumber": "REG-2", "countryCode": "US", "sequenceNumber": 2}
			]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, initial) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{initial}");
	let first_id = initial["rows"]["studyRegistrationNumbers"][0]["id"]
		.as_str()
		.ok_or("missing first registration id")?;
	let sibling = initial["rows"]["studyRegistrationNumbers"][1].clone();

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"studyRegistrationNumbers": [{
				"id": first_id,
				"registrationNumber": "",
				"registrationNumberNullFlavor": "ASKU"
			}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, after_registration) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{after_registration}");
	assert_eq!(
		after_registration["rows"]["studyRegistrationNumbers"][0]
			["registration_number"],
		serde_json::Value::Null
	);
	assert_eq!(
		after_registration["rows"]["studyRegistrationNumbers"][0]
			["registration_number_null_flavor"],
		"ASKU"
	);
	assert_eq!(
		after_registration["rows"]["studyRegistrationNumbers"][1],
		sibling
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"studyRegistrationNumbers": [{
				"id": first_id,
				"countryCode": " ",
				"countryCodeNullFlavor": "ASKU"
			}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, after_country) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{after_country}");
	assert_eq!(
		after_country["rows"]["studyRegistrationNumbers"][0]["country_code"],
		serde_json::Value::Null
	);
	assert_eq!(
		after_country["rows"]["studyRegistrationNumbers"][0]
			["country_code_null_flavor"],
		"ASKU"
	);
	assert_eq!(
		after_country["rows"]["studyRegistrationNumbers"][1],
		sibling
	);

	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities": ["ich"], "rows": {
			"studyRegistrationNumbers": [{
				"id": first_id,
				"registrationNumber": "REG-CONFLICT",
				"registrationNumberNullFlavor": "ASKU"
			}]
		}}),
	)
	.await?;
	assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
	assert_eq!(
		body["error"]["data"]["detail"]["path"],
		"studyInformation.studyRegistrationNumbers.0.registrationNumberNullFlavor"
	);
	let (status, unchanged) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{unchanged}");
	assert_eq!(unchanged, after_country);
	Ok(())
}
