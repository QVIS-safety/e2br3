//! Terminology import pipeline: parse, stage, activate, rollback.
//!
//! This module owns all non-HTTP logic for loading MedDRA and WHODrug
//! dictionaries. HTTP handlers in `terminology_rest` are thin wrappers that
//! call these functions.

use crate::ctx::Ctx;
use crate::model::store::dbx::Dbx;
use crate::model::ModelManager;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{types::Uuid, FromRow, Postgres, QueryBuilder};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
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

const MAX_TERMINOLOGY_ZIP_ENTRIES: usize = 256;
const MAX_TERMINOLOGY_ZIP_ENTRY_BYTES: usize = 250 * 1024 * 1024;
const MAX_TERMINOLOGY_ZIP_EXPANDED_BYTES: usize = 512 * 1024 * 1024;
const MAX_C3_ZIP_ENTRY_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_C3_ZIP_EXPANDED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

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

#[derive(Debug, Clone, Copy)]
struct WhodrugPositionalFormat {
	source_name: &'static str,
	basename: &'static str,
	min_columns: usize,
	code_1_idx: usize,
	code_2_idx: usize,
	code_3_idx: usize,
	record_id_idx: Option<usize>,
	name_idx: Option<usize>,
	atc_idx: Option<usize>,
}

const WHODRUG_B3_DD: WhodrugPositionalFormat = WhodrugPositionalFormat {
	source_name: "B3 DD.csv",
	basename: "dd.csv",
	min_columns: 12,
	code_1_idx: 0,
	code_2_idx: 1,
	code_3_idx: 2,
	record_id_idx: None,
	name_idx: Some(11),
	atc_idx: None,
};

const WHODRUG_B3_DDA: WhodrugPositionalFormat = WhodrugPositionalFormat {
	source_name: "B3 DDA.csv",
	basename: "dda.csv",
	min_columns: 5,
	code_1_idx: 0,
	code_2_idx: 1,
	code_3_idx: 2,
	record_id_idx: None,
	name_idx: None,
	atc_idx: Some(4),
};

const WHODRUG_C3_MP: WhodrugPositionalFormat = WhodrugPositionalFormat {
	source_name: "C3 MP.csv",
	basename: "mp.csv",
	min_columns: 22,
	code_1_idx: 2,
	code_2_idx: 3,
	code_3_idx: 4,
	record_id_idx: Some(0),
	name_idx: Some(8),
	atc_idx: None,
};

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
	if matches!(
		dictionary,
		"meddra"
			| "whodrug"
			| "iso3166"
			| "ich_constrained_ucum"
			| "edqm" | "mfds_product"
	) {
		return Ok(());
	}
	Err(bad_input(
		"dictionary must be one of: meddra, whodrug, iso3166, \
		 ich_constrained_ucum, edqm, mfds_product",
	))
}

pub fn parse_meddra_upload(bytes: &[u8]) -> Result<Vec<MeddraRow>> {
	let mut zip = ZipArchive::new(Cursor::new(bytes))
		.map_err(|e| bad_input(format!("invalid MedDRA zip: {e}")))?;
	if zip.len() > MAX_TERMINOLOGY_ZIP_ENTRIES {
		return Err(bad_input(format!(
			"terminology zip contains more than {MAX_TERMINOLOGY_ZIP_ENTRIES} entries"
		)));
	}

	let mut expanded_bytes = 0usize;
	let llt =
		read_zip_file_case_insensitive(&mut zip, "llt.asc", &mut expanded_bytes)?;
	let mdhier =
		read_zip_file_case_insensitive(&mut zip, "mdhier.asc", &mut expanded_bytes)?;

	let mut dedup: BTreeMap<String, MeddraRow> = BTreeMap::new();

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
		insert_term(&mut dedup, code, term, "LLT");
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

	let rows = dedup.into_values().collect::<Vec<_>>();

	if rows.is_empty() {
		return Err(bad_input("No MedDRA rows parsed from llt.asc/mdhier.asc"));
	}

	Ok(rows)
}

pub fn parse_whodrug_upload(bytes: &[u8]) -> Result<Vec<WhodrugRow>> {
	if let Some(entries) = whodrug_zip_entries(bytes)? {
		if has_official_signature(&entries, WHODRUG_B3_DD)? {
			return parse_whodrug_b3_zip_entries(&entries);
		}
		if has_official_signature(&entries, WHODRUG_C3_MP)? {
			return parse_whodrug_c3_zip_entries(&entries);
		}

		for (name, entry_bytes) in &entries {
			if is_whodrug_zip_metadata_or_doc(name) {
				continue;
			}
			if let Ok(rows) = parse_whodrug_delimited(entry_bytes) {
				if !rows.is_empty() {
					return Ok(rows);
				}
			}
		}
		return Err(bad_input(
			"No supported WHODrug file found in uploaded zip; expected official B3 DD.csv, official C3 MP.csv, or a headered product CSV",
		));
	}

	parse_whodrug_delimited(bytes)
}

pub fn parse_whodrug_cas_numbers(bytes: &[u8]) -> Result<Vec<String>> {
	let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) else {
		return Ok(Vec::new());
	};
	if zip.len() > MAX_TERMINOLOGY_ZIP_ENTRIES {
		return Err(bad_input(format!(
			"terminology zip contains more than {MAX_TERMINOLOGY_ZIP_ENTRIES} entries"
		)));
	}
	let mut has_c3_mp = false;
	let mut sun = None;
	let mut expanded_bytes = 0usize;
	for idx in 0..zip.len() {
		let entry = zip
			.by_index(idx)
			.map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
		if !entry.is_file() || is_whodrug_zip_metadata_or_doc(entry.name()) {
			continue;
		}
		let basename = zip_basename(entry.name());
		if basename.eq_ignore_ascii_case(WHODRUG_C3_MP.basename) {
			has_c3_mp = true;
		} else if basename.eq_ignore_ascii_case("sun.csv") {
			let bytes = read_limited_zip_entry(
				entry,
				&mut expanded_bytes,
				"whodrug SUN.csv",
			)?;
			sun = Some(bytes);
		}
	}
	if !has_c3_mp {
		return Ok(Vec::new());
	}
	let sun = sun.ok_or_else(|| bad_input("Missing official WHODrug C3 SUN.csv"))?;
	parse_whodrug_sun(&sun)
}

