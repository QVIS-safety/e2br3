// Terminology REST endpoints for MedDRA, WHODrug, ISO Countries, E2B Code Lists

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use csv::ReaderBuilder;
use lib_core::model::acs::{
	TERMINOLOGY_APPROVE, TERMINOLOGY_IMPORT, TERMINOLOGY_READ,
};
use lib_core::model::terminology::{
	E2bCodeList, E2bCodeListBmc, IsoCountry, IsoCountryBmc, MeddraTerm,
	MeddraTermBmc, WhodrugProduct, WhodrugProductBmc,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{types::Uuid, FromRow, Postgres, QueryBuilder};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

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
pub struct TerminologyLanguageParams {
	pub language: Option<String>,
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

#[derive(Serialize)]
pub struct TerminologyImportResult {
	pub dictionary: String,
	pub version: String,
	pub language: String,
	pub loaded_rows: i64,
	pub dry_run: bool,
	pub status: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TerminologyReleaseRow {
	pub id: i64,
	pub dictionary: String,
	pub version: String,
	pub language: String,
	pub status: String,
	pub source_path: Option<String>,
	pub source_checksum: Option<String>,
	pub loaded_rows: i64,
	pub approved_by: Option<Uuid>,
	pub approved_at: Option<sqlx::types::time::OffsetDateTime>,
	pub activated_by: Option<Uuid>,
	pub activated_at: Option<sqlx::types::time::OffsetDateTime>,
	pub rollback_from_version: Option<String>,
	pub note: Option<String>,
	pub created_at: sqlx::types::time::OffsetDateTime,
	pub updated_at: sqlx::types::time::OffsetDateTime,
}

#[derive(Debug, Clone)]
struct MeddraRow {
	code: String,
	term: String,
	level: String,
}

#[derive(Debug, Clone)]
struct WhodrugRow {
	code: String,
	drug_name: String,
	atc_code: Option<String>,
}

/// GET /api/terminology/meddra?q={term}&limit={count}&version={version}
/// Search MedDRA terms by name
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
/// Search WHODrug products by name
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
/// List all active ISO countries
pub async fn list_countries(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<IsoCountry>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	tracing::debug!("{:<12} - rest list_countries", "HANDLER");

	let countries = IsoCountryBmc::list_all(&ctx, &mm).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: countries })))
}

/// GET /api/terminology/code-lists?list_name={name}
/// Get E2B code list values by list name
pub async fn get_code_list(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<CodeListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<E2bCodeList>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;
	tracing::debug!(
		"{:<12} - rest get_code_list list_name={}",
		"HANDLER",
		params.list_name
	);

	let codes =
		E2bCodeListBmc::get_by_list_name(&ctx, &mm, &params.list_name).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: codes })))
}

/// POST /api/terminology/import/meddra?version=27.1&language=en&dry_run=false
pub async fn import_meddra(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_IMPORT)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let bytes = read_upload_bytes(multipart).await?;
	let rows = parse_meddra_upload(&bytes)?;

	if !params.dry_run {
		let checksum = sha256_hex(&bytes);
		stage_meddra_rows(
			&mm,
			ctx.user_id(),
			&rows,
			&params.version,
			&language,
			&checksum,
		)
		.await?;
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

/// POST /api/terminology/import/whodrug?version=2025.09&language=en&dry_run=false
pub async fn import_whodrug(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyImportParams>,
	multipart: Multipart,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyImportResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_IMPORT)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let bytes = read_upload_bytes(multipart).await?;
	let rows = parse_whodrug_upload(&bytes)?;

	if !params.dry_run {
		let checksum = sha256_hex(&bytes);
		stage_whodrug_rows(
			&mm,
			ctx.user_id(),
			&rows,
			&params.version,
			&language,
			&checksum,
		)
		.await?;
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

/// GET /api/terminology/releases?dictionary=meddra&language=en
pub async fn list_releases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(params): Query<TerminologyReleaseListParams>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<TerminologyReleaseRow>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_READ)?;

	let dictionary = params.dictionary.as_deref();
	let language = params.language.as_deref();
	let releases = fetch_releases(&mm, dictionary, language).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: releases })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/approve?language=en&note=...
