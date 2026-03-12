use crate::web::rest::compliance::{capture_e_signature, ESignatureInput};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	SAFETY_REPORT_CREATE, SAFETY_REPORT_DELETE, SAFETY_REPORT_READ,
	SAFETY_REPORT_UPDATE,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::safety_report::{
	SafetyReportIdentification, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SafetyReportIdentificationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use std::borrow::Cow;
use uuid::Uuid;

fn is_unique_violation(err: &lib_core::model::Error) -> bool {
	matches!(err, lib_core::model::Error::UniqueViolation { .. })
		|| matches!(
			err.as_database_error().and_then(|db| db.code()),
			Some(Cow::Borrowed("23505"))
		) || {
		let text = format!("{err:?}").to_ascii_lowercase();
		text.contains("duplicate") || text.contains("unique")
	}
}

/// POST /api/cases/{case_id}/safety-report
pub async fn create_safety_report_identification(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SafetyReportIdentificationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SafetyReportIdentification>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	match SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await {
		Ok(entity) => {
			return Ok((StatusCode::OK, Json(DataRestResult { data: entity })));
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {}
		Err(err) => return Err(err.into()),
	}

	match SafetyReportIdentificationBmc::create(&ctx, &mm, data).await {
		Ok(_) => {
			let entity =
				SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id)
					.await?;
			Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
		}
		Err(err) if is_unique_violation(&err) => {
			match SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id)
				.await
			{
				Ok(entity) => {
					Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
				}
				Err(_) => Err(err.into()),
			}
		}
		Err(err) => Err(err.into()),
	}
}

/// GET /api/cases/{case_id}/safety-report
pub async fn get_safety_report_identification(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SafetyReportIdentification>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_READ)?;
	let entity =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/safety-report
pub async fn update_safety_report_identification(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<SafetyReportUpdateRequest>,
) -> Result<(StatusCode, Json<DataRestResult<SafetyReportIdentification>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let SafetyReportUpdateRequest {
		data,
		reason_for_change,
		e_signature,
	} = params;
	let ctx_for_update =
		if requires_nullification_compliance(&ctx, &mm, case_id, &data).await? {
			let reason = reason_for_change
			.and_then(|value| {
				let trimmed = value.trim().to_string();
				if trimmed.is_empty() {
					None
				} else {
					Some(trimmed)
				}
			})
			.ok_or(Error::BadRequest {
				message:
					"reason_for_change is required when nullification_code marks case nullified"
						.to_string(),
			})?;
			let e_signature = e_signature.ok_or(Error::BadRequest {
			message:
				"e_signature is required when nullification_code marks case nullified"
					.to_string(),
		})?;
			let signature_id = capture_e_signature(
				&ctx,
				&mm,
				Some(case_id),
				"CASE_NULLIFICATION",
				&crate::web::rest::compliance::ComplianceActionInput {
					reason_for_change: reason.clone(),
					e_signature,
				},
			)
			.await?;
			ctx.with_compliance(Some(reason), Some(signature_id))
		} else {
			ctx.clone()
		};

	SafetyReportIdentificationBmc::update_by_case(
		&ctx_for_update,
		&mm,
		case_id,
		data,
	)
	.await?;
	let entity =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/safety-report
pub async fn delete_safety_report_identification(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, SAFETY_REPORT_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	SafetyReportIdentificationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SafetyReportUpdateRequest {
	pub data: SafetyReportIdentificationForUpdate,
	pub reason_for_change: Option<String>,
	pub e_signature: Option<ESignatureInput>,
}

async fn requires_nullification_compliance(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	data: &SafetyReportIdentificationForUpdate,
) -> Result<bool> {
	let incoming_code = data
		.nullification_code
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty());
	if incoming_code.is_none() {
		return Ok(false);
	}
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let status = case.status.trim().to_ascii_lowercase();
	Ok(status != "nullified")
}
