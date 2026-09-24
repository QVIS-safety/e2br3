// Terminology REST endpoints for MedDRA, WHODrug, ISO Countries, E2B Code Lists

use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::terminology::{
	E2bCodeList, E2bCodeListBmc, FdaHierarchicalCodeList,
	FdaHierarchicalCodeListBmc, IsoCountry, IsoCountryBmc, MeddraTerm,
	MeddraTermBmc, MfdsProduct, MfdsProductBmc, MfdsProductSubstance, UcumUnit,
	UcumUnitBmc, WhodrugProduct, WhodrugProductBmc,
};
use lib_core::model::terminology_import::{self, TerminologyReleaseRow};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	with_authorized_terminology_mutation, with_authorized_terminology_read, Error,
	Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

// -- Params

const MAX_TERMINOLOGY_UPLOAD_BYTES: usize = 250 * 1024 * 1024;
pub const MAX_WHODRUG_UPLOAD_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_WHODRUG_REQUEST_BYTES: usize = MAX_WHODRUG_UPLOAD_BYTES + 1024 * 1024;

#[derive(Deserialize)]
pub struct TerminologySearchParams {
	pub q: String,
	#[serde(default = "default_limit")]
	pub limit: i64,
	pub version: Option<String>,
	pub language: Option<String>,
	pub level: Option<String>,
}

fn default_limit() -> i64 {
	20
}

#[derive(Deserialize)]
pub struct CodeListParams {
	pub list_name: String,
}

#[derive(Deserialize)]
pub struct FdaHierarchicalCodeSearchParams {
	pub list_name: String,
	pub q: String,
	#[serde(default = "default_limit")]
	pub limit: i64,
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

fn map_import_err(err: terminology_import::ImportError) -> Error {
	match err {
		terminology_import::ImportError::BadInput(msg) => {
			Error::BadRequest { message: msg }
		}
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
			return read_field_limited(field, MAX_TERMINOLOGY_UPLOAD_BYTES).await;
		}
	}

	Err(Error::BadRequest {
		message: "missing terminology file field".to_string(),
	})
}

async fn read_field_limited(
	mut field: Field<'_>,
	max_bytes: usize,
) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	while let Some(chunk) = field.chunk().await.map_err(|err| Error::BadRequest {
		message: format!("multipart read error: {err}"),
	})? {
		if bytes.len().saturating_add(chunk.len()) > max_bytes {
			return Err(Error::BadRequest {
				message: format!("terminology upload exceeds {max_bytes} bytes"),
			});
		}
		bytes.extend_from_slice(&chunk);
	}
	Ok(bytes)
}

struct TemporaryUpload {
	path: PathBuf,
	checksum: String,
	cleaned: bool,
}

impl TemporaryUpload {
	fn cleanup(&mut self) -> Result<()> {
		std::fs::remove_file(&self.path).map_err(|err| Error::BadRequest {
			message: format!("terminology temporary file cleanup error: {err}"),
		})?;
		self.cleaned = true;
		Ok(())
	}
}

impl Drop for TemporaryUpload {
	fn drop(&mut self) {
		if !self.cleaned {
			if let Err(err) = std::fs::remove_file(&self.path) {
				tracing::warn!(path = %self.path.display(), %err, "failed to clean up WHODrug upload");
			}
		}
	}
}

async fn spool_whodrug_upload(mut multipart: Multipart) -> Result<TemporaryUpload> {
	while let Some(mut field) =
		multipart
			.next_field()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("multipart error: {err}"),
			})? {
		if field.name() != Some("file") {
			continue;
		}
		let path = std::env::temp_dir()
			.join(format!("e2br3-whodrug-{}.zip", Uuid::new_v4()));
		let std_file = OpenOptions::new()
			.write(true)
			.create_new(true)
			.mode(0o600)
			.open(&path)
			.map_err(|err| Error::BadRequest {
				message: format!("terminology temporary file error: {err}"),
			})?;
		let mut upload = TemporaryUpload {
			path,
			checksum: String::new(),
			cleaned: false,
		};
		let mut file = tokio::fs::File::from_std(std_file);
		let mut size = 0usize;
		let mut digest = Sha256::new();
		while let Some(chunk) =
			field.chunk().await.map_err(|err| Error::BadRequest {
				message: format!("multipart read error: {err}"),
			})? {
			size =
				size.checked_add(chunk.len())
					.ok_or_else(|| Error::BadRequest {
						message: "terminology upload size overflow".to_string(),
					})?;
			if size > MAX_WHODRUG_UPLOAD_BYTES {
				return Err(Error::BadRequest { message: format!("terminology upload exceeds {MAX_WHODRUG_UPLOAD_BYTES} bytes") });
			}
			digest.update(&chunk);
			file.write_all(&chunk)
				.await
				.map_err(|err| Error::BadRequest {
					message: format!(
						"terminology temporary file write error: {err}"
					),
				})?;
		}
		file.flush().await.map_err(|err| Error::BadRequest {
			message: format!("terminology temporary file flush error: {err}"),
		})?;
		drop(file);
		upload.checksum = format!("{:x}", digest.finalize());
		return Ok(upload);
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
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologySearchParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<MeddraTerm>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let terms = MeddraTermBmc::search(
				ctx,
				mm,
				&params.q,
				params.version.as_deref(),
				params.language.as_deref(),
				params.level.as_deref(),
				params.limit,
			)
			.await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: terms })))
		})
	})
	.await
}

