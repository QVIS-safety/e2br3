use clap::{Args, Parser, Subcommand};
use lib_core::ctx::Ctx;
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::terminology_import::parse_whodrug_upload;
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
		set_loader_context(mm).await?;

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

		retire_other_active_releases(mm, "meddra", &args.version, &args.language)
			.await?;

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

	with_loader_txn(mm, || async {
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
		.await
	})
	.await?;

	for chunk in rows.chunks(1000) {
		with_whodrug_row_audit_disabled(mm, || async {
			upsert_whodrug_rows(mm, chunk, &args.version, &args.language, false)
				.await
		})
		.await?;
	}

	with_whodrug_row_audit_disabled(mm, || async {
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE whodrug_products SET active = false WHERE language = $1 AND active = true",
				)
				.bind(&args.language),
			)
			.await?;

		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE whodrug_products SET active = true WHERE version = $1 AND language = $2",
				)
				.bind(&args.version)
				.bind(&args.language),
			)
			.await?;

		retire_other_active_releases(mm, "whodrug", &args.version, &args.language)
			.await?;

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
	})
	.await?;

	println!("Whodrug load committed successfully.");
	Ok(())
}

async fn with_loader_txn<F, Fut>(
	mm: &ModelManager,
	run: F,
) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	mm.dbx().begin_txn().await?;
	let run_result = async {
		set_loader_context(mm).await?;
		run().await
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await?;
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn with_whodrug_row_audit_disabled<F, Fut>(
	mm: &ModelManager,
	run: F,
) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	mm.dbx().begin_txn().await?;
	let run_result = async {
		mm.dbx()
			.execute(sqlx::query(
				"ALTER TABLE whodrug_products DISABLE TRIGGER audit_whodrug_products",
			))
			.await?;
		set_loader_context(mm).await?;
		run().await?;
		mm.dbx().execute(sqlx::query("RESET ROLE")).await?;
		mm.dbx()
			.execute(sqlx::query(
				"ALTER TABLE whodrug_products ENABLE TRIGGER audit_whodrug_products",
			))
			.await?;
		Ok::<(), Box<dyn std::error::Error>>(())
	}
	.await;

	match run_result {
		Ok(_) => {
			mm.dbx().commit_txn().await?;
			Ok(())
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

async fn set_loader_context(
	mm: &ModelManager,
) -> Result<(), Box<dyn std::error::Error>> {
	let root_ctx = Ctx::root_ctx();
	set_full_context_dbx(
		mm.dbx(),
		root_ctx.user_id(),
		root_ctx.organization_id(),
		root_ctx.role(),
	)
	.await?;
	Ok(())
}

async fn retire_other_active_releases(
	mm: &ModelManager,
	dictionary: &str,
	version: &str,
	language: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE terminology_releases
				 SET status = 'retired', updated_at = NOW()
				 WHERE dictionary = $1
				   AND language = $2
				   AND version <> $3
				   AND status = 'active'",
			)
			.bind(dictionary)
			.bind(language)
			.bind(version),
		)
		.await?;
	Ok(())
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
	active: bool,
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
				.push_bind(active);
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

	let rows = dedup.into_iter().map(|(_, row)| row).collect::<Vec<_>>();

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
		let bytes = fs::read(input)?;
		return parse_whodrug_upload(&bytes)
			.map(|rows| {
				rows.into_iter()
					.map(|row| WhodrugRow {
						code: row.code,
						drug_name: row.drug_name,
						atc_code: row.atc_code,
					})
					.collect()
			})
			.map_err(|err| err.into());
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

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use std::sync::atomic::{AtomicU64, Ordering};
	use zip::write::SimpleFileOptions;
	use zip::{CompressionMethod, ZipWriter};

	static ZIP_COUNTER: AtomicU64 = AtomicU64::new(0);

	#[test]
	fn parse_meddra_directory_keeps_one_row_per_code_for_database_key() {
		let dir = std::env::temp_dir().join(format!(
			"terminology-loader-meddra-test-{}-{}",
			std::process::id(),
			ZIP_COUNTER.fetch_add(1, Ordering::Relaxed)
		));
		fs::create_dir_all(&dir).unwrap();
		fs::write(
			dir.join("llt.asc"),
			"10000001$LLT preferred duplicate$$$$$$$$$$$$$$$$$$$$\n",
		)
		.unwrap();
		fs::write(
			dir.join("mdhier.asc"),
			"10000001$20000001$30000001$40000001$PT duplicate$HLT term$HLGT term$SOC term$$$$\n",
		)
		.unwrap();

		let rows = parse_meddra(&dir).expect("MedDRA directory should parse");

		assert_eq!(rows.len(), 4);
		let duplicate_code_rows = rows
			.iter()
			.filter(|row| row.code == "10000001")
			.collect::<Vec<_>>();
		assert_eq!(duplicate_code_rows.len(), 1);
		assert_eq!(duplicate_code_rows[0].term, "LLT preferred duplicate");
		assert_eq!(duplicate_code_rows[0].level, "LLT");
		let _ = fs::remove_dir_all(dir);
	}

	#[tokio::test]
	async fn load_meddra_allows_sequential_versions_for_same_language() {
		std::env::var("SERVICE_DB_URL")
			.expect("SERVICE_DB_URL must be set for terminology loader DB test");
		std::env::set_var("SERVICE_WEB_FOLDER", "web-folder");
		let mm = ModelManager::new().await.expect("model manager");
		let tag = format!(
			"t{}{}",
			std::process::id(),
			ZIP_COUNTER.fetch_add(1, Ordering::Relaxed)
		);
		let language =
			format!("x{}", ZIP_COUNTER.fetch_add(1, Ordering::Relaxed) % 10);
		let version_v1 = format!("{}-v1", tag);
		let version_v2 = format!("{}-v2", tag);
		let zip_v1 = write_zip(&[
			("llt.asc", "10000001$First version term$$$$$$$$$$$$$$$$$$$$\n"),
			(
				"mdhier.asc",
				"10000002$20000001$30000001$40000001$First PT$First HLT$First HLGT$First SOC$$$$\n",
			),
		]);
		let zip_v2 = write_zip(&[
			("llt.asc", "10000001$Second version term$$$$$$$$$$$$$$$$$$$$\n"),
			(
				"mdhier.asc",
				"10000002$20000001$30000001$40000001$Second PT$Second HLT$Second HLGT$Second SOC$$$$\n",
			),
		]);

		load_meddra(
			&mm,
			&LoadArgs {
				input: zip_v1.clone(),
				version: version_v1.clone(),
				language: language.clone(),
				dry_run: false,
			},
		)
		.await
		.expect("first MedDRA load");
		load_meddra(
			&mm,
			&LoadArgs {
				input: zip_v2.clone(),
				version: version_v2.clone(),
				language: language.clone(),
				dry_run: false,
			},
		)
		.await
		.expect("second MedDRA load for same language");

		mm.dbx()
			.begin_txn()
			.await
			.expect("begin verification transaction");
		set_loader_context(&mm)
			.await
			.expect("set verification loader context");
		let counts: Vec<(String, bool, i64)> = mm
			.dbx()
			.fetch_all(
				sqlx::query_as(
					"SELECT version, active, COUNT(*)
				 FROM meddra_terms
				 WHERE language = $1 AND version IN ($2, $3)
				 GROUP BY version, active
				 ORDER BY version, active",
				)
				.bind(&language)
				.bind(&version_v1)
				.bind(&version_v2),
			)
			.await
			.expect("loaded row counts");
		let release_statuses: Vec<(String, String)> = mm
			.dbx()
			.fetch_all(
				sqlx::query_as(
					"SELECT version, status
				 FROM terminology_releases
				 WHERE dictionary = 'meddra' AND language = $1 AND version IN ($2, $3)
				 ORDER BY version",
				)
				.bind(&language)
				.bind(&version_v1)
				.bind(&version_v2),
			)
			.await
			.expect("release statuses");
		mm.dbx()
			.rollback_txn()
			.await
			.expect("rollback verification transaction");

		assert!(counts
			.iter()
			.any(|row| row == &(version_v1.clone(), false, 5)));
		assert!(counts
			.iter()
			.any(|row| row == &(version_v2.clone(), true, 5)));
		assert!(release_statuses
			.iter()
			.any(|row| row == &(version_v1.clone(), "retired".to_string())));
		assert!(release_statuses
			.iter()
			.any(|row| row == &(version_v2.clone(), "active".to_string())));

		let _ = fs::remove_file(zip_v1);
		let _ = fs::remove_file(zip_v2);
	}

	#[test]
	fn parse_whodrug_zip_supports_official_b3_rows() {
		let zip_path = write_zip(&[
			("DD.csv", "000001,01,001,6,N,,001,,01,,854,METHYLDOPA\n"),
			("DDA.csv", "000001,01,001,6,C02AB,111,*\n"),
		]);

		let rows = parse_whodrug(&zip_path).expect("official B3 zip should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "METHYLDOPA");
		assert_eq!(rows[0].atc_code.as_deref(), Some("C02AB"));
		let _ = fs::remove_file(zip_path);
	}

	#[test]
	fn parse_whodrug_zip_supports_official_c3_rows_without_atc_mapping() {
		let zip_path = write_zip(&[
			(
				"MP.csv",
				"1,,000001,01,001,0000000001,0000000001,Y,Methyldopa,,,,,N/A,,0,001,N/A,,001,19851231,20170907\n",
			),
			("ATC.csv", "C02AB,ANTIHYPERTENSIVES\n"),
		]);

		let rows = parse_whodrug(&zip_path).expect("official C3 zip should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "Methyldopa");
		assert_eq!(rows[0].atc_code, None);
		let _ = fs::remove_file(zip_path);
	}

	#[test]
	fn parse_whodrug_zip_with_docs_dd_uses_generic_product_csv() {
		let zip_path = write_zip(&[
			("docs/DD.csv", "code,drug_name\nDOC,Documentation\n"),
			(
				"products.csv",
				"drug_code,drug_name,atc_code\n000001-01-001,Methyldopa,C02AB\n",
			),
		]);

		let rows =
			parse_whodrug(&zip_path).expect("generic product CSV should parse");

		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].code, "000001-01-001");
		assert_eq!(rows[0].drug_name, "Methyldopa");
		assert_eq!(rows[0].atc_code.as_deref(), Some("C02AB"));
		let _ = fs::remove_file(zip_path);
	}

	fn write_zip(entries: &[(&str, &str)]) -> PathBuf {
		let path = std::env::temp_dir().join(format!(
			"terminology-loader-test-{}-{}.zip",
			std::process::id(),
			ZIP_COUNTER.fetch_add(1, Ordering::Relaxed)
		));
		let file = fs::File::create(&path).unwrap();
		let mut zip = ZipWriter::new(file);
		let options = SimpleFileOptions::default()
			.compression_method(CompressionMethod::Deflated);
		for (name, content) in entries {
			zip.start_file(name, options).unwrap();
			zip.write_all(content.as_bytes()).unwrap();
		}
		zip.finish().unwrap();
		path
	}
}