pub async fn approve_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyApproveParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());
	validate_dictionary(&path.dictionary)?;

	let updated = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, TerminologyReleaseRow>(
				"UPDATE terminology_releases
				 SET status = 'approved',
				     approved_by = $4,
				     approved_at = NOW(),
				     note = COALESCE($5, note),
				     updated_at = NOW()
				 WHERE dictionary = $1
				   AND version = $2
				   AND language = $3
				   AND status IN ('validated', 'approved')
				 RETURNING *",
			)
			.bind(&path.dictionary)
			.bind(&path.version)
			.bind(&language)
			.bind(ctx.user_id())
			.bind(params.note),
		)
		.await
		.map_err(map_store_err)?;

	let data = updated.ok_or_else(|| Error::BadRequest {
		message: "release not found or not in approvable status".to_string(),
	})?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/activate?language=en
pub async fn activate_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let data = activate_release_tx(
		&mm,
		ctx.user_id(),
		&path.dictionary,
		&path.version,
		&language,
		false,
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}

/// POST /api/terminology/releases/{dictionary}/{version}/rollback?language=en
pub async fn rollback_release(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(path): Path<ReleasePath>,
	Query(params): Query<TerminologyActivateParams>,
) -> Result<(StatusCode, Json<DataRestResult<TerminologyReleaseRow>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TERMINOLOGY_APPROVE)?;
	let language = params.language.unwrap_or_else(|| "en".to_string());

	let data = activate_release_tx(
		&mm,
		ctx.user_id(),
		&path.dictionary,
		&path.version,
		&language,
		true,
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data })))
}

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

fn parse_meddra_upload(bytes: &[u8]) -> Result<Vec<MeddraRow>> {
	let mut zip =
		ZipArchive::new(Cursor::new(bytes)).map_err(|err| Error::BadRequest {
			message: format!("invalid MedDRA zip: {err}"),
		})?;

	let llt = read_zip_file_case_insensitive(&mut zip, "llt.asc")?;
	let mdhier = read_zip_file_case_insensitive(&mut zip, "mdhier.asc")?;

	let mut dedup: BTreeMap<(String, String), String> = BTreeMap::new();

	for line in llt.lines() {
		let cols: Vec<&str> = line.split('$').collect();
		if cols.len() < 2 {
			continue;
		}
		let code = cols[0].trim();
		let term = cols[1].trim();
		if code.is_empty() || term.is_empty() {
			continue;
		}
		dedup
			.entry((code.to_string(), "LLT".to_string()))
			.or_insert_with(|| term.to_string());
	}

	for line in mdhier.lines() {
		let cols: Vec<&str> = line.split('$').collect();
		if cols.len() < 8 {
			continue;
		}
		insert_term(&mut dedup, cols[0], cols[4], "PT");
		insert_term(&mut dedup, cols[1], cols[5], "HLT");
		insert_term(&mut dedup, cols[2], cols[6], "HLGT");
		insert_term(&mut dedup, cols[3], cols[7], "SOC");
	}

	let rows = dedup
		.into_iter()
		.map(|((code, level), term)| MeddraRow { code, term, level })
		.collect::<Vec<_>>();

	if rows.is_empty() {
		return Err(Error::BadRequest {
			message: "No MedDRA rows parsed from llt.asc/mdhier.asc".to_string(),
		});
	}

	Ok(rows)
}

