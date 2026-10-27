use crate::web::rest::compliance::{capture_e_signature, ComplianceActionInput};
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::model::authorization::CaseMutationKind;
use lib_core::model::message_header::MessageHeader;
use lib_core::model::submission_receiver_option::{
	SubmissionReceiverOption, SubmissionReceiverOptionBmc,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::ParamsForUpdate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::submission::prepare_outbound_message_header;
use crate::submission::{
	apply_gateway_ack_by_remote, create_submission_idempotent,
	dispatch_reserved_submission, get_ack_download, get_reconcile_runtime_status,
	get_submission, get_submission_artifact, get_submission_dispatch_state,
	list_by_case, list_submission_events, list_submission_history,
	reconcile_due_submissions_with_runtime_status, GatewayAckCallbackInput,
	SubmissionAuthority, SubmissionDispatchStateRecord, SubmissionEventRecord,
	SubmissionHistoryRecord, SubmissionReconcileResult,
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
pub struct SubmissionReceiverOptionList {
	pub items: Vec<SubmissionReceiverOption>,
}

#[derive(Debug, Deserialize)]
pub struct SubmissionReceiverSelectionInput {
	pub authority: String,
	pub receiver_label: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct ReceiverOptionsQuery {
	pub authority: String,
}

/// POST /api/cases/{id}/submissions/fda
pub async fn submit_case_to_fda(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	headers: HeaderMap,
	payload: Option<Json<ComplianceActionInput>>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	let (authorized_ctx, record, created) =
		lib_rest_core::with_authorized_case_mutation(
			&ctx,
			&snapshot,
			&mm,
			case_id,
			"submission.execute",
			CaseMutationKind::Submission,
			move |ctx, mm| {
				Box::pin(submit_case_authorized(
					ctx,
					mm,
					case_id,
					SubmissionAuthority::Fda,
					headers,
					payload,
				))
			},
		)
		.await?;
	let record = if created {
		dispatch_reserved_submission(&authorized_ctx, &mm, record).await?
	} else {
		record
	};
	Ok((StatusCode::CREATED, Json(DataRestResult { data: record })))
}

/// POST /api/cases/{id}/submissions/mfds
pub async fn submit_case_to_mfds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	headers: HeaderMap,
	payload: Option<Json<ComplianceActionInput>>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	let (authorized_ctx, record, created) =
		lib_rest_core::with_authorized_case_mutation(
			&ctx,
			&snapshot,
			&mm,
			case_id,
			"submission.execute",
			CaseMutationKind::Submission,
			move |ctx, mm| {
				Box::pin(submit_case_authorized(
					ctx,
					mm,
					case_id,
					SubmissionAuthority::Mfds,
					headers,
					payload,
				))
			},
		)
		.await?;
	let record = if created {
		dispatch_reserved_submission(&authorized_ctx, &mm, record).await?
	} else {
		record
	};
	Ok((StatusCode::CREATED, Json(DataRestResult { data: record })))
}

async fn submit_case_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	headers: HeaderMap,
	payload: Option<Json<ComplianceActionInput>>,
) -> Result<(lib_core::ctx::Ctx, SubmissionRecord, bool)> {
	let payload = payload.ok_or(Error::BadRequest {
		message: "reason_for_change and e_signature are required for submission"
			.to_string(),
	})?;
	let compliance = payload.0;
	compliance.validate()?;
	let signature_id =
		capture_e_signature(ctx, mm, Some(case_id), "CASE_SUBMISSION", &compliance)
			.await?;
	let ctx_with_compliance = ctx.with_compliance(
		Some(compliance.reason_for_change.trim().to_string()),
		Some(signature_id),
	);
	let idempotency_key = headers
		.get("x-idempotency-key")
		.map(|value| {
			value
				.to_str()
				.map(str::to_string)
				.map_err(|_| Error::BadRequest {
					message: "x-idempotency-key must be valid text".into(),
				})
		})
		.transpose()?;
	let (record, created) = create_submission_idempotent(
		&ctx_with_compliance,
		mm,
		case_id,
		authority,
		idempotency_key,
	)
	.await?;
	Ok((ctx_with_compliance, record, created))
}

/// GET /api/cases/{id}/submissions
pub async fn list_case_submissions(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<CaseSubmissionList>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_collection(
		&ctx,
		&snapshot,
		&_mm,
		move |ctx, mm| {
			Box::pin(async move {
				if !lib_rest_core::case_matches_user_scope(ctx, mm, case_id).await? {
					return Err(Error::PermissionDenied {
						required_permission: "submission.history.list".to_string(),
					});
				}
				let rows = list_by_case(ctx, mm, case_id).await?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: CaseSubmissionList { items: rows },
					}),
				))
			})
		},
	)
	.await
}

