use super::case_support::*;
use super::validation_common;
use axum::body::Bytes;
use axum::http::{header, HeaderMap};

async fn create_cioms_ready_case(
	app: &axum::Router,
	cookie: &str,
	case_number: &str,
) -> Result<Uuid> {
	let (status, value) = create_case_with_payload(
		app,
		cookie,
		json!({
			"data": {
				"safetyReportIdentification": {
					"safetyReportId": case_number
				},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");
	let case_id =
		Uuid::parse_str(value["data"]["id"].as_str().ok_or("missing case id")?)?;

	let reaction_id = validation_common::create_reaction(
		app,
		cookie,
		case_id,
		1,
		"CIOMS test reaction",
	)
	.await?;
	let drug_id = validation_common::create_drug(
		app,
		cookie,
		case_id,
		1,
		"1",
		"CIOMS Test Drug",
	)
	.await?;
	let (status, value) = validation_common::post_json(
		app,
		cookie,
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		json!({
			"data": {
				"drug_id": drug_id,
				"reaction_id": reaction_id,
				"dechallenge_result": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{value:?}");

	Ok(case_id)
}

async fn get_cioms(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<(StatusCode, HeaderMap, Bytes)> {
	let request = Request::builder()
		.method("GET")
		.uri(format!(
			"/api/cases/{case_id}/export/cioms.pdf?include_notation=false"
		))
		.header("cookie", cookie)
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let headers = response.headers().clone();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, headers, body))
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
	!needle.is_empty() && haystack.windows(needle.len()).any(|part| part == needle)
}

fn count_bytes(haystack: &[u8], needle: &[u8]) -> usize {
	if needle.is_empty() {
		return 0;
	}
	haystack
		.windows(needle.len())
		.filter(|part| *part == needle)
		.count()
}

#[serial]
#[tokio::test]
async fn test_cioms_export_returns_generated_pdf_attachment() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);
	// Keep the identifier on one line in either CIOMS orientation.
	let suffix = Uuid::new_v4().simple().to_string();
	let case_number = format!("CIOMS-{}", &suffix[..12]);
	let case_id = create_cioms_ready_case(&app, &cookie, &case_number).await?;

	let (status, headers, body) = get_cioms(&app, &cookie, case_id).await?;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		headers
			.get(header::CONTENT_TYPE)
			.and_then(|value| value.to_str().ok()),
		Some("application/pdf")
	);
	let expected_disposition =
		format!("attachment; filename=\"{case_number}-cioms.pdf\"");
	assert_eq!(
		headers
			.get(header::CONTENT_DISPOSITION)
			.and_then(|value| value.to_str().ok()),
		Some(expected_disposition.as_str())
	);
	assert!(body.starts_with(b"%PDF-1.4\n"));
	assert!(body.ends_with(b"%%EOF\n"));
	assert!(contains_bytes(&body, b"xref\n"));
	assert!(contains_bytes(&body, b"startxref\n"));
	assert!(
		contains_bytes(&body, case_number.as_bytes()),
		"the generated PDF must render its unique manufacturer control number"
	);
	let page_count = count_bytes(&body, b"/Type /Page /Parent 2 0 R");
	assert!(
		page_count >= 1,
		"the generated PDF must contain a page object"
	);
	assert!(contains_bytes(
		&body,
		format!("/Count {page_count} >>").as_bytes()
	));

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_cioms_export_enforces_permission_scope_and_case_existence(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let owner_org = seed_org_with_users(&mm, "owner-admin", "owner-viewer").await?;
	let other_org = seed_org_with_users(&mm, "other-admin", "other-viewer").await?;
	let owner_admin_token =
		generate_web_token(&owner_org.admin.email, owner_org.admin.token_salt)?;
	let owner_viewer_token =
		generate_web_token(&owner_org.viewer.email, owner_org.viewer.token_salt)?;
	let other_admin_token =
		generate_web_token(&other_org.admin.email, other_org.admin.token_salt)?;
	let owner_admin_cookie = cookie_header(&owner_admin_token.to_string());
	let owner_viewer_cookie = cookie_header(&owner_viewer_token.to_string());
	let other_admin_cookie = cookie_header(&other_admin_token.to_string());
	let app = web_server::app(mm);
	let owner_case =
		create_case(&app, &owner_admin_cookie, owner_org.org_id).await?;
	let other_case =
		create_case(&app, &other_admin_cookie, other_org.org_id).await?;

	let (viewer_status, _, _) =
		get_cioms(&app, &owner_viewer_cookie, owner_case).await?;
	assert_eq!(viewer_status, StatusCode::FORBIDDEN);

	let (cross_org_status, _, _) =
		get_cioms(&app, &owner_admin_cookie, other_case).await?;
	assert_eq!(cross_org_status, StatusCode::FORBIDDEN);

	let (missing_status, _, _) =
		get_cioms(&app, &owner_admin_cookie, Uuid::new_v4()).await?;
	assert_eq!(missing_status, StatusCode::NOT_FOUND);

	Ok(())
}
