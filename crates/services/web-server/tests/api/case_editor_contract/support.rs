use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) async fn post_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn patch_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("PATCH")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let request = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let response = app.clone().oneshot(request).await?;
	let status = response.status();
	let body = to_bytes(response.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice(&body)?))
}

pub(super) async fn create_case_for_editor(
	app: &axum::Router,
	cookie: &str,
	safety_report_prefix: &str,
	_appendices: &[&str],
) -> Result<String> {
	let safety_report_id = format!("{safety_report_prefix}-{}", Uuid::new_v4());
	let (status, body) = post_json(
		app,
		cookie,
		"/api/cases",
		json!({
			"data": {
				"safetyReportIdentification": {"safetyReportId": safety_report_id},
				"status": "draft"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing created case id")?
		.to_string())
}

async fn case_status(
	mm: &ModelManager,
	user_id: Uuid,
	org_id: Uuid,
	case_id: &str,
) -> Result<String> {
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let status = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (String,)>("SELECT status FROM cases WHERE id = $1")
				.bind(Uuid::parse_str(case_id)?),
		)
		.await?
		.0;
	mm.dbx().commit_txn().await?;
	Ok(status)
}

pub struct FieldSpec {
	pub page_id: &'static str,
	pub code: &'static str,
}

fn contract_document(page_id: &str) -> Result<Value> {
	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
	let path = root
		.join("registry/editor-contracts")
		.join(format!("{}.json", page_id.to_lowercase()));
	Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn contract_field(contract: &Value, page_id: &str, code: &str) -> Result<Value> {
	contract["fields"]
		.as_array()
		.and_then(|fields| fields.iter().find(|field| field["code"] == code))
		.cloned()
		.ok_or_else(|| format!("missing editor contract {page_id}/{code}").into())
}

fn build_path(path: &str, value: Value) -> Value {
	fn build(segments: &[&str], value: Value) -> Value {
		if segments.is_empty() {
			return value;
		}
		let segment = segments[0];
		if segment == "[]" {
			let mut child = build(&segments[1..], value);
			if let Some(object) = child.as_object_mut() {
				object.insert("sequenceNumber".to_string(), json!(1));
			}
			return Value::Array(vec![child]);
		}
		if let Some(key) = segment.strip_suffix("[]") {
			let mut child = build(&segments[1..], value);
			if let Some(object) = child.as_object_mut() {
				object.insert("sequenceNumber".to_string(), json!(1));
			}
			return Value::Object(Map::from_iter([(
				key.to_string(),
				Value::Array(vec![child]),
			)]));
		}
		Value::Object(Map::from_iter([(
			segment.to_string(),
			build(&segments[1..], value),
		)]))
	}

	build(&path.split('.').collect::<Vec<_>>(), value)
}

fn read_path<'a>(mut value: &'a Value, path: &str) -> Option<&'a Value> {
	for segment in path.split('.') {
		if segment == "[]" {
			value = value.as_array()?.first()?;
		} else if let Some(key) = segment.strip_suffix("[]") {
			value = value.get(key)?.as_array()?.first()?;
		} else {
			value = value.get(segment)?;
		}
	}
	Some(value)
}

fn merge_json(base: &mut Value, overlay: Value) {
	match (base, overlay) {
		(Value::Object(base), Value::Object(overlay)) => {
			for (key, value) in overlay {
				merge_json(base.entry(key).or_insert(Value::Null), value);
			}
		}
		(Value::Array(base), Value::Array(overlay)) => {
			for (index, value) in overlay.into_iter().enumerate() {
				if index < base.len() {
					merge_json(&mut base[index], value);
				} else {
					base.push(value);
				}
			}
		}
		(base, overlay) => *base = overlay,
	}
}

async fn repeatable_row_id(
	page_id: &str,
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<Option<String>> {
	Ok(match page_id {
		"LR" => {
			Some(create_literature_reference_fixture(app, cookie, case_id).await?)
		}
		"AE" => Some(create_reaction_contract_fixture(app, cookie, case_id).await?),
		"LB" => Some(create_test_result_fixture(app, cookie, case_id).await?),
		"DG" => Some(create_drug_fixture(app, cookie, case_id).await?),
		"DH" => Some(create_past_drug_history_fixture(app, cookie, case_id).await?),
		_ => None,
	})
}

async fn create_literature_reference_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/editor/pages/LR/rows"),
		json!({
			"authorities": ["ich"],
			"rows": {
				"literatureReference": {"sequenceNumber": 1}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["rowId"]
		.as_str()
		.ok_or("missing literature reference fixture id")?
		.to_string())
}

pub(super) async fn create_reaction_contract_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Fixture reaction",
				"primary_source_reaction_translation": "Fixture translation",
				"reaction_meddra_version": "27.1",
				"reaction_meddra_code": "10019211",
				"serious": false,
				"outcome": "2"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing reaction fixture id")?
		.to_string())
}

async fn create_test_result_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/test-results"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"test_name": "Fixture test",
				"test_result_value": "0",
				"test_result_unit": "fixture"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing test result fixture id")?
		.to_string())
}

