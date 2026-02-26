use clap::{Args, Parser, Subcommand};
use lib_core::model::ModelManager;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, QueryBuilder};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Parser, Debug)]
#[command(name = "terminology-loader")]
#[command(about = "Load MedDRA and WHODrug dictionaries into SafetyDB")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	Meddra(LoadArgs),
	Whodrug(LoadArgs),
}

#[derive(Args, Debug, Clone)]
struct LoadArgs {
	#[arg(long)]
	input: PathBuf,
	#[arg(long)]
	version: String,
	#[arg(long, default_value = "en")]
	language: String,
	#[arg(long)]
	dry_run: bool,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();
	let mm = ModelManager::new().await?;

	match cli.command {
		Commands::Meddra(args) => load_meddra(&mm, &args).await?,
		Commands::Whodrug(args) => load_whodrug(&mm, &args).await?,
	}

	Ok(())
}

async fn load_meddra(
	mm: &ModelManager,
	args: &LoadArgs,
) -> Result<(), Box<dyn std::error::Error>> {
	let rows = parse_meddra(&args.input)?;
	println!(
		"Meddra parse complete: rows={}, version={}, language={}",
		rows.len(),
		args.version,
		args.language
	);

	if args.dry_run {
		println!("Dry run complete. No DB changes were made.");
		return Ok(());
	}

	let checksum = sha256_if_file(&args.input);
	let source_path = args.input.to_string_lossy().to_string();

	mm.dbx().begin_txn().await?;
	let run_result = async {
		upsert_release_header(
			mm,
			"meddra",
			&args.version,
			&args.language,
			"loading",
			&source_path,
			checksum.as_deref(),
			rows.len() as i64,
		)
		.await?;

		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE meddra_terms SET active = false WHERE language = $1 AND active = true",
				)
				.bind(&args.language),
			)
			.await?;

		upsert_meddra_rows(mm, &rows, &args.version, &args.language).await?;

		upsert_release_header(
			mm,
			"meddra",
			&args.version,
			&args.language,
			"active",
			&source_path,
			checksum.as_deref(),
			rows.len() as i64,
		)
		.await?;

		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE terminology_releases
					 SET activated_at = NOW(), updated_at = NOW()
					 WHERE dictionary = 'meddra' AND version = $1 AND language = $2",
				)
				.bind(&args.version)
				.bind(&args.language),
			)
			.await?;

		Ok::<(), Box<dyn std::error::Error>>(())
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await?;
			println!("Meddra load committed successfully.");
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn load_whodrug(
	mm: &ModelManager,
	args: &LoadArgs,
) -> Result<(), Box<dyn std::error::Error>> {
	let rows = parse_whodrug(&args.input)?;
	println!(
		"Whodrug parse complete: rows={}, version={}, language={}",
		rows.len(),
		args.version,
		args.language
	);

	if args.dry_run {
		println!("Dry run complete. No DB changes were made.");
		return Ok(());
	}

	let checksum = sha256_if_file(&args.input);
	let source_path = args.input.to_string_lossy().to_string();

	mm.dbx().begin_txn().await?;
	let run_result = async {
		upsert_release_header(
			mm,
			"whodrug",
			&args.version,
			&args.language,
			"loading",
			&source_path,
			checksum.as_deref(),
			rows.len() as i64,
		)
		.await?;

		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE whodrug_products SET active = false WHERE language = $1 AND active = true",
				)
				.bind(&args.language),
			)
			.await?;

		upsert_whodrug_rows(mm, &rows, &args.version, &args.language).await?;

		upsert_release_header(
			mm,
			"whodrug",
			&args.version,
			&args.language,
			"active",
			&source_path,
			checksum.as_deref(),
			rows.len() as i64,
		)
		.await?;

		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE terminology_releases
					 SET activated_at = NOW(), updated_at = NOW()
					 WHERE dictionary = 'whodrug' AND version = $1 AND language = $2",
				)
				.bind(&args.version)
				.bind(&args.language),
			)
			.await?;

		Ok::<(), Box<dyn std::error::Error>>(())
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await?;
			println!("Whodrug load committed successfully.");
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
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
) -> Result<(), Box<dyn std::error::Error>> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO terminology_releases
				 (dictionary, version, language, status, source_path, source_checksum, loaded_rows, created_at, updated_at)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
				 ON CONFLICT (dictionary, version, language)
				 DO UPDATE SET
				   status = EXCLUDED.status,
				   source_path = EXCLUDED.source_path,
				   source_checksum = EXCLUDED.source_checksum,
				   loaded_rows = EXCLUDED.loaded_rows,
				   updated_at = NOW()",
			)
			.bind(dictionary)
			.bind(version)
			.bind(language)
			.bind(status)
			.bind(source_path)
			.bind(checksum)
			.bind(loaded_rows),
		)
		.await?;
	Ok(())
}

async fn upsert_meddra_rows(
	mm: &ModelManager,
	rows: &[MeddraRow],
	version: &str,
	language: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
				.push_bind(true);
		});
		qb.push(
			" ON CONFLICT (code, version, language)
			  DO UPDATE SET
			    term = EXCLUDED.term,
			    level = EXCLUDED.level,
			    active = EXCLUDED.active",
		);
		mm.dbx().execute(qb.build()).await?;
	}
	Ok(())
}