fn parse_whodrug_upload(bytes: &[u8]) -> Result<Vec<WhodrugRow>> {
	if let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) {
		for idx in 0..zip.len() {
			let mut entry = zip.by_index(idx).map_err(|err| Error::BadRequest {
				message: format!("whodrug zip read error: {err}"),
			})?;
			if !entry.is_file() {
				continue;
			}
			let name = entry.name().to_ascii_lowercase();
			if !is_delimited_name(&name) {
				continue;
			}
			let mut entry_bytes = Vec::new();
			entry.read_to_end(&mut entry_bytes).map_err(|err| {
				Error::BadRequest {
					message: format!("whodrug zip file read error: {err}"),
				}
			})?;
			if let Ok(rows) = parse_whodrug_delimited(&entry_bytes) {
				if !rows.is_empty() {
					return Ok(rows);
				}
			}
		}
		return Err(Error::BadRequest {
			message: "No parseable WHODrug delimited file in uploaded zip"
				.to_string(),
		});
	}

	parse_whodrug_delimited(bytes)
}

fn parse_whodrug_delimited(bytes: &[u8]) -> Result<Vec<WhodrugRow>> {
	let delim = detect_delimiter(bytes);
	let mut rdr = ReaderBuilder::new()
		.has_headers(true)
		.delimiter(delim)
		.from_reader(Cursor::new(bytes));

	let headers = rdr
		.headers()
		.map_err(|err| Error::BadRequest {
			message: format!("whodrug header parse error: {err}"),
		})?
		.iter()
		.map(normalize_header)
		.collect::<Vec<_>>();

	let code_idx = find_header_idx(
		&headers,
		&["code", "drug_code", "record_id", "drugid", "drecno", "mpid"],
	)
	.ok_or_else(|| Error::BadRequest {
		message: "Missing WHODrug code column".to_string(),
	})?;

	let name_idx = find_header_idx(
		&headers,
		&[
			"drug_name",
			"name",
			"drugname",
			"medicinal_product_name",
			"medicinal product name",
			"product_name",
		],
	)
	.ok_or_else(|| Error::BadRequest {
		message: "Missing WHODrug product name column".to_string(),
	})?;

	let atc_idx = find_header_idx(&headers, &["atc", "atc_code", "atc1"]);
	let mut rows = Vec::new();
	let mut seen = HashSet::new();

	for rec in rdr.records() {
		let rec = rec.map_err(|err| Error::BadRequest {
			message: format!("whodrug row parse error: {err}"),
		})?;
		let code = rec.get(code_idx).unwrap_or("").trim();
		let drug_name = rec.get(name_idx).unwrap_or("").trim();
		if code.is_empty() || drug_name.is_empty() {
			continue;
		}
		let atc_code = atc_idx
			.and_then(|idx| rec.get(idx))
			.map(|v| v.trim())
			.filter(|v| !v.is_empty())
			.map(|v| v.to_string());

		if seen.insert(code.to_string()) {
			rows.push(WhodrugRow {
				code: code.to_string(),
				drug_name: drug_name.to_string(),
				atc_code,
			});
		}
	}

	if rows.is_empty() {
		return Err(Error::BadRequest {
			message: "No WHODrug rows parsed from upload".to_string(),
		});
	}

	Ok(rows)
}