async fn create_drug_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Fixture product",
				"action_taken": "1"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing drug fixture id")?
		.to_string())
}

async fn create_patient_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/patient"),
		json!({
			"data": {
				"case_id": case_id,
				"patient_initials": "FIXTURE"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing patient fixture id")?
		.to_string())
}

async fn create_past_drug_history_fixture(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
) -> Result<String> {
	let patient_id = create_patient_fixture(app, cookie, case_id).await?;
	let (status, body) = post_json(
		app,
		cookie,
		&format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({
			"data": {
				"patient_id": patient_id,
				"sequence_number": 1,
				"drug_name": "Fixture prior drug",
				"indication_meddra_code": "10012345"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	Ok(body["data"]["id"]
		.as_str()
		.ok_or("missing past drug fixture id")?
		.to_string())
}

pub async fn verify_field(spec: FieldSpec) -> Result<()> {
	let contract = contract_document(spec.page_id)?;
	let field = contract_field(&contract, spec.page_id, spec.code)?;
	let owner = field["patch"]["owner"]
		.as_str()
		.ok_or("missing patch owner")?;
	let payload_path = field["payloadPath"].as_str().ok_or("missing payloadPath")?;
	let projection_path = field["projectionPath"]
		.as_str()
		.ok_or("missing projectionPath")?;
	let round_trip = field["roundTripValue"].clone();
	let projection_value = field
		.get("projectionValue")
		.cloned()
		.unwrap_or_else(|| round_trip.clone());
	let authority = field["authority"]
		.as_str()
		.ok_or("missing authority")?
		.to_lowercase();

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE users SET access_blind_allowed = true WHERE id = $1",
			)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-{}-{}", spec.page_id, Uuid::new_v4()),
		&[&authority],
	)
	.await?;
	let sentinel_status =
		case_status(&mm, seed.admin.id, seed.org_id, &case_id).await?;
	let row_id = repeatable_row_id(spec.page_id, &app, &cookie, &case_id).await?;
	let mut row = field.get("fixture").cloned().unwrap_or_else(|| json!({}));
	merge_json(&mut row, build_path(payload_path, round_trip.clone()));
	if let Some(assessments) = row
		.get_mut("drugReactionAssessments")
		.and_then(Value::as_array_mut)
	{
		let reaction_id =
			create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
		for assessment in assessments {
			assessment
				.as_object_mut()
				.ok_or("drug reaction assessment fixture must be an object")?
				.insert("reactionId".to_string(), json!(reaction_id));
		}
	}
	let mut rows = contract
		.get("rowPrerequisites")
		.and_then(|fixtures| fixtures.get(owner))
		.cloned()
		.unwrap_or_else(|| json!({}));
	if let Some(field_fixture) = field.get("rowsFixture").cloned() {
		merge_json(&mut rows, field_fixture);
	}
	merge_json(&mut rows, json!({ owner: row }));
	let uri = row_id.as_ref().map_or_else(
		|| format!("/api/cases/{case_id}/editor/pages/{}", spec.page_id),
		|row_id| {
			format!(
				"/api/cases/{case_id}/editor/pages/{}/rows/{row_id}",
				spec.page_id
			)
		},
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&uri,
		json!({
			"authorities": [authority],
			"rows": rows
		}),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}/{}, {body}",
		spec.page_id,
		spec.code
	);

	let (status, body) = get_json(&app, &cookie, &uri).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}/{}, {body}",
		spec.page_id,
		spec.code
	);
	let projection_root = if row_id.is_some() {
		&body["data"]
	} else {
		&body["rows"]
	};
	let actual = read_path(projection_root, projection_path).ok_or_else(|| {
		format!(
			"missing projection {} for {}/{} in {}",
			projection_path, spec.page_id, spec.code, body
		)
	})?;
	assert_eq!(actual, &projection_value, "{}/{}", spec.page_id, spec.code);
	assert_eq!(
		case_status(&mm, seed.admin.id, seed.org_id, &case_id).await?,
		sentinel_status,
		"{}/{} changed unrelated case status",
		spec.page_id,
		spec.code
	);

	if let Some(invalid_value) = field
		.get("constraint")
		.and_then(|constraint| constraint.get("invalidValue"))
		.cloned()
	{
		let invalid_payload_path = field
			.get("constraint")
			.and_then(|constraint| constraint.get("ruleCode"))
			.and_then(Value::as_str)
			.filter(|rule| rule.ends_with(".NULLFLAVOR.ALLOWED"))
			.filter(|_| !payload_path.ends_with("NullFlavor"))
			.map_or_else(
				|| payload_path.to_string(),
				|_| format!("{payload_path}NullFlavor"),
			);
		let mut invalid_row =
			field.get("fixture").cloned().unwrap_or_else(|| json!({}));
		merge_json(
			&mut invalid_row,
			build_path(&invalid_payload_path, invalid_value),
		);
		let mut invalid_rows = contract
			.get("rowPrerequisites")
			.and_then(|fixtures| fixtures.get(owner))
			.cloned()
			.unwrap_or_else(|| json!({}));
		if let Some(field_fixture) = field.get("rowsFixture").cloned() {
			merge_json(&mut invalid_rows, field_fixture);
		}
		merge_json(&mut invalid_rows, json!({ owner: invalid_row }));
		let (status, body) = patch_json(
			&app,
			&cookie,
			&uri,
			json!({
				"authorities": [authority],
				"rows": invalid_rows
			}),
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::UNPROCESSABLE_ENTITY,
			"{}/{}, {body}",
			spec.page_id,
			spec.code
		);
		assert_eq!(body["error"]["message"], "CONSTRAINT_VIOLATION");
		assert_eq!(
			body["error"]["data"]["detail"]["ruleCode"],
			field["constraint"]["ruleCode"]
		);
		if let Some(issue_path) = field["constraint"]["issuePath"].as_str() {
			assert_eq!(
				body["error"]["data"]["detail"]["path"],
				issue_path.replace("[]", ".0")
			);
		}
	}
	Ok(())
}

