use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::CASE_READ;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::ModelManager;
use lib_core::validation::{
	infer_regulatory_authority_from_receivers, validate_case_for_profile,
	validate_case_for_profiles, CaseValidationReport, ValidationProfile,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ValidationQuery {
	pub profile: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationAllQuery {
	pub profiles: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaseValidationBundle {
	pub case_id: Uuid,
	pub profiles: Vec<String>,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub reports: Vec<CaseValidationReport>,
}

#[derive(Debug)]
pub(crate) struct CaseValidationSummary {
	pub case_id: Uuid,
	pub profiles: Vec<String>,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub reports: Vec<CaseValidationReport>,
}

impl CaseValidationSummary {
	pub(crate) fn total_count(&self) -> usize {
		self.blocking_count + self.non_blocking_count
	}
}

fn parse_profiles_query(value: &str) -> Result<Vec<ValidationProfile>> {
	let mut profiles = Vec::new();
	for raw in value
		.split(',')
		.map(str::trim)
		.filter(|raw| !raw.is_empty())
	{
		let profile =
			ValidationProfile::parse(raw).ok_or_else(|| Error::BadRequest {
				message: format!(
					"invalid validation profile '{raw}' (expected: ich, fda or mfds)"
				),
			})?;
		if !profiles.contains(&profile) {
			profiles.push(profile);
		}
	}
	if profiles.is_empty() {
		Err(Error::BadRequest {
			message: "profiles is required and must include ich, fda or mfds"
				.to_string(),
		})
	} else {
		Ok(profiles)
	}
}

async fn resolve_profile(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profile: Option<&str>,
) -> Result<ValidationProfile> {
	if let Some(value) = profile {
		return ValidationProfile::parse(value).ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid validation profile '{value}' (expected: ich, fda or mfds)"
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

	Ok(authority.to_validation_profile())
}

pub(crate) async fn validation_summary_for_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	profiles: &[ValidationProfile],
) -> Result<CaseValidationSummary> {
	let reports = validate_case_for_profiles(ctx, mm, case_id, profiles).await?;
	let blocking_count: usize = reports.iter().map(|r| r.blocking_count).sum();
	let non_blocking_count: usize =
		reports.iter().map(|r| r.non_blocking_count).sum();
	let ok = reports.iter().all(|r| r.ok);
	let profiles = reports
		.iter()
		.map(|r| r.profile.clone())
		.collect::<Vec<_>>();

	Ok(CaseValidationSummary {
		case_id,
		profiles,
		ok,
		blocking_count,
		non_blocking_count,
		reports,
	})
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

	let profile =
		resolve_profile(&ctx, &mm, case_id, query.profile.as_deref()).await?;

	let report = validate_case_for_profile(&ctx, &mm, case_id, profile).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: report })))
}

/// GET /api/cases/{case_id}/validation/all
/// Returns validation reports for the profiles explicitly requested by the caller.
pub async fn validate_case_all(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<ValidationAllQuery>,
) -> Result<(StatusCode, Json<DataRestResult<CaseValidationBundle>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let profiles = query
		.profiles
		.as_deref()
		.ok_or_else(|| Error::BadRequest {
			message: "profiles is required for validation/all".to_string(),
		})
		.and_then(parse_profiles_query)?;
	let summary = validation_summary_for_case(&ctx, &mm, case_id, &profiles).await?;
	CaseValidationSummaryBmc::upsert_for_reports(
		&ctx,
		&mm,
		case_id,
		&summary.reports,
	)
	.await?;

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseValidationBundle {
				case_id: summary.case_id,
				profiles: summary.profiles,
				ok: summary.ok,
				blocking_count: summary.blocking_count,
				non_blocking_count: summary.non_blocking_count,
				reports: summary.reports,
			},
		}),
	))
}