pub fn parse_whodrug_sun(bytes: &[u8]) -> Result<Vec<String>> {
	let mut rdr = ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_reader(Cursor::new(bytes));
	let mut cas_numbers = BTreeSet::new();
	for (idx, rec) in rdr.records().enumerate() {
		let rec = rec.map_err(|e| {
			bad_input(format!("whodrug SUN.csv row parse error: {e}"))
		})?;
		if is_blank_record(&rec) {
			continue;
		}
		if rec.len() < 6 {
			return Err(bad_input(format!(
				"WHODrug C3 SUN.csv row {} has {} columns; expected at least 6",
				idx + 1,
				rec.len()
			)));
		}
		let cas = rec.get(1).unwrap_or("").trim();
		if cas.len() != 10 || !cas.bytes().all(|value| value.is_ascii_digit()) {
			return Err(bad_input(format!(
				"WHODrug C3 SUN.csv row {} has invalid 10-digit CAS number",
				idx + 1
			)));
		}
		cas_numbers.insert(cas.to_string());
	}
	if cas_numbers.is_empty() {
		return Err(bad_input("No CAS numbers parsed from WHODrug C3 SUN.csv"));
	}
	Ok(cas_numbers.into_iter().collect())
}

fn whodrug_zip_entries(bytes: &[u8]) -> Result<Option<Vec<(String, Vec<u8>)>>> {
	let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) else {
		return Ok(None);
	};
	if zip.len() > MAX_TERMINOLOGY_ZIP_ENTRIES {
		return Err(bad_input(format!(
			"terminology zip contains more than {MAX_TERMINOLOGY_ZIP_ENTRIES} entries"
		)));
	}
	let mut entries = Vec::new();
	let mut expanded_bytes = 0usize;
	for idx in 0..zip.len() {
		let entry = zip
			.by_index(idx)
			.map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
		if !entry.is_file() {
			continue;
		}
		let name = entry.name().to_string();
		if !is_delimited_name(&name.to_ascii_lowercase()) {
			continue;
		}
		let entry_bytes =
			read_limited_zip_entry(entry, &mut expanded_bytes, "whodrug zip file")?;
		entries.push((name, entry_bytes));
	}
	Ok(Some(entries))
}

fn parse_whodrug_b3_zip_entries(
	entries: &[(String, Vec<u8>)],
) -> Result<Vec<WhodrugRow>> {
	let dd = official_zip_entry_by_basename(entries, WHODRUG_B3_DD.basename)
		.ok_or_else(|| bad_input("Missing official WHODrug B3 DD.csv"))?;
	let atc_by_code =
		official_zip_entry_by_basename(entries, WHODRUG_B3_DDA.basename)
			.map(|bytes| parse_whodrug_positional_atc(bytes, WHODRUG_B3_DDA))
			.transpose()?
			.unwrap_or_default();

	parse_whodrug_positional_products(dd, WHODRUG_B3_DD, &atc_by_code)
}

fn parse_whodrug_c3_zip_entries(
	entries: &[(String, Vec<u8>)],
) -> Result<Vec<WhodrugRow>> {
	let mp = official_zip_entry_by_basename(entries, WHODRUG_C3_MP.basename)
		.ok_or_else(|| bad_input("Missing official WHODrug C3 MP.csv"))?;
	let atc_by_code = BTreeMap::new();

	parse_whodrug_positional_products(mp, WHODRUG_C3_MP, &atc_by_code)
}

fn parse_whodrug_positional_products(
	bytes: &[u8],
	format: WhodrugPositionalFormat,
	atc_by_code: &BTreeMap<String, String>,
) -> Result<Vec<WhodrugRow>> {
	let name_idx = format.name_idx.ok_or_else(|| {
		bad_input(format!("{} has no product name column", format.source_name))
	})?;
	let mut rdr = ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_reader(Cursor::new(bytes));
	let mut rows = Vec::new();
	let mut seen = HashSet::new();

	for (idx, rec) in rdr.records().enumerate() {
		let rec = rec.map_err(|e| {
			bad_input(format!(
				"whodrug {} row parse error: {e}",
				format.source_name
			))
		})?;
		let row_number = idx + 1;
		if is_blank_record(&rec) {
			continue;
		}
		validate_positional_row(&rec, format, row_number)?;
		let drug_code = whodrug_joined_code(
			&rec,
			format.code_1_idx,
			format.code_2_idx,
			format.code_3_idx,
		)
		.ok_or_else(|| {
			bad_input(format!(
				"{} row {row_number} is missing WHODrug code segments",
				format.source_name
			))
		})?;
		let c3_identity = if format.record_id_idx.is_some() {
			Some(parse_c3_product_identity(&rec, row_number)?)
		} else {
			None
		};
		let code = if let Some((record_id, _)) = c3_identity {
			record_id.to_string()
		} else {
			drug_code.clone()
		};
		let drug_name = if let Some((_, drug_name)) = c3_identity {
			drug_name
		} else {
			rec.get(name_idx).unwrap_or("").trim()
		};
		if drug_name.is_empty() {
			return Err(bad_input(format!(
				"{} row {row_number} is missing drug name",
				format.source_name
			)));
		}
		if !seen.insert(code.clone()) {
			if format.record_id_idx.is_some() {
				return Err(bad_input(format!(
					"{} row {row_number} has duplicate Record_Id {code}",
					format.source_name
				)));
			}
			continue;
		}
		rows.push(WhodrugRow {
			atc_code: atc_by_code.get(&drug_code).cloned(),
			code,
			drug_name: drug_name.to_string(),
		});
	}

	if rows.is_empty() {
		return Err(bad_input(format!(
			"No WHODrug rows parsed from {}",
			format.source_name
		)));
	}

	Ok(rows)
}

fn parse_whodrug_positional_atc(
	bytes: &[u8],
	format: WhodrugPositionalFormat,
) -> Result<BTreeMap<String, String>> {
	let atc_idx = format.atc_idx.ok_or_else(|| {
		bad_input(format!("{} has no ATC column", format.source_name))
	})?;
	let mut rdr = ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_reader(Cursor::new(bytes));
	let mut by_code = BTreeMap::new();

	for (idx, rec) in rdr.records().enumerate() {
		let rec = rec.map_err(|e| {
			bad_input(format!(
				"whodrug {} row parse error: {e}",
				format.source_name
			))
		})?;
		let row_number = idx + 1;
		if is_blank_record(&rec) {
			continue;
		}
		validate_positional_row(&rec, format, row_number)?;
		let code = whodrug_joined_code(
			&rec,
			format.code_1_idx,
			format.code_2_idx,
			format.code_3_idx,
		)
		.ok_or_else(|| {
			bad_input(format!(
				"{} row {row_number} is missing WHODrug code segments",
				format.source_name
			))
		})?;
		let atc = rec.get(atc_idx).unwrap_or("").trim();
		if !atc.is_empty() {
			by_code.entry(code).or_insert_with(|| atc.to_string());
		}
	}

	Ok(by_code)
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
		let rec =
			rec.map_err(|e| bad_input(format!("whodrug row parse error: {e}")))?;
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
	rows: &[MeddraRow],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_platform_service_context(dbx).await?;
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
		Ok::<(), ImportError>(())
	}
	.await;

	finish_txn(dbx, run_result).await
}

