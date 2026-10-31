pub(super) mod ae_restore;
pub(super) mod ci_rows;
pub(super) mod device_children;
pub(super) mod dg_restore;
pub(super) mod dg_rows;
pub(super) mod dm_rows;
pub(super) mod documents;
pub(super) mod lb_restore;
pub(super) mod nr_rows;
pub(super) mod parents;
pub(super) mod relatedness;
pub(super) mod restore;
pub(super) mod rp_rows;
pub(super) mod scope;
pub(super) mod si_rows;

// Shared by all five row verifiers so their existing setup also checks retries.
pub(super) async fn assert_row_lifecycle_replay(
	app: &axum::Router,
	cookie: &str,
	mm: &lib_core::model::ModelManager,
	seed: &crate::common::SeedOrgUsers,
	lifecycle: &serde_json::Value,
	page_uri: &str,
	restore_request: &serde_json::Value,
) -> crate::common::Result<()> {
	use super::{audit, support};
	use axum::body::Body;
	use axum::http::{Request, StatusCode};
	use serde_json::Value;
	use tower::ServiceExt;
	let ctx = &lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;

	fn row_state(data: &Value, owner: &str) -> crate::common::Result<Value> {
		let mut row = data[owner]
			.as_object()
			.ok_or("missing persisted row")?
			.clone();
		// Updating an unchanged row may still touch its modification metadata.
		row.remove("updated_at");
		row.remove("updated_by");
		Ok(Value::Object(row))
	}

	let rows = restore_request["rows"].as_object().ok_or("missing rows")?;
	assert_eq!(rows.len(), 1);
	let (owner, row) = rows.iter().next().ok_or("missing row owner")?;
	let row_id = row["id"].as_str().ok_or("missing persisted row id")?;
	let row_uri = format!("{page_uri}/rows/{row_id}");
	let case_id = uuid::Uuid::parse_str(
		page_uri
			.strip_prefix("/api/cases/")
			.and_then(|rest| rest.split('/').next())
			.ok_or("invalid editor page URI")?,
	)?;
	let page = page_uri.rsplit('/').next().ok_or("missing page")?;
	let has_post_restore = matches!(page, "AE" | "LB" | "DG");
	let repeat_delete_status = if has_post_restore {
		StatusCode::NOT_FOUND
	} else {
		StatusCode::NO_CONTENT
	};
	let (status, original) = support::get_json(app, cookie, &row_uri).await?;
	assert_eq!(status, StatusCode::OK, "{original}");
	let original_row = row_state(&original["data"], owner)?;
	assert_eq!(original_row["id"], row_id);
	assert_eq!(original_row["deleted"], false);
	let (status, list) = support::get_json(app, cookie, page_uri).await?;
	assert_eq!(status, StatusCode::OK, "{list}");
	let original_ids: Vec<Value> = list["rows"]["rows"]
		.as_array()
		.ok_or("missing row list")?
		.iter()
		.map(|row| row["id"].clone())
		.collect();
	assert_eq!(
		original_ids
			.iter()
			.filter(|id| id.as_str() == Some(row_id))
			.count(),
		1
	);

	let mut ordinary_request = restore_request.clone();
	ordinary_request["rows"][owner]
		.as_object_mut()
		.ok_or("missing restore row")?
		.remove("deleted");
	let mut calls = vec![
		("PATCH", restore_request.clone(), StatusCode::OK, false),
		("PATCH", restore_request.clone(), StatusCode::OK, false),
		("DELETE", Value::Null, StatusCode::NO_CONTENT, true),
		("DELETE", Value::Null, repeat_delete_status, true),
		("PATCH", ordinary_request, StatusCode::OK, true),
	];
	if has_post_restore {
		calls.push(("POST", Value::Null, StatusCode::OK, false));
		calls.push(("POST", Value::Null, StatusCode::OK, false));
	} else {
		calls.push(("PATCH", restore_request.clone(), StatusCode::OK, false));
	}
	calls.push(("PATCH", restore_request.clone(), StatusCode::OK, false));
	for (method, request, expected_status, deleted) in calls {
		let (status, before) = support::get_json(app, cookie, &row_uri).await?;
		assert_eq!(status, StatusCode::OK, "{before}");
		let audit_before =
			audit::capture(mm, ctx, case_id, &lifecycle["audit"], &before["data"])
				.await?;
		let uri = if method == "POST" {
			format!("{row_uri}/restore")
		} else {
			row_uri.clone()
		};
		let body = if request.is_null() {
			Body::empty()
		} else {
			Body::from(request.to_string())
		};
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method(method)
					.uri(&uri)
					.header("cookie", cookie)
					.header("content-type", "application/json")
					.header("x-e2br3-reason-for-change", support::ROW_CHANGE_REASON)
					.body(body)?,
			)
			.await?;
		assert_eq!(response.status(), expected_status, "{method} {uri}");
		let (status, after) = support::get_json(app, cookie, &row_uri).await?;
		assert_eq!(status, StatusCode::OK, "{after}");
		let mut expected_row = original_row.clone();
		expected_row["deleted"] = Value::Bool(deleted);
		assert_eq!(
			row_state(&after["data"], owner)?,
			expected_row,
			"{method} {uri} changed identity, values or children"
		);
		audit::assert_mutation(
			mm,
			ctx,
			case_id,
			&lifecycle["audit"],
			audit_before,
			&after["data"],
			support::ROW_CHANGE_REASON,
		)
		.await?;
		let (status, list) = support::get_json(app, cookie, page_uri).await?;
		assert_eq!(status, StatusCode::OK, "{list}");
		let ids: Vec<Value> = list["rows"]["rows"]
			.as_array()
			.ok_or("missing row list")?
			.iter()
			.map(|row| row["id"].clone())
			.collect();
		let expected_ids: Vec<Value> = original_ids
			.iter()
			.filter(|id| !deleted || id.as_str() != Some(row_id))
			.cloned()
			.collect();
		assert_eq!(
			ids, expected_ids,
			"{method} {uri} changed the row count/identity"
		);
	}
	let mut guarded = vec![
		("PATCH", row_uri.clone(), restore_request.clone()),
		("DELETE", row_uri.clone(), Value::Null),
	];
	if has_post_restore {
		guarded.push(("POST", format!("{row_uri}/restore"), Value::Null));
	}
	let mut create_request = restore_request.clone();
	let create_row = create_request["rows"][owner]
		.as_object_mut()
		.ok_or("missing create row")?;
	create_row.remove("id");
	create_row.remove("deleted");
	guarded.push(("POST", format!("{page_uri}/rows"), create_request));
	assert_editor_request_guards(app, cookie, mm, seed, &row_uri, &guarded).await?;
	Ok(())
}

