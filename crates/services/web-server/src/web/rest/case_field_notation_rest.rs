use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::e2b_field_notation::E2bFieldNotationBmc;
use lib_core::model::ModelManager;
use lib_rest_core::{Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldNotationQuery {
	e2b_code: String,
	record_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveFieldNotationRequest {
	e2b_code: String,
	record_id: Option<Uuid>,
	notation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldNotationResponse {
	id: Option<Uuid>,
	e2b_code: String,
	record_id: Option<Uuid>,
	notation: Option<String>,
}

fn validate_e2b_code(e2b_code: &str) -> Result<&str> {
	let e2b_code = e2b_code.trim();
	if e2b_code.is_empty() || e2b_code.len() > 64 {
		return Err(Error::BadRequest {
			message: "e2bCode must contain 1-64 characters".to_string(),
		});
	}
	if !e2b_code
		.chars()
		.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
	{
		return Err(Error::BadRequest {
			message: "e2bCode contains invalid characters".to_string(),
		});
	}
	Ok(e2b_code)
}

pub async fn get_field_notation(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<FieldNotationQuery>,
) -> Result<(StatusCode, Json<FieldNotationResponse>)> {
	let ctx = ctx_w.0;
	let e2b_code = validate_e2b_code(&query.e2b_code)?.to_string();
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"field-notation",
		move |ctx, mm| {
			Box::pin(async move {
				let row = E2bFieldNotationBmc::get(
					ctx,
					mm,
					case_id,
					query.record_id,
					&e2b_code,
				)
				.await
				.map_err(Error::Model)?;
				Ok((
					StatusCode::OK,
					Json(FieldNotationResponse {
						id: row.as_ref().map(|row| row.id),
						e2b_code,
						record_id: query.record_id,
						notation: row.map(|row| row.notation),
					}),
				))
			})
		},
	)
	.await
}

pub async fn save_field_notation(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<SaveFieldNotationRequest>,
) -> Result<(StatusCode, Json<FieldNotationResponse>)> {
	let ctx = ctx_w.0;
	let e2b_code = validate_e2b_code(&request.e2b_code)?.to_string();
	if request.notation.len() > 10_000 {
		return Err(Error::BadRequest {
			message: "notation must not exceed 10000 characters".to_string(),
		});
	}
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"field-notation",
		move |ctx, mm| {
			Box::pin(async move {
				let row = E2bFieldNotationBmc::upsert(
					ctx,
					mm,
					case_id,
					request.record_id,
					&e2b_code,
					request.notation.trim(),
				)
				.await
				.map_err(Error::Model)?;
				Ok((
					StatusCode::OK,
					Json(FieldNotationResponse {
						id: Some(row.id),
						e2b_code: row.e2b_code,
						record_id: row.record_id,
						notation: Some(row.notation),
					}),
				))
			})
		},
	)
	.await
}

pub async fn delete_field_notation(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<FieldNotationQuery>,
) -> Result<(StatusCode, Json<FieldNotationResponse>)> {
	let ctx = ctx_w.0;
	let e2b_code = validate_e2b_code(&query.e2b_code)?.to_string();
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"field-notation",
		move |ctx, mm| {
			Box::pin(async move {
				E2bFieldNotationBmc::delete(
					ctx,
					mm,
					case_id,
					query.record_id,
					&e2b_code,
				)
				.await
				.map_err(Error::Model)?;
				Ok((
					StatusCode::OK,
					Json(FieldNotationResponse {
						id: None,
						e2b_code,
						record_id: query.record_id,
						notation: None,
					}),
				))
			})
		},
	)
	.await
}