pub async fn stage_whodrug_rows(
	mm: &ModelManager,
	uploader_id: Uuid,
	rows: &[WhodrugRow],
	cas_numbers: &[String],
	version: &str,
	language: &str,
	checksum: &str,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_platform_service_context(dbx).await?;
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
		Ok::<(), ImportError>(())
	}
	.await;
	finish_txn(dbx, run_result).await?;

	for chunk in rows.chunks(1000) {
		dbx.begin_txn().await.map_err(store_err)?;
		let run_result = async {
			set_platform_service_context(dbx).await?;
			upsert_whodrug_rows(mm, chunk, version, language, false).await?;
			Ok::<(), ImportError>(())
		}
		.await;
		finish_txn(dbx, run_result).await?;
	}
	for chunk in cas_numbers.chunks(1000) {
		dbx.begin_txn().await.map_err(store_err)?;
		let run_result = async {
			set_platform_service_context(dbx).await?;
			upsert_whodrug_cas_numbers(mm, chunk, version, language, false).await?;
			Ok::<(), ImportError>(())
		}
		.await;
		finish_txn(dbx, run_result).await?;
	}

	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_platform_service_context(dbx).await?;
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
		Ok::<(), ImportError>(())
	}
	.await;

	finish_txn(dbx, run_result).await
}

pub fn whodrug_zip_is_official_c3(path: &Path) -> Result<bool> {
	let mut file = File::open(path)
		.map_err(|e| bad_input(format!("whodrug upload open error: {e}")))?;
	let mut signature = [0u8; 4];
	let signature_len = file
		.read(&mut signature)
		.map_err(|e| bad_input(format!("whodrug upload read error: {e}")))?;
	file.rewind()
		.map_err(|e| bad_input(format!("whodrug upload seek error: {e}")))?;
	let mut zip = match ZipArchive::new(file) {
		Ok(zip) => zip,
		Err(_) if signature_len < 2 || signature[..2] != *b"PK" => return Ok(false),
		Err(err) => return Err(bad_input(format!("whodrug zip read error: {err}"))),
	};
	if zip.len() > MAX_TERMINOLOGY_ZIP_ENTRIES {
		return Err(bad_input(format!(
			"terminology zip contains more than {MAX_TERMINOLOGY_ZIP_ENTRIES} entries"
		)));
	}
	let mut mp_count = 0;
	for idx in 0..zip.len() {
		let entry = zip
			.by_index(idx)
			.map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
		if entry.enclosed_name().is_none() {
			return Err(bad_input("WHODrug zip contains an unsafe entry path"));
		}
		if entry.is_file()
			&& zip_basename(entry.name()).eq_ignore_ascii_case("mp.csv")
		{
			mp_count += 1;
		}
	}
	if mp_count > 1 {
		return Err(bad_input("duplicate WHODrug C3 MP.csv entries"));
	}
	Ok(mp_count == 1)
}

fn prepare_whodrug_c3_files(path: &Path, version: &str) -> Result<(File, File)> {
	let file = File::open(path)
		.map_err(|e| bad_input(format!("whodrug upload open error: {e}")))?;
	let mut zip = ZipArchive::new(file)
		.map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
	if zip.len() > MAX_TERMINOLOGY_ZIP_ENTRIES {
		return Err(bad_input(format!(
			"terminology zip contains more than {MAX_TERMINOLOGY_ZIP_ENTRIES} entries"
		)));
	}
	let mut expanded = 0u64;
	let mut mp_idx = None;
	let mut sun_idx = None;
	let mut version_idx = None;
	for idx in 0..zip.len() {
		let entry = zip
			.by_index(idx)
			.map_err(|e| bad_input(format!("whodrug zip read error: {e}")))?;
		if entry.enclosed_name().is_none() {
			return Err(bad_input("WHODrug zip contains an unsafe entry path"));
		}
		if !entry.is_file() {
			continue;
		}
		if entry.size() > MAX_C3_ZIP_ENTRY_BYTES {
			return Err(bad_input("WHODrug C3 entry exceeds archive limit"));
		}
		expanded = expanded
			.checked_add(entry.size())
			.ok_or_else(|| bad_input("WHODrug C3 expanded size overflow"))?;
		if expanded > MAX_C3_ZIP_EXPANDED_BYTES {
			return Err(bad_input("WHODrug C3 archive exceeds expanded size limit"));
		}
		let name = entry.name();
		let slot = if zip_basename(name).eq_ignore_ascii_case("mp.csv") {
			Some(&mut mp_idx)
		} else if zip_basename(name).eq_ignore_ascii_case("sun.csv") {
			Some(&mut sun_idx)
		} else if name.eq_ignore_ascii_case("version.csv") {
			Some(&mut version_idx)
		} else {
			None
		};
		if let Some(slot) = slot {
			if slot.replace(idx).is_some() {
				return Err(bad_input(format!("duplicate WHODrug C3 entry {name}")));
			}
		}
	}
	let mp_idx =
		mp_idx.ok_or_else(|| bad_input("Missing official WHODrug C3 MP.csv"))?;
	let sun_idx =
		sun_idx.ok_or_else(|| bad_input("Missing official WHODrug C3 SUN.csv"))?;
	let version_idx =
		version_idx.ok_or_else(|| bad_input("Missing WHODrug root Version.csv"))?;
	let mut version_entry = zip
		.by_index(version_idx)
		.map_err(|e| bad_input(format!("whodrug Version.csv read error: {e}")))?;
	let mut version_bytes = Vec::new();
	version_entry
		.by_ref()
		.take(4097)
		.read_to_end(&mut version_bytes)
		.map_err(|e| bad_input(format!("whodrug Version.csv read error: {e}")))?;
	if version_bytes.len() > 4096 {
		return Err(bad_input("WHODrug root Version.csv exceeds 4096 bytes"));
	}
	let mut version_reader = ReaderBuilder::new()
		.has_headers(false)
		.from_reader(version_bytes.as_slice());
	let mut version_rows = version_reader.records();
	let version_row = version_rows
		.next()
		.ok_or_else(|| bad_input("WHODrug root Version.csv is empty"))?
		.map_err(|e| bad_input(format!("whodrug Version.csv parse error: {e}")))?;
	if version_rows.next().is_some() {
		return Err(bad_input(
			"WHODrug root Version.csv must contain exactly one row",
		));
	}
	let package_short = version_row
		.get(1)
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.ok_or_else(|| bad_input("WHODrug root Version.csv has no short version"))?;
	if package_short.len() > 20 {
		return Err(bad_input(
			"WHODrug package short version exceeds 20 characters",
		));
	}
	if package_short != version {
		return Err(bad_input(format!(
			"WHODrug version {version} does not match package short version {package_short}"
		)));
	}
	drop(version_reader);
	drop(version_entry);

	let mp = copy_c3_entry_to_anonymous_file(&mut zip, mp_idx, "MP.csv")?;
	let sun = copy_c3_entry_to_anonymous_file(&mut zip, sun_idx, "SUN.csv")?;
	Ok((mp, sun))
}

