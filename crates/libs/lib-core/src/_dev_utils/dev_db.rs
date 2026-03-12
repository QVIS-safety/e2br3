use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;

type Db = Pool<Postgres>;

// NOTE: Hardcode to prevent deployed system db update.
const PG_DEV_POSTGRES_URL: &str = "postgres://postgres:welcome@localhost/postgres";
const PG_DEV_APP_ADMIN_URL: &str = "postgres://postgres:welcome@localhost/app_db";
const PG_DEV_APP_URL: &str = "postgres://app_user:dev_only_pwd@localhost/app_db";

// sql files
const SQL_RECREATE_DB_FILE_NAME: &str = "00-recreate-db.sql";
const SQL_DIR: &str = "docs/dev_initial";

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
	let sql_dir = base_dir.join(SQL_DIR);

	// -- Create the app_db/app_user with the postgres user.
	{
		let sql_recreate_db_file = sql_dir.join(SQL_RECREATE_DB_FILE_NAME);
		let root_db = new_db_pool(PG_DEV_POSTGRES_URL).await?;
		pexec(&root_db, &sql_recreate_db_file).await?;
	}

	// -- Get sql files.
	let mut paths: Vec<PathBuf> = fs::read_dir(sql_dir)?
		.filter_map(|entry| entry.ok().map(|e| e.path()))
		.collect();
	paths.sort();

	// -- SQL Execute each file.
	let app_db = new_db_pool(PG_DEV_APP_URL).await?;

	for path in paths {
		let path_str = path.to_string_lossy();

		if path_str.ends_with(".sql")
			&& !path_str.ends_with(SQL_RECREATE_DB_FILE_NAME)
		{
			pexec(&app_db, &path).await?;
		}
	}

	apply_compatibility_alters(&app_db).await?;

	// NOTE: Demo user data and passwords are set via SQL seed files (13-e2br3-seed.sql)

	Ok(())
}

pub async fn ensure_dev_schema_compatibility(
) -> Result<(), Box<dyn std::error::Error>> {
	let admin_app_db = match new_db_pool(PG_DEV_APP_ADMIN_URL).await {
		Ok(db) => db,
		Err(_) => new_db_pool(PG_DEV_APP_URL).await?,
	};
	apply_compatibility_alters(&admin_app_db).await?;
	Ok(())
}

