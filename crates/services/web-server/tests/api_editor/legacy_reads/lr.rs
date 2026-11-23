use crate::case_editor_contract_web::support::{
	create_case_for_editor, get_json, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

#[serial]
#[tokio::test]
async fn test_lr_editor_retains_deleted_reference_for_restore_only() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm)?;
	let case_id =
		create_case_for_editor(&app, &cookie, "LR-RESTORE", &["ich"]).await?;
	let uri = format!("/api/cases/{case_id}/editor/pages/LR");
	for sequence in [2, 1] {
		let (status, body) = post_json(&app, &cookie, &format!("{uri}/rows"), json!({
			"authorities":["ich"], "rows":{"literatureReference":{
				"sequenceNumber":sequence, "referenceText":format!("Isolated reference {sequence}"),
				"documentBase64":"Zml4dHVyZQ==", "fileName":"fixture.txt",
				"mediaType":"text/plain", "representation":"B64"
			}}
		})).await?;
		assert_eq!(status, StatusCode::CREATED, "{body}");
	}
	let (status, initial) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{initial}");
	let initial_rows = initial["rows"]["rows"]
		.as_array()
		.ok_or("missing LR rows")?;
	assert_eq!(initial_rows.len(), 2);
	assert_eq!(initial_rows[0]["sequenceNumber"], 1);
	assert_eq!(initial_rows[1]["sequenceNumber"], 2);
	let id = initial_rows[0]["id"].as_str().ok_or("missing LR id")?;
	let row_uri = format!("{uri}/rows/{id}");
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(&row_uri)
				.header("cookie", &cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert!(response.status().is_success());
	for deleted in [true, false] {
		if !deleted {
			let (status, body) = patch_json(
				&app,
				&cookie,
				&row_uri,
				json!({"authorities":["ich"],"rows":{"literatureReference":{"deleted":false}}}),
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{body}");
		}
		for suffix in ["", "?include_rows=true"] {
			let (status, body) =
				get_json(&app, &cookie, &format!("{uri}{suffix}")).await?;
			assert_eq!(status, StatusCode::OK, "{body}");
			let mut expected = initial_rows.clone();
			expected[0]["deleted"] = json!(deleted);
			assert_eq!(
				body["rows"]["rows"],
				json!(expected),
				"editor must retain identity, ordering, text and attachment"
			);
			if !suffix.is_empty() {
				let detail =
					&body["rows"]["rowDetails"][id]["data"]["literatureReference"];
				assert_eq!(detail["deleted"], deleted);
				assert_eq!(detail["document_base64"], "Zml4dHVyZQ==");
				assert_eq!(detail["reference_text"], "Isolated reference 1");
			}
		}
		let (status, body) = get_json(
			&app,
			&cookie,
			&format!("/api/cases/{case_id}/safety-report/literature"),
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		let active = body["data"]
			.as_array()
			.ok_or("missing active literature list")?;
		assert_eq!(active.len(), if deleted { 1 } else { 2 });
		assert!(active.iter().all(|row| row["deleted"] == false));
	}
	Ok(())
}