fn copy_c3_entry_to_anonymous_file(
	zip: &mut ZipArchive<File>,
	entry_idx: usize,
	label: &str,
) -> Result<File> {
	let mut entry = zip
		.by_index(entry_idx)
		.map_err(|err| bad_input(format!("whodrug {label} read error: {err}")))?;
	let path =
		std::env::temp_dir().join(format!("e2br3-whodrug-c3-{}", Uuid::new_v4()));
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create_new(true)
		.mode(0o600)
		.open(&path)
		.map_err(|err| {
			ImportError::Store(format!("WHODrug C3 temporary file error: {err}"))
		})?;
	std::fs::remove_file(&path).map_err(|err| {
		ImportError::Store(format!("WHODrug C3 temporary unlink error: {err}"))
	})?;
	let copied = std::io::copy(
		&mut entry.by_ref().take(MAX_C3_ZIP_ENTRY_BYTES + 1),
		&mut file,
	)
	.map_err(|err| bad_input(format!("whodrug {label} read error: {err}")))?;
	if copied > MAX_C3_ZIP_ENTRY_BYTES {
		return Err(bad_input(format!("WHODrug C3 {label} exceeds entry limit")));
	}
	file.rewind().map_err(|err| {
		ImportError::Store(format!("WHODrug C3 temporary seek error: {err}"))
	})?;
	Ok(file)
}

pub async fn stage_whodrug_c3_zip(
	mm: &ModelManager,
	uploader_id: Uuid,
	path: &Path,
	version: &str,
	language: &str,
	checksum: &str,
	dry_run: bool,
) -> Result<i64> {
	let path = path.to_path_buf();
	let version_owned = version.to_string();
	let (mp, sun) = tokio::task::spawn_blocking(move || {
		prepare_whodrug_c3_files(&path, &version_owned)
	})
	.await
	.map_err(|err| {
		ImportError::Store(format!("WHODrug C3 preparation task failed: {err}"))
	})??;

	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let result = async {
		set_platform_service_context(dbx).await?;
		dbx.execute(
			sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
				.bind(format!("whodrug:{version}:{language}")),
		)
		.await
		.map_err(store_err)?;
		let existing: (i64,) = dbx
			.fetch_one(
				sqlx::query_as(
					"SELECT (SELECT COUNT(*) FROM terminology_releases WHERE dictionary='whodrug' AND version=$1 AND language=$2) + (SELECT COUNT(*) FROM whodrug_products WHERE version=$1 AND language=$2) + (SELECT COUNT(*) FROM controlled_terminology_terms WHERE dictionary='whodrug' AND version=$1 AND language=$2)",
				)
				.bind(version)
				.bind(language),
			)
			.await
			.map_err(store_err)?;
		if existing.0 != 0 {
			return Err(bad_input(format!(
				"WHODrug release {version}/{language} already exists"
			)));
		}
		dbx.execute(sqlx::query(
			"CREATE TEMP TABLE whodrug_product_import_stage (
			 code VARCHAR(20) NOT NULL, drug_name TEXT NOT NULL, atc_code VARCHAR(20),
			 version VARCHAR(20) NOT NULL, language VARCHAR(2) NOT NULL,
			 PRIMARY KEY (code, version, language)) ON COMMIT DROP",
		)).await.map_err(store_err)?;
		dbx.execute(sqlx::query(
			"CREATE TEMP TABLE whodrug_cas_import_stage (
			 code VARCHAR(100) NOT NULL, version VARCHAR(40) NOT NULL, language VARCHAR(10) NOT NULL,
			 PRIMARY KEY (code, version, language)) ON COMMIT DROP",
		)).await.map_err(store_err)?;

		let mut reader = ReaderBuilder::new().has_headers(false).flexible(true).from_reader(mp.take(MAX_C3_ZIP_ENTRY_BYTES + 1));
		let mut batch = Vec::with_capacity(1000);
		let mut count = 0i64;
		for (idx, record) in reader.records().enumerate() {
			let record = record.map_err(|e| bad_input(format!("whodrug C3 MP.csv row {} parse error: {e}", idx + 1)))?;
			if is_blank_record(&record) { continue; }
			let (record_id, drug_name) = parse_c3_product_identity(&record, idx + 1)?;
			batch.push((record_id.to_string(), drug_name.to_string()));
			count += 1;
			if batch.len() == 1000 { insert_c3_product_stage(dbx, &batch, version, language).await?; batch.clear(); }
		}
		if !batch.is_empty() { insert_c3_product_stage(dbx, &batch, version, language).await?; }
		if count == 0 { return Err(bad_input("No WHODrug rows parsed from C3 MP.csv")); }
		if reader.into_inner().limit() == 0 { return Err(bad_input("WHODrug C3 MP.csv exceeds entry limit")); }

		let mut reader = ReaderBuilder::new().has_headers(false).flexible(true).from_reader(sun.take(MAX_C3_ZIP_ENTRY_BYTES + 1));
		let mut batch = Vec::with_capacity(1000);
		let mut cas_count = 0usize;
		for (idx, record) in reader.records().enumerate() {
			let record = record.map_err(|e| bad_input(format!("whodrug SUN.csv row {} parse error: {e}", idx + 1)))?;
			if is_blank_record(&record) { continue; }
			if record.len() < 6 { return Err(bad_input(format!("WHODrug C3 SUN.csv row {} has {} columns; expected at least 6", idx + 1, record.len()))); }
			let code = record.get(1).map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| bad_input(format!("WHODrug SUN.csv row {} is missing CAS number", idx + 1)))?;
			if code.len() != 10 || !code.bytes().all(|value| value.is_ascii_digit()) { return Err(bad_input(format!("WHODrug C3 SUN.csv row {} has invalid 10-digit CAS number", idx + 1))); }
			batch.push(code.to_string());
			cas_count += 1;
			if batch.len() == 1000 { insert_c3_cas_stage(dbx, &batch, version, language).await?; batch.clear(); }
		}
		if !batch.is_empty() { insert_c3_cas_stage(dbx, &batch, version, language).await?; }
		if cas_count == 0 { return Err(bad_input("No CAS numbers parsed from WHODrug C3 SUN.csv")); }
		if reader.into_inner().limit() == 0 { return Err(bad_input("WHODrug C3 SUN.csv exceeds entry limit")); }
		if dry_run { return Ok(count); }
		dbx.execute(sqlx::query(
			"INSERT INTO whodrug_products (code, drug_name, atc_code, version, language, active)
			 SELECT code, drug_name, atc_code, version, language, false FROM whodrug_product_import_stage",
		)).await.map_err(store_err)?;
		dbx.execute(sqlx::query(
			"INSERT INTO controlled_terminology_terms (dictionary, version, language, scope, code, active)
			 SELECT 'whodrug', version, language, 'cas', code, false FROM whodrug_cas_import_stage",
		)).await.map_err(store_err)?;
		dbx.execute(
			sqlx::query(
				"INSERT INTO terminology_releases
				 (dictionary, version, language, status, source_path, source_checksum,
				  loaded_rows, activated_by, created_at, updated_at)
				 VALUES ('whodrug', $1, $2, 'validated', 'upload', $3, $4, $5, NOW(), NOW())",
			)
			.bind(version)
			.bind(language)
			.bind(checksum)
			.bind(count)
			.bind(uploader_id),
		)
		.await
		.map_err(store_err)?;
		Ok(count)
	}.await;
	if dry_run {
		let rollback = dbx.rollback_txn().await.map_err(store_err);
		return match (result, rollback) {
			(Ok(count), Ok(())) => Ok(count),
			(Err(err), _) => Err(err),
			(Ok(_), Err(err)) => Err(err),
		};
	}
	finish_txn(dbx, result).await
}

