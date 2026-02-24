use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{CASE_READ, CASE_UPDATE};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Serialize;
use uuid::Uuid;

use crate::submission::{
	apply_mock_ack, create_fda_submission, get_submission, list_by_case,
	MockAckInput, SubmissionRecord,
};

#[derive(Debug, Serialize)]
pub struct CaseSubmissionList {
	pub items: Vec<SubmissionRecord>,
}

/// POST /api/cases/{id}/submissions/fda
pub async fn submit_case_to_fda(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let record = create_fda_submission(&ctx, &mm, case_id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: record })))
}

/// GET /api/cases/{id}/submissions
pub async fn list_case_submissions(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSubmissionList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let rows = list_by_case(&ctx, &_mm, case_id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseSubmissionList { items: rows },
		}),
	))
}

/// GET /api/submissions/{id}
pub async fn get_case_submission(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(submission_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let record = get_submission(&ctx, &_mm, submission_id).await?.ok_or(
		Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		},
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: record })))
}

/// POST /api/submissions/{id}/acks/mock
pub async fn post_mock_ack(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(submission_id): Path<Uuid>,
	Json(input): Json<MockAckInput>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let record = apply_mock_ack(&ctx, &_mm, submission_id, input).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: record })))
}