async fn stage_meddra_rows(
	mm: &ModelManager,
	uploader_id: Uuid,
	rows: &[MeddraRow],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	mm.dbx().begin_txn().await.map_err(map_store_err)?;
	let run_result = async {
		upsert_release_header(
			mm,
			"meddra",
			version,
			language,
			"loading",
			"upload",
			Some(checksum),
			rows.len() as i64,
			Some(uploader_id),
			None,
			None,
		)
		.await?;

		upsert_meddra_rows(mm, rows, version, language, false).await?;

		upsert_release_header(
			mm,
			"meddra",
			version,
			language,
			"validated",
			"upload",
			Some(checksum),
			rows.len() as i64,
			Some(uploader_id),
			None,
			None,
		)
		.await?;

		Ok::<(), Error>(())
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await.map_err(map_store_err)?;
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn stage_whodrug_rows(
	mm: &ModelManager,
	uploader_id: Uuid,
	rows: &[WhodrugRow],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	mm.dbx().begin_txn().await.map_err(map_store_err)?;
	let run_result = async {
		upsert_release_header(
			mm,
			"whodrug",
			version,
			language,
			"loading",
			"upload",
			Some(checksum),
			rows.len() as i64,
			Some(uploader_id),
			None,
			None,
		)
		.await?;

		upsert_whodrug_rows(mm, rows, version, language, false).await?;

		upsert_release_header(
			mm,
			"whodrug",
			version,
			language,
			"validated",
			"upload",
			Some(checksum),
			rows.len() as i64,
			Some(uploader_id),
			None,
			None,
		)
		.await?;

		Ok::<(), Error>(())
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await.map_err(map_store_err)?;
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn activate_release_tx(
	mm: &ModelManager,
	actor_user_id: Uuid,
	dictionary: &str,
	target_version: &str,
	language: &str,
	is_rollback: bool,
) -> Result<TerminologyReleaseRow> {
	validate_dictionary(dictionary)?;

	mm.dbx().begin_txn().await.map_err(map_store_err)?;
	let run_result = async {
		let target = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, TerminologyReleaseRow>(
					"SELECT * FROM terminology_releases
					 WHERE dictionary = $1 AND version = $2 AND language = $3",
				)
				.bind(dictionary)
				.bind(target_version)
				.bind(language),
			)
			.await
			.map_err(map_store_err)?
			.ok_or_else(|| Error::BadRequest {
				message: "target release not found".to_string(),
			})?;

		if !matches!(
			target.status.as_str(),
			"approved" | "validated" | "active" | "retired"
		) {
			return Err(Error::BadRequest {
				message: "target release status is not activatable".to_string(),
			});
		}

		let current_active_version = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (String,)>(
					"SELECT version FROM terminology_releases
					 WHERE dictionary = $1 AND language = $2 AND status = 'active'
					 ORDER BY activated_at DESC NULLS LAST, updated_at DESC
					 LIMIT 1",
				)
				.bind(dictionary)
				.bind(language),
			)
			.await
			.map_err(map_store_err)?
			.map(|v| v.0);

		match dictionary {
			"meddra" => {
				mm.dbx()
					.execute(
						sqlx::query(
							"UPDATE meddra_terms SET active = false WHERE language = $1 AND active = true",
						)
						.bind(language),
					)
					.await
					.map_err(map_store_err)?;
				let changed = mm
					.dbx()
					.execute(
						sqlx::query(
							"UPDATE meddra_terms
							 SET active = true
							 WHERE version = $1 AND language = $2",
						)
						.bind(target_version)
						.bind(language),
					)
					.await
					.map_err(map_store_err)?;
				if changed == 0 {
					return Err(Error::BadRequest {
						message: "target MedDRA rows were not staged".to_string(),
					});
				}
			}
			"whodrug" => {
				mm.dbx()
					.execute(
						sqlx::query(
							"UPDATE whodrug_products SET active = false WHERE language = $1 AND active = true",
						)
						.bind(language),
					)
					.await
					.map_err(map_store_err)?;
				let changed = mm
					.dbx()
					.execute(
						sqlx::query(
							"UPDATE whodrug_products
							 SET active = true
							 WHERE version = $1 AND language = $2",
						)
						.bind(target_version)
						.bind(language),
					)
					.await
					.map_err(map_store_err)?;
				if changed == 0 {
					return Err(Error::BadRequest {
						message: "target WHODrug rows were not staged".to_string(),
					});
				}
			}
			_ => {
				return Err(Error::BadRequest {
					message: "invalid dictionary".to_string(),
				});
			}
		}

		if let Some(prev_version) = current_active_version.as_deref() {
			if prev_version != target_version {
				mm.dbx()
					.execute(
						sqlx::query(
							"UPDATE terminology_releases
							 SET status = 'retired', updated_at = NOW()
							 WHERE dictionary = $1 AND version = $2 AND language = $3",
						)
						.bind(dictionary)
						.bind(prev_version)
						.bind(language),
					)
					.await
					.map_err(map_store_err)?;
			}
		}

		let rollback_from_version = if is_rollback {
			current_active_version.as_deref()
		} else {
			None
		};

		let updated = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, TerminologyReleaseRow>(
					"UPDATE terminology_releases
					 SET status = 'active',
					     activated_at = NOW(),
					     activated_by = $4,
					     rollback_from_version = $5,
					     updated_at = NOW()
					 WHERE dictionary = $1 AND version = $2 AND language = $3
					 RETURNING *",
				)
				.bind(dictionary)
				.bind(target_version)
				.bind(language)
				.bind(actor_user_id)
				.bind(rollback_from_version),
			)
			.await
			.map_err(map_store_err)?;

		Ok::<TerminologyReleaseRow, Error>(updated)
	}
	.await;

	match run_result {
		Ok(data) => {
			mm.dbx().commit_txn().await.map_err(map_store_err)?;
			Ok(data)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn fetch_releases(
	mm: &ModelManager,
	dictionary: Option<&str>,
	language: Option<&str>,
) -> Result<Vec<TerminologyReleaseRow>> {
	let mut qb: QueryBuilder<Postgres> =
		QueryBuilder::new("SELECT * FROM terminology_releases");
	let mut has_where = false;
	if let Some(dict) = dictionary {
		validate_dictionary(dict)?;
		qb.push(if !has_where { " WHERE " } else { " AND " });
		qb.push("dictionary = ").push_bind(dict);
		has_where = true;
	}
	if let Some(lang) = language {
		qb.push(if !has_where { " WHERE " } else { " AND " });
		qb.push("language = ").push_bind(lang);
	}
	qb.push(" ORDER BY updated_at DESC, id DESC");

	mm.dbx()
		.fetch_all(qb.build_query_as::<TerminologyReleaseRow>())
		.await
		.map_err(map_store_err)
}

async fn upsert_release_header(
	mm: &ModelManager,
	dictionary: &str,
	version: &str,
	language: &str,
	status: &str,
	source_path: &str,
	checksum: Option<&str>,
	loaded_rows: i64,
	activated_by: Option<Uuid>,
	rollback_from_version: Option<&str>,
	note: Option<&str>,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO terminology_releases
				 (dictionary, version, language, status, source_path, source_checksum, loaded_rows,
				  activated_by, rollback_from_version, note, created_at, updated_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
				 ON CONFLICT (dictionary, version, language)
				 DO UPDATE SET
				   status = EXCLUDED.status,
				   source_path = EXCLUDED.source_path,
				   source_checksum = EXCLUDED.source_checksum,
				   loaded_rows = EXCLUDED.loaded_rows,
				   activated_by = COALESCE(EXCLUDED.activated_by, terminology_releases.activated_by),
				   rollback_from_version = COALESCE(EXCLUDED.rollback_from_version, terminology_releases.rollback_from_version),
				   note = COALESCE(EXCLUDED.note, terminology_releases.note),
				   updated_at = NOW()",
			)
			.bind(dictionary)
			.bind(version)
			.bind(language)
			.bind(status)
			.bind(source_path)
			.bind(checksum)
			.bind(loaded_rows)
			.bind(activated_by)
			.bind(rollback_from_version)
			.bind(note),
		)
		.await
		.map_err(map_store_err)?;
	Ok(())
}

async fn upsert_meddra_rows(
	mm: &ModelManager,
	rows: &[MeddraRow],
	version: &str,
	language: &str,
	active: bool,
) -> Result<()> {
	const BATCH: usize = 1000;
	for chunk in rows.chunks(BATCH) {
		let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
			"INSERT INTO meddra_terms (code, term, level, version, language, active) ",
		);
		qb.push_values(chunk, |mut b, row| {
			b.push_bind(&row.code)
				.push_bind(&row.term)
				.push_bind(&row.level)
				.push_bind(version)
				.push_bind(language)
				.push_bind(active);
		});
		qb.push(
			" ON CONFLICT (code, version, language)
			  DO UPDATE SET
			    term = EXCLUDED.term,
			    level = EXCLUDED.level,
			    active = EXCLUDED.active",
		);
		mm.dbx().execute(qb.build()).await.map_err(map_store_err)?;
	}
	Ok(())
}

async fn upsert_whodrug_rows(
	mm: &ModelManager,
	rows: &[WhodrugRow],
	version: &str,
	language: &str,
	active: bool,
) -> Result<()> {
	const BATCH: usize = 1000;
	for chunk in rows.chunks(BATCH) {
		let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
			"INSERT INTO whodrug_products (code, drug_name, atc_code, version, language, active) ",
		);
		qb.push_values(chunk, |mut b, row| {
			b.push_bind(&row.code)
				.push_bind(&row.drug_name)
				.push_bind(&row.atc_code)
				.push_bind(version)
				.push_bind(language)
				.push_bind(active);
		});
		qb.push(
			" ON CONFLICT (code, version, language)
			  DO UPDATE SET
			    drug_name = EXCLUDED.drug_name,
			    atc_code = EXCLUDED.atc_code,
			    active = EXCLUDED.active",
		);
		mm.dbx().execute(qb.build()).await.map_err(map_store_err)?;
	}
	Ok(())
}

fn read_zip_file_case_insensitive(
	zip: &mut ZipArchive<Cursor<&[u8]>>,
	target_name: &str,
) -> Result<String> {
	let target_name = target_name.to_ascii_lowercase();
	for i in 0..zip.len() {
		let mut file = zip.by_index(i).map_err(|err| Error::BadRequest {
			message: format!("zip read error: {err}"),
		})?;
		if !file.is_file() {
			continue;
		}
		let name = file.name().rsplit('/').next().unwrap_or("");
		if name.eq_ignore_ascii_case(&target_name) {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)
				.map_err(|err| Error::BadRequest {
					message: format!("zip entry read error: {err}"),
				})?;
			return Ok(String::from_utf8_lossy(&bytes).into_owned());
		}
	}
	Err(Error::BadRequest {
		message: format!("missing required file in zip: {target_name}"),
	})
}