async fn insert_c3_product_stage(
	dbx: &Dbx,
	rows: &[(String, String)],
	version: &str,
	language: &str,
) -> Result<()> {
	let mut qb = QueryBuilder::<Postgres>::new("INSERT INTO whodrug_product_import_stage (code, drug_name, atc_code, version, language) ");
	qb.push_values(rows, |mut row, (code, name)| {
		row.push_bind(code)
			.push_bind(name)
			.push_bind(Option::<String>::None)
			.push_bind(version)
			.push_bind(language);
	});
	dbx.execute(qb.build()).await.map_err(|e| {
		if e.to_string().contains("duplicate key") {
			bad_input("duplicate WHODrug C3 Record_Id")
		} else {
			store_err(e)
		}
	})?;
	Ok(())
}

async fn insert_c3_cas_stage(
	dbx: &Dbx,
	rows: &[String],
	version: &str,
	language: &str,
) -> Result<()> {
	let mut qb = QueryBuilder::<Postgres>::new(
		"INSERT INTO whodrug_cas_import_stage (code, version, language) ",
	);
	qb.push_values(rows, |mut row, code| {
		row.push_bind(code).push_bind(version).push_bind(language);
	});
	qb.push(" ON CONFLICT (code, version, language) DO NOTHING");
	dbx.execute(qb.build()).await.map_err(store_err)?;
	Ok(())
}

// -- Activation / rollback

pub async fn activate_release_tx(
	mm: &ModelManager,
	actor_user_id: Uuid,
	dictionary: &str,
	target_version: &str,
	language: &str,
	is_rollback: bool,
) -> Result<TerminologyReleaseRow> {
	validate_dictionary(dictionary)?;

	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let run_result = async {
		set_platform_service_context(dbx).await?;

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
				dbx.execute(
					sqlx::query(
						"UPDATE controlled_terminology_terms
						 SET active = false
						 WHERE dictionary = 'whodrug' AND scope = 'cas'
						   AND language = $1 AND active = true",
					)
					.bind(language),
				)
				.await
				.map_err(store_err)?;
				dbx.execute(
					sqlx::query(
						"UPDATE controlled_terminology_terms
						 SET active = true
						 WHERE dictionary = 'whodrug' AND scope = 'cas'
						   AND version = $1 AND language = $2",
					)
					.bind(target_version)
					.bind(language),
				)
				.await
				.map_err(store_err)?;
			}
			"iso3166" | "ich_constrained_ucum" | "edqm" => {
				dbx.execute(
					sqlx::query(
						"UPDATE controlled_terminology_terms
						 SET active = false
						 WHERE dictionary = $1 AND language = $2 AND active = true",
					)
					.bind(dictionary)
					.bind(language),
				)
				.await
				.map_err(store_err)?;
				let changed = dbx
					.execute(
						sqlx::query(
							"UPDATE controlled_terminology_terms
							 SET active = true
							 WHERE dictionary = $1 AND version = $2 AND language = $3",
						)
						.bind(dictionary)
						.bind(target_version)
						.bind(language),
					)
					.await
					.map_err(store_err)?;
				if changed == 0 {
					return Err(bad_input("target controlled terminology rows were not staged"));
				}
			}
			"mfds_product" => {
				dbx.execute(
					sqlx::query("UPDATE mfds_products SET active = false WHERE active = true"),
				)
				.await
				.map_err(store_err)?;
				let changed = dbx
					.execute(
						sqlx::query(
							"UPDATE mfds_products SET active = true WHERE version = $1",
						)
						.bind(target_version),
					)
					.await
					.map_err(store_err)?;
				if changed == 0 {
					return Err(bad_input("target MFDS product rows were not staged"));
				}
				dbx.execute(
					sqlx::query("UPDATE mfds_product_substances SET active = false WHERE active = true"),
				)
				.await
				.map_err(store_err)?;
				dbx.execute(
					sqlx::query("UPDATE mfds_product_substances SET active = true WHERE version = $1")
						.bind(target_version),
				)
				.await
				.map_err(store_err)?;
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
	actor_user_id: Uuid,
	dictionary: &str,
	version: &str,
	language: &str,
	note: Option<&str>,
) -> Result<Option<TerminologyReleaseRow>> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(store_err)?;
	let result = async {
		set_platform_service_context(dbx).await?;
		dbx.fetch_optional(
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
			.bind(actor_user_id)
			.bind(note),
		)
		.await
		.map_err(store_err)
	}
	.await;
	finish_txn(dbx, result).await
}

