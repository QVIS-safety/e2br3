// Terminology REST endpoints for MedDRA, WHODrug, ISO Countries, E2B Code Lists

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::ctx::{Ctx, ROLE_SYSTEM_ADMIN};
use lib_core::model::acs::{
	TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT, TERMINOLOGY_READ,
};
use lib_core::model::terminology::{
	E2bCodeList, E2bCodeListBmc, IsoCountry, IsoCountryBmc, MeddraTerm,
	MeddraTermBmc, UcumUnit, UcumUnitBmc, WhodrugProduct, WhodrugProductBmc,
};
use lib_core::model::terminology_import::{
	self, TerminologyReleaseRow,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};

// -- Params

#[derive(Deserialize)]
pub struct TerminologySearchParams {
	pub q: String,
	#[serde(default = "default_limit")]
	pub limit: i64,
	pub version: Option<String>,
}

fn default_limit() -> i64 {
	20
}

#[derive(Deserialize)]
pub struct CodeListParams {
	pub list_name: String,
}

#[derive(Deserialize)]
pub struct TerminologyImportParams {
	pub version: String,
	pub language: Option<String>,
	#[serde(default)]
	pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct TerminologyReleaseListParams {
	pub dictionary: Option<String>,
	pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct TerminologyApproveParams {
	pub language: Option<String>,
	pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct TerminologyActivateParams {
	pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct ReleasePath {
	pub dictionary: String,
	pub version: String,
}

// -- Result types

#[derive(Serialize)]
pub struct TerminologyImportResult {
	pub dictionary: String,
	pub version: String,
	pub language: String,
	pub loaded_rows: i64,
	pub dry_run: bool,
	pub status: String,
}

// -- Auth helpers

fn require_system_admin(ctx: &Ctx) -> Result<()> {
	if ctx.role() != ROLE_SYSTEM_ADMIN {
		return Err(Error::PermissionDenied {
			required_permission: "system_admin".to_string(),
		});
	}
	Ok(())
}

fn map_import_err(err: terminology_import::ImportError) -> Error {
	match err {
		terminology_import::ImportError::BadInput(msg) => Error::BadRequest { message: msg },
		terminology_import::ImportError::Store(msg) => {
			Error::Model(lib_core::model::Error::Store(msg))
		}
	}
}

// -- Upload helper

async fn read_upload_bytes(mut multipart: Multipart) -> Result<Vec<u8>> {
	while let Some(field) =
		multipart
			.next_field()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("multipart error: {err}"),
			})? {
		let name = field.name().map(|v| v.to_string());
		if name.as_deref() == Some("file") {
			let bytes = field.bytes().await.map_err(|err| Error::BadRequest {
				message: format!("multipart read error: {err}"),
			})?;
			return Ok(bytes.to_vec());
		}
	}

	Err(Error::BadRequest {
		message: "missing terminology file field".to_string(),
	})
}

// -- Handlers

/// GET /api/terminology/meddra?q={term}&limit={count}&version={version}
pub async fn search_meddra(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologySearchParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<MeddraTerm>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	tracing::debug!(
		"{:<12} - rest search_meddra q={} limit={}",
		"HANDLER",
		params.q,
		params.limit
	);

	let terms = MeddraTermBmc::search(
		&ctx,
		&mm,
		&params.q,
		params.version.as_deref(),
		params.limit,
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: terms })))
}

/// GET /api/terminology/whodrug?q={term}&limit={count}
pub async fn search_whodrug(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologySearchParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<WhodrugProduct>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	tracing::debug!(
		"{:<12} - rest search_whodrug q={} limit={}",
		"HANDLER",
		params.q,
		params.limit
	);

	let products =
		WhodrugProductBmc::search(&ctx, &mm, &params.q, params.limit).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: products })))
}

/// GET /api/terminology/countries
pub async fn list_countries(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<IsoCountry>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	let countries = IsoCountryBmc::list_all(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: countries })))
}

/// GET /api/terminology/code-lists?list_name={name}
pub async fn get_code_list(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<CodeListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<E2bCodeList>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	let codes =
		E2bCodeListBmc::get_by_list_name(&ctx, &mm, &params.list_name).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: codes })))
}

/// GET /api/terminology/ucum-units
pub async fn list_ucum_units(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<UcumUnit>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	let units = UcumUnitBmc::list_all(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: units })))
}

/// POST /api/terminology/import/meddra
pub async fn import_meddra(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_IMPORT)?;
	require_system_admin(&ctx)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let bytes = read_upload_bytes(multipart).await?;
	let rows = terminology_import::parse_meddra_upload(&bytes).map_err(map_import_err)?;

	if !params.dry_run {
		let checksum = terminology_import::sha256_hex(&bytes);
		terminology_import::stage_meddra_rows(
			&mm,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
			&rows,
			&params.version,
			&language,
			&checksum,
		)
		.await
		.map_err(map_import_err)?;
	}

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: TerminologyImportResult {
				dictionary: "meddra".to_string(),
				version: params.version,
				language,
				loaded_rows: rows.len() as i64,
				dry_run: params.dry_run,
				status: if params.dry_run {
					"dry_run_validated".to_string()
				} else {
					"validated".to_string()
				},
			},
		}),
	))
}

/// POST /api/terminology/import/whodrug
pub async fn import_whodrug(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_IMPORT)?;
	require_system_admin(&ctx)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let bytes = read_upload_bytes(multipart).await?;
	let rows = terminology_import::parse_whodrug_upload(&bytes).map_err(map_import_err)?;

	if !params.dry_run {
		let checksum = terminology_import::sha256_hex(&bytes);
		terminology_import::stage_whodrug_rows(
			&mm,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
			&rows,
			&params.version,
			&language,
			&checksum,
		)
		.await
		.map_err(map_import_err)?;
	}

	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: TerminologyImportResult {
				dictionary: "whodrug".to_string(),
				version: params.version,
				language,
				loaded_rows: rows.len() as i64,
				dry_run: params.dry_run,
				status: if params.dry_run {
					"dry_run_validated".to_string()
				} else {
					"validated".to_string()
				},
			},
		}),
	))
}

/// GET /api/terminology/releases
pub async fn list_releases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyReleaseListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<TerminologyReleaseRow>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	require_system_admin(&ctx)?;

	let releases = terminology_import::fetch_releases(
		&mm,
		params.dictionary.as_deref(),
		params.language.as_deref(),
	)
	.await
	.map_err(map_import_err)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: releases })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/approve
pub async fn approve_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyApproveParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	require_system_admin(&ctx)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());
	terminology_import::validate_dictionary(&path.dictionary).map_err(map_import_err)?;

	let data = terminology_import::approve_release(
		&mm,
		&path.dictionary,
		&path.version,
		&language,
		ctx.user_id(),
		params.note.as_deref(),
	)
	.await
	.map_err(|e| Error::BadRequest {
		message: e.to_string(),
	})?
	.ok_or_else(|| Error::BadRequest {
		message: "release not found or not in approvable status".to_string(),
	})?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/activate
pub async fn activate_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	require_system_admin(&ctx)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let data = terminology_import::activate_release_tx(
		&mm,
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
		&path.dictionary,
		&path.version,
		&language,
		false,
	)
	.await
	.map_err(map_import_err)?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/rollback
pub async fn rollback_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	require_system_admin(&ctx)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let data = terminology_import::activate_release_tx(
		&mm,
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
		&path.dictionary,
		&path.version,
		&language,
		true,
	)
	.await
	.map_err(map_import_err)?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}