use super::audit::case_audit_ids;
use super::support::{get_json, patch_json};
use crate::common::Result;
use axum::http::StatusCode;
use lib_core::{ctx::Ctx, model::ModelManager};
use serde_json::{json, Value};

// Request and projection paths share these owners; singleton parentInfo is an object.
const SOFT_CHILD_TABLES: &[(&str, &str)] = &[
	("/documentsHeldBySender", "documents_held_by_sender"),
	("/otherCaseIdentifiers", "other_case_identifiers"),
	("/linkedReports", "linked_report_numbers"),
	("/primarySources", "primary_sources"),
	("/patientIdentifiers", "patient_identifiers"),
	("/medicalHistoryEpisodes", "medical_history_episodes"),
	("/reportedCauses", "reported_causes_of_death"),
	("/autopsyCauses", "autopsy_causes_of_death"),
	("/parentInfo", "parent_information"),
	("/parentMedicalHistory", "parent_medical_history"),
	("/parentPastDrugs", "parent_past_drug_history"),
	("/studyRegistrationNumbers", "study_registration_numbers"),
	(
		"/studyInformation/fdaCrossReportedIndNumbers",
		"study_fda_cross_reported_inds",
	),
	("/senderDiagnoses", "sender_diagnoses"),
	("/caseSummaryInformation", "case_summary_information"),
	("/drug/activeSubstances", "drug_active_substances"),
	("/drug/dosageInformation", "dosage_information"),
	("/drug/indications", "drug_indications"),
	("/drug/drugReactionAssessments", "relatedness_assessments"),
];

fn editor_case_id(uri: &str) -> Result<&str> {
	uri.strip_prefix("/api/cases/")
		.and_then(|rest| rest.split('/').next())
		.ok_or_else(|| "invalid editor page URI".into())
}