async fn apply_compatibility_alters(
	db: &Db,
) -> Result<(), Box<dyn std::error::Error>> {
	sqlx::query("ALTER TABLE cases DROP CONSTRAINT IF EXISTS case_status_valid")
		.execute(db)
		.await?;
	sqlx::query(
		"UPDATE cases SET status = 'reviewed'
		 WHERE lower(status) = lower(chr(113)||chr(99)||chr(101)||chr(100))",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"UPDATE cases SET status = 'reviewed'
		 WHERE lower(status) = lower(chr(99)||chr(104)||chr(101)||chr(99)||chr(107)||chr(101)||chr(100))",
	)
	.execute(db)
	.await?;
	execute_ignoring_duplicate_constraint(
		db,
		"ALTER TABLE cases
		 ADD CONSTRAINT case_status_valid
		 CHECK (status IN ('draft', 'reviewed', 'validated', 'locked', 'submitted', 'archived', 'nullified'))",
	)
	.await?;
	sqlx::query(
		"ALTER TABLE cases
		 ADD COLUMN IF NOT EXISTS mfds_report_type VARCHAR(20)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE cases
		 ADD COLUMN IF NOT EXISTS report_year VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE cases
		 ADD COLUMN IF NOT EXISTS source_document_name TEXT",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE cases
		 ADD COLUMN IF NOT EXISTS source_document_base64 TEXT",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE cases
		 ADD COLUMN IF NOT EXISTS source_document_media_type TEXT",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_death_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_life_threatening_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_hospitalization_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_disabling_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_congenital_anomaly_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reactions
		 ADD COLUMN IF NOT EXISTS criteria_other_medically_important_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE patient_information
		 ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE safety_report_identification
		 ADD COLUMN IF NOT EXISTS transmission_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE safety_report_identification
		 ADD COLUMN IF NOT EXISTS date_first_received_from_source_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE safety_report_identification
		 ADD COLUMN IF NOT EXISTS date_of_most_recent_information_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_information
		 ADD COLUMN IF NOT EXISTS parent_birth_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_information
		 ADD COLUMN IF NOT EXISTS parent_age_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_information
		 ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE medical_history_episodes
		 ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE medical_history_episodes
		 ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE past_drug_history
		 ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE past_drug_history
		 ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE patient_death_information
		 ADD COLUMN IF NOT EXISTS date_of_death_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_medical_history
		 ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_medical_history
		 ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_past_drug_history
		 ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE parent_past_drug_history
		 ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE study_information
		 ADD COLUMN IF NOT EXISTS study_type_reaction_kr1 VARCHAR(1)
		 CHECK (study_type_reaction_kr1 IN ('1', '2', '3', '4'))",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE primary_sources
		 ADD COLUMN IF NOT EXISTS qualification_kr1 VARCHAR(1)
		 CHECK (qualification_kr1 IN ('1', '2'))",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE relatedness_assessments
		 ADD COLUMN IF NOT EXISTS result_of_assessment_kr2 VARCHAR(2000)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE reported_causes_of_death
		 ADD COLUMN IF NOT EXISTS comments TEXT",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE autopsy_causes_of_death
		 ADD COLUMN IF NOT EXISTS comments TEXT",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_reaction_assessments
		 ADD COLUMN IF NOT EXISTS administration_start_interval_value DECIMAL(10,2)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_reaction_assessments
		 ADD COLUMN IF NOT EXISTS administration_start_interval_unit VARCHAR(3)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_reaction_assessments
		 ADD COLUMN IF NOT EXISTS last_dose_interval_value DECIMAL(10,2)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_reaction_assessments
		 ADD COLUMN IF NOT EXISTS last_dose_interval_unit VARCHAR(3)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_information
		 ADD COLUMN IF NOT EXISTS cumulative_dose_first_reaction_value DECIMAL(15,5)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_information
		 ADD COLUMN IF NOT EXISTS cumulative_dose_first_reaction_unit VARCHAR(50)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_information
		 ADD COLUMN IF NOT EXISTS gestation_period_exposure_value DECIMAL(10,2)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_information
		 ADD COLUMN IF NOT EXISTS gestation_period_exposure_unit VARCHAR(50)",
	)
	.execute(db)
	.await?;
	sqlx::query(
		"ALTER TABLE dosage_information
		 ADD COLUMN IF NOT EXISTS route_termid_version VARCHAR(10)",
	)
	.execute(db)
	.await?;
	sqlx::query("ALTER TABLE users ENABLE ROW LEVEL SECURITY")
		.execute(db)
		.await?;
	sqlx::query("ALTER TABLE users FORCE ROW LEVEL SECURITY")
		.execute(db)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS users_org_isolation_select ON users")
		.execute(db)
		.await?;
	execute_ignoring_duplicate_policy(
		db,
		"CREATE POLICY users_org_isolation_select ON users
		 FOR SELECT
		 TO e2br3_app_role
		 USING (
		 	organization_id = current_organization_id()
		 	OR is_current_user_admin()
		 	OR email = current_setting('app.auth_email', true)
		 )",
	)
	.await?;
	sqlx::query("DROP POLICY IF EXISTS users_org_isolation_modify ON users")
		.execute(db)
		.await?;
	execute_ignoring_duplicate_policy(
		db,
		"CREATE POLICY users_org_isolation_modify ON users
		 FOR ALL
		 TO e2br3_app_role
		 USING (is_current_user_admin())
		 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	Ok(())
}

async fn execute_ignoring_duplicate_policy(
	db: &Db,
	sql: &str,
) -> Result<(), sqlx::Error> {
	match sqlx::query(sql).execute(db).await {
		Ok(_) => Ok(()),
		Err(err)
			if err
				.as_database_error()
				.and_then(|db_err| db_err.code())
				.map(|code| code == "42710")
				.unwrap_or(false) =>
		{
			Ok(())
		}
		Err(err) => Err(err),
	}
}

async fn execute_ignoring_duplicate_constraint(
	db: &Db,
	sql: &str,
) -> Result<(), sqlx::Error> {
	match sqlx::query(sql).execute(db).await {
		Ok(_) => Ok(()),
		Err(err)
			if err
				.as_database_error()
				.and_then(|db_err| db_err.code())
				.map(|code| code == "42710")
				.unwrap_or(false) =>
		{
			Ok(())
		}
		Err(err) => Err(err),
	}
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