// -- Private helpers

async fn set_platform_service_context(dbx: &Dbx) -> Result<()> {
	crate::model::store::set_full_context_from_ctx_dbx(dbx, &Ctx::root_ctx())
		.await
		.map_err(store_err)
}

async fn finish_txn<T>(dbx: &Dbx, result: Result<T>) -> Result<T> {
	match result {
		Ok(value) => {
			dbx.commit_txn().await.map_err(store_err)?;
			Ok(value)
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

async fn upsert_whodrug_cas_numbers(
	mm: &ModelManager,
	cas_numbers: &[String],
	version: &str,
	language: &str,
	active: bool,
) -> Result<()> {
	let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
		"INSERT INTO controlled_terminology_terms
		 (dictionary, version, language, scope, code, active) ",
	);
	qb.push_values(cas_numbers, |mut row, cas| {
		row.push_bind("whodrug")
			.push_bind(version)
			.push_bind(language)
			.push_bind("cas")
			.push_bind(cas)
			.push_bind(active);
	});
	qb.push(
		" ON CONFLICT (dictionary, version, language, scope, code)
		  DO UPDATE SET active = EXCLUDED.active",
	);
	mm.dbx().execute(qb.build()).await.map_err(store_err)?;
	Ok(())
}

fn read_zip_file_case_insensitive(
	zip: &mut ZipArchive<Cursor<&[u8]>>,
	target_name: &str,
	expanded_bytes: &mut usize,
) -> Result<String> {
	let target_name = target_name.to_ascii_lowercase();
	for i in 0..zip.len() {
		let file = zip
			.by_index(i)
			.map_err(|e| bad_input(format!("zip read error: {e}")))?;
		if !file.is_file() {
			continue;
		}
		let name = file.name().rsplit('/').next().unwrap_or("");
		if name.eq_ignore_ascii_case(&target_name) {
			let bytes = read_limited_zip_entry(
				file,
				expanded_bytes,
				"terminology zip entry",
			)?;
			return Ok(String::from_utf8_lossy(&bytes).into_owned());
		}
	}
	Err(bad_input(format!(
		"missing required file in zip: {target_name}"
	)))
}

fn read_limited_zip_entry<R: Read>(
	mut reader: R,
	expanded_bytes: &mut usize,
	label: &str,
) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	let mut entry_bytes = 0usize;
	let mut chunk = [0u8; 64 * 1024];
	loop {
		let count = reader
			.read(&mut chunk)
			.map_err(|e| bad_input(format!("{label} read error: {e}")))?;
		if count == 0 {
			break;
		}
		entry_bytes = entry_bytes
			.checked_add(count)
			.ok_or_else(|| bad_input(format!("{label} size overflow")))?;
		*expanded_bytes = expanded_bytes
			.checked_add(count)
			.ok_or_else(|| bad_input(format!("{label} expanded size overflow")))?;
		if entry_bytes > MAX_TERMINOLOGY_ZIP_ENTRY_BYTES
			|| *expanded_bytes > MAX_TERMINOLOGY_ZIP_EXPANDED_BYTES
		{
			return Err(bad_input(format!(
				"{label} exceeds terminology archive limits"
			)));
		}
		bytes.extend_from_slice(&chunk[..count]);
	}
	Ok(bytes)
}

fn insert_term(
	dedup: &mut BTreeMap<String, MeddraRow>,
	code: &str,
	term: &str,
	level: &str,
) {
	let code = code.trim();
	let term = term.trim();
	if code.is_empty() || term.is_empty() {
		return;
	}
	let next = MeddraRow {
		code: code.to_string(),
		term: term.to_string(),
		level: level.to_string(),
	};
	match dedup.get(code) {
		Some(existing)
			if meddra_level_rank(&existing.level) <= meddra_level_rank(level) => {}
		_ => {
			dedup.insert(code.to_string(), next);
		}
	}
}

fn meddra_level_rank(level: &str) -> u8 {
	match level {
		"LLT" => 0,
		"PT" => 1,
		"HLT" => 2,
		"HLGT" => 3,
		"SOC" => 4,
		_ => u8::MAX,
	}
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

fn has_official_signature(
	entries: &[(String, Vec<u8>)],
	format: WhodrugPositionalFormat,
) -> Result<bool> {
	Ok(official_zip_entry_by_basename(entries, format.basename).is_some())
}

fn official_zip_entry_by_basename<'a>(
	entries: &'a [(String, Vec<u8>)],
	basename: &str,
) -> Option<&'a [u8]> {
	entries
		.iter()
		.find(|(name, _)| {
			!is_whodrug_zip_metadata_or_doc(name)
				&& zip_basename(name).eq_ignore_ascii_case(basename)
		})
		.map(|(_, bytes)| bytes.as_slice())
}

fn zip_basename(name: &str) -> &str {
	name.rsplit('/').next().unwrap_or(name)
}

fn is_whodrug_zip_metadata_or_doc(name: &str) -> bool {
	let lower_name = name.to_ascii_lowercase();
	let basename = zip_basename(&lower_name);
	matches!(
		basename,
		"version.csv" | "readme.csv" | "readme.txt" | "license.csv" | "license.txt"
	) || lower_name.split('/').any(|part| {
		matches!(
			part,
			"doc" | "docs" | "documentation" | "manual" | "manuals"
		)
	})
}

fn whodrug_joined_code(
	rec: &csv::StringRecord,
	code_1_idx: usize,
	code_2_idx: usize,
	code_3_idx: usize,
) -> Option<String> {
	let code_1 = rec.get(code_1_idx)?.trim();
	let code_2 = rec.get(code_2_idx)?.trim();
	let code_3 = rec.get(code_3_idx)?.trim();
	if code_1.is_empty() || code_2.is_empty() || code_3.is_empty() {
		return None;
	}
	Some(format!("{code_1}-{code_2}-{code_3}"))
}