async fn patch_and_reload(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	uri: &str,
	payload: Value,
) -> Result<(StatusCode, Value)> {
	let case_id = uuid::Uuid::parse_str(editor_case_id(uri)?)?;
	let mut snapshots = Vec::new();
	for (path, table) in SOFT_CHILD_TABLES {
		let Some(value) = payload["rows"].pointer(path) else {
			continue;
		};
		let rows = match value {
			Value::Array(rows) => rows.as_slice(),
			Value::Object(_) => std::slice::from_ref(value),
			_ => return Err(format!("invalid lifecycle owner {path}").into()),
		};
		for row in rows {
			let Some(id) = row.get("id") else { continue };
			let id = uuid::Uuid::parse_str(id.as_str().ok_or("invalid row ID")?)?;
			let projection = json!({"id":id});
			let spec = json!({"table":table,"column":"deleted",
				"record":{"source":"projection","path":"id"}});
			let before =
				super::audit::capture(mm, ctx, case_id, &spec, &projection).await?;
			snapshots.push((spec, projection, before));
		}
	}
	let (status, response) = super::support::patch_json_with_reason(
		app,
		cookie,
		uri,
		payload,
		Some(super::support::ROW_CHANGE_REASON),
	)
	.await?;
	if !status.is_success() {
		for (_, _, before) in snapshots {
			super::audit::assert_rejected(mm, ctx, case_id, before).await?;
		}
		return Ok((status, response));
	}
	for (spec, projection, before) in snapshots {
		super::audit::assert_mutation(
			mm,
			ctx,
			case_id,
			&spec,
			before,
			&projection,
			super::support::ROW_CHANGE_REASON,
		)
		.await?;
	}
	let (read_status, persisted) = get_json(app, cookie, uri).await?;
	assert_eq!(read_status, StatusCode::OK, "{persisted}");
	let key = if uri.contains("/rows/") {
		"data"
	} else {
		"rows"
	};
	assert_eq!(
		response.get(key).ok_or("PATCH projection missing")?,
		persisted.get(key).ok_or("GET projection missing")?,
		"PATCH response differs from persisted GET"
	);
	Ok((status, persisted))
}

