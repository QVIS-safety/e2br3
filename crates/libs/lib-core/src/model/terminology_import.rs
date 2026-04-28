//! Terminology import pipeline: parse, stage, activate, rollback.
//!
//! This module owns all non-HTTP logic for loading MedDRA and WHODrug
//! dictionaries. HTTP handlers in `terminology_rest` are thin wrappers that
//! call these functions.

use crate::model::store::dbx::Dbx;
use crate::model::ModelManager;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{types::Uuid, FromRow, Postgres, QueryBuilder};
use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

// -- Public error alias

pub type Result<T> = std::result::Result<T, ImportError>;

#[derive(Debug)]
pub enum ImportError {
	BadInput(String),
	Store(String),
}

impl std::fmt::Display for ImportError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ImportError::BadInput(msg) => write!(f, "bad input: {msg}"),
			ImportError::Store(msg) => write!(f, "store error: {msg}"),
		}
	}
}

impl std::error::Error for ImportError {}

fn bad_input(msg: impl Into<String>) -> ImportError {
	ImportError::BadInput(msg.into())
}

fn store_err<E: std::fmt::Display>(err: E) -> ImportError {
	ImportError::Store(err.to_string())
}

// -- Row types

#[derive(Debug, Clone)]
pub struct MeddraRow {
	pub code: String,
	pub term: String,
	pub level: String,
}

#[derive(Debug, Clone)]
pub struct WhodrugRow {
	pub code: String,
	pub drug_name: String,
	pub atc_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

// -- Parsing

pub fn sha256_hex(bytes: &[u8]) -> String {
	let mut hasher = Sha256::new();
	hasher.update(bytes);
	format!("{:x}", hasher.finalize())
}

pub fn validate_dictionary(dictionary: &str) -> Result<()> {
	if matches!(dictionary, "meddra" | "whodrug") {
		return Ok(());
	}
	Err(bad_input("dictionary must be one of: meddra, whodrug"))
}

pub fn parse_meddra_upload(bytes: &[u8]) -> Result<Vec<MeddraRow>> {
	let mut zip =
		ZipArchive::new(Cursor::new(bytes)).map_err(|e| bad_input(format!("invalid MedDRA zip: {e}")))?;

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
		return Err(bad_input("No MedDRA rows parsed from llt.asc/mdhier.asc"));
	}

	Ok(rows)
}

pub fn parse_whodrug_upload(bytes: &[u8]) -> Result<Vec<WhodrugRow>> {
	if let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) {
		for idx in 0..zip.len() {
			let mut entry = zip.by_index(idx).map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
			if !entry.is_file() {
				continue;
			}
			let name = entry.name().to_ascii_lowercase();
			if !is_delimited_name(&name) {
				continue;
			}
			let mut entry_bytes = Vec::new();
			entry.read_to_end(&mut entry_bytes).map_err(|e| bad_input(format!("whodrug zip file read error: {e}")))?;
			if let Ok(rows) = parse_whodrug_delimited(&entry_bytes) {
				if !rows.is_empty() {
					return Ok(rows);
				}
			}
		}
		return Err(bad_input("No parseable WHODrug delimited file in uploaded zip"));
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
		.map_err(|e| bad_input(format!("whodrug header parse error: {e}")))?
		.iter()
		.map(normalize_header)
		.collect::<Vec<_>>();

	let code_idx = find_header_idx(
		&headers,
		&["code", "drug_code", "record_id", "drugid", "drecno", "mpid"],
	)
	.ok_or_else(|| bad_input("Missing WHODrug code column"))?;

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
	.ok_or_else(|| bad_input("Missing WHODrug product name column"))?;

	let atc_idx = find_header_idx(&headers, &["atc", "atc_code", "atc1"]);
	let mut rows = Vec::new();
	let mut seen = HashSet::new();

	for rec in rdr.records() {
		let rec = rec.map_err(|e| bad_input(format!("whodrug row parse error: {e}")))?;
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
		return Err(bad_input("No WHODrug rows parsed from upload"));
	}

	Ok(rows)
}

// -- Staging

pub async fn stage_meddra_rows(
	mm: &ModelManager,
	uploader_id: Uuid,
	organization_id: Uuid,
	role: &str,
	rows: &[MeddraRow],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_full_context(dbx, uploader_id, organization_id, role).await?;
		upsert_release_header(
			mm, "meddra", version, language, "loading", "upload",
			Some(checksum), rows.len() as i64, Some(uploader_id), None, None,
		)
		.await?;
		upsert_meddra_rows(mm, rows, version, language, false).await?;
		upsert_release_header(
			mm, "meddra", version, language, "validated", "upload",
			Some(checksum), rows.len() as i64, Some(uploader_id), None, None,
		)
		.await?;
		Ok::<(), ImportError>(())
	}
	.await;

	finish_txn(dbx, run_result).await
}

pub async fn stage_whodrug_rows(
	mm: &ModelManager,
	uploader_id: Uuid,
	organization_id: Uuid,
	role: &str,
	rows: &[WhodrugRow],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_full_context(dbx, uploader_id, organization_id, role).await?;
		upsert_release_header(
			mm, "whodrug", version, language, "loading", "upload",
			Some(checksum), rows.len() as i64, Some(uploader_id), None, None,
		)
		.await?;
		upsert_whodrug_rows(mm, rows, version, language, false).await?;
		upsert_release_header(
			mm, "whodrug", version, language, "validated", "upload",
			Some(checksum), rows.len() as i64, Some(uploader_id), None, None,
		)
		.await?;
		Ok::<(), ImportError>(())
	}
	.await;

	finish_txn(dbx, run_result).await
}