/// GET /api/submissions/receiver-options
pub async fn list_receiver_options(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Query(query): Query<ReceiverOptionsQuery>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SubmissionReceiverOptionList>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_collection_action(
		&ctx,
		&snapshot,
		&mm,
		"submission.receiver.list",
		move |ctx, mm| {
			Box::pin(async move {
				let authority = query.authority.trim().to_ascii_lowercase();
				if !matches!(authority.as_str(), "fda" | "mfds") {
					return Err(Error::BadRequest {
						message: format!(
							"unsupported receiver option authority: {}",
							query.authority
						),
					});
				}
				let rows = SubmissionReceiverOptionBmc::list_by_authority(
					ctx, mm, &authority,
				)
				.await?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: SubmissionReceiverOptionList { items: rows },
					}),
				))
			})
		},
	)
	.await
}

/// PUT /api/cases/{id}/submission-receiver
pub async fn apply_submission_receiver_selection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<SubmissionReceiverSelectionInput>>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"submission.execute",
		CaseMutationKind::SubmissionReceiver,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let authority = lib_core::regulatory::RegulatoryAuthority::parse(
					data.authority.trim(),
				)
				.ok_or_else(|| Error::BadRequest {
					message: format!(
						"unsupported receiver authority: {}",
						data.authority
					),
				})?;
				let entity = prepare_outbound_message_header(
					ctx,
					mm,
					case_id,
					authority,
					data.receiver_label.as_deref(),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/submissions/{id}
pub async fn get_case_submission(
	State(_mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(submission_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionRecord>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_read(
		&ctx,
		&snapshot,
		&_mm,
		submission_id,
		move |ctx, mm| {
			Box::pin(async move {
				let record = get_submission(ctx, mm, submission_id).await?.ok_or(
					Error::BadRequest {
						message: format!("submission not found: {submission_id}"),
					},
				)?;
				Ok((StatusCode::OK, Json(DataRestResult { data: record })))
			})
		},
	)
	.await
}

/// GET /api/submissions/{id}/events
pub async fn list_submission_event_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(submission_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionEventList>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_read(
		&ctx,
		&snapshot,
		&mm,
		submission_id,
		move |ctx, mm| {
			Box::pin(async move {
				let rows = list_submission_events(ctx, mm, submission_id).await?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: SubmissionEventList { items: rows },
					}),
				))
			})
		},
	)
	.await
}

/// GET /api/submissions/{id}/acks/{level}/download
pub async fn download_submission_ack_text(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((submission_id, level)): Path<(Uuid, u8)>,
) -> Result<Response> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_read(
		&ctx,
		&snapshot,
		&mm,
		submission_id,
		move |ctx, mm| {
			Box::pin(async move {
				let ack = get_ack_download(ctx, mm, submission_id, level)
					.await?
					.ok_or(Error::BadRequest {
						message: format!(
				"submission ACK level {level} not found for submission {submission_id}"
			),
					})?;
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
					HeaderValue::from_str(&format!(
						"attachment; filename=\"{file_name}\""
					))
					.map_err(|err| Error::BadRequest {
						message: format!(
							"invalid ACK download filename header: {err}"
						),
					})?,
				);
				Ok(response)
			})
		},
	)
	.await
}