fn insert_term(
	dedup: &mut BTreeMap<(String, String), String>,
	code: &str,
	term: &str,
	level: &str,
) {
	let code = code.trim();
	let term = term.trim();
	if code.is_empty() || term.is_empty() {
		return;
	}
	dedup
		.entry((code.to_string(), level.to_string()))
		.or_insert_with(|| term.to_string());
}

fn detect_delimiter(bytes: &[u8]) -> u8 {
	let head = String::from_utf8_lossy(bytes);
	let sample = head.lines().take(3).collect::<Vec<_>>().join("\n");
	let candidates = [(b',', ','), (b'\t', '\t'), (b';', ';'), (b'|', '|')];
	let mut best = (b',', 0usize);
	for (delim_byte, delim_char) in candidates {
		let count = sample.matches(delim_char).count();
		if count > best.1 {
			best = (delim_byte, count);
		}
	}
	best.0
}

fn find_header_idx(headers: &[String], aliases: &[&str]) -> Option<usize> {
	for (idx, header) in headers.iter().enumerate() {
		if aliases.iter().any(|a| *a == header) {
			return Some(idx);
		}
	}
	None
}

fn normalize_header(value: &str) -> String {
	value
		.trim()
		.to_ascii_lowercase()
		.replace(['-', '_', '.'], " ")
		.split_whitespace()
		.collect::<Vec<_>>()
		.join("_")
}

fn is_delimited_name(name: &str) -> bool {
	name.ends_with(".csv") || name.ends_with(".tsv") || name.ends_with(".txt")
}

fn sha256_hex(bytes: &[u8]) -> String {
	let mut hasher = Sha256::new();
	hasher.update(bytes);
	format!("{:x}", hasher.finalize())
}

fn validate_dictionary(dictionary: &str) -> Result<()> {
	if matches!(dictionary, "meddra" | "whodrug") {
		return Ok(());
	}
	Err(Error::BadRequest {
		message: "dictionary must be one of: meddra, whodrug".to_string(),
	})
}

fn map_store_err<E: std::fmt::Display>(err: E) -> Error {
	lib_core::model::Error::Store(err.to_string()).into()
}
