use crate::web::rest::compliance::{capture_e_signature, ComplianceActionInput};
use axum::extract::{Path, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::model::acs::{XML_EXPORT, XML_EXPORT_READ};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Serialize;
use uuid::Uuid;

use crate::submission::{
	apply_gateway_ack_by_remote, apply_mock_ack, create_submission_idempotent,
	get_ack_download, get_reconcile_runtime_status, get_submission,
	get_submission_dispatch_state, list_by_case, list_submission_events,
	list_submission_history, reconcile_due_submissions_with_runtime_status,
	GatewayAckCallbackInput, MockAckInput, SubmissionAuthority,
	SubmissionDispatchStateRecord, SubmissionEventRecord, SubmissionHistoryRecord,
	SubmissionReconcileResult, SubmissionReconcileRuntimeStatus, SubmissionRecord,
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
	require_permission(&ctx, XML_EXPORT)?;
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
	require_permission(&ctx, XML_EXPORT)?;
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
	require_permission(&ctx, XML_EXPORT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &_mm, case_id).await?;
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
	require_permission(&ctx, XML_EXPORT_READ)?;
	let record = get_submission(&ctx, &_mm, submission_id).await?.ok_or(
		Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		},
	)?;
	lib_rest_core::require_case_read_allowed(&ctx, &_mm, record.case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: record })))
}

/// GET /api/submissions/{id}/events
pub async fn list_submission_event_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(submission_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionEventList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;
	let record = get_submission(&ctx, &mm, submission_id).await?.ok_or(
		Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		},
	)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, record.case_id).await?;
	let rows = list_submission_events(&ctx, &mm, submission_id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: SubmissionEventList { items: rows },
		}),
	))
}

/// GET /api/submissions/{id}/acks/{level}/download
pub async fn download_submission_ack_text(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((submission_id, level)): Path<(Uuid, u8)>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;
	let ack = get_ack_download(&ctx, &mm, submission_id, level)
		.await?
		.ok_or(Error::BadRequest {
			message: format!(
				"submission ACK level {level} not found for submission {submission_id}"
			),
		})?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, ack.case_id).await?;

	let mut text = format!(
		"Submission ID: {}\nCase ID: {}\nACK Level: {}\nSuccess: {}\nReceived At: {}\n",
		ack.submission_id, ack.case_id, ack.level, ack.success, ack.received_at
	);
	if let Some(code) = ack.code.as_deref() {
		text.push_str(&format!("ACK Code: {code}\n"));
	}
	if let Some(message) = ack.message.as_deref() {
		text.push_str(&format!("ACK Message: {message}\n"));
	}
	if let Some(raw_payload) = ack.raw_payload.as_ref() {
		let pretty = serde_json::to_string_pretty(raw_payload)
			.unwrap_or_else(|_| raw_payload.to_string());
		text.push_str("\nRaw Payload:\n");
		text.push_str(&pretty);
		text.push('\n');
	}

	let file_name = format!("submission-{submission_id}-ack{level}.txt");
	let mut response = (StatusCode::OK, text).into_response();
	response.headers_mut().insert(
		header::CONTENT_TYPE,
		HeaderValue::from_static("text/plain; charset=utf-8"),
	);
	response.headers_mut().insert(
		header::CONTENT_DISPOSITION,
		HeaderValue::from_str(&format!("attachment; filename=\"{file_name}\""))
			.map_err(|err| Error::BadRequest {
				message: format!("invalid ACK download filename header: {err}"),
			})?,
	);
	Ok(response)
}

/// GET /api/submissions/history
pub async fn list_all_submission_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionHistoryList>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, XML_EXPORT_READ)?;
	let dbx = mm.dbx();
	dbx.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	if let Err(err) =
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await
			.map_err(Error::from)
	{
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}
	let history = match list_submission_history(&ctx, &mm).await {
		Ok(history) => {
			dbx.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			history
		}
		Err(err) => {
			let _ = dbx.rollback_txn().await;
			return Err(err);
		}
	};
	let mut rows = Vec::with_capacity(history.len());
	for row in history {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, row.case_id).await? {
			rows.push(row);
		}
	}
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
	require_permission(&ctx, XML_EXPORT_READ)?;
	let record = get_submission(&ctx, &mm, submission_id).await?.ok_or(
		Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		},
	)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, record.case_id).await?;
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
	require_permission(&ctx, XML_EXPORT)?;
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
