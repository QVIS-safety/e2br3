use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::CASE_READ;
use lib_core::model::case_validation_report_cache::CaseValidationReportCacheBmc;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::ModelManager;
use lib_core::validation::{
	infer_regulatory_authority_from_receivers, validate_case_for_authority,
	CaseValidationReport, RegulatoryAuthority,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ValidationQuery {
	pub authority: Option<String>,
}

async fn resolve_authority(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: Option<&str>,
) -> Result<RegulatoryAuthority> {
	if let Some(value) = authority {
		return RegulatoryAuthority::parse(value).ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid validation authority '{value}' (expected: ich, fda or mfds)"
			),
		});
	}

	let header = match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await {
		Ok(header) => Some(header),
		Err(lib_core::model::Error::EntityUuidNotFound { entity, id })
			if entity == "message_headers" && id == case_id =>
		{
			None
		}
		Err(err) => return Err(err.into()),
	};

	let authority = infer_regulatory_authority_from_receivers(
		header
			.as_ref()
			.and_then(|h| h.batch_receiver_identifier.as_deref()),
		header
			.as_ref()
			.map(|h| h.message_receiver_identifier.as_str()),
	);

	Ok(authority)
}

/// GET /api/cases/{case_id}/validation
/// Returns case validation issues split as blocking/non-blocking for the wizard.
pub async fn validate_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<ValidationQuery>,
) -> Result<(StatusCode, Json<DataRestResult<CaseValidationReport>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let authority =
		resolve_authority(&ctx, &mm, case_id, query.authority.as_deref()).await?;

	if let Some(report) = CaseValidationReportCacheBmc::get_fresh(
		&ctx,
		&mm,
		case_id,
		authority.as_str(),
	)
	.await?
	{
		return Ok((StatusCode::OK, Json(DataRestResult { data: report })));
	}

	let report = validate_case_for_authority(&ctx, &mm, case_id, authority).await?;
	CaseValidationSummaryBmc::upsert_for_reports(
		&ctx,
		&mm,
		case_id,
		&[report.clone()],
	)
	.await?;
	CaseValidationReportCacheBmc::upsert(&ctx, &mm, case_id, &report).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: report })))
}
