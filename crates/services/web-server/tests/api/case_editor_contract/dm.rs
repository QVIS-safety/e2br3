use super::support::field_contract_test;
const PAGE_ID: &str = "DM";
include!("generated/dm_fields.rs");

#[serial_test::serial]
#[tokio::test]
async fn d_10_1_value_nullflavor_value() -> crate::common::Result<()> {
	super::support::verify_dm_parent_transition(
		super::support::DmParentField::Identification,
	)
	.await
}

#[serial_test::serial]
#[tokio::test]
async fn d_10_6_value_nullflavor_value() -> crate::common::Result<()> {
	super::support::verify_dm_parent_transition(super::support::DmParentField::Sex)
		.await
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
