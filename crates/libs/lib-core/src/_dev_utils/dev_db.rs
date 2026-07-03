use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;

type Db = Pool<Postgres>;

// NOTE: Hardcode to prevent deployed system db update.
const PG_DEV_POSTGRES_URL: &str = "postgres://postgres:welcome@localhost/postgres";
const PG_DEV_APP_URL: &str = "postgres://app_user:dev_only_pwd@localhost/app_db";

// sql files
const SQL_RECREATE_DB_FILE_NAME: &str = "00-recreate-db.sql";
const DB_DIR: &str = "db";

pub async fn init_dev_db() -> Result<(), Box<dyn std::error::Error>> {
	info!("{:<12} - init_dev_db()", "FOR-DEV-ONLY");

	// -- Get the sql_dir
	// Note: This is because cargo test and cargo run won't give the same
	//       current_dir given the worspace layout.
	let current_dir = std::env::current_dir().unwrap();
	let v: Vec<_> = current_dir.components().collect();
	let path_comp = v.get(v.len().wrapping_sub(3));
	let base_dir = if Some(true) == path_comp.map(|c| c.as_os_str() == "crates") {
		v[..v.len() - 3].iter().collect::<PathBuf>()
	} else {
		current_dir.clone()
	};
	let db_dir = base_dir.join(DB_DIR);

	// -- Create the app_db/app_user with the postgres user.
	{
		let sql_recreate_db_file =
			db_dir.join("admin").join(SQL_RECREATE_DB_FILE_NAME);
		let root_db = new_db_pool(PG_DEV_POSTGRES_URL).await?;
		pexec(&root_db, &sql_recreate_db_file).await?;
	}

	// -- SQL Execute each file.
	let app_db = new_db_pool(PG_DEV_APP_URL).await?;

	for group in ["bootstrap", "migrations", "seed"] {
		let mut paths: Vec<PathBuf> = fs::read_dir(db_dir.join(group))?
			.filter_map(|entry| entry.ok().map(|e| e.path()))
			.collect();
		paths.sort();

		for path in paths {
			if path.extension().is_some_and(|ext| ext == "sql") {
				pexec(&app_db, &path).await?;
			}
		}
	}

	// NOTE: Demo user data and passwords are set via SQL seed files in db/seed/.

	Ok(())
}

async fn pexec(db: &Db, file: &Path) -> Result<(), sqlx::Error> {
	info!("{:<12} - pexec: {file:?}", "FOR-DEV-ONLY");

	// -- Read the file.
	let content = fs::read_to_string(file)?;

	// Split statements while respecting $$ and quoted strings.
	let sqls = split_sql(&content);

	for sql in sqls {
		if let Err(e) = sqlx::query(&sql).execute(db).await {
			if should_ignore_role_error(&sql, &e) {
				println!(
					"pexec warning: skipping role creation due to permission error:\n{sql}\nreason:\n{e}"
				);
				continue;
			}

			if should_ignore_policy_role_error(&sql, &e) {
				println!(
					"pexec warning: skipping policy creation due to missing role:\n{sql}\nreason:\n{e}"
				);
				continue;
			}

			if should_ignore_grant_role_error(&sql, &e) {
				println!(
					"pexec warning: skipping grant due to missing role:\n{sql}\nreason:\n{e}"
				);
				continue;
			}

			println!("pexec error while running:\n{sql}");
			println!("cause:\n{e}");
			return Err(e);
		}
	}

	Ok(())
}

async fn new_db_pool(db_con_url: &str) -> Result<Db, sqlx::Error> {
	PgPoolOptions::new()
		.max_connections(2)
		.acquire_timeout(Duration::from_secs(5))
		.connect(db_con_url)
		.await
}

fn split_sql(content: &str) -> Vec<String> {
	let mut statements = Vec::new();
	let mut buf = String::new();
	let mut in_dollar = false;
	let mut in_single = false;
	let mut in_line_comment = false;
	let mut in_block_comment = false;
	let mut chars = content.chars().peekable();

	while let Some(c) = chars.next() {
		let next = chars.peek().copied();

		if !in_dollar
			&& !in_single
			&& !in_block_comment
			&& c == '-'
			&& next == Some('-')
		{
			in_line_comment = true;
			buf.push(c);
			buf.push(chars.next().unwrap());
			continue;
		}

		if in_line_comment {
			if c == '\n' {
				in_line_comment = false;
			}
			buf.push(c);
			continue;
		}

		if !in_dollar
			&& !in_single
			&& !in_line_comment
			&& c == '/'
			&& next == Some('*')
		{
			in_block_comment = true;
			buf.push(c);
			buf.push(chars.next().unwrap());
			continue;
		}

		if in_block_comment {
			if c == '*' && next == Some('/') {
				in_block_comment = false;
				buf.push(c);
				buf.push(chars.next().unwrap());
				continue;
			}
			buf.push(c);
			continue;
		}

		if !in_dollar && c == '\'' {
			if chars.peek() == Some(&'\'') {
				// Escaped quote inside a string.
				buf.push(c);
				buf.push(chars.next().unwrap());
				continue;
			}
			in_single = !in_single;
			buf.push(c);
			continue;
		}

		if !in_single && c == '$' && chars.peek() == Some(&'$') {
			in_dollar = !in_dollar;
			buf.push(c);
			buf.push(chars.next().unwrap());
			continue;
		}

		if !in_dollar && !in_single && c == ';' {
			let stmt = buf.trim();
			if !stmt.is_empty() {
				statements.push(stmt.to_string());
			}
			buf.clear();
			continue;
		}

		buf.push(c);
	}

	if !buf.trim().is_empty() {
		statements.push(buf.trim().to_string());
	}

	statements
}

fn should_ignore_role_error(sql: &str, err: &sqlx::Error) -> bool {
	let has_create_role = sql.to_ascii_lowercase().contains("create role");
	if !has_create_role {
		return false;
	}

	match err {
		sqlx::Error::Database(db_err) => {
			matches!(db_err.code().as_deref(), Some("42501"))
		}
		_ => false,
	}
}

fn should_ignore_policy_role_error(sql: &str, err: &sqlx::Error) -> bool {
	let has_create_policy = sql.to_ascii_lowercase().contains("create policy");
	if !has_create_policy {
		return false;
	}

	match err {
		sqlx::Error::Database(db_err) => {
			matches!(db_err.code().as_deref(), Some("42704"))
		}
		_ => false,
	}
}

fn should_ignore_grant_role_error(sql: &str, err: &sqlx::Error) -> bool {
	let has_grant = sql.to_ascii_lowercase().contains("grant ");
	if !has_grant {
		return false;
	}

	match err {
		sqlx::Error::Database(db_err) => {
			matches!(db_err.code().as_deref(), Some("42704"))
		}
		_ => false,
	}
}
