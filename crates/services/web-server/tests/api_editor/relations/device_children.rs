use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::json;

pub(crate) async fn device_list_replacement_removes_children(
	_page_id: &str,
	_check_id: &str,
) -> crate::common::Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let id =
		create_case_for_editor(&app, &cookie, "UI-REGRESSION", &["ich"]).await?;
	let base = format!("/api/cases/{id}/editor/pages");
	let (status, body) = post_json(&app, &cookie, &format!("{base}/DG/rows"), json!({"authorities":["ich"],"rows":{"drug":{
		"drugCharacterization":"1", "medicinalProduct":"DEVICE TEST",
        "mfdsDeviceInfo":{"serialNumber":"SN-1","udiManufacturingDate":"20260901","models":[{"value":"MODEL-1"}]},
        "fdaDevices":[{"deviceBrandName":"DEVICE-1","followUpTypes":[{"valueCode":"1"}]}]
    }}})).await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let uri = format!("{base}/DG/rows/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	let drug = &saved["data"]["drug"];
	assert_eq!(drug["drug_characterization"], "1", "{saved}");
	assert_eq!(
		drug["fdaDevices"].as_array().map(Vec::len),
		Some(1),
		"{saved}"
	);
	assert!(
		!drug["deviceCharacteristics"]
			.as_array()
			.unwrap()
			.iter()
			.any(|r| r["value_value"] == "SN-1"),
		"{saved}"
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({"authorities":["ich"],"rows":{"drug":{"fdaDevices":[]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (_, saved) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		saved["data"]["drug"]["fdaDevices"].as_array().map(Vec::len),
		Some(0),
		"{saved}"
	);

	Ok(())
}