async fn toggle_editor_case_lock(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	locked: bool,
) -> Result<()> {
	let case_uri = format!("/api/cases/{}", editor_case_id(uri)?);
	let (status, before) = get_json(app, cookie, &case_uri).await?;
	assert_eq!(status, StatusCode::OK, "{before}");
	assert_eq!(before["data"]["is_locked"], !locked);
	let expected_status = before["data"]["status"]
		.as_str()
		.ok_or("missing case status")?;
	let (status, body) = super::support::post_json(
		app,
		cookie,
		&format!("{case_uri}/lock/toggle"),
		json!({"data":{"expected_status":expected_status}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	let (status, after) = get_json(app, cookie, &case_uri).await?;
	assert_eq!(status, StatusCode::OK, "{after}");
	assert_eq!(after["data"]["is_locked"], locked);
	Ok(())
}

// Each owner gets its own request so one denied row cannot mask another resource.
async fn assert_child_mutation_guards(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	seed: &crate::common::SeedOrgUsers,
	uri: &str,
	authority: &str,
	expected_owners: usize,
) -> Result<()> {
	let (status, saved) = get_json(app, cookie, uri).await?;
	assert_eq!(status, StatusCode::OK, "{saved}");
	let key = if uri.contains("/rows/") {
		"data"
	} else {
		"rows"
	};
	let mut requests = Vec::new();
	let mut owners = 0;
	for (path, _) in SOFT_CHILD_TABLES {
		let Some(value) = saved[key].pointer(path) else {
			continue;
		};
		let row = match value {
			Value::Array(rows) if !rows.is_empty() => &rows[0],
			Value::Object(_) if value.get("id").is_some() => value,
			_ => continue,
		};
		let id = row["id"].as_str().ok_or("guard row missing ID")?;
		owners += 1;
		for deleted in [None, Some(true), Some(false)] {
			let mut patch = json!({"id":id});
			if let Some(deleted) = deleted {
				patch["deleted"] = json!(deleted);
			}
			if value.is_array() {
				patch = json!([patch]);
			}
			for segment in path.trim_start_matches('/').split('/').rev() {
				patch = json!({segment:patch});
			}
			requests.push((
				"PATCH",
				uri.to_string(),
				json!({"authorities":[authority],"rows":patch}),
			));
		}
	}
	assert_eq!(
		owners, expected_owners,
		"lifecycle guard owner inventory changed: {uri}"
	);
	assert_editor_request_guards(app, cookie, mm, seed, uri, &requests).await
}

async fn assert_editor_request_guards(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	seed: &crate::common::SeedOrgUsers,
	uri: &str,
	requests: &[(&str, String, Value)],
) -> Result<()> {
	use crate::common::{cookie_header, seed_org_with_users};
	use lib_auth::token::generate_web_token;
	use tower::ServiceExt;
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let viewer = generate_web_token(&seed.viewer.email, seed.viewer.token_salt)?;
	let viewer_cookie = cookie_header(&viewer.to_string());
	let foreign = seed_org_with_users(mm, "adminpwd", "viewpwd").await?;
	let foreign_token =
		generate_web_token(&foreign.admin.email, foreign.admin.token_salt)?;
	let foreign_cookie = cookie_header(&foreign_token.to_string());
	assert!(!requests.is_empty());
	for (denied_cookie, expected_status, locked) in [
		(viewer_cookie.as_str(), StatusCode::FORBIDDEN, false),
		(foreign_cookie.as_str(), StatusCode::NOT_FOUND, false),
		(cookie, StatusCode::FORBIDDEN, true),
	] {
		if locked {
			toggle_editor_case_lock(app, cookie, uri, true).await?;
		}
		let (status, before) = get_json(app, cookie, uri).await?;
		assert_eq!(status, StatusCode::OK, "{before}");
		let audit_before = case_audit_ids(mm, &ctx, editor_case_id(uri)?).await?;
		let (status, read) = get_json(app, denied_cookie, uri).await?;
		assert_eq!(
			status,
			if expected_status == StatusCode::NOT_FOUND {
				StatusCode::NOT_FOUND
			} else {
				StatusCode::OK
			},
			"{read}"
		);
		for (method, request_uri, request) in requests {
			let response = app
				.clone()
				.oneshot(
					axum::http::Request::builder()
						.method(*method)
						.uri(request_uri)
						.header("cookie", denied_cookie)
						.header("content-type", "application/json")
						.header(
							"x-e2br3-reason-for-change",
							super::support::ROW_CHANGE_REASON,
						)
						.body(if request.is_null() {
							axum::body::Body::empty()
						} else {
							axum::body::Body::from(request.to_string())
						})?,
				)
				.await?;
			let status = response.status();
			let body =
				axum::body::to_bytes(response.into_body(), usize::MAX).await?;
			assert_eq!(
				status,
				expected_status,
				"{method} {request_uri}: {request}: {}",
				String::from_utf8_lossy(&body)
			);
			let (status, after) = get_json(app, cookie, uri).await?;
			assert_eq!(status, StatusCode::OK, "{after}");
			assert_eq!(
				after, before,
				"rejected row mutation changed persisted projection"
			);
			assert_eq!(
				case_audit_ids(mm, &ctx, editor_case_id(uri)?).await?,
				audit_before,
				"rejected row mutation wrote a case audit entry"
			);
		}
		if locked {
			toggle_editor_case_lock(app, cookie, uri, false).await?;
		}
	}
	Ok(())
}

// The rejected request must leave both the API projection and audit stream unchanged.
#[allow(clippy::too_many_arguments)]
async fn assert_malformed_row_id_rejected_without_mutation(
	app: &axum::Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	uri: &str,
	case_id: &str,
	authority: &str,
	rows: Value,
) -> Result<()> {
	let (status, before) = get_json(app, cookie, uri).await?;
	assert_eq!(status, StatusCode::OK, "{before}");
	let audit_before = case_audit_ids(mm, ctx, case_id).await?;
	let (status, body) = patch_json(
		app,
		cookie,
		uri,
		json!({"authorities":[authority],"rows":rows}),
	)
	.await?;
	assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
	let (status, after) = get_json(app, cookie, uri).await?;
	assert_eq!(status, StatusCode::OK, "{after}");
	assert_eq!(after, before, "malformed row ID mutated the projection");
	assert_eq!(
		case_audit_ids(mm, ctx, case_id).await?,
		audit_before,
		"malformed row ID created audit entries"
	);
	Ok(())
}
