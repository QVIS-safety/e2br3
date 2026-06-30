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
	PatchValue, SafetyReportIdentification, SafetyReportIdentificationBmc,
	SafetyReportIdentificationForCreate, SafetyReportIdentificationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	is_unique_violation, require_case_write_allowed, require_permission, Error,
	Result,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use uuid::Uuid;

fn create_payload_to_update(
	data: SafetyReportIdentificationForCreate,
) -> SafetyReportIdentificationForUpdate {
	SafetyReportIdentificationForUpdate {
		safety_report_id: data.safety_report_id,
		version: data.version,
		transmission_date: data.transmission_date,
		transmission_date_null_flavor: data.transmission_date_null_flavor,
		report_type: data
			.report_type
			.map(PatchValue::Value)
			.unwrap_or(PatchValue::Missing),
		date_first_received_from_source: data.date_first_received_from_source,
		date_first_received_from_source_null_flavor: data
			.date_first_received_from_source_null_flavor,
		date_of_most_recent_information: data.date_of_most_recent_information,
		date_of_most_recent_information_null_flavor: data
			.date_of_most_recent_information_null_flavor,
		fulfil_expedited_criteria: data
			.fulfil_expedited_criteria
			.map(PatchValue::Value)
			.unwrap_or(PatchValue::Missing),
		local_criteria_report_type: data
			.local_criteria_report_type
			.map(PatchValue::Value)
			.unwrap_or(PatchValue::Missing),
		combination_product_report_indicator: data
			.combination_product_report_indicator
			.map(PatchValue::Value)
			.unwrap_or(PatchValue::Missing),
		worldwide_unique_id: data.worldwide_unique_id,
		first_sender_type: data.first_sender_type,
		additional_documents_available: data.additional_documents_available,
		other_case_identifiers_exist: data.other_case_identifiers_exist,
		nullification_code: data.nullification_code,
		nullification_reason: data.nullification_reason,
		receiver_organization: data.receiver_organization,
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
		Ok(_) => {
			SafetyReportIdentificationBmc::update_by_case(
				&ctx,
				&mm,
				case_id,
				create_payload_to_update(data),
			)
			.await?;
			let entity =
				SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, case_id)
					.await?;
			return Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })));
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
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
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
	let marks_nullified = data
		.nullification_code
		.as_deref()
		.map(str::trim)
		.map(|value| value == "1")
		.unwrap_or(false);
	if !marks_nullified {
		return Ok(false);
	}
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let status = case.status.trim().to_ascii_lowercase();
	Ok(status != "nullified")
}