macro_rules! field_contract_test {
	($name:ident, $code:literal) => {
		#[serial_test::serial]
		#[tokio::test]
		async fn $name() -> crate::common::Result<()> {
			super::support::verify_field(super::support::FieldSpec {
				page_id: PAGE_ID,
				code: $code,
			})
			.await
		}
	};
}

pub(crate) use field_contract_test;

/// Compare the batched page with canonical row routes using real persisted rows.
pub(super) async fn assert_batched_rows(
	app: &axum::Router,
	cookie: &str,
	case_id: &str,
	section: &str,
) -> Result<()> {
	let uri = format!("/api/cases/{case_id}/editor/pages/{section}");
	let (status, page) = get_json(app, cookie, &uri).await?;
	assert_eq!(status, StatusCode::OK, "{page}");
	assert!(page["rows"].get("rowDetails").is_none());
	let (status, expanded) =
		get_json(app, cookie, &format!("{uri}?include_rows=true")).await?;
	assert_eq!(status, StatusCode::OK, "{expanded}");
	assert_eq!(expanded["rows"]["rows"], page["rows"]["rows"]);
	let rows = page["rows"]["rows"].as_array().ok_or("missing rows")?;
	let details = expanded["rows"]["rowDetails"]
		.as_object()
		.ok_or("missing batched details")?;
	assert_eq!(details.len(), rows.len());
	for row in rows {
		let id = row["id"].as_str().ok_or("missing row id")?;
		let (status, individual) =
			get_json(app, cookie, &format!("{uri}/rows/{id}")).await?;
		assert_eq!(status, StatusCode::OK, "{individual}");
		assert_eq!(details.get(id).ok_or("missing canonical row")?, &individual);
	}
	Ok(())
}
