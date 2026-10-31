use super::support::field_contract_test;
use super::support::{create_case_for_editor, get_json, patch_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::{json, Map};
use uuid::Uuid;
const PAGE_ID: &str = "DM";
include!("generated/dm_fields.rs");

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

#[serial_test::serial]
#[tokio::test]
async fn d_10_1_value_nullflavor_value() -> crate::common::Result<()> {
	verify_dm_parent_transition(DmParentField::Identification).await
}

#[serial_test::serial]
#[tokio::test]
async fn d_10_6_value_nullflavor_value() -> crate::common::Result<()> {
	verify_dm_parent_transition(DmParentField::Sex).await
}

#[serial_test::serial]
#[tokio::test]
async fn patient_identity_can_be_cleared_and_set_to_null_flavor(
) -> crate::common::Result<()> {
	use super::support::{create_case_for_editor, get_json, patch_json};
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
