use crate::runtime_settings;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use uuid::Uuid;
use validator::{
	infer_regulatory_authority_from_receivers, validate_case_for_authorities,
	CaseValidationReport, RegulatoryAuthority,
};

#[derive(Debug, Deserialize)]
pub struct ValidationQuery {
	pub authority: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ValidationAuthoritiesQuery {
	pub authorities: Option<String>,
}

impl ValidationAuthoritiesQuery {
	pub fn resolve(&self) -> Result<Vec<RegulatoryAuthority>> {
		let mut authorities = Vec::new();
		for value in self.authorities.as_deref().unwrap_or("ich").split(',') {
			let authority = super::case_rest::parse_authority_or_bad_request(value)?;
			if !authorities.contains(&authority) {
				authorities.push(authority);
			}
		}
		Ok(authorities)
	}
}

pub(crate) async fn resolve_authority(
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
	let settings = runtime_settings::load(ctx, mm).await?;
	Ok(settings
		.appendices
		.iter()
		.copied()
		.find(|configured| *configured == authority)
		.unwrap_or(settings.appendices[0]))
}

pub async fn refresh_case_validation_cache(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authorities: &[RegulatoryAuthority],
) -> Result<Vec<CaseValidationReport>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	let result: Result<Vec<CaseValidationReport>> = async {
		set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;
		// Mutations use this same Case lock. Hold it across data loading and cache writes.
		// ponytail: serialize validation per Case; use revisions if long validations cause contention.
		mm.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM cases WHERE id = $1 FOR UPDATE",
				)
				.bind(case_id),
			)
			.await
			.map_err(lib_core::model::Error::from)?;
		let reports =
			validate_case_for_authorities(ctx, mm, case_id, authorities).await?;
		CaseValidationSummaryBmc::upsert_for_reports(ctx, mm, case_id, &reports)
			.await?;
		Ok(reports)
	}
	.await;
	match result {
		Ok(reports) => {
			mm.dbx()
				.commit_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Ok(reports)
		}
		Err(error) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(error)
		}
	}
}

/// GET /api/cases/{case_id}/validation
/// Returns all case validation issues and counts for the wizard.
pub async fn validate_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<ValidationQuery>,
) -> Result<(StatusCode, Json<DataRestResult<CaseValidationReport>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"case-validation",
		move |ctx, mm| {
			Box::pin(async move {
				let authority =
					resolve_authority(ctx, mm, case_id, query.authority.as_deref())
						.await?;

				let report =
					refresh_case_validation_cache(ctx, mm, case_id, &[authority])
						.await?
						.remove(0);
				Ok((StatusCode::OK, Json(DataRestResult { data: report })))
			})
		},
	)
	.await
}