fn validate_positional_row(
	rec: &csv::StringRecord,
	format: WhodrugPositionalFormat,
	row_number: usize,
) -> Result<()> {
	if rec.len() < format.min_columns {
		return Err(bad_input(format!(
			"{} row {row_number} has {} columns; expected at least {}",
			format.source_name,
			rec.len(),
			format.min_columns
		)));
	}
	looks_like_positional_whodrug_row(rec, format, row_number)
}

fn looks_like_positional_whodrug_row(
	rec: &csv::StringRecord,
	format: WhodrugPositionalFormat,
	row_number: usize,
) -> Result<()> {
	let code_1 = rec.get(format.code_1_idx).unwrap_or("").trim();
	let code_2 = rec.get(format.code_2_idx).unwrap_or("").trim();
	let code_3 = rec.get(format.code_3_idx).unwrap_or("").trim();
	if !is_digits(code_1) || !is_code_segment(code_2) || !is_code_segment(code_3) {
		return Err(bad_input(format!(
			"{} row {row_number} does not look like a positional WHODrug row",
			format.source_name
		)));
	}
	Ok(())
}

fn is_blank_record(rec: &csv::StringRecord) -> bool {
	rec.iter().all(|field| field.trim().is_empty())
}

fn parse_c3_product_identity<'a>(
	record: &'a csv::StringRecord,
	row_number: usize,
) -> Result<(&'a str, &'a str)> {
	validate_positional_row(record, WHODRUG_C3_MP, row_number)?;
	let record_id = record
		.get(0)
		.ok_or_else(|| {
			bad_input(format!("C3 MP.csv row {row_number} is missing Record_Id"))
		})?
		.trim();
	if record_id.is_empty() || record_id.len() > 10 || !is_code_segment(record_id) {
		return Err(bad_input(format!(
			"C3 MP.csv row {row_number} has invalid Record_Id"
		)));
	}
	let drug_name = record
		.get(8)
		.ok_or_else(|| {
			bad_input(format!("C3 MP.csv row {row_number} is missing drug name"))
		})?
		.trim();
	if drug_name.is_empty() {
		return Err(bad_input(format!(
			"C3 MP.csv row {row_number} is missing drug name"
		)));
	}
	Ok((record_id, drug_name))
}