// -- Activation / rollback

pub async fn activate_release_tx(
	mm: &ModelManager,
	actor_user_id: Uuid,
	organization_id: Uuid,
	role: &str,
	dictionary: &str,
	target_version: &str,
	language: &str,
	is_rollback: bool,
) -> Result<TerminologyReleaseRow> {
	validate_dictionary(dictionary)?;

	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_full_context(dbx, actor_user_id, organization_id, role).await?;

		let target = dbx
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
			.map_err(store_err)?
			.ok_or_else(|| bad_input("target release not found"))?;

		if !matches!(
			target.status.as_str(),
			"approved" | "validated" | "active" | "retired"
		) {
			return Err(bad_input("target release status is not activatable"));
		}

		let current_active_version = dbx
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
			.map_err(store_err)?
			.map(|v| v.0);

		match dictionary {
			"meddra" => {
				dbx.execute(
					sqlx::query(
						"UPDATE meddra_terms SET active = false WHERE language = $1 AND active = true",
					)
					.bind(language),
				)
				.await
				.map_err(store_err)?;
				let changed = dbx
					.execute(
						sqlx::query(
							"UPDATE meddra_terms SET active = true WHERE version = $1 AND language = $2",
						)
						.bind(target_version)
						.bind(language),
					)
					.await
					.map_err(store_err)?;
				if changed == 0 {
					return Err(bad_input("target MedDRA rows were not staged"));
				}
			}
			"whodrug" => {
				dbx.execute(
					sqlx::query(
						"UPDATE whodrug_products SET active = false WHERE language = $1 AND active = true",
					)
					.bind(language),
				)
				.await
				.map_err(store_err)?;
				let changed = dbx
					.execute(
						sqlx::query(
							"UPDATE whodrug_products SET active = true WHERE version = $1 AND language = $2",
						)
						.bind(target_version)
						.bind(language),
					)
					.await
					.map_err(store_err)?;
				if changed == 0 {
					return Err(bad_input("target WHODrug rows were not staged"));
				}
			}
			_ => return Err(bad_input("invalid dictionary")),
		}

		if let Some(prev_version) = current_active_version.as_deref() {
			if prev_version != target_version {
				dbx.execute(
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
				.map_err(store_err)?;
			}
		}

		let rollback_from_version = if is_rollback {
			current_active_version.as_deref()
		} else {
			None
		};

		let updated = dbx
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
			.map_err(store_err)?;

		Ok::<TerminologyReleaseRow, ImportError>(updated)
	}
	.await;

	match run_result {
		Ok(data) => {
			dbx.commit_txn().await.map_err(store_err)?;
			Ok(data)
		}
		Err(err) => {
			let _ = dbx.rollback_txn().await;
			Err(err)
		}
	}
}

// -- Read helpers

pub async fn fetch_releases(
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
		.map_err(store_err)
}

/// Mark a staged/validated release as approved, recording the approver.
/// Returns the updated row, or `None` when the release is not found or
/// is not in an approvable status (`validated` or `approved`).
pub async fn approve_release(
	mm: &ModelManager,
	dictionary: &str,
	version: &str,
	language: &str,
	approved_by: sqlx::types::Uuid,
	note: Option<&str>,
) -> Result<Option<TerminologyReleaseRow>> {
	mm.dbx()
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
			.bind(dictionary)
			.bind(version)
			.bind(language)
			.bind(approved_by)
			.bind(note),
		)
		.await
		.map_err(store_err)
}

// -- Private helpers

async fn set_full_context(
	dbx: &Dbx,
	user_id: Uuid,
	organization_id: Uuid,
	role: &str,
) -> Result<()> {
	crate::model::store::set_full_context_dbx(dbx, user_id, organization_id, role)
		.await
		.map_err(|e| store_err(e))
}

async fn finish_txn(dbx: &Dbx, result: Result<()>) -> Result<()> {
	match result {
		Ok(_) => {
			dbx.commit_txn().await.map_err(store_err)?;
			Ok(())
		}
		Err(err) => {
			let _ = dbx.rollback_txn().await;
			Err(err)
		}
	}
}

pub async fn upsert_release_header(
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
		.map_err(store_err)?;
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
		mm.dbx().execute(qb.build()).await.map_err(store_err)?;
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
		mm.dbx().execute(qb.build()).await.map_err(store_err)?;
	}
	Ok(())
}

fn read_zip_file_case_insensitive(
	zip: &mut ZipArchive<Cursor<&[u8]>>,
	target_name: &str,
) -> Result<String> {
	let target_name = target_name.to_ascii_lowercase();
	for i in 0..zip.len() {
		let mut file = zip.by_index(i).map_err(|e| bad_input(format!("zip read error: {e}")))?;
		if !file.is_file() {
			continue;
		}
		let name = file.name().rsplit('/').next().unwrap_or("");
		if name.eq_ignore_ascii_case(&target_name) {
			let mut bytes = Vec::new();
			file.read_to_end(&mut bytes)
				.map_err(|e| bad_input(format!("zip entry read error: {e}")))?;
			return Ok(String::from_utf8_lossy(&bytes).into_owned());
		}
	}
	Err(bad_input(format!("missing required file in zip: {target_name}")))
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
