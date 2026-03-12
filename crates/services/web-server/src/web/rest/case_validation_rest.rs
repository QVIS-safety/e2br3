use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::CASE_READ;
use lib_core::model::case::CaseBmc;
use lib_core::model::message_header::MessageHeaderBmc;
use lib_core::model::ModelManager;
use lib_core::xml::validate::{
	validate_case_for_profile, CaseValidationReport, ValidationProfile,
};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ValidationQuery {
	pub profile: Option<String>,
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

fn parse_profiles_from_appendices_json(
	value: &str,
) -> Option<Vec<ValidationProfile>> {
	let parsed: Vec<Value> = serde_json::from_str(value).ok()?;
	let mut profiles = Vec::new();
	for item in parsed {
		let Some(raw) = item.as_str() else {
			continue;
		};
		let Some(profile) = ValidationProfile::parse(raw) else {
			continue;
		};
		if !profiles.contains(&profile) {
			profiles.push(profile);
		}
	}
	if profiles.is_empty() {
		None
	} else {
		Some(profiles)
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

	if let Ok(case) = CaseBmc::get(ctx, mm, case_id).await {
		if let Some(value) = case.validation_profile.as_deref() {
			if let Some(parsed) = ValidationProfile::parse(value) {
				return Ok(parsed);
			}
		}
	}

	let header =
		match MessageHeaderBmc::get_by_case(&Ctx::root_ctx(), mm, case_id).await {
			Ok(header) => Some(header),
			Err(lib_core::model::Error::EntityUuidNotFound { entity, id })
				if entity == "message_headers" && id == case_id =>
			{
				None
			}
			Err(err) => return Err(err.into()),
		};

	let inferred = header
		.as_ref()
		.and_then(|h| h.batch_receiver_identifier.as_deref())
		.map(|v| v.trim().to_ascii_uppercase());

	let is_mfds = inferred
		.as_deref()
		.map(|value| value.contains("MFDS"))
		.unwrap_or(false);

	Ok(if is_mfds {
		ValidationProfile::Mfds
	} else {
		ValidationProfile::Fda
	})
}

async fn resolve_profiles(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<ValidationProfile>> {
	if let Ok(case) = CaseBmc::get(ctx, mm, case_id).await {
		if let Some(value) = case.appendices_json.as_deref() {
			if let Some(parsed) = parse_profiles_from_appendices_json(value) {
				return Ok(parsed);
			}
		}
		if let Some(value) = case.validation_profile.as_deref() {
			if let Some(parsed) = ValidationProfile::parse(value) {
				return Ok(vec![parsed]);
			}
		}
	}

	Ok(vec![resolve_profile(ctx, mm, case_id, None).await?])
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

	let profile =
		resolve_profile(&ctx, &mm, case_id, query.profile.as_deref()).await?;

	let report = validate_case_for_profile(&ctx, &mm, case_id, profile).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: report })))
}

/// GET /api/cases/{case_id}/validation/all
/// Returns validation reports for all selected appendices on the case.
pub async fn validate_case_all(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<CaseValidationBundle>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;

	let profiles = resolve_profiles(&ctx, &mm, case_id).await?;
	let mut reports = Vec::with_capacity(profiles.len());
	let mut blocking_count = 0usize;
	let mut non_blocking_count = 0usize;
	let mut ok = true;
	for profile in profiles {
		let report = validate_case_for_profile(&ctx, &mm, case_id, profile).await?;
		blocking_count += report.blocking_count;
		non_blocking_count += report.non_blocking_count;
		ok &= report.ok;
		reports.push(report);
	}
	let profiles = reports
		.iter()
		.map(|report| report.profile.clone())
		.collect::<Vec<_>>();

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CaseValidationBundle {
				case_id,
				profiles,
				ok,
				blocking_count,
				non_blocking_count,
				reports,
			},
		}),
	))
}
