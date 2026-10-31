use crate::case_support::create_case;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn request_json(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let request = builder.body(Body::from(match body {
		Some(body) => body.to_string(),
		None => String::new(),
	}))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let value = serde_json::from_slice::<Value>(&bytes).map_err(|err| {
		format!(
			"failed to parse {method} field-notation response as JSON: {err}; body={}",
			String::from_utf8_lossy(&bytes)
		)
	})?;
	Ok((status, value))
}

async fn create_drug_record(
	app: &axum::Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<Uuid> {
	let (status, body) = request_json(
		app,
		cookie,
		"POST",
		format!("/api/cases/{case_id}/drugs"),
		Some(json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Notation Test Drug"
		}})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!("create drug status {status} body {body}").into());
	}
	let id = body["data"]["id"]
		.as_str()
		.ok_or_else(|| format!("create drug response missing data.id: {body}"))?;
	Ok(Uuid::parse_str(id)?)
}

async fn notation_audit_actions(
	mm: &lib_core::model::ModelManager,
	notation_id: Uuid,
) -> Result<Vec<String>> {
	let mut tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *tx)
		.await?;
	let actions = sqlx::query_scalar::<_, String>(
		"SELECT action FROM audit_logs
		 WHERE table_name = 'case_e2b_field_notations' AND record_id = $1
		 ORDER BY id",
	)
	.bind(notation_id)
	.fetch_all(&mut *tx)
	.await?;
	tx.rollback().await?;
	Ok(actions)
}

#[serial]
#[tokio::test]
async fn field_notation_scalar_lifecycle_preserves_id_and_audits_changes(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let endpoint = format!("/api/cases/{case_id}/field-notation");
	let query = format!("{endpoint}?e2bCode=C.1.1");

	let (status, saved) = request_json(
		&app,
		&cookie,
		"PUT",
		endpoint.clone(),
		Some(json!({
			"e2bCode": "C.1.1",
			"notation": "  initial notation  "
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	assert_eq!(saved["e2bCode"], "C.1.1");
	assert_eq!(saved["recordId"], Value::Null);
	assert_eq!(saved["notation"], "initial notation");
	let notation_id = Uuid::parse_str(
		saved["id"]
			.as_str()
			.ok_or_else(|| format!("save response missing id: {saved}"))?,
	)?;

	let (status, read) =
		request_json(&app, &cookie, "GET", query.clone(), None).await?;
	assert_eq!(status, StatusCode::OK, "{read}");
	assert_eq!(read, saved);

	let (status, updated) = request_json(
		&app,
		&cookie,
		"PUT",
		endpoint.clone(),
		Some(json!({
			"e2bCode": "C.1.1",
			"notation": "updated notation"
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{updated}");
	assert_eq!(updated["id"], notation_id.to_string());
	assert_eq!(updated["notation"], "updated notation");

	let (status, blank) = request_json(
		&app,
		&cookie,
		"PUT",
		endpoint,
		Some(json!({
			"e2bCode": "C.1.1",
			"notation": "   "
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{blank}");
	assert_eq!(blank["id"], notation_id.to_string());
	assert_eq!(blank["notation"], "");

	let (status, deleted) =
		request_json(&app, &cookie, "DELETE", query.clone(), None).await?;
	assert_eq!(status, StatusCode::OK, "{deleted}");
	assert_eq!(deleted["id"], Value::Null);
	assert_eq!(deleted["e2bCode"], "C.1.1");
	assert_eq!(deleted["recordId"], Value::Null);
	assert_eq!(deleted["notation"], "");

	let (status, missing) = request_json(&app, &cookie, "GET", query, None).await?;
	assert_eq!(status, StatusCode::OK, "{missing}");
	assert_eq!(missing, deleted);
	assert_eq!(
		notation_audit_actions(&mm, notation_id).await?,
		vec![
			"CREATE".to_string(),
			"UPDATE".to_string(),
			"UPDATE".to_string(),
			"DELETE".to_string()
		]
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn field_notation_keeps_scalar_and_actual_record_subjects_isolated(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let drug_id = create_drug_record(&app, &cookie, case_id).await?;
	let endpoint = format!("/api/cases/{case_id}/field-notation");

	let (status, scalar) = request_json(
		&app,
		&cookie,
		"PUT",
		endpoint.clone(),
		Some(json!({"e2bCode": "G.k.2.2", "notation": "case scalar"})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{scalar}");
	let (status, record) = request_json(
		&app,
		&cookie,
		"PUT",
		endpoint,
		Some(json!({
			"e2bCode": "G.k.2.2",
			"recordId": drug_id,
			"notation": "drug row"
		})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{record}");
	assert_eq!(record["recordId"], drug_id.to_string());
	assert_ne!(scalar["id"], record["id"]);

	let (status, scalar_read) = request_json(
		&app,
		&cookie,
		"GET",
		format!("/api/cases/{case_id}/field-notation?e2bCode=G.k.2.2"),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{scalar_read}");
	assert_eq!(scalar_read, scalar);
	let (status, record_read) = request_json(
		&app,
		&cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/field-notation?e2bCode=G.k.2.2&recordId={drug_id}"
		),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{record_read}");
	assert_eq!(record_read, record);

	Ok(())
}

#[serial]
#[tokio::test]
async fn viewer_cannot_change_field_notation_or_its_audit_history() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let viewer_token =
		generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer_token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case(&app, &admin_cookie, seed.org_id).await?;
	let endpoint = format!("/api/cases/{case_id}/field-notation");
	let query = format!("{endpoint}?e2bCode=D.1");

	let (status, baseline) = request_json(
		&app,
		&admin_cookie,
		"PUT",
		endpoint.clone(),
		Some(json!({"e2bCode": "D.1", "notation": "admin note"})),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{baseline}");
	let notation_id = Uuid::parse_str(
		baseline["id"]
			.as_str()
			.ok_or_else(|| format!("save response missing id: {baseline}"))?,
	)?;
	let audit_before = notation_audit_actions(&mm, notation_id).await?;
	assert_eq!(audit_before, vec!["CREATE".to_string()]);

	let (status, viewer_read) =
		request_json(&app, &viewer_cookie, "GET", query.clone(), None).await?;
	assert_eq!(status, StatusCode::OK, "{viewer_read}");
	assert_eq!(viewer_read, baseline);

	let (status, body) = request_json(
		&app,
		&viewer_cookie,
		"PUT",
		endpoint,
		Some(json!({"e2bCode": "D.1", "notation": "viewer overwrite"})),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
	let (status, body) =
		request_json(&app, &viewer_cookie, "DELETE", query.clone(), None).await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

	let (status, after) =
		request_json(&app, &admin_cookie, "GET", query, None).await?;
	assert_eq!(status, StatusCode::OK, "{after}");
	assert_eq!(after, baseline);
	assert_eq!(
		notation_audit_actions(&mm, notation_id).await?,
		audit_before
	);

	Ok(())
}
