use crate::web::rest::compliance::{capture_e_signature, ComplianceActionInput};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::Json;
use lib_core::model::acs::{CASE_READ, CASE_UPDATE};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Serialize;
use uuid::Uuid;

use crate::submission::{
	apply_gateway_ack_by_remote, apply_mock_ack, create_submission_idempotent,
	get_reconcile_runtime_status, get_submission, get_submission_dispatch_state,
	list_by_case, list_submission_events, list_submission_history,
	reconcile_due_submissions_with_runtime_status, GatewayAckCallbackInput,
	MockAckInput, SubmissionAuthority, SubmissionDispatchStateRecord,
	SubmissionEventRecord, SubmissionHistoryRecord, SubmissionReconcileResult,
	SubmissionReconcileRuntimeStatus, SubmissionRecord,
};

#[derive(Debug, Serialize)]
pub struct CaseSubmissionList {
	pub items: Vec<SubmissionRecord>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionEventList {
	pub items: Vec<SubmissionEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionHistoryList {
	pub items: Vec<SubmissionHistoryRecord>,
}

#[derive(Debug, Serialize)]
pub struct SubmissionDispatchStateData {
	pub state: SubmissionDispatchStateRecord,
}

#[derive(Debug, Serialize)]
pub struct SubmissionReconcileData {
	pub result: SubmissionReconcileResult,
}

#[derive(Debug, Serialize)]
pub struct SubmissionReconcileStatusData {
	pub status: SubmissionReconcileRuntimeStatus,
}

#[derive(Debug, serde::Deserialize)]
pub struct ReconcileRequestInput {
	pub limit: Option<i64>,
}

/// POST /api/cases/{id}/submissions/fda
pub async fn submit_case_to_fda(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	headers: HeaderMap,
	payload: Option<Json<ComplianceActionInput>>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let payload = payload.ok_or(Error::BadRequest {
		message: "reason_for_change and e_signature are required for submission"
			.to_string(),
	})?;
	let compliance = payload.0;
	compliance.validate()?;
	let authority = SubmissionAuthority::Fda;
	let signature_id = capture_e_signature(
		&ctx,
		&mm,
		Some(case_id),
		"CASE_SUBMISSION",
		&compliance,
	)
	.await?;
	let ctx_with_compliance = ctx.with_compliance(
		Some(compliance.reason_for_change.trim().to_string()),
		Some(signature_id),
	);
	let idempotency_key = headers.get("x-idempotency-key").and_then(header_to_str);
	let record = create_submission_idempotent(
		&ctx_with_compliance,
		&mm,
		case_id,
		authority,
		idempotency_key,
	)
	.await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: record })))
}

/// POST /api/cases/{id}/submissions/mfds
pub async fn submit_case_to_mfds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	headers: HeaderMap,
	payload: Option<Json<ComplianceActionInput>>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let payload = payload.ok_or(Error::BadRequest {
		message: "reason_for_change and e_signature are required for submission"
			.to_string(),
	})?;
	let compliance = payload.0;
	compliance.validate()?;
	let authority = SubmissionAuthority::Mfds;
	let signature_id = capture_e_signature(
		&ctx,
		&mm,
		Some(case_id),
		"CASE_SUBMISSION",
		&compliance,
	)
	.await?;
	let ctx_with_compliance = ctx.with_compliance(
		Some(compliance.reason_for_change.trim().to_string()),
		Some(signature_id),
	);
	let idempotency_key = headers.get("x-idempotency-key").and_then(header_to_str);
	let record = create_submission_idempotent(
		&ctx_with_compliance,
		&mm,
		case_id,
		authority,
		idempotency_key,
	)
	.await?;
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

/// GET /api/submissions/{id}/events
pub async fn list_submission_event_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(submission_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionEventList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let rows = list_submission_events(&ctx, &mm, submission_id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionEventList { items: rows },
		}),
	))
}

/// GET /api/submissions/history
pub async fn list_all_submission_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionHistoryList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let rows = list_submission_history(&ctx, &mm).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionHistoryList { items: rows },
		}),
	))
}

/// GET /api/submissions/{id}/dispatch-state
pub async fn get_submission_dispatch_state_view(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(submission_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SubmissionDispatchStateData>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let state = get_submission_dispatch_state(&ctx, &mm, submission_id)
		.await?
		.ok_or(Error::BadRequest {
			message: format!("submission dispatch state not found: {submission_id}"),
		})?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionDispatchStateData { state },
		}),
	))
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

/// POST /internal/submissions/callbacks/ack
pub async fn post_gateway_ack_callback(
	State(mm): State<ModelManager>,
	headers: HeaderMap,
	Json(input): Json<GatewayAckCallbackInput>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let expected =
		std::env::var("AS2_CALLBACK_TOKEN").map_err(|_| Error::BadRequest {
			message: "AS2_CALLBACK_TOKEN is required for gateway callback endpoint"
				.to_string(),
		})?;
	let incoming = headers
		.get("x-callback-token")
		.and_then(|v| header_to_str(v))
		.ok_or(Error::BadRequest {
			message: "missing x-callback-token".to_string(),
		})?;
	if incoming != expected {
		return Err(Error::BadRequest {
			message: "invalid x-callback-token".to_string(),
		});
	}
	let record = apply_gateway_ack_by_remote(&mm, input).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: record })))
}

/// POST /internal/submissions/reconcile
pub async fn post_reconcile_due_submissions(
	State(mm): State<ModelManager>,
	headers: HeaderMap,
	payload: Option<Json<ReconcileRequestInput>>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionReconcileData>>)> {
	validate_internal_token(&headers)?;
	let limit = payload.and_then(|p| p.0.limit).unwrap_or(25);
	let result = reconcile_due_submissions_with_runtime_status(&mm, limit).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionReconcileData { result },
		}),
	))
}

/// GET /internal/submissions/reconcile/status
pub async fn get_reconcile_status(
	headers: HeaderMap,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SubmissionReconcileStatusData>>,
)> {
	validate_internal_token(&headers)?;
	let status = get_reconcile_runtime_status();
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionReconcileStatusData { status },
		}),
	))
}

fn header_to_str(value: &HeaderValue) -> Option<String> {
	value.to_str().ok().map(|v| v.to_string())
}

fn validate_internal_token(headers: &HeaderMap) -> Result<()> {
	let expected =
		std::env::var("AS2_CALLBACK_TOKEN").map_err(|_| Error::BadRequest {
			message:
				"AS2_CALLBACK_TOKEN is required for internal submission endpoints"
					.to_string(),
		})?;
	let incoming = headers
		.get("x-callback-token")
		.and_then(header_to_str)
		.ok_or(Error::BadRequest {
			message: "missing x-callback-token".to_string(),
		})?;
	if incoming != expected {
		return Err(Error::BadRequest {
			message: "invalid x-callback-token".to_string(),
		});
	}
	Ok(())
}