/// GET /api/terminology/whodrug?q={term}&limit={count}
pub async fn search_whodrug(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologySearchParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<WhodrugProduct>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let products =
				WhodrugProductBmc::search(ctx, mm, &params.q, params.limit).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: products })))
		})
	})
	.await
}

/// GET /api/terminology/mfds-products?q={term}&limit={count}
pub async fn search_mfds_products(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologySearchParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<MfdsProduct>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let products =
				MfdsProductBmc::search(ctx, mm, &params.q, params.limit).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: products })))
		})
	})
	.await
}

/// GET /api/terminology/mfds-products/{item_seq}/substances
pub async fn list_mfds_product_substances(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(item_seq): Path<String>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<MfdsProductSubstance>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let rows = MfdsProductBmc::substances(ctx, mm, &item_seq).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: rows })))
		})
	})
	.await
}

/// GET /api/terminology/countries
pub async fn list_countries(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<IsoCountry>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, |ctx, mm| {
		Box::pin(async move {
			let countries = IsoCountryBmc::list_all(ctx, mm).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: countries })))
		})
	})
	.await
}

/// GET /api/terminology/code-lists?list_name={name}
pub async fn get_code_list(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<CodeListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<E2bCodeList>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			let codes =
				E2bCodeListBmc::get_by_list_name(ctx, mm, &params.list_name).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: codes })))
		})
	})
	.await
}

/// GET /api/terminology/fda-code-search?list_name={name}&q={term}&limit={count}
pub async fn search_fda_hierarchical_code(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<FdaHierarchicalCodeSearchParams>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<FdaHierarchicalCodeList>>>,
)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |ctx, mm| {
		Box::pin(async move {
			if params.q.trim().chars().count() < 2 {
				return Ok((StatusCode::OK, Json(DataRestResult { data: vec![] })));
			}
			let rows = FdaHierarchicalCodeListBmc::search(
				ctx,
				mm,
				&params.list_name,
				params.q.trim(),
				params.limit,
			)
			.await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: rows })))
		})
	})
	.await
}

/// GET /api/terminology/ucum-units
pub async fn list_ucum_units(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<UcumUnit>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, |ctx, mm| {
		Box::pin(async move {
			let units = UcumUnitBmc::list_all(ctx, mm).await?;
			Ok((StatusCode::OK, Json(DataRestResult { data: units })))
		})
	})
	.await
}

/// POST /api/terminology/import/meddra
pub async fn import_meddra(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_mutation(
		&ctx,
		&snapshot,
		&mm,
		"import:meddra",
		move |ctx, mm| {
			Box::pin(async move {
				import_meddra_authorized(ctx, mm, params, multipart).await
			})
		},
	)
	.await
}

