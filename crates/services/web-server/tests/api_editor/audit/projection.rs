use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use uuid::Uuid;

pub(crate) async fn fda_device_scalar_editor_save_projects_audit_trail(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
	use axum::body::{to_bytes, Body};
	use axum::http::Request;
	use tower::ServiceExt;

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DG-DEVICE-AUDIT-{}", Uuid::new_v4()),
		&["fda"],
	)
	.await?;
	let rows_uri = format!("/api/cases/{case_id}/editor/pages/DG/rows");
	let (status, body) = post_json(
		&app,
		&cookie,
		&rows_uri,
		json!({"authorities": ["fda"], "rows": {"drug": {
			"sequenceNumber": 3,
			"drugCharacterization": "1",
			"medicinalProduct": "Audited device drug",
			"fdaDevices": [{"deviceBrandName": "Audit device"}]
		}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["rowId"].as_str().ok_or("missing drug id")?;
	let row_uri = format!("{rows_uri}/{drug_id}");
	let (_, saved) = get_json(&app, &cookie, &row_uri).await?;
	let device_id = saved["data"]["drug"]["fdaDevices"][0]["id"]
		.as_str()
		.ok_or("missing device id")?;
	let reason = "All-field validation FDA device scalar audit";
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("PATCH")
				.uri(&row_uri)
				.header("cookie", &cookie)
				.header("content-type", "application/json")
				.header("x-e2br3-reason-for-change", reason)
				.body(Body::from(
					json!({"authorities": ["fda"], "rows": {"drug": {
						"fdaDevices": [{
							"id": device_id,
							"deviceBrandName": "Audit device",
							"deviceUsage": "2",
							"deviceLotNumber": "LOT-AUDIT-1"
						}]
					}}})
					.to_string(),
				))?,
		)
		.await?;
	let status = response.status();
	let body: Value =
		serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["device_usage"], "2",
		"{body}"
	);
	assert_eq!(
		body["data"]["drug"]["fdaDevices"][0]["device_lot_number"], "LOT-AUDIT-1",
		"{body}"
	);

	let (status, audit) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}/audit-trail"))
			.await?;
	assert_eq!(status, StatusCode::OK, "{audit}");
	let rows = audit["data"].as_array().ok_or("missing audit rows")?;
	for (item, value) in [
		("FDA Device Usage (FDA.G.k.12.r.8)", "2"),
		("FDA Device Lot Number (FDA.G.k.12.r.9)", "LOT-AUDIT-1"),
	] {
		let row = rows
			.iter()
			.find(|row| row["item"] == item && row["reason"] == reason)
			.ok_or_else(|| format!("missing {item} audit row: {audit}"))?;
		assert_eq!(row["value"], value, "{row}");
		assert_eq!(row["rowNo1"], "3", "{row}");
		assert_eq!(row["rowNo2"], "1", "{row}");
		assert_eq!(row["rowNo3"], "", "{row}");
	}
	Ok(())
}