async fn upsert_whodrug_rows(
	mm: &ModelManager,
	rows: &[WhodrugRow],
	version: &str,
	language: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
				.push_bind(true);
		});
		qb.push(
			" ON CONFLICT (code, version, language)
			  DO UPDATE SET
			    drug_name = EXCLUDED.drug_name,
			    atc_code = EXCLUDED.atc_code,
			    active = EXCLUDED.active",
		);
		mm.dbx().execute(qb.build()).await?;
	}
	Ok(())
}

fn parse_meddra(input: &Path) -> Result<Vec<MeddraRow>, Box<dyn std::error::Error>> {
	let llt = read_named_file(input, "llt.asc")?
		.ok_or_else(|| "Could not find llt.asc in input path".to_string())?;
	let mdhier = read_named_file(input, "mdhier.asc")?
		.ok_or_else(|| "Could not find mdhier.asc in input path".to_string())?;

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
		return Err("No MedDRA rows parsed from llt.asc/mdhier.asc".into());
	}

	Ok(rows)
}

fn parse_whodrug(
	input: &Path,
) -> Result<Vec<WhodrugRow>, Box<dyn std::error::Error>> {
	if input.is_file()
		&& input.extension().map(|e| e.eq_ignore_ascii_case("zip")) == Some(true)
	{
		let f = fs::File::open(input)?;
		let mut zip = ZipArchive::new(f)?;
		for idx in 0..zip.len() {
			let mut entry = zip.by_index(idx)?;
			if !entry.is_file() {
				continue;
			}
			let name = entry.name().to_ascii_lowercase();
			if !is_delimited_name(&name) {
				continue;
			}
			let mut bytes = Vec::new();
			entry.read_to_end(&mut bytes)?;
			if let Ok(rows) = parse_whodrug_delimited(&bytes) {
				if !rows.is_empty() {
					return Ok(rows);
				}
			}
		}
		return Err("No parseable delimited WHODrug file found in zip".into());
	}

	if input.is_dir() {
		let mut candidates = Vec::new();
		for entry in WalkDir::new(input).into_iter().flatten() {
			if !entry.file_type().is_file() {
				continue;
			}
			let path = entry.path();
			let name = path
				.file_name()
				.map(|n| n.to_string_lossy().to_ascii_lowercase())
				.unwrap_or_default();
			if is_delimited_name(&name) {
				candidates.push(path.to_path_buf());
			}
		}
		candidates.sort();
		for path in candidates {
			let bytes = fs::read(&path)?;
			if let Ok(rows) = parse_whodrug_delimited(&bytes) {
				if !rows.is_empty() {
					return Ok(rows);
				}
			}
		}
		return Err("No parseable delimited WHODrug file found in directory".into());
	}

	let bytes = fs::read(input)?;
	let rows = parse_whodrug_delimited(&bytes)?;
	if rows.is_empty() {
		return Err("No WHODrug rows parsed from input file".into());
	}
	Ok(rows)
}

fn parse_whodrug_delimited(
	bytes: &[u8],
) -> Result<Vec<WhodrugRow>, Box<dyn std::error::Error>> {
	let delim = detect_delimiter(bytes);
	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(true)
		.delimiter(delim)
		.from_reader(Cursor::new(bytes));

	let headers = rdr
		.headers()?
		.iter()
		.map(normalize_header)
		.collect::<Vec<_>>();

	let code_idx = find_header_idx(
		&headers,
		&["code", "drug_code", "record_id", "drugid", "drecno", "mpid"],
	)
	.ok_or_else(|| "Missing WHODrug code column".to_string())?;

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
	.ok_or_else(|| "Missing WHODrug product name column".to_string())?;

	let atc_idx = find_header_idx(&headers, &["atc", "atc_code", "atc1"]);

	let mut rows = Vec::new();
	let mut seen = HashSet::new();
	for rec in rdr.records() {
		let rec = rec?;
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

	Ok(rows)
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

fn read_named_file(
	input: &Path,
	target_name: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
	let target_name = target_name.to_ascii_lowercase();

	if input.is_file()
		&& input.extension().map(|e| e.eq_ignore_ascii_case("zip")) == Some(true)
	{
		let f = fs::File::open(input)?;
		let mut zip = ZipArchive::new(f)?;
		for i in 0..zip.len() {
			let mut file = zip.by_index(i)?;
			if !file.is_file() {
				continue;
			}
			let name = file.name().rsplit('/').next().unwrap_or("");
			if name.eq_ignore_ascii_case(&target_name) {
				let mut bytes = Vec::new();
				file.read_to_end(&mut bytes)?;
				return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()));
			}
		}
		return Ok(None);
	}

	if input.is_dir() {
		for entry in WalkDir::new(input).into_iter().flatten() {
			if !entry.file_type().is_file() {
				continue;
			}
			let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
			if name == target_name {
				let bytes = fs::read(entry.path())?;
				return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()));
			}
		}
	}

	Ok(None)
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

fn sha256_if_file(path: &Path) -> Option<String> {
	if !path.is_file() {
		return None;
	}
	let bytes = fs::read(path).ok()?;
	let mut hasher = Sha256::new();
	hasher.update(bytes);
	Some(hex::encode(hasher.finalize()))
}