fn is_digits(value: &str) -> bool {
	!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

fn is_code_segment(value: &str) -> bool {
	!value.is_empty() && value.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use zip::write::SimpleFileOptions;
	use zip::{CompressionMethod, ZipWriter};

	#[test]
	fn validates_every_supported_release_dictionary() {
		for dictionary in [
			"meddra",
			"whodrug",
			"iso3166",
			"ich_constrained_ucum",
			"edqm",
			"mfds_product",
		] {
			validate_dictionary(dictionary).unwrap_or_else(|err| {
				panic!("{dictionary} should be supported: {err}")
			});
		}
		assert!(validate_dictionary("unknown").is_err());
	}

	#[test]
	fn parse_meddra_zip_keeps_one_row_per_code_for_database_key() {
		let zip = make_zip(&[
			("llt.asc", "10000001$LLT preferred duplicate$$$$$$$$$$$$$$$$$$$$\n"),
			(
				"mdhier.asc",
				"10000001$20000001$30000001$40000001$PT duplicate$HLT term$HLGT term$SOC term$$$$\n",
			),
		]);

		let rows = parse_meddra_upload(&zip).expect("MedDRA zip should parse");

		assert_eq!(rows.len(), 4);
		let duplicate_code_rows = rows
			.iter()
			.filter(|row| row.code == "10000001")
			.collect::<Vec<_>>();
		assert_eq!(duplicate_code_rows.len(), 1);
		assert_eq!(duplicate_code_rows[0].term, "LLT preferred duplicate");
		assert_eq!(duplicate_code_rows[0].level, "LLT");
	}

	#[test]
	fn parse_whodrug_official_b3_zip_uses_dd_and_dda_rows() {
		let zip = make_zip(&[
			("b3/DD.csv", "000001,01,001,6,N,,001,,01,,854,METHYLDOPA\n"),
			("b3/DDA.csv", "000001,01,001,6,C02AB,111,*\n"),
		]);

		let rows =
			parse_whodrug_upload(&zip).expect("official B3 rows should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "METHYLDOPA");
		assert_eq!(rows[0].atc_code.as_deref(), Some("C02AB"));
	}

	#[test]
	fn parse_whodrug_official_b3_zip_accepts_alphanumeric_sequence_codes() {
		let zip = make_zip(&[
			(
				"DD.csv",
				"152686,A0,001,6,N,,001,,01,,854,EXAMPLE PRODUCT\n000027,01,A00,6,T,25,371,,01,,143,HJERTEMAGNYL [ACETYLSALICYLIC ACID]\n",
			),
			("DDA.csv", "152686,A0,001,6,J07BN,231,*\n"),
		]);

		let rows = parse_whodrug_upload(&zip)
			.expect("official B3 alphanumeric sequence should parse");

		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].code, "152686-A0-001");
		assert_eq!(rows[0].drug_name, "EXAMPLE PRODUCT");
		assert_eq!(rows[0].atc_code.as_deref(), Some("J07BN"));
		assert_eq!(rows[1].code, "000027-01-A00");
		assert_eq!(rows[1].drug_name, "HJERTEMAGNYL [ACETYLSALICYLIC ACID]");
		assert_eq!(rows[1].atc_code, None);
	}

	#[test]
	fn parse_whodrug_official_c3_zip_uses_mp_rows_without_unproven_atc_mapping() {
		let zip = make_zip(&[
			(
				"c3/MP.csv",
				"RID0000001,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\nRID2,,000001,01,001,0000000001,0000000001,N,Aldomet,,,,,USA,,6546,010,USA,68,001,19851231,20250930\n",
			),
			("c3/ATC.csv", "C02AB,ANTIHYPERTENSIVES\n"),
		]);

		let rows =
			parse_whodrug_upload(&zip).expect("official C3 rows should parse");

		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].code, "RID0000001");
		assert_eq!(rows[0].drug_name, "Methyldopa");
		assert_eq!(rows[0].atc_code, None);
		assert_eq!(rows[1].code, "RID2");
		assert_eq!(rows[1].drug_name, "Aldomet");
		assert_eq!(rows[1].atc_code, None);
	}

	#[test]
	fn parse_whodrug_official_c3_zip_rejects_duplicate_record_id() {
		let zip = make_zip(&[(
			"MP.csv",
			"RID1,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\nRID1,,000002,01,001,0000000001,0000000001,N,Aldomet,,,,,USA,,6546,010,USA,68,001,19851231,20250930\n",
		)]);

		let err = parse_whodrug_upload(&zip)
			.expect_err("duplicate C3 Record_Id should fail");

		assert_bad_input_contains(
			err,
			"C3 MP.csv row 2 has duplicate Record_Id RID1",
		);
	}

	#[test]
	fn parse_whodrug_official_c3_zip_rejects_invalid_record_id() {
		for record_id in ["", "RID-1", "RID12345678"] {
			let row = format!(
				"{record_id},,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\n"
			);
			let zip = make_zip(&[("MP.csv", row.as_str())]);

			let err = parse_whodrug_upload(&zip)
				.expect_err("invalid C3 Record_Id should fail");

			assert_bad_input_contains(err, "C3 MP.csv row 1 has invalid Record_Id");
		}
	}

	#[test]
	fn parse_whodrug_official_c3_zip_reads_sun_cas_numbers() {
		let zip = make_zip(&[
			(
				"MP.csv",
				"1,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\n",
			),
			(
				"SUN.csv",
				"1,0000050000,EN,Formaldehyde solution,,180\n2,0000050011,EN,Guanidine hydrochloride,72,002\n",
			),
		]);

		assert_eq!(
			parse_whodrug_cas_numbers(&zip).expect("official C3 CAS rows"),
			vec!["0000050000".to_string(), "0000050011".to_string()]
		);
	}

	#[test]
	fn parse_whodrug_official_c3_zip_requires_sun() {
		let zip = make_zip(&[(
			"MP.csv",
			"1,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\n",
		)]);

		let err = parse_whodrug_cas_numbers(&zip)
			.expect_err("official C3 without SUN must fail");
		assert_bad_input_contains(err, "SUN.csv");
	}

	#[test]
	fn parse_whodrug_generic_headered_csv_still_parses() {
		let rows = parse_whodrug_upload(
			b"drug_code,drug_name,atc_code\n000001-01-001,Methyldopa,C02AB\n",
		)
		.expect("generic headered WHODrug csv should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "Methyldopa");
		assert_eq!(rows[0].atc_code.as_deref(), Some("C02AB"));
	}

	#[test]
	fn parse_whodrug_zip_with_docs_dd_uses_generic_product_csv() {
		let zip = make_zip(&[
			("docs/DD.csv", "code,drug_name\nDOC,Documentation\n"),
			(
				"products.csv",
				"drug_code,drug_name,atc_code\n000001-01-001,Methyldopa,C02AB\n",
			),
		]);

		let rows =
			parse_whodrug_upload(&zip).expect("generic product CSV should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "Methyldopa");
		assert_eq!(rows[0].atc_code.as_deref(), Some("C02AB"));
	}

	#[test]
	fn parse_whodrug_official_b3_zip_rejects_truncated_dd_rows() {
		let zip = make_zip(&[(
			"DD.csv",
			"000001,01,001,6,N,,001,,01,,854,METHYLDOPA\n000002,01,001\n",
		)]);

		let err =
			parse_whodrug_upload(&zip).expect_err("truncated B3 DD row should fail");

		assert_bad_input_contains(err, "B3 DD.csv row 2");
	}

	#[test]
	fn parse_whodrug_root_malformed_dd_fails_before_generic_fallback() {
		let zip = make_zip(&[
			("DD.csv", "000001,01,001\n"),
			(
				"products.csv",
				"drug_code,drug_name,atc_code\n000001-01-001,Methyldopa,C02AB\n",
			),
		]);

		let err = parse_whodrug_upload(&zip)
			.expect_err("malformed root DD.csv should fail as official B3");

		assert_bad_input_contains(err, "B3 DD.csv row 1");
	}

	#[test]
	fn parse_whodrug_official_b3_zip_rejects_truncated_dda_rows() {
		let zip = make_zip(&[
			("DD.csv", "000001,01,001,6,N,,001,,01,,854,METHYLDOPA\n"),
			("DDA.csv", "000001,01,001,6,C02AB,111,*\n000001,01\n"),
		]);

		let err = parse_whodrug_upload(&zip)
			.expect_err("truncated B3 DDA row should fail");

		assert_bad_input_contains(err, "B3 DDA.csv row 2");
	}

	#[test]
	fn parse_whodrug_official_c3_zip_rejects_truncated_mp_rows() {
		let zip = make_zip(&[(
			"MP.csv",
			"1,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\n1,,000002\n",
		)]);

		let err =
			parse_whodrug_upload(&zip).expect_err("truncated C3 MP row should fail");

		assert_bad_input_contains(err, "C3 MP.csv row 2");
	}

	#[test]
	fn parse_whodrug_root_malformed_mp_fails_before_generic_fallback() {
		let zip = make_zip(&[
			("MP.csv", "1,,000001\n"),
			(
				"products.csv",
				"drug_code,drug_name,atc_code\n000001-01-001,Methyldopa,C02AB\n",
			),
		]);

		let err = parse_whodrug_upload(&zip)
			.expect_err("malformed root MP.csv should fail as official C3");

		assert_bad_input_contains(err, "C3 MP.csv row 1");
	}

	#[test]
	fn parse_whodrug_unsupported_zip_ignores_metadata_and_fails_clearly() {
		let zip = make_zip(&[
			("Version.csv", "code,drug_name\n2025.09,Not a product row\n"),
			("docs/readme.csv", "code,drug_name\nDOC,Documentation\n"),
		]);

		let err =
			parse_whodrug_upload(&zip).expect_err("metadata-only zip should fail");

		match err {
			ImportError::BadInput(msg) => {
				assert!(
					msg.contains("supported WHODrug"),
					"unexpected error message: {msg}"
				);
			}
			other => panic!("unexpected error: {other}"),
		}
	}

	fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
		let mut cursor = Cursor::new(Vec::<u8>::new());
		{
			let mut zip = ZipWriter::new(&mut cursor);
			let options = SimpleFileOptions::default()
				.compression_method(CompressionMethod::Deflated);
			for (name, content) in entries {
				zip.start_file(name, options).unwrap();
				zip.write_all(content.as_bytes()).unwrap();
			}
			zip.finish().unwrap();
		}
		cursor.into_inner()
	}

	fn assert_bad_input_contains(err: ImportError, expected: &str) {
		match err {
			ImportError::BadInput(msg) => {
				assert!(
					msg.contains(expected),
					"expected error to contain {expected:?}, got {msg:?}"
				);
			}
			other => panic!("unexpected error: {other}"),
		}
	}
}