/// GET /api/submissions/history
pub async fn list_all_submission_history(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<SubmissionHistoryList>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm| {
			Box::pin(async move {
				let history = list_submission_history(ctx, mm).await?;
				let mut rows = Vec::with_capacity(history.len());
				for row in history {
					if lib_rest_core::case_matches_user_scope(ctx, mm, row.case_id)
						.await?
					{
						rows.push(row);
					}
				}
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: SubmissionHistoryList { items: rows },
					}),
				))
			})
		},
	)
	.await
}

/// GET /api/submissions/{id}/dispatch-state
pub async fn get_submission_dispatch_state_view(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(submission_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SubmissionDispatchStateData>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_submission_read(
		&ctx,
		&snapshot,
		&mm,
		submission_id,
		move |ctx, mm| {
			Box::pin(async move {
				let state = get_submission_dispatch_state(ctx, mm, submission_id)
					.await?
					.ok_or(Error::BadRequest {
						message: format!(
							"submission dispatch state not found: {submission_id}"
						),
					})?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: SubmissionDispatchStateData { state },
					}),
				))
			})
		},
	)
	.await
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
		.and_then(header_to_str)
		.ok_or(Error::BadRequest {
			message: "missing x-callback-token".to_string(),
		})?;
	if !tokens_equal(&expected, &incoming) {
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
	let expected = std::env::var("SUBMISSION_RECONCILE_TOKEN").map_err(|_| {
		Error::BadRequest {
			message:
				"SUBMISSION_RECONCILE_TOKEN is required for reconcile endpoints"
					.to_string(),
		}
	})?;
	let incoming = headers
		.get("x-reconcile-token")
		.and_then(header_to_str)
		.ok_or(Error::BadRequest {
			message: "missing x-reconcile-token".to_string(),
		})?;
	if !tokens_equal(&expected, &incoming) {
		return Err(Error::BadRequest {
			message: "invalid x-reconcile-token".to_string(),
		});
	}
	Ok(())
}

fn tokens_equal(expected: &str, incoming: &str) -> bool {
	let expected = expected.as_bytes();
	let incoming = incoming.as_bytes();
	let mut difference = expected.len() ^ incoming.len();
	for index in 0..expected.len().max(incoming.len()) {
		let expected_byte = expected.get(index).copied().unwrap_or(0);
		let incoming_byte = incoming.get(index).copied().unwrap_or(0);
		difference |= usize::from(expected_byte ^ incoming_byte);
	}
	difference == 0
}

/// Download the immutable bytes captured for this attempt, never the current Case.
pub async fn download_submission_xml(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<Response> {
	let record = lib_rest_core::with_authorized_submission_read(
		&ctx_w.0,
		&snapshot,
		&mm,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				get_submission(ctx, mm, id).await?.ok_or(Error::BadRequest {
					message: "submission not found".into(),
				})
			})
		},
	)
	.await?;
	lib_rest_core::with_authorized_case_export(
		&ctx_w.0,
		&snapshot,
		&mm,
		&[record.case_id],
		move |ctx, mm| {
			Box::pin(async move {
				let (_, xml) = get_submission_artifact(ctx, mm, id).await?;
				lib_core::model::xml_export_history::XmlExportHistoryBmc::record(
					mm,
					ctx,
					record.case_id,
					None,
					&format!("submission-{id}.xml"),
					"success",
					None,
				)
				.await?;
				let mut response = (StatusCode::OK, xml).into_response();
				response.headers_mut().insert(
					header::CONTENT_TYPE,
					HeaderValue::from_static("application/xml"),
				);
				response.headers_mut().insert(
					header::CONTENT_DISPOSITION,
					HeaderValue::from_str(&format!(
						"attachment; filename=\"submission-{id}.xml\""
					))
					.map_err(|e| Error::BadRequest {
						message: e.to_string(),
					})?,
				);
				Ok(response)
			})
		},
	)
	.await
}