async fn import_meddra_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	params: TerminologyImportParams,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let bytes = read_upload_bytes(multipart).await?;
	let rows =
		terminology_import::parse_meddra_upload(&bytes).map_err(map_import_err)?;

	if !params.dry_run {
		let checksum = terminology_import::sha256_hex(&bytes);
		terminology_import::stage_meddra_rows(
			mm,
			ctx.user_id(),
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
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_mutation(
		&ctx,
		&snapshot,
		&mm,
		"import:whodrug",
		move |ctx, mm| {
			Box::pin(async move {
				import_whodrug_authorized(ctx, mm, params, multipart).await
			})
		},
	)
	.await
}

async fn import_whodrug_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	params: TerminologyImportParams,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let mut upload = spool_whodrug_upload(multipart).await?;
	let is_c3 = terminology_import::whodrug_zip_is_official_c3(&upload.path)
		.map_err(map_import_err)?;
	let loaded_rows = if is_c3 {
		terminology_import::stage_whodrug_c3_zip(
			mm,
			ctx.user_id(),
			&upload.path,
			&params.version,
			&language,
			&upload.checksum,
			params.dry_run,
		)
		.await
		.map_err(map_import_err)?
	} else {
		let metadata = tokio::fs::metadata(&upload.path).await.map_err(|err| {
			Error::BadRequest {
				message: format!("terminology temporary file error: {err}"),
			}
		})?;
		if metadata.len() > MAX_TERMINOLOGY_UPLOAD_BYTES as u64 {
			return Err(Error::BadRequest { message: format!("non-C3 terminology upload exceeds {MAX_TERMINOLOGY_UPLOAD_BYTES} bytes") });
		}
		let bytes = tokio::fs::read(&upload.path).await.map_err(|err| {
			Error::BadRequest {
				message: format!("terminology temporary file read error: {err}"),
			}
		})?;
		let rows = terminology_import::parse_whodrug_upload(&bytes)
			.map_err(map_import_err)?;
		let cas_numbers = terminology_import::parse_whodrug_cas_numbers(&bytes)
			.map_err(map_import_err)?;
		if !params.dry_run {
			terminology_import::stage_whodrug_rows(
				mm,
				ctx.user_id(),
				&rows,
				&cas_numbers,
				&params.version,
				&language,
				&upload.checksum,
			)
			.await
			.map_err(map_import_err)?;
		}
		rows.len() as i64
	};

	upload.cleanup()?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: TerminologyImportResult {
				dictionary: "whodrug".to_string(),
				version: params.version,
				language,
				loaded_rows,
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
	snapshot: AuthorizationSnapshotW,
	Query(params): Query<TerminologyReleaseListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<TerminologyReleaseRow>>>)> {
	let ctx = ctx_w.0;
	with_authorized_terminology_read(&ctx, &snapshot, &mm, move |_ctx, mm| {
		Box::pin(async move {
			let releases = terminology_import::fetch_releases(
				mm,
				params.dictionary.as_deref(),
				params.language.as_deref(),
			)
			.await
			.map_err(map_import_err)?;
			Ok((StatusCode::OK, Json(DataRestResult { data: releases })))
		})
	})
	.await
}

/// POST /api/terminology/releases/{dictionary}/{version}/approve
pub async fn approve_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyApproveParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	let fingerprint = format!("approve:{}:{}", path.dictionary, path.version);
	with_authorized_terminology_mutation(
		&ctx,
		&snapshot,
		&mm,
		fingerprint,
		move |ctx, mm| {
			Box::pin(async move {
				let language = params.language.unwrap_or_else(|| "en".to_string());
				terminology_import::validate_dictionary(&path.dictionary)
					.map_err(map_import_err)?;
				let data = terminology_import::approve_release(
					mm,
					ctx.user_id(),
					&path.dictionary,
					&path.version,
					&language,
					params.note.as_deref(),
				)
				.await
				.map_err(|e| Error::BadRequest {
					message: e.to_string(),
				})?
				.ok_or_else(|| Error::BadRequest {
					message: "release not found or not in approvable status"
						.to_string(),
				})?;
				Ok((StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}

/// POST /api/terminology/releases/{dictionary}/{version}/activate
pub async fn activate_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	let fingerprint = format!("activate:{}:{}", path.dictionary, path.version);
	with_authorized_terminology_mutation(
		&ctx,
		&snapshot,
		&mm,
		fingerprint,
		move |ctx, mm| {
			Box::pin(async move {
				let language = params.language.unwrap_or_else(|| "en".to_string());
				let data = terminology_import::activate_release_tx(
					mm,
					ctx.user_id(),
					&path.dictionary,
					&path.version,
					&language,
					false,
				)
				.await
				.map_err(map_import_err)?;
				Ok((StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}

/// POST /api/terminology/releases/{dictionary}/{version}/rollback
pub async fn rollback_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	let fingerprint = format!("rollback:{}:{}", path.dictionary, path.version);
	with_authorized_terminology_mutation(
		&ctx,
		&snapshot,
		&mm,
		fingerprint,
		move |ctx, mm| {
			Box::pin(async move {
				let language = params.language.unwrap_or_else(|| "en".to_string());
				let data = terminology_import::activate_release_tx(
					mm,
					ctx.user_id(),
					&path.dictionary,
					&path.version,
					&language,
					true,
				)
				.await
				.map_err(map_import_err)?;
				Ok((StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}
