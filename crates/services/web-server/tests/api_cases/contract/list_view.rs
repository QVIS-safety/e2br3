use super::support::{get_raw, post_raw};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn test_case_list_view_projects_reference_grid_fields() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let suffix = Uuid::new_v4().simple().to_string();
	let case_no = format!("CASE-LIST-{suffix}");
	let worldwide_unique_no = format!("WWUID-{suffix}");
	let sender = format!("Sender Org {suffix}");
	let receiver = format!("Receiver Org {suffix}");
	let ae_term = format!("Headache {suffix}");
	let study_no = format!("STUDY-{suffix}");
	let manufacturer = format!("Manufacturer {suffix}");

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": case_no},
				"dgPrdKey": "DG-12345",
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let case_id = body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string();

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report"),
		json!({
			"data": {
				"case_id": case_id,
				"transmission_date": "2026-05-01",
				"report_type": "2",
				"date_first_received_from_source": "2026-04-30",
				"date_of_most_recent_information": "2026-05-01",
				"worldwide_unique_id": worldwide_unique_no
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report/senders"),
		json!({
			"data": {
				"case_id": case_id,
				"organization_name": sender,
				"sender_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/receiver"),
		json!({
			"data": {
				"case_id": case_id,
				"organization_name": receiver,
				"receiver_type": "2"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": ae_term,
				"reaction_meddra_code": "10019211",
				"reaction_meddra_version": "27.1",
				"serious": false,
				"criteria_death": true
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/safety-report/studies"),
		json!({
			"data": {
				"case_id": case_id,
				"study_name": "Pivotal Migraine Study",
				"sponsor_study_number": study_no,
				"study_type_reaction": "1"
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) = post_raw(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Example Product",
				"manufacturer_name": manufacturer
			}
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);

	let (status, raw_body) =
		get_raw(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(status, StatusCode::OK);
	let detail: Value = serde_json::from_slice(&raw_body)?;
	let updated_at = detail["data"]
		.get("updated_at")
		.ok_or("case detail is missing updated_at")?;
	assert!(!updated_at.is_null());

	let (status, raw_body) = get_raw(&app, &cookie, "/api/cases/list-view").await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	let row = items
		.iter()
		.find(|item| item["caseNo"].as_str() == Some(case_no.as_str()))
		.ok_or("missing projected case row")?;

	assert_eq!(
		row["worldwideUniqueNo"].as_str(),
		Some(worldwide_unique_no.as_str()),
		"{row:?}"
	);
	assert_eq!(row["sender"].as_str(), Some(sender.as_str()), "{row:?}");
	assert_eq!(row.get("updatedAt"), Some(updated_at), "{row:?}");
	assert_eq!(row["meddra"].as_str(), Some("10019211"), "{row:?}");
	assert_eq!(row["aeTerm"].as_str(), Some(ae_term.as_str()), "{row:?}");
	assert_eq!(row["sae"].as_str(), Some("Yes"), "{row:?}");
	assert_eq!(row["studyNo"].as_str(), Some(study_no.as_str()), "{row:?}");
	assert_eq!(
		row["dateOfCreation"].as_str(),
		Some("2026-05-01"),
		"{row:?}"
	);
	assert_eq!(
		row["dateOfMostRecentInformation"].as_str(),
		Some("2026-05-01"),
		"{row:?}"
	);
	assert_eq!(
		row["manufacturer"].as_str(),
		Some(manufacturer.as_str()),
		"{row:?}"
	);
	assert_eq!(row["receiver"].as_str(), Some(receiver.as_str()), "{row:?}");
	assert_eq!(
		row["typeOfReport"].as_str(),
		Some("Report from study"),
		"{row:?}"
	);

	let case_id = Uuid::parse_str(&case_id)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query("UPDATE reactions SET deleted = true WHERE case_id = $1")
		.bind(case_id)
		.execute(&mut *tx)
		.await?;
	tx.commit().await?;

	let (status, raw_body) = get_raw(&app, &cookie, "/api/cases/list-view").await?;
	assert_eq!(status, StatusCode::OK);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let case_id_string = case_id.to_string();
	let row = body["data"]["items"]
		.as_array()
		.ok_or("missing list-view items after reaction deletion")?
		.iter()
		.find(|item| item["caseId"].as_str() == Some(case_id_string.as_str()))
		.ok_or("missing projected case row after reaction deletion")?;
	assert_eq!(row["meddra"].as_str(), Some("N/A"), "{row:?}");
	assert_eq!(row["aeTerm"].as_str(), Some("N/A"), "{row:?}");
	assert_eq!(row["sae"].as_str(), Some("No"), "{row:?}");
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_list_view_handles_case_without_safety_report_row() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = Uuid::new_v4();

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	sqlx::query(
		"INSERT INTO cases (id, organization_id, status, created_by, updated_by)
		 VALUES ($1, $2, 'draft', $3, $3)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, raw_body) = get_raw(&app, &cookie, "/api/cases/list-view").await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let body: Value = serde_json::from_slice(&raw_body)?;
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing list-view items")?;
	let case_id_string = case_id.to_string();
	let row = items
		.iter()
		.find(|item| item["caseId"].as_str() == Some(case_id_string.as_str()))
		.ok_or("missing orphan case row")?;

	assert_eq!(row["caseNo"].as_str(), Some(case_id_string.as_str()));
	assert_eq!(row["fu"].as_i64(), Some(0));
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_list_view_honors_limit_and_offset() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm);

	let suffix = Uuid::new_v4().simple().to_string();
	for index in 1..=2 {
		let (status, raw_body) = post_raw(
			&app,
			&cookie,
			"/api/cases",
			json!({
				"data": {
					"safetyReportIdentification": {"safetyReportId": format!("CASE-LIST-PAGED-{suffix}-{index}")},
					"status": "draft"
				}
			}),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::CREATED,
			"{}",
			String::from_utf8_lossy(&raw_body)
		);
	}

	let (status, raw_body) = get_raw(
		&app,
		&cookie,
		"/api/cases/list-view?list_options%5Blimit%5D=1&list_options%5Boffset%5D=0&list_options%5Border_bys%5D=%21created_at",
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let first_body: Value = serde_json::from_slice(&raw_body)?;
	let first_items = first_body["data"]["items"]
		.as_array()
		.ok_or("missing first list-view items")?;
	assert_eq!(first_items.len(), 1, "{first_body:?}");

	let (status, raw_body) = get_raw(
		&app,
		&cookie,
		"/api/cases/list-view?list_options%5Blimit%5D=1&list_options%5Boffset%5D=1&list_options%5Border_bys%5D=%21created_at",
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&raw_body)
	);
	let second_body: Value = serde_json::from_slice(&raw_body)?;
	let second_items = second_body["data"]["items"]
		.as_array()
		.ok_or("missing second list-view items")?;
	assert_eq!(second_items.len(), 1, "{second_body:?}");
	assert_ne!(
		second_items[0]["caseId"], first_items[0]["caseId"],
		"offset should move to the next ordered row"
	);

	Ok(())
}
