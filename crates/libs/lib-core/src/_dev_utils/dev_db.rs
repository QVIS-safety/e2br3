use crate::ctx::{ROLE_SYSTEM_ADMIN, SYSTEM_ORG_ID, SYSTEM_USER_ID};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

type Db = Pool<Postgres>;

// NOTE: Hardcode to prevent deployed system db update.
const PG_DEV_POSTGRES_URL: &str = "postgres://postgres:welcome@localhost/postgres";
const PG_DEV_APP_ADMIN_URL: &str = "postgres://postgres:welcome@localhost/app_db";
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

	apply_compatibility_alters(&app_db).await?;

	// NOTE: Demo user data and passwords are set via SQL seed files in db/seed/.

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
	let mut tx = db.begin().await?;
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	sqlx::query("SELECT set_current_user_context($1)")
		.bind(system_user_id)
		.execute(&mut *tx)
		.await?;
	sqlx::query("SELECT set_org_context($1, $2)")
		.bind(system_org_id)
		.bind(ROLE_SYSTEM_ADMIN)
		.execute(&mut *tx)
		.await?;

	sqlx::query("ALTER TABLE cases DROP CONSTRAINT IF EXISTS case_status_valid")
		.execute(&mut *tx)
		.await?;
	sqlx::query(
		"UPDATE cases SET status = 'reviewed'
		 WHERE lower(status) = lower(chr(113)||chr(99)||chr(101)||chr(100))",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"UPDATE cases SET status = 'reviewed'
		 WHERE lower(status) = lower(chr(99)||chr(104)||chr(101)||chr(99)||chr(107)||chr(101)||chr(100))",
	)
	.execute(&mut *tx)
	.await?;
	execute_ignoring_duplicate_constraint(
		&mut tx,
		"ALTER TABLE cases
		 ADD CONSTRAINT case_status_valid
		 CHECK (status IN ('draft', 'reviewed', 'validated', 'locked', 'submitted', 'deleted', 'archived', 'nullified'))",
	)
	.await?;
	sqlx::query(
		"ALTER TABLE drug_information DROP CONSTRAINT IF EXISTS drug_information_drug_characterization_check",
	)
	.execute(&mut *tx)
	.await?;
	execute_ignoring_duplicate_constraint(
		&mut tx,
		"ALTER TABLE drug_information
		 ADD CONSTRAINT drug_information_drug_characterization_check
		 CHECK (drug_characterization IN ('1', '2', '3', '4'))",
	)
	.await?;
	for sql in [
		"DROP INDEX IF EXISTS idx_sender_presaves_authority",
		"DROP INDEX IF EXISTS idx_receiver_presaves_authority",
		"DROP INDEX IF EXISTS idx_product_presaves_authority",
		"DROP INDEX IF EXISTS idx_reporter_presaves_authority",
		"DROP INDEX IF EXISTS idx_study_presaves_authority",
		"DROP INDEX IF EXISTS idx_narrative_presaves_authority",
		"ALTER TABLE sender_presaves DROP CONSTRAINT IF EXISTS sender_presaves_authority_valid",
		"ALTER TABLE sender_presaves DROP COLUMN IF EXISTS authority",
		"ALTER TABLE sender_presaves DROP COLUMN IF EXISTS name",
		"ALTER TABLE sender_presaves DROP COLUMN IF EXISTS comments",
		"ALTER TABLE receiver_presaves DROP CONSTRAINT IF EXISTS receiver_presaves_authority_valid",
		"ALTER TABLE receiver_presaves DROP COLUMN IF EXISTS authority",
		"ALTER TABLE receiver_presaves DROP COLUMN IF EXISTS name",
		"ALTER TABLE receiver_presaves DROP COLUMN IF EXISTS comments",
		"ALTER TABLE product_presaves DROP CONSTRAINT IF EXISTS product_presaves_authority_valid",
		"ALTER TABLE product_presaves DROP COLUMN IF EXISTS authority",
		"ALTER TABLE product_presaves DROP COLUMN IF EXISTS name",
		"ALTER TABLE product_presaves DROP COLUMN IF EXISTS comments",
		"ALTER TABLE product_presaves ADD COLUMN IF NOT EXISTS original_manufacturer VARCHAR(500)",
		"ALTER TABLE product_presaves DROP COLUMN IF EXISTS drug_characterization",
		"ALTER TABLE product_presaves DROP COLUMN IF EXISTS drug_generic_name",
			"ALTER TABLE product_presaves DROP COLUMN IF EXISTS manufacturer_name",
			"ALTER TABLE product_presaves DROP COLUMN IF EXISTS fda_ind_number_occurred",
			"ALTER TABLE product_presaves DROP COLUMN IF EXISTS fda_pre_anda_number_occurred",
			"DROP TABLE IF EXISTS product_presave_fda_cross_reported_inds CASCADE",
			"DROP TABLE IF EXISTS product_presave_mfds_regional_items CASCADE",
			"ALTER TABLE sender_presave_responsible_persons ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
			r#"DO $$
DECLARE
	has_sender_department BOOLEAN;
	has_sender_person_given_name BOOLEAN;
BEGIN
	SELECT EXISTS (
		SELECT 1
		FROM information_schema.columns
		WHERE table_name = 'sender_presaves'
		  AND column_name = 'department'
	) INTO has_sender_department;

	SELECT EXISTS (
		SELECT 1
		FROM information_schema.columns
		WHERE table_name = 'sender_presaves'
		  AND column_name = 'person_given_name'
	) INTO has_sender_person_given_name;

	IF has_sender_department OR has_sender_person_given_name THEN
		EXECUTE format($sql$
			INSERT INTO sender_presave_responsible_persons (
				sender_presave_id,
				sequence_number,
				department,
				person_given_name,
				is_default,
				deleted,
				created_by,
				updated_by
			)
			SELECT
				p.id,
				COALESCE((
					SELECT MAX(r.sequence_number) + 1
					FROM sender_presave_responsible_persons r
					WHERE r.sender_presave_id = p.id
				), 1),
				%s,
				%s,
				NOT EXISTS (
					SELECT 1
					FROM sender_presave_responsible_persons r
					WHERE r.sender_presave_id = p.id
					  AND r.is_default
					  AND NOT r.deleted
				),
				false,
				p.created_by,
				COALESCE(p.updated_by, p.created_by)
			FROM sender_presaves p
			WHERE NOT EXISTS (
				SELECT 1
				FROM sender_presave_responsible_persons r
				WHERE r.sender_presave_id = p.id
				  AND NOT r.deleted
				  AND (r.department IS NOT NULL OR r.person_given_name IS NOT NULL)
			)
			  AND (%s)
		$sql$,
			CASE WHEN has_sender_department THEN 'p.department' ELSE 'NULL::VARCHAR(500)' END,
			CASE WHEN has_sender_person_given_name THEN 'p.person_given_name' ELSE 'NULL::VARCHAR(200)' END,
			concat_ws(
				' OR ',
				CASE WHEN has_sender_department THEN 'p.department IS NOT NULL' END,
				CASE WHEN has_sender_person_given_name THEN 'p.person_given_name IS NOT NULL' END
			)
		);
	END IF;
END $$"#,
			"ALTER TABLE sender_presaves DROP COLUMN IF EXISTS person_given_name",
				"ALTER TABLE sender_presaves DROP COLUMN IF EXISTS department",
				"ALTER TABLE sender_presaves ADD COLUMN IF NOT EXISTS organization_name_notation VARCHAR(50)",
			"ALTER TABLE sender_presave_gateways ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
			"ALTER TABLE reporter_presaves DROP CONSTRAINT IF EXISTS reporter_presaves_authority_valid",
		"ALTER TABLE reporter_presaves DROP COLUMN IF EXISTS authority",
		"ALTER TABLE reporter_presaves DROP COLUMN IF EXISTS name",
		"ALTER TABLE reporter_presaves DROP COLUMN IF EXISTS comments",
		"ALTER TABLE reporter_presaves DROP COLUMN IF EXISTS email",
		"ALTER TABLE reporter_presaves ADD COLUMN IF NOT EXISTS qualification_kr1 VARCHAR(50)",
		"ALTER TABLE reporter_presaves ADD COLUMN IF NOT EXISTS reporter_name_null_flavor VARCHAR(4)",
		"ALTER TABLE reporter_presaves ADD COLUMN IF NOT EXISTS reporter_address_null_flavor VARCHAR(4)",
		"ALTER TABLE reporter_presaves ADD COLUMN IF NOT EXISTS qualification_null_flavor VARCHAR(4)",
		"ALTER TABLE study_presaves DROP CONSTRAINT IF EXISTS study_presaves_authority_valid",
		"ALTER TABLE study_presaves DROP COLUMN IF EXISTS authority",
			"ALTER TABLE study_presaves DROP COLUMN IF EXISTS name",
			"ALTER TABLE study_presaves DROP COLUMN IF EXISTS comments",
			"ALTER TABLE study_presaves ADD COLUMN IF NOT EXISTS exclude_case_key_from_sync BOOLEAN",
			"ALTER TABLE study_presaves ALTER COLUMN sponsor_study_number TYPE VARCHAR(50) USING LEFT(sponsor_study_number, 50)",
			"ALTER TABLE study_presaves DROP CONSTRAINT IF EXISTS study_presaves_sponsor_study_number_kind_valid",
			"UPDATE study_presaves
			 SET sponsor_study_number_kind = NULL
			 WHERE sponsor_study_number_kind IS NOT NULL
			   AND sponsor_study_number_kind NOT IN ('STUDY_NO', 'PROTOCOL_NO')",
			"ALTER TABLE study_presaves
			 ADD CONSTRAINT study_presaves_sponsor_study_number_kind_valid CHECK (
			 	sponsor_study_number_kind IS NULL
			 	OR sponsor_study_number_kind IN ('STUDY_NO', 'PROTOCOL_NO')
			 )",
			"ALTER TABLE study_presave_registration_numbers ALTER COLUMN registration_number TYPE VARCHAR(50) USING LEFT(registration_number, 50)",
		"CREATE TABLE IF NOT EXISTS study_presave_products (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			study_presave_id UUID NOT NULL REFERENCES study_presaves(id) ON DELETE CASCADE,
			sequence_number INTEGER NOT NULL,
			product_presave_id UUID REFERENCES product_presaves(id) ON DELETE SET NULL,
			product_name VARCHAR(255),
			deleted BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
			updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,
			CONSTRAINT study_presave_products_sequence_unique UNIQUE (study_presave_id, sequence_number)
		)",
		"CREATE TABLE IF NOT EXISTS study_presave_reporters (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			study_presave_id UUID NOT NULL REFERENCES study_presaves(id) ON DELETE CASCADE,
			sequence_number INTEGER NOT NULL,
			reporter_presave_id UUID REFERENCES reporter_presaves(id) ON DELETE SET NULL,
			reporter_organization VARCHAR(255),
			reporter_given_name VARCHAR(100),
			reporter_qualification VARCHAR(50),
			deleted BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
			updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,
			CONSTRAINT study_presave_reporters_sequence_unique UNIQUE (study_presave_id, sequence_number)
		)",
		"CREATE INDEX IF NOT EXISTS idx_study_presave_products_parent ON study_presave_products(study_presave_id)",
		"CREATE INDEX IF NOT EXISTS idx_study_presave_reporters_parent ON study_presave_reporters(study_presave_id)",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON study_presave_products TO e2br3_app_role",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON study_presave_reporters TO e2br3_app_role",
		"ALTER TABLE study_presave_products ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE study_presave_products FORCE ROW LEVEL SECURITY",
		"DROP POLICY IF EXISTS study_presave_products_via_parent ON study_presave_products",
		"CREATE POLICY study_presave_products_via_parent ON study_presave_products
			FOR ALL TO e2br3_app_role
			USING (
				EXISTS (
					SELECT 1 FROM study_presaves p
					WHERE p.id = study_presave_products.study_presave_id
					AND (p.organization_id = current_organization_id() OR is_current_user_admin())
				)
			)
			WITH CHECK (
				EXISTS (
					SELECT 1 FROM study_presaves p
					WHERE p.id = study_presave_products.study_presave_id
					AND (p.organization_id = current_organization_id() OR is_current_user_admin())
				)
			)",
		"ALTER TABLE study_presave_reporters ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE study_presave_reporters FORCE ROW LEVEL SECURITY",
		"DROP POLICY IF EXISTS study_presave_reporters_via_parent ON study_presave_reporters",
		"CREATE POLICY study_presave_reporters_via_parent ON study_presave_reporters
			FOR ALL TO e2br3_app_role
			USING (
				EXISTS (
					SELECT 1 FROM study_presaves p
					WHERE p.id = study_presave_reporters.study_presave_id
					AND (p.organization_id = current_organization_id() OR is_current_user_admin())
				)
			)
			WITH CHECK (
				EXISTS (
					SELECT 1 FROM study_presaves p
					WHERE p.id = study_presave_reporters.study_presave_id
					AND (p.organization_id = current_organization_id() OR is_current_user_admin())
				)
			)",
		"DROP TRIGGER IF EXISTS audit_study_presave_products ON study_presave_products",
		"CREATE TRIGGER audit_study_presave_products AFTER INSERT OR UPDATE OR DELETE ON study_presave_products
			FOR EACH ROW EXECUTE FUNCTION audit_trigger_function()",
		"DROP TRIGGER IF EXISTS audit_study_presave_reporters ON study_presave_reporters",
		"CREATE TRIGGER audit_study_presave_reporters AFTER INSERT OR UPDATE OR DELETE ON study_presave_reporters
			FOR EACH ROW EXECUTE FUNCTION audit_trigger_function()",
		"DROP TRIGGER IF EXISTS update_study_presave_products_updated_at ON study_presave_products",
		"CREATE TRIGGER update_study_presave_products_updated_at BEFORE UPDATE ON study_presave_products
			FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()",
		"DROP TRIGGER IF EXISTS update_study_presave_reporters_updated_at ON study_presave_reporters",
		"CREATE TRIGGER update_study_presave_reporters_updated_at BEFORE UPDATE ON study_presave_reporters
			FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()",
		"ALTER TABLE narrative_presaves DROP CONSTRAINT IF EXISTS narrative_presaves_authority_valid",
		"ALTER TABLE narrative_presaves DROP COLUMN IF EXISTS authority",
		"ALTER TABLE narrative_presaves DROP COLUMN IF EXISTS name",
		"ALTER TABLE narrative_presaves DROP COLUMN IF EXISTS comments",
		"ALTER TABLE narrative_presaves DROP COLUMN IF EXISTS reporter_comments",
		"ALTER TABLE narrative_presaves DROP COLUMN IF EXISTS sender_comments",
		"ALTER TABLE narrative_presaves ADD COLUMN IF NOT EXISTS additional_information TEXT",
		"DROP TABLE IF EXISTS narrative_presave_sender_diagnoses",
		"DROP TABLE IF EXISTS narrative_presave_case_summaries",
		"DELETE FROM sender_presave_gateways WHERE lower(gateway_authority) NOT IN ('fda', 'mfds')",
		"ALTER TABLE sender_presave_gateways DROP CONSTRAINT IF EXISTS sender_presave_gateways_authority_valid",
		"ALTER TABLE sender_presave_gateways DROP COLUMN IF EXISTS ema_sender_identifier",
		"ALTER TABLE sender_presave_gateways ADD CONSTRAINT sender_presave_gateways_authority_valid CHECK (gateway_authority IN ('fda', 'mfds'))",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS mfds_report_type VARCHAR(20)",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS fda_report_type VARCHAR(20)",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS report_year VARCHAR(4)",
		"DROP TABLE IF EXISTS receiver_presave_routes CASCADE",
		"CREATE TABLE IF NOT EXISTS submission_receiver_options (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
			authority VARCHAR(20) NOT NULL,
			sequence_number INTEGER NOT NULL,
			receiver_label VARCHAR(120) NOT NULL,
			condition_page VARCHAR(20) NOT NULL DEFAULT 'CI',
			condition_field_code VARCHAR(80) NOT NULL,
			condition_operator VARCHAR(20) NOT NULL DEFAULT 'EQ',
			condition_value_code VARCHAR(40) NOT NULL,
			condition_value_label VARCHAR(120),
			batch_receiver_identifier VARCHAR(60) NOT NULL,
			message_receiver_identifier VARCHAR(60) NOT NULL,
			deleted BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			created_by UUID REFERENCES users(id),
			updated_by UUID REFERENCES users(id),
			CONSTRAINT submission_receiver_options_authority_valid CHECK (authority IN ('fda', 'mfds')),
			CONSTRAINT submission_receiver_options_condition_page_valid CHECK (condition_page = 'CI'),
			CONSTRAINT submission_receiver_options_operator_valid CHECK (condition_operator = 'EQ')
		)",
		"CREATE INDEX IF NOT EXISTS idx_submission_receiver_options_org_authority
			ON submission_receiver_options(organization_id, authority, sequence_number)
			WHERE deleted = false",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_submission_receiver_options_label_unique
			ON submission_receiver_options(organization_id, authority, receiver_label)
			WHERE deleted = false",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_submission_receiver_options_condition_unique
			ON submission_receiver_options(
				organization_id,
				authority,
				condition_field_code,
				condition_value_code,
				message_receiver_identifier
			)
			WHERE deleted = false",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON submission_receiver_options TO e2br3_app_role",
		"ALTER TABLE submission_receiver_options ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE submission_receiver_options FORCE ROW LEVEL SECURITY",
		"DROP POLICY IF EXISTS submission_receiver_options_org_isolation ON submission_receiver_options",
		"CREATE POLICY submission_receiver_options_org_isolation ON submission_receiver_options
			FOR ALL TO e2br3_app_role
			USING (
				organization_id = current_organization_id() OR is_current_user_admin()
			)
			WITH CHECK (
				organization_id = current_organization_id() OR is_current_user_admin()
			)",
		"INSERT INTO submission_receiver_options (
			organization_id,
			authority,
			sequence_number,
			receiver_label,
			condition_field_code,
			condition_value_code,
			condition_value_label,
			batch_receiver_identifier,
			message_receiver_identifier
		)
		SELECT o.id, v.authority, v.sequence_number, v.receiver_label, v.condition_field_code,
			   v.condition_value_code, v.condition_value_label, v.batch_receiver_identifier,
			   v.message_receiver_identifier
		FROM organizations o
		CROSS JOIN (VALUES
			('fda', 1, 'FDA(CDER IND)', 'FDA_REPORT_TYPE', '1', 'CDER IND', 'ZZFDA_PREMKT', 'CDER_IND'),
			('fda', 2, 'FDA(CDER IND-exempt BA/BE)', 'FDA_REPORT_TYPE', '2', 'CDER IND-exempt BA/BE', 'ZZFDA_PREMKT', 'CDER_IND_EXEMPT_BA_BE'),
			('fda', 3, 'FDA(CBER IND)', 'FDA_REPORT_TYPE', '3', 'CBER IND', 'ZZFDA_PREMKT', 'CBER_IND'),
			('fda', 4, 'FDA(Postmarket)', 'FDA_REPORT_TYPE', '4', 'Postmarket', 'ZZFDA', 'CDER'),
			('mfds', 1, 'MFDS(CT)', 'MFDS_REPORT_TYPE', '1', '임상시험계획의 승인을 받은 자', 'MFDS_CT', 'CT'),
			('mfds', 2, 'MFDS(CU)', 'MFDS_REPORT_TYPE', '2', '임상시험용의약품의 치료목적 사용승인을 받은 자', 'MFDS_CU', 'CU'),
			('mfds', 3, 'MFDS(KR)', 'MFDS_REPORT_TYPE', '3', '시판 후 이상사례 국내보고', 'MFDS', 'KR'),
			('mfds', 4, 'MFDS(FR)', 'MFDS_REPORT_TYPE', '4', '시판 후 이상사례 국외보고', 'MFDS_FR', 'FR'),
			('mfds', 5, 'MFDS(CF)', 'MFDS_REPORT_TYPE', '5', '임상시험계획의 승인을 받은 자 (국외)', 'MFDS_CF', 'CF')
		) AS v(authority, sequence_number, receiver_label, condition_field_code, condition_value_code, condition_value_label, batch_receiver_identifier, message_receiver_identifier)
		ON CONFLICT DO NOTHING",
		"UPDATE submission_receiver_options
			SET condition_value_label = CASE receiver_label
					WHEN 'MFDS(CT)' THEN '임상시험계획의 승인을 받은 자'
					WHEN 'MFDS(CU)' THEN '임상시험용의약품의 치료목적 사용승인을 받은 자'
					WHEN 'MFDS(KR)' THEN '시판 후 이상사례 국내보고'
					WHEN 'MFDS(FR)' THEN '시판 후 이상사례 국외보고'
					ELSE condition_value_label
				END
			WHERE authority = 'mfds'
			  AND condition_field_code = 'MFDS_REPORT_TYPE'
			  AND (
				(receiver_label = 'MFDS(CT)' AND condition_value_code = '1' AND condition_value_label = 'CT')
				OR (receiver_label = 'MFDS(CU)' AND condition_value_code = '2' AND condition_value_label = 'CU')
				OR (receiver_label = 'MFDS(KR)' AND condition_value_code = '3' AND condition_value_label = 'KR')
				OR (receiver_label = 'MFDS(FR)' AND condition_value_code = '4' AND condition_value_label = 'FR')
			  )",
		"DROP TRIGGER IF EXISTS audit_submission_receiver_options ON submission_receiver_options",
		"CREATE TRIGGER audit_submission_receiver_options AFTER INSERT OR UPDATE OR DELETE ON submission_receiver_options
			FOR EACH ROW EXECUTE FUNCTION audit_trigger_function()",
		"DROP TRIGGER IF EXISTS update_submission_receiver_options_updated_at ON submission_receiver_options",
		"CREATE TRIGGER update_submission_receiver_options_updated_at BEFORE UPDATE ON submission_receiver_options
			FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS review_receivers_json TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_routes_json TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_status TEXT NOT NULL DEFAULT 'Saved'",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_assigned_role TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_assigned_user_id UUID REFERENCES users(id) ON DELETE SET NULL",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_due_at TIMESTAMPTZ",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_description TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS workflow_updated_at TIMESTAMPTZ NOT NULL DEFAULT now()",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS source_document_name TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS source_document_base64 TEXT",
		"ALTER TABLE cases ADD COLUMN IF NOT EXISTS source_document_media_type TEXT",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_death_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_life_threatening_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_hospitalization_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_disabling_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_congenital_anomaly_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS criteria_other_medically_important_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS included_in_ema_ime_list BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS expectedness VARCHAR(1)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS severity VARCHAR(20)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS required_intervention_null_flavor VARCHAR(4)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_ae_classification VARCHAR(1)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_ae_outcome VARCHAR(2)",
		"ALTER TABLE reactions DROP CONSTRAINT IF EXISTS reactions_expectedness_check",
		"ALTER TABLE reactions ADD CONSTRAINT reactions_expectedness_check CHECK (expectedness IS NULL OR expectedness IN ('1', '2'))",
		"ALTER TABLE reactions DROP CONSTRAINT IF EXISTS reactions_mfds_device_ae_classification_check",
		"ALTER TABLE reactions ADD CONSTRAINT reactions_mfds_device_ae_classification_check CHECK (mfds_device_ae_classification IS NULL OR mfds_device_ae_classification IN ('0', '1'))",
		"ALTER TABLE reactions DROP CONSTRAINT IF EXISTS reactions_mfds_device_ae_outcome_check",
		"ALTER TABLE reactions ADD CONSTRAINT reactions_mfds_device_ae_outcome_check CHECK (mfds_device_ae_outcome IS NULL OR mfds_device_ae_outcome IN ('3', '4', '5', '8', '9', '10', '11', '12'))",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_cause_medical_device BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_cause_procedure_issue BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_cause_patient_condition BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_cause_unable_to_assess BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_cause_other VARCHAR(20000)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_reason VARCHAR(20000)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_recall BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_repair BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_inspection BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_replacement BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_improvement BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_monitoring BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_notification BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_label_change BOOLEAN",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS mfds_device_action_other VARCHAR(20000)",
		"ALTER TABLE reactions ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE reactions DROP CONSTRAINT IF EXISTS unique_reaction_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_reactions_active_sequence_unique ON reactions(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE test_results ALTER COLUMN normal_low_value TYPE VARCHAR(50)",
		"ALTER TABLE test_results ALTER COLUMN normal_high_value TYPE VARCHAR(50)",
		"ALTER TABLE test_results ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE test_results DROP CONSTRAINT IF EXISTS unique_test_result_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_test_results_active_sequence_unique ON test_results(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE past_drug_history ALTER COLUMN mpid TYPE VARCHAR(200)",
		"ALTER TABLE past_drug_history ALTER COLUMN phpid TYPE VARCHAR(200)",
		"ALTER TABLE past_drug_history ADD COLUMN IF NOT EXISTS mfds_medicinal_product_version VARCHAR(20)",
		"ALTER TABLE past_drug_history ADD COLUMN IF NOT EXISTS mfds_medicinal_product_id VARCHAR(10)",
		"ALTER TABLE patient_information ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS transmission_date_null_flavor VARCHAR(4)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS date_first_received_from_source_null_flavor VARCHAR(4)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS date_of_most_recent_information_null_flavor VARCHAR(4)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS safety_report_id VARCHAR(100)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS version INTEGER",
		"UPDATE safety_report_identification SET version = 1 WHERE version IS NULL",
		"ALTER TABLE safety_report_identification ALTER COLUMN version SET DEFAULT 1",
		"ALTER TABLE safety_report_identification ALTER COLUMN version SET NOT NULL",
		"ALTER TABLE safety_report_identification ALTER COLUMN safety_report_id SET NOT NULL",
		"CREATE INDEX IF NOT EXISTS idx_safety_report_identification_report_id
		 ON safety_report_identification(safety_report_id)",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_safety_report_identification_report_version
		 ON safety_report_identification(safety_report_id, version)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS local_criteria_report_type VARCHAR(10)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS combination_product_report_indicator VARCHAR(10)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS worldwide_unique_id VARCHAR(100)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS first_sender_type VARCHAR(1)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS additional_documents_available BOOLEAN",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS other_case_identifiers_exist BOOLEAN",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS nullification_code VARCHAR(10)",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS nullification_reason TEXT",
		"ALTER TABLE safety_report_identification ADD COLUMN IF NOT EXISTS receiver_organization VARCHAR(200)",
		"ALTER TABLE safety_report_identification ALTER COLUMN report_type DROP NOT NULL",
		"ALTER TABLE safety_report_identification DROP CONSTRAINT IF EXISTS safety_report_identification_report_type_check",
		"ALTER TABLE safety_report_identification ADD CONSTRAINT safety_report_identification_report_type_check CHECK (report_type IS NULL OR report_type IN ('1', '2', '3', '4'))",
		"ALTER TABLE safety_report_identification ALTER COLUMN fulfil_expedited_criteria DROP NOT NULL",
		"ALTER TABLE sender_information ALTER COLUMN sender_type DROP NOT NULL",
		"ALTER TABLE sender_information ALTER COLUMN organization_name DROP NOT NULL",
		"ALTER TABLE sender_information ADD COLUMN IF NOT EXISTS health_professional_type_kr1 VARCHAR(20)",
		"ALTER TABLE parent_information ADD COLUMN IF NOT EXISTS parent_birth_date_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_information ADD COLUMN IF NOT EXISTS parent_age_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_information ADD COLUMN IF NOT EXISTS last_menstrual_period_date_null_flavor VARCHAR(4)",
		"ALTER TABLE medical_history_episodes ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
		"ALTER TABLE medical_history_episodes ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
		"ALTER TABLE past_drug_history ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
		"ALTER TABLE past_drug_history ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
		"ALTER TABLE patient_death_information ADD COLUMN IF NOT EXISTS date_of_death_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_medical_history ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_medical_history ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_past_drug_history ADD COLUMN IF NOT EXISTS start_date_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_past_drug_history ADD COLUMN IF NOT EXISTS end_date_null_flavor VARCHAR(4)",
		"ALTER TABLE parent_past_drug_history ADD COLUMN IF NOT EXISTS mfds_medicinal_product_version VARCHAR(20)",
		"ALTER TABLE parent_past_drug_history ADD COLUMN IF NOT EXISTS mfds_medicinal_product_id VARCHAR(10)",
		"ALTER TABLE patient_identifiers ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE patient_identifiers DROP CONSTRAINT IF EXISTS unique_patient_identifier_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_patient_identifiers_active_sequence_unique ON patient_identifiers(patient_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE medical_history_episodes ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE medical_history_episodes DROP CONSTRAINT IF EXISTS unique_med_history_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_medical_history_episodes_active_sequence_unique ON medical_history_episodes(patient_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE past_drug_history ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE past_drug_history DROP CONSTRAINT IF EXISTS unique_past_drug_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_past_drug_history_active_sequence_unique ON past_drug_history(patient_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE reported_causes_of_death ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE reported_causes_of_death DROP CONSTRAINT IF EXISTS unique_reported_death_cause",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_reported_causes_of_death_active_sequence_unique ON reported_causes_of_death(death_info_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE autopsy_causes_of_death ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE autopsy_causes_of_death DROP CONSTRAINT IF EXISTS unique_autopsy_death_cause",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_autopsy_causes_of_death_active_sequence_unique ON autopsy_causes_of_death(death_info_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE parent_information ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE parent_information DROP CONSTRAINT IF EXISTS unique_parent_per_patient",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_parent_information_active_patient_unique ON parent_information(patient_id) WHERE deleted = false",
		"ALTER TABLE parent_medical_history ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE parent_medical_history DROP CONSTRAINT IF EXISTS unique_parent_med_history_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_parent_medical_history_active_sequence_unique ON parent_medical_history(parent_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE parent_past_drug_history ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE parent_past_drug_history DROP CONSTRAINT IF EXISTS unique_parent_past_drug_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_parent_past_drug_history_active_sequence_unique ON parent_past_drug_history(parent_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE study_information ADD COLUMN IF NOT EXISTS study_type_reaction_kr1 VARCHAR(1) CHECK (study_type_reaction_kr1 IN ('1', '2', '3', '4'))",
		"ALTER TABLE study_information ADD COLUMN IF NOT EXISTS fda_ind_number_occurred VARCHAR(10)",
		"ALTER TABLE study_information ADD COLUMN IF NOT EXISTS fda_pre_anda_number_occurred VARCHAR(10)",
		"ALTER TABLE study_information ADD COLUMN IF NOT EXISTS source_study_presave_id UUID REFERENCES study_presaves(id) ON DELETE SET NULL",
		"CREATE INDEX IF NOT EXISTS idx_study_info_source_presave ON study_information(source_study_presave_id)",
		"ALTER TABLE study_information ALTER COLUMN study_name TYPE VARCHAR(2000)",
		"ALTER TABLE study_information ALTER COLUMN sponsor_study_number TYPE VARCHAR(50) USING LEFT(sponsor_study_number, 50)",
		"ALTER TABLE study_registration_numbers ALTER COLUMN registration_number TYPE VARCHAR(50) USING LEFT(registration_number, 50)",
		"ALTER TABLE literature_references ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE literature_references DROP CONSTRAINT IF EXISTS unique_lit_ref_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_literature_references_active_sequence_unique ON literature_references(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE documents_held_by_sender ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE documents_held_by_sender DROP CONSTRAINT IF EXISTS unique_documents_held_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_held_by_sender_active_sequence_unique ON documents_held_by_sender(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE study_registration_numbers ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE study_registration_numbers DROP CONSTRAINT IF EXISTS unique_study_reg_num",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_study_registration_numbers_active_sequence_unique ON study_registration_numbers(study_information_id, sequence_number) WHERE deleted = false",
		"CREATE TABLE IF NOT EXISTS study_fda_cross_reported_inds (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), study_information_id UUID NOT NULL REFERENCES study_information(id) ON DELETE CASCADE, ind_number VARCHAR(10) NOT NULL, sequence_number INTEGER NOT NULL, deleted BOOLEAN NOT NULL DEFAULT false, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT, updated_by UUID REFERENCES users(id) ON DELETE RESTRICT)",
		"ALTER TABLE study_fda_cross_reported_inds ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE study_fda_cross_reported_inds DROP CONSTRAINT IF EXISTS unique_study_fda_cross_reported_ind",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_study_fda_cross_reported_inds_active_sequence_unique ON study_fda_cross_reported_inds(study_information_id, sequence_number) WHERE deleted = false",
		"CREATE INDEX IF NOT EXISTS idx_study_fda_cross_reported_inds ON study_fda_cross_reported_inds(study_information_id)",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON study_fda_cross_reported_inds TO e2br3_app_role",
		"ALTER TABLE sender_information ADD COLUMN IF NOT EXISTS source_sender_presave_id UUID REFERENCES sender_presaves(id) ON DELETE SET NULL",
		"CREATE INDEX IF NOT EXISTS idx_sender_info_source_presave ON sender_information(source_sender_presave_id)",
		"ALTER TABLE primary_sources ADD COLUMN IF NOT EXISTS qualification_kr1 VARCHAR(1) CHECK (qualification_kr1 IN ('1', '2'))",
		"ALTER TABLE primary_sources ADD COLUMN IF NOT EXISTS source_reporter_presave_id UUID REFERENCES reporter_presaves(id) ON DELETE SET NULL",
		"CREATE INDEX IF NOT EXISTS idx_primary_sources_source_presave ON primary_sources(source_reporter_presave_id)",
		"ALTER TABLE primary_sources ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE primary_sources DROP CONSTRAINT IF EXISTS unique_primary_source_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_primary_sources_active_sequence_unique ON primary_sources(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE other_case_identifiers ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE other_case_identifiers DROP CONSTRAINT IF EXISTS unique_other_identifier_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_other_case_identifiers_active_sequence_unique ON other_case_identifiers(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE linked_report_numbers ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE linked_report_numbers DROP CONSTRAINT IF EXISTS unique_linked_report_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_linked_report_numbers_active_sequence_unique ON linked_report_numbers(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE relatedness_assessments ADD COLUMN IF NOT EXISTS result_of_assessment_kr2 VARCHAR(2000)",
		"ALTER TABLE reported_causes_of_death ADD COLUMN IF NOT EXISTS comments TEXT",
		"ALTER TABLE autopsy_causes_of_death ADD COLUMN IF NOT EXISTS comments TEXT",
		"ALTER TABLE drug_reaction_assessments ADD COLUMN IF NOT EXISTS administration_start_interval_value DECIMAL(10,2)",
		"ALTER TABLE drug_reaction_assessments ADD COLUMN IF NOT EXISTS administration_start_interval_unit VARCHAR(3)",
		"ALTER TABLE drug_reaction_assessments ADD COLUMN IF NOT EXISTS last_dose_interval_value DECIMAL(10,2)",
		"ALTER TABLE drug_reaction_assessments ADD COLUMN IF NOT EXISTS last_dose_interval_unit VARCHAR(3)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS cumulative_dose_first_reaction_value DECIMAL(15,5)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS cumulative_dose_first_reaction_unit VARCHAR(50)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS gestation_period_exposure_value DECIMAL(10,2)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS gestation_period_exposure_unit VARCHAR(50)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS drug_generic_name VARCHAR(2000)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS drug_authorization_number VARCHAR(100)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS drug_additional_information TEXT",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS fda_other_characterization VARCHAR(50)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS source_product_presave_id UUID REFERENCES product_presaves(id) ON DELETE SET NULL",
		"CREATE INDEX IF NOT EXISTS idx_drug_info_source_presave ON drug_information(source_product_presave_id)",
		"ALTER TABLE drug_information ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE drug_information DROP CONSTRAINT IF EXISTS unique_drug_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_information_active_sequence_unique ON drug_information(case_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE narrative_information ADD COLUMN IF NOT EXISTS additional_information TEXT",
		"ALTER TABLE narrative_information ADD COLUMN IF NOT EXISTS source_narrative_presave_id UUID REFERENCES narrative_presaves(id) ON DELETE SET NULL",
		"CREATE INDEX IF NOT EXISTS idx_narrative_source_presave ON narrative_information(source_narrative_presave_id)",
		"ALTER TABLE sender_diagnoses ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE sender_diagnoses DROP CONSTRAINT IF EXISTS unique_diagnosis_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_sender_diagnoses_active_sequence_unique ON sender_diagnoses(narrative_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE case_summary_information ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE case_summary_information DROP CONSTRAINT IF EXISTS unique_case_summary_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_case_summary_information_active_sequence_unique ON case_summary_information(narrative_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE drug_active_substances ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE drug_active_substances DROP CONSTRAINT IF EXISTS unique_substance_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_active_substances_active_sequence_unique ON drug_active_substances(drug_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE dosage_information ADD COLUMN IF NOT EXISTS continuing BOOLEAN",
		"ALTER TABLE dosage_information ADD COLUMN IF NOT EXISTS route_termid VARCHAR(50)",
		"ALTER TABLE dosage_information ADD COLUMN IF NOT EXISTS route_termid_version VARCHAR(10)",
		"ALTER TABLE dosage_information ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE dosage_information DROP CONSTRAINT IF EXISTS unique_dosage_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_dosage_information_active_sequence_unique ON dosage_information(drug_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE drug_indications ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE drug_indications DROP CONSTRAINT IF EXISTS unique_indication_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_indications_active_sequence_unique ON drug_indications(drug_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE drug_device_characteristics ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE drug_device_characteristics DROP CONSTRAINT IF EXISTS unique_drug_device_characteristic_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_device_characteristics_active_sequence_unique ON drug_device_characteristics(drug_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE drug_recurrence_information ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE drug_recurrence_information DROP CONSTRAINT IF EXISTS unique_drug_recurrence_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_recurrence_information_active_sequence_unique ON drug_recurrence_information(drug_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE relatedness_assessments ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE relatedness_assessments DROP CONSTRAINT IF EXISTS unique_relatedness_sequence",
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_relatedness_assessments_active_sequence_unique ON relatedness_assessments(drug_reaction_assessment_id, sequence_number) WHERE deleted = false",
		"ALTER TABLE users ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE users FORCE ROW LEVEL SECURITY",
		"ALTER TABLE users DROP CONSTRAINT IF EXISTS user_role_valid",
		"ALTER TABLE users ADD CONSTRAINT user_role_valid CHECK (
			role IN ('system_admin', 'sponsor_admin_cro', 'sponsor_admin_company', 'user')
			OR role ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
		)",
		"ALTER TABLE users ADD COLUMN IF NOT EXISTS access_blind_allowed BOOLEAN",
		"ALTER TABLE users ADD COLUMN IF NOT EXISTS active_sender_identifier TEXT",
		"CREATE TABLE IF NOT EXISTS user_organization_memberships (
			user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
			active BOOLEAN NOT NULL DEFAULT true,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			created_by UUID REFERENCES users(id) ON DELETE SET NULL,
			updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
			PRIMARY KEY (user_id, organization_id)
		)",
		"CREATE INDEX IF NOT EXISTS idx_user_organization_memberships_org
			ON user_organization_memberships(organization_id)",
		"INSERT INTO user_organization_memberships (
			user_id,
			organization_id,
			active,
			created_by,
			updated_by
		)
		SELECT
			u.id,
			u.organization_id,
			true,
			COALESCE(u.created_by, '00000000-0000-0000-0000-000000000001'::UUID),
			u.updated_by
		FROM users u
		WHERE u.organization_id <> '00000000-0000-0000-0000-000000000000'::UUID
		ON CONFLICT (user_id, organization_id) DO NOTHING",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON user_organization_memberships TO e2br3_app_role",
		"ALTER TABLE user_organization_memberships ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE user_organization_memberships FORCE ROW LEVEL SECURITY",
			"ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS organization_id UUID",
			"ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS change_category TEXT",
			"CREATE OR REPLACE FUNCTION set_compliance_context(
				p_change_reason TEXT,
				p_change_category TEXT,
				p_e_signature_id TEXT
			)
			RETURNS void
			LANGUAGE plpgsql
			SECURITY DEFINER
			AS $$
			BEGIN
				PERFORM set_config('app.change_reason', COALESCE(p_change_reason, ''), true);
				PERFORM set_config('app.change_category', COALESCE(p_change_category, ''), true);
				PERFORM set_config('app.e_signature_id', COALESCE(p_e_signature_id, ''), true);
			END;
			$$",
			"CREATE OR REPLACE FUNCTION get_current_change_category()
			RETURNS TEXT
			LANGUAGE plpgsql
			STABLE
			AS $$
			DECLARE
				v_category TEXT;
			BEGIN
				v_category := current_setting('app.change_category', true);
				IF v_category IS NULL OR btrim(v_category) = '' THEN
					RETURN NULL;
				END IF;
				RETURN v_category;
			END;
			$$",
			"GRANT EXECUTE ON FUNCTION set_compliance_context(TEXT, TEXT, TEXT) TO e2br3_app_role",
			"GRANT EXECUTE ON FUNCTION get_current_change_category() TO e2br3_app_role",
			"CREATE OR REPLACE FUNCTION audit_logs_hash_chain_before_insert()
			RETURNS TRIGGER
			LANGUAGE plpgsql
			SECURITY DEFINER
			AS $$
			DECLARE
				v_prev_hash TEXT;
				v_payload TEXT;
			BEGIN
				IF NEW.created_at IS NULL THEN
					NEW.created_at := NOW();
				END IF;

				LOCK TABLE audit_logs IN SHARE ROW EXCLUSIVE MODE;

				SELECT entry_hash
				  INTO v_prev_hash
				  FROM audit_logs
				 ORDER BY id DESC
				 LIMIT 1;

				NEW.prev_hash := COALESCE(v_prev_hash, repeat('0', 64));

				v_payload := concat_ws(
					'|',
					COALESCE(NEW.id::TEXT, ''),
					NEW.prev_hash,
					NEW.table_name,
					NEW.record_id::TEXT,
					NEW.action,
					NEW.user_id::TEXT,
					COALESCE(NEW.reason_for_change, ''),
					COALESCE(NEW.change_category, ''),
					COALESCE(NEW.e_signature_id::TEXT, ''),
					COALESCE(NEW.old_values::TEXT, 'null'),
					COALESCE(NEW.new_values::TEXT, 'null'),
					COALESCE(NEW.changed_fields::TEXT, 'null'),
					COALESCE(NEW.ip_address::TEXT, ''),
					COALESCE(NEW.user_agent, ''),
					to_char(NEW.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')
				);

				NEW.entry_hash := encode(digest(v_payload, 'sha256'), 'hex');
				RETURN NEW;
			END;
			$$",
			"CREATE OR REPLACE FUNCTION audit_trigger_function()
			RETURNS TRIGGER
			LANGUAGE plpgsql
			SECURITY DEFINER
			AS $$
			DECLARE
				v_user_id UUID;
				v_old_business JSONB;
				v_new_business JSONB;
				v_changed_fields JSONB;
			BEGIN
				v_user_id := get_current_user_context();

				IF TG_OP = 'INSERT' THEN
					v_changed_fields := compute_audit_changed_fields(NULL, to_jsonb(NEW));
					INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, change_category, e_signature_id, old_values, new_values, changed_fields)
					VALUES (
						TG_TABLE_NAME,
						NEW.id,
						audit_log_organization_id(TG_TABLE_NAME, NEW.id, NULL, to_jsonb(NEW)),
						'CREATE',
						v_user_id,
						get_current_change_reason(),
						get_current_change_category(),
						get_current_esignature_id(),
						NULL,
						to_jsonb(NEW),
						v_changed_fields
					);
					RETURN NEW;

				ELSIF TG_OP = 'UPDATE' THEN
					v_old_business := to_jsonb(OLD) - 'updated_at' - 'updated_by';
					v_new_business := to_jsonb(NEW) - 'updated_at' - 'updated_by';
					IF v_old_business = v_new_business THEN
						RETURN NEW;
					END IF;

					v_changed_fields := compute_audit_changed_fields(v_old_business, v_new_business);
					INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, change_category, e_signature_id, old_values, new_values, changed_fields)
					VALUES (
						TG_TABLE_NAME,
						NEW.id,
						audit_log_organization_id(TG_TABLE_NAME, NEW.id, to_jsonb(OLD), to_jsonb(NEW)),
						'UPDATE',
						v_user_id,
						get_current_change_reason(),
						get_current_change_category(),
						get_current_esignature_id(),
						to_jsonb(OLD),
						to_jsonb(NEW),
						v_changed_fields
					);
					RETURN NEW;

				ELSIF TG_OP = 'DELETE' THEN
					v_changed_fields := compute_audit_changed_fields(to_jsonb(OLD), NULL);
					INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, change_category, e_signature_id, old_values, new_values, changed_fields)
					VALUES (
						TG_TABLE_NAME,
						OLD.id,
						audit_log_organization_id(TG_TABLE_NAME, OLD.id, to_jsonb(OLD), NULL),
						'DELETE',
						v_user_id,
						get_current_change_reason(),
						get_current_change_category(),
						get_current_esignature_id(),
						to_jsonb(OLD),
						NULL,
						v_changed_fields
					);
					RETURN OLD;
				END IF;

			EXCEPTION
				WHEN OTHERS THEN
					RAISE EXCEPTION 'Audit trail logging failed for table %.%: %. User context may not be set.',
						TG_TABLE_SCHEMA, TG_TABLE_NAME, SQLERRM;
			END;
			$$",
			"CREATE OR REPLACE FUNCTION audit_log_organization_id(
				p_table_name TEXT,
				p_record_id UUID,
				p_old_values JSONB,
				p_new_values JSONB
			)
			RETURNS UUID
			LANGUAGE plpgsql
			SECURITY DEFINER
			AS $$
			DECLARE
				v_values JSONB;
				v_org_id UUID;
				v_case_id UUID;
				v_submission_id UUID;
			BEGIN
				IF p_table_name = 'organizations' THEN
					RETURN p_record_id;
				END IF;

				v_values := COALESCE(p_new_values, p_old_values, '{}'::JSONB);
				v_org_id := NULLIF(v_values->>'organization_id', '')::UUID;
				IF v_org_id IS NOT NULL THEN
					RETURN v_org_id;
				END IF;

				v_case_id := NULLIF(v_values->>'case_id', '')::UUID;
				IF v_case_id IS NOT NULL THEN
					SELECT c.organization_id INTO v_org_id
					FROM cases c
					WHERE c.id = v_case_id;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				v_submission_id := NULLIF(v_values->>'submission_id', '')::UUID;
				IF v_submission_id IS NOT NULL THEN
					SELECT c.organization_id INTO v_org_id
					FROM case_submissions cs
					JOIN cases c ON c.id = cs.case_id
					WHERE cs.id = v_submission_id;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				IF p_table_name IN ('sender_presave_gateways', 'sender_presave_responsible_persons') THEN
					SELECT p.organization_id INTO v_org_id
					FROM sender_presaves p
					WHERE p.id = NULLIF(v_values->>'sender_presave_id', '')::UUID;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				IF p_table_name = 'receiver_presave_consignees' THEN
					SELECT p.organization_id INTO v_org_id
					FROM receiver_presaves p
					WHERE p.id = NULLIF(v_values->>'receiver_presave_id', '')::UUID;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				IF p_table_name IN (
					'product_presave_substances',
					'product_presave_mfds_device_items'
				) THEN
					SELECT p.organization_id INTO v_org_id
					FROM product_presaves p
					WHERE p.id = NULLIF(v_values->>'product_presave_id', '')::UUID;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				IF p_table_name IN (
					'study_presave_registration_numbers',
					'study_presave_products',
					'study_presave_reporters'
				) THEN
					SELECT p.organization_id INTO v_org_id
					FROM study_presaves p
					WHERE p.id = NULLIF(v_values->>'study_presave_id', '')::UUID;
					IF v_org_id IS NOT NULL THEN RETURN v_org_id; END IF;
				END IF;

				RETURN COALESCE(current_organization_id(), '00000000-0000-0000-0000-000000000000'::UUID);
			END;
			$$",
			"UPDATE audit_logs
			 SET organization_id = COALESCE(
			 	NULLIF(new_values->>'organization_id', '')::UUID,
		 	NULLIF(old_values->>'organization_id', '')::UUID
		 )
		 WHERE organization_id IS NULL
		   AND COALESCE(new_values->>'organization_id', old_values->>'organization_id') IS NOT NULL",
		"UPDATE audit_logs l
		 SET organization_id = l.record_id
		 WHERE l.organization_id IS NULL
		   AND l.table_name = 'organizations'
		   AND EXISTS (SELECT 1 FROM organizations o WHERE o.id = l.record_id)",
		"UPDATE audit_logs l
		 SET organization_id = c.organization_id
		 FROM cases c
		 WHERE l.organization_id IS NULL
		   AND (
		   	(l.table_name = 'cases' AND l.record_id = c.id)
		   	OR NULLIF(COALESCE(l.new_values->>'case_id', l.old_values->>'case_id'), '')::UUID = c.id
		   )",
		"UPDATE audit_logs l
		 SET organization_id = c.organization_id
		 FROM case_submissions cs
		 JOIN cases c ON c.id = cs.case_id
		 WHERE l.organization_id IS NULL
		   AND NULLIF(COALESCE(l.new_values->>'submission_id', l.old_values->>'submission_id'), '')::UUID = cs.id",
		"UPDATE audit_logs
		 SET organization_id = '00000000-0000-0000-0000-000000000000'::UUID
		 WHERE organization_id IS NULL",
		"ALTER TABLE audit_logs ALTER COLUMN organization_id SET NOT NULL",
		"ALTER TABLE audit_logs ALTER COLUMN organization_id SET DEFAULT COALESCE(current_organization_id(), '00000000-0000-0000-0000-000000000000'::UUID)",
		"ALTER TABLE app_settings ADD COLUMN IF NOT EXISTS organization_id UUID",
		"ALTER TABLE whodrug_products ALTER COLUMN drug_name TYPE TEXT",
		"UPDATE app_settings
		 SET organization_id = '00000000-0000-0000-0000-000000000000'::UUID
		 WHERE organization_id IS NULL",
		"ALTER TABLE app_settings ALTER COLUMN organization_id SET NOT NULL",
		"DO $$
		 BEGIN
				 IF EXISTS (
				     SELECT 1
				     FROM pg_constraint
				     WHERE conname = 'app_settings_pkey'
				       AND conrelid = 'app_settings'::regclass
				 ) THEN
				     ALTER TABLE app_settings DROP CONSTRAINT app_settings_pkey;
				 END IF;
		 END $$",
		"ALTER TABLE app_settings ADD CONSTRAINT app_settings_pkey PRIMARY KEY (organization_id, key)",
		"CREATE TABLE IF NOT EXISTS dashboard_notices (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE RESTRICT,
			notice_key text NOT NULL,
			title text NOT NULL,
			body text,
			effective_date text,
			expire_date text,
			writer text,
			sort_order integer NOT NULL DEFAULT 0,
			created_at timestamptz NOT NULL DEFAULT now(),
			updated_at timestamptz NOT NULL DEFAULT now(),
			updated_by uuid NULL REFERENCES users(id) ON DELETE SET NULL,
			UNIQUE (organization_id, notice_key)
		)",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON dashboard_notices TO e2br3_app_role",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS id UUID DEFAULT gen_random_uuid()",
		"UPDATE permission_profiles SET id = gen_random_uuid() WHERE id IS NULL",
		"ALTER TABLE permission_profiles ALTER COLUMN id SET NOT NULL",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS description TEXT",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS organization_id UUID",
		"UPDATE permission_profiles
		 SET organization_id = '00000000-0000-0000-0000-000000000000'::UUID
		 WHERE organization_id IS NULL",
		"ALTER TABLE permission_profiles ALTER COLUMN organization_id SET NOT NULL",
		"UPDATE permission_profiles SET name = left(name, 128) WHERE length(name) > 128",
		"UPDATE permission_profiles SET description = left(description, 512) WHERE description IS NOT NULL AND length(description) > 512",
		"ALTER TABLE permission_profiles ALTER COLUMN name TYPE VARCHAR(128)",
		"ALTER TABLE permission_profiles ALTER COLUMN description TYPE VARCHAR(512)",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS privileges_json JSONB NOT NULL DEFAULT '[]'::jsonb",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS built_in BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS editable BOOLEAN NOT NULL DEFAULT true",
		"ALTER TABLE permission_profiles ADD COLUMN IF NOT EXISTS sponsor_admin_capable BOOLEAN NOT NULL DEFAULT false",
		"DO $$
		 BEGIN
		     IF EXISTS (
		         SELECT 1 FROM information_schema.columns
		         WHERE table_name = 'users'
		           AND column_name = 'permission_profile_id'
		     ) AND EXISTS (
		         SELECT 1 FROM information_schema.columns
		         WHERE table_name = 'permission_profiles'
		           AND column_name = 'profile_id'
		     ) THEN
		         UPDATE users u
		         SET role = pp.id::text
		         FROM permission_profiles pp
		         WHERE u.permission_profile_id IS NOT NULL
		           AND pp.profile_id = u.permission_profile_id;
		     END IF;
		 END $$",
		"ALTER TABLE users DROP COLUMN IF EXISTS permission_profile_id",
		"DO $$
		 BEGIN
		     IF EXISTS (
		         SELECT 1
		         FROM pg_constraint
		         WHERE conname = 'permission_profiles_pkey'
		           AND conrelid = 'permission_profiles'::regclass
		     ) THEN
		         ALTER TABLE permission_profiles DROP CONSTRAINT permission_profiles_pkey;
		     END IF;
		     IF NOT EXISTS (
		         SELECT 1
		         FROM pg_constraint
		         WHERE conname = 'permission_profiles_pkey'
		           AND conrelid = 'permission_profiles'::regclass
		     ) THEN
		         ALTER TABLE permission_profiles ADD CONSTRAINT permission_profiles_pkey PRIMARY KEY (id);
		     END IF;
		 END $$",
		"ALTER TABLE permission_profiles DROP COLUMN IF EXISTS profile_id",
		"DROP TRIGGER IF EXISTS audit_permission_profiles ON permission_profiles",
		"CREATE TRIGGER audit_permission_profiles AFTER INSERT OR UPDATE OR DELETE ON permission_profiles
		 FOR EACH ROW EXECUTE FUNCTION audit_trigger_function()",
		"DROP POLICY IF EXISTS users_org_isolation_select ON users",
		"DROP POLICY IF EXISTS user_organization_memberships_read ON user_organization_memberships",
		"DROP POLICY IF EXISTS user_organization_memberships_modify ON user_organization_memberships",
		"DROP POLICY IF EXISTS orgs_select ON organizations",
	] {
		sqlx::query(sql).execute(&mut *tx).await?;
	}
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS case_validation_summaries (
			case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
			appendix VARCHAR(16) NOT NULL,
			page_id VARCHAR(16) NOT NULL,
			blocking_count INTEGER NOT NULL DEFAULT 0,
			non_blocking_count INTEGER NOT NULL DEFAULT 0,
			required_count INTEGER NOT NULL DEFAULT 0,
			stale BOOLEAN NOT NULL DEFAULT FALSE,
			generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
			PRIMARY KEY (case_id, appendix, page_id),
			CONSTRAINT case_validation_summary_appendix_valid
				CHECK (appendix IN ('ich', 'fda', 'mfds')),
			CONSTRAINT case_validation_summary_counts_non_negative
				CHECK (
					blocking_count >= 0
					AND non_blocking_count >= 0
					AND required_count >= 0
				)
		)",
	)
	.execute(&mut *tx)
	.await?;
	for sql in [
		"CREATE INDEX IF NOT EXISTS idx_case_validation_summaries_case
		 ON case_validation_summaries(case_id)",
		"CREATE INDEX IF NOT EXISTS idx_case_validation_summaries_page
		 ON case_validation_summaries(case_id, page_id, stale)",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON case_validation_summaries TO e2br3_app_role",
		"ALTER TABLE case_validation_summaries ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE case_validation_summaries FORCE ROW LEVEL SECURITY",
		"DROP POLICY IF EXISTS case_validation_summaries_via_case ON case_validation_summaries",
	] {
		sqlx::query(sql).execute(&mut *tx).await?;
	}
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY case_validation_summaries_via_case ON case_validation_summaries
		 FOR ALL
		 TO e2br3_app_role
		 USING (
			 EXISTS (
				 SELECT 1 FROM cases c
				 WHERE c.id = case_validation_summaries.case_id
				   AND (
					   c.organization_id = current_organization_id()
					   OR is_current_user_admin()
				   )
			 )
		 )
		 WITH CHECK (
			 EXISTS (
				 SELECT 1 FROM cases c
				 WHERE c.id = case_validation_summaries.case_id
				   AND (
					   c.organization_id = current_organization_id()
					   OR is_current_user_admin()
				   )
			 )
		 )",
	)
	.await?;
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS case_validation_reports (
			case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
			authority TEXT NOT NULL,
			report JSONB NOT NULL,
			stale BOOLEAN NOT NULL DEFAULT false,
			generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
			PRIMARY KEY (case_id, authority),
			CONSTRAINT case_validation_reports_authority_valid CHECK (authority IN ('ich', 'fda', 'mfds'))
		)",
	)
	.execute(&mut *tx)
	.await?;
	for sql in [
		"DO $$
		 BEGIN
		   IF EXISTS (
		     SELECT 1 FROM information_schema.columns
		      WHERE table_name = 'case_validation_reports'
		        AND column_name = 'profile'
		   ) AND NOT EXISTS (
		     SELECT 1 FROM information_schema.columns
		      WHERE table_name = 'case_validation_reports'
		        AND column_name = 'authority'
		   ) THEN
		     ALTER TABLE case_validation_reports RENAME COLUMN profile TO authority;
		   END IF;
		 END $$",
		"ALTER TABLE case_validation_reports
		 DROP CONSTRAINT IF EXISTS case_validation_reports_profile_valid",
		"ALTER TABLE case_validation_reports
		 DROP CONSTRAINT IF EXISTS case_validation_reports_authority_valid",
		"ALTER TABLE case_validation_reports
		 ADD CONSTRAINT case_validation_reports_authority_valid
		 CHECK (authority IN ('ich', 'fda', 'mfds'))",
		"DROP INDEX IF EXISTS idx_case_validation_reports_case_fresh",
		"CREATE INDEX IF NOT EXISTS idx_case_validation_reports_case_fresh
		 ON case_validation_reports (case_id, authority)
		 WHERE stale = false",
		"GRANT SELECT, INSERT, UPDATE, DELETE ON case_validation_reports TO e2br3_app_role",
		"ALTER TABLE case_validation_reports ENABLE ROW LEVEL SECURITY",
		"ALTER TABLE case_validation_reports FORCE ROW LEVEL SECURITY",
		"DROP POLICY IF EXISTS case_validation_reports_via_case ON case_validation_reports",
	] {
		sqlx::query(sql).execute(&mut *tx).await?;
	}
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY case_validation_reports_via_case ON case_validation_reports
		 FOR ALL
		 TO e2br3_app_role
		 USING (
			 EXISTS (
				 SELECT 1 FROM cases c
				 WHERE c.id = case_validation_reports.case_id
				   AND (
					   c.organization_id = current_organization_id()
					   OR is_current_user_admin()
				   )
			 )
		 )
		 WITH CHECK (
			 EXISTS (
				 SELECT 1 FROM cases c
				 WHERE c.id = case_validation_reports.case_id
				   AND (
					   c.organization_id = current_organization_id()
					   OR is_current_user_admin()
				   )
			 )
		 )",
	)
	.await?;
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS case_workflow_events (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
			from_status TEXT NOT NULL,
			from_role TEXT,
			from_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
			to_status TEXT NOT NULL,
			target_role TEXT,
			target_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
			comment TEXT,
			date_of_most_recent TEXT,
			due_at TIMESTAMPTZ,
			acted_by UUID NOT NULL REFERENCES users(id),
			actor_role_id TEXT NOT NULL DEFAULT 'unknown',
			used_admin_override BOOLEAN NOT NULL DEFAULT false,
			override_reason TEXT,
			created_at TIMESTAMPTZ NOT NULL DEFAULT now()
		)",
	)
	.execute(&mut *tx)
	.await?;
	for sql in [
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS actor_role_id TEXT NOT NULL DEFAULT 'unknown'",
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS used_admin_override BOOLEAN NOT NULL DEFAULT false",
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS override_reason TEXT",
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS from_role TEXT",
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS from_user_id UUID",
		"ALTER TABLE case_workflow_events ADD COLUMN IF NOT EXISTS date_of_most_recent TEXT",
	] {
		sqlx::query(sql).execute(&mut *tx).await?;
	}
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_cases_workflow_status ON cases(workflow_status)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_case_workflow_events_case ON case_workflow_events(case_id, created_at DESC)",
	)
	.execute(&mut *tx)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
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
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY users_org_isolation_modify ON users
		 FOR ALL
		 TO e2br3_app_role
		 USING (
				 organization_id = current_organization_id()
				 OR is_current_user_admin()
		 )
		 WITH CHECK (
				 organization_id = current_organization_id()
				 OR is_current_user_admin()
		 )",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY user_organization_memberships_read ON user_organization_memberships
		 FOR SELECT
		 TO e2br3_app_role
		 USING (
		 	user_id = get_current_user_context()
		 	OR is_current_user_admin()
		 )",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY user_organization_memberships_modify ON user_organization_memberships
		 FOR ALL
		 TO e2br3_app_role
		 USING (is_current_user_admin())
		 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY orgs_select ON organizations
		 FOR SELECT
		 TO e2br3_app_role
		 USING (
		 	id = current_organization_id()
		 	OR is_current_user_admin()
		 	OR EXISTS (
		 		SELECT 1
		 		FROM user_organization_memberships membership
		 		WHERE membership.organization_id = organizations.id
		 		  AND membership.user_id = get_current_user_context()
		 		  AND membership.active = true
		 	)
		 )",
	)
	.await?;
	sqlx::query(
		"CREATE OR REPLACE FUNCTION is_current_user_admin() RETURNS BOOLEAN AS $$
		BEGIN
			    RETURN COALESCE(current_setting('app.current_user_role', true), '') = 'system_admin';
		EXCEPTION
		    WHEN OTHERS THEN
		        RETURN false;
		END;
		$$ LANGUAGE plpgsql STABLE",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_audit_logs_org_created_at ON audit_logs(organization_id, created_at DESC)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_audit_logs_org_table_record_created_at ON audit_logs(organization_id, table_name, record_id, created_at DESC)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_audit_logs_org_user_created_at ON audit_logs(organization_id, user_id, created_at DESC)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_app_settings_org_key ON app_settings(organization_id, key)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_dashboard_notices_org_order ON dashboard_notices(organization_id, sort_order, created_at)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE INDEX IF NOT EXISTS idx_permission_profiles_org ON permission_profiles(organization_id)",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"WITH duplicate_roles AS (
			SELECT id,
				       row_number() OVER (
				           PARTITION BY organization_id, lower(btrim(name))
				           ORDER BY updated_at ASC, id ASC
				       ) AS duplicate_rank
			FROM permission_profiles
		)
		UPDATE permission_profiles pp
		SET name = pp.name || ' (' || pp.id || ')'
		FROM duplicate_roles dr
		WHERE pp.id = dr.id
		  AND dr.duplicate_rank > 1",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"CREATE UNIQUE INDEX IF NOT EXISTS idx_permission_profiles_org_name_unique
		 ON permission_profiles(organization_id, lower(btrim(name)))",
	)
	.execute(&mut *tx)
	.await?;
	sqlx::query("DROP POLICY IF EXISTS audit_logs_append_only ON audit_logs")
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY audit_logs_append_only ON audit_logs
		 FOR INSERT
		 TO e2br3_app_role
		 WITH CHECK (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )",
	)
	.await?;
	sqlx::query(
		"DROP POLICY IF EXISTS audit_logs_read_for_admin_manager ON audit_logs",
	)
	.execute(&mut *tx)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY audit_logs_read_for_admin_manager ON audit_logs
		 FOR SELECT
		 TO e2br3_app_role
		 	USING (
		 		(
		 			COALESCE(current_setting('app.current_user_role', true), '') IN (
		 				'system_admin',
		 				'sponsor_admin_cro',
		 				'sponsor_admin_company'
		 			)
		 			OR EXISTS (
		 				SELECT 1
		 				FROM permission_profiles pp
		 				WHERE pp.id::text = COALESCE(current_setting('app.current_user_role', true), '')
		 				  AND pp.active = true
		 				  AND pp.privileges_json @> '[{\"menu_key\":\"audit\",\"can_read\":true}]'::jsonb
		 			)
		 		)
		 		AND (
		 			organization_id = current_organization_id()
		 			OR is_current_user_admin()
		 		)
		 	)",
	)
	.await?;
	sqlx::query("ALTER TABLE app_settings ENABLE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query("ALTER TABLE app_settings FORCE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS app_settings_org_isolation ON app_settings")
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY app_settings_org_isolation ON app_settings
		 FOR ALL
		 TO e2br3_app_role
		 USING (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )
		 WITH CHECK (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )",
	)
	.await?;
	sqlx::query("ALTER TABLE dashboard_notices ENABLE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query("ALTER TABLE dashboard_notices FORCE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query(
		"DROP POLICY IF EXISTS dashboard_notices_org_isolation ON dashboard_notices",
	)
	.execute(&mut *tx)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY dashboard_notices_org_isolation ON dashboard_notices
		 FOR ALL
		 TO e2br3_app_role
		 USING (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )
		 WITH CHECK (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )",
	)
	.await?;
	sqlx::query("ALTER TABLE permission_profiles ENABLE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query("ALTER TABLE permission_profiles FORCE ROW LEVEL SECURITY")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS permission_profiles_org_isolation ON permission_profiles")
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY permission_profiles_org_isolation ON permission_profiles
		 FOR ALL
		 TO e2br3_app_role
		 USING (
			 organization_id = current_organization_id()
			 OR is_current_user_admin()
		 )
		 WITH CHECK (
				 organization_id = current_organization_id()
				 OR is_current_user_admin()
		 )",
	)
	.await?;
	sqlx::query("DROP POLICY IF EXISTS meddra_terms_read ON meddra_terms")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS meddra_terms_insert ON meddra_terms")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS meddra_terms_update ON meddra_terms")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS meddra_terms_delete ON meddra_terms")
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY meddra_terms_read ON meddra_terms
			 FOR SELECT
			 TO e2br3_app_role
		 USING (active = true OR is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY meddra_terms_insert ON meddra_terms
			 FOR INSERT
			 TO e2br3_app_role
			 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY meddra_terms_update ON meddra_terms
			 FOR UPDATE
			 TO e2br3_app_role
			 USING (is_current_user_admin())
			 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY meddra_terms_delete ON meddra_terms
			 FOR DELETE
			 TO e2br3_app_role
			 USING (is_current_user_admin())",
	)
	.await?;
	sqlx::query("DROP POLICY IF EXISTS whodrug_products_read ON whodrug_products")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS whodrug_products_insert ON whodrug_products")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS whodrug_products_update ON whodrug_products")
		.execute(&mut *tx)
		.await?;
	sqlx::query("DROP POLICY IF EXISTS whodrug_products_delete ON whodrug_products")
		.execute(&mut *tx)
		.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY whodrug_products_read ON whodrug_products
			 FOR SELECT
			 TO e2br3_app_role
		 USING (active = true OR is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY whodrug_products_insert ON whodrug_products
			 FOR INSERT
			 TO e2br3_app_role
			 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY whodrug_products_update ON whodrug_products
			 FOR UPDATE
			 TO e2br3_app_role
			 USING (is_current_user_admin())
			 WITH CHECK (is_current_user_admin())",
	)
	.await?;
	execute_ignoring_duplicate_policy(
		&mut tx,
		"CREATE POLICY whodrug_products_delete ON whodrug_products
			 FOR DELETE
			 TO e2br3_app_role
			 USING (is_current_user_admin())",
	)
	.await?;
	for sql in dirty_trigger_compatibility_sql() {
		sqlx::query(sql).execute(&mut *tx).await?;
	}
	for (code, name, unit_type) in [
		("mg/dL", "milligram per deciliter", "concentration"),
		("U/L", "unit per liter", "activity concentration"),
		("mmol/L", "millimole per liter", "concentration"),
	] {
		sqlx::query(
			"INSERT INTO ucum_units (code, display_name, unit_type, active)
			 VALUES ($1, $2, $3, true)
			 ON CONFLICT (code) DO UPDATE SET
			 	display_name = EXCLUDED.display_name,
			 	unit_type = EXCLUDED.unit_type,
			 	active = true",
		)
		.bind(code)
		.bind(name)
		.bind(unit_type)
		.execute(&mut *tx)
		.await?;
	}
	tx.commit().await?;
	Ok(())
}

fn dirty_trigger_compatibility_sql() -> &'static [&'static str] {
	&[
		"DROP TRIGGER IF EXISTS trg_dirty_c_safety_report_identification ON safety_report_identification",
		"DROP TRIGGER IF EXISTS aa_dirty_c_safety_report_identification ON safety_report_identification",
		"CREATE TRIGGER aa_dirty_c_safety_report_identification AFTER INSERT OR UPDATE OR DELETE ON safety_report_identification FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_sender_information ON sender_information",
		"DROP TRIGGER IF EXISTS aa_dirty_c_sender_information ON sender_information",
		"CREATE TRIGGER aa_dirty_c_sender_information AFTER INSERT OR UPDATE OR DELETE ON sender_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_literature_references ON literature_references",
		"DROP TRIGGER IF EXISTS aa_dirty_c_literature_references ON literature_references",
		"CREATE TRIGGER aa_dirty_c_literature_references AFTER INSERT OR UPDATE OR DELETE ON literature_references FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_documents_held_by_sender ON documents_held_by_sender",
		"DROP TRIGGER IF EXISTS aa_dirty_c_documents_held_by_sender ON documents_held_by_sender",
		"CREATE TRIGGER aa_dirty_c_documents_held_by_sender AFTER INSERT OR UPDATE OR DELETE ON documents_held_by_sender FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_study_information ON study_information",
		"DROP TRIGGER IF EXISTS aa_dirty_c_study_information ON study_information",
		"CREATE TRIGGER aa_dirty_c_study_information AFTER INSERT OR UPDATE OR DELETE ON study_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_study_registration_numbers ON study_registration_numbers",
		"DROP TRIGGER IF EXISTS aa_dirty_c_study_registration_numbers ON study_registration_numbers",
		"CREATE TRIGGER aa_dirty_c_study_registration_numbers AFTER INSERT OR UPDATE OR DELETE ON study_registration_numbers FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c_from_study_info()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_study_fda_cross_reported_inds ON study_fda_cross_reported_inds",
		"DROP TRIGGER IF EXISTS aa_dirty_c_study_fda_cross_reported_inds ON study_fda_cross_reported_inds",
		"CREATE TRIGGER aa_dirty_c_study_fda_cross_reported_inds AFTER INSERT OR UPDATE OR DELETE ON study_fda_cross_reported_inds FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c_from_study_info()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_primary_sources ON primary_sources",
		"DROP TRIGGER IF EXISTS aa_dirty_c_primary_sources ON primary_sources",
		"CREATE TRIGGER aa_dirty_c_primary_sources AFTER INSERT OR UPDATE OR DELETE ON primary_sources FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_receiver_information ON receiver_information",
		"DROP TRIGGER IF EXISTS aa_dirty_c_receiver_information ON receiver_information",
		"CREATE TRIGGER aa_dirty_c_receiver_information AFTER INSERT OR UPDATE OR DELETE ON receiver_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_other_case_identifiers ON other_case_identifiers",
		"DROP TRIGGER IF EXISTS aa_dirty_c_other_case_identifiers ON other_case_identifiers",
		"CREATE TRIGGER aa_dirty_c_other_case_identifiers AFTER INSERT OR UPDATE OR DELETE ON other_case_identifiers FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_c_linked_report_numbers ON linked_report_numbers",
		"DROP TRIGGER IF EXISTS aa_dirty_c_linked_report_numbers ON linked_report_numbers",
		"CREATE TRIGGER aa_dirty_c_linked_report_numbers AFTER INSERT OR UPDATE OR DELETE ON linked_report_numbers FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_c()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_patient_information ON patient_information",
		"DROP TRIGGER IF EXISTS aa_dirty_d_patient_information ON patient_information",
		"CREATE TRIGGER aa_dirty_d_patient_information AFTER INSERT OR UPDATE OR DELETE ON patient_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_patient_identifiers ON patient_identifiers",
		"DROP TRIGGER IF EXISTS aa_dirty_d_patient_identifiers ON patient_identifiers",
		"CREATE TRIGGER aa_dirty_d_patient_identifiers AFTER INSERT OR UPDATE OR DELETE ON patient_identifiers FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_medical_history_episodes ON medical_history_episodes",
		"DROP TRIGGER IF EXISTS aa_dirty_d_medical_history_episodes ON medical_history_episodes",
		"CREATE TRIGGER aa_dirty_d_medical_history_episodes AFTER INSERT OR UPDATE OR DELETE ON medical_history_episodes FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_past_drug_history ON past_drug_history",
		"DROP TRIGGER IF EXISTS aa_dirty_d_past_drug_history ON past_drug_history",
		"CREATE TRIGGER aa_dirty_d_past_drug_history AFTER INSERT OR UPDATE OR DELETE ON past_drug_history FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_patient_death_information ON patient_death_information",
		"DROP TRIGGER IF EXISTS aa_dirty_d_patient_death_information ON patient_death_information",
		"CREATE TRIGGER aa_dirty_d_patient_death_information AFTER INSERT OR UPDATE OR DELETE ON patient_death_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_reported_causes_of_death ON reported_causes_of_death",
		"DROP TRIGGER IF EXISTS aa_dirty_d_reported_causes_of_death ON reported_causes_of_death",
		"CREATE TRIGGER aa_dirty_d_reported_causes_of_death AFTER INSERT OR UPDATE OR DELETE ON reported_causes_of_death FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_death_info()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_autopsy_causes_of_death ON autopsy_causes_of_death",
		"DROP TRIGGER IF EXISTS aa_dirty_d_autopsy_causes_of_death ON autopsy_causes_of_death",
		"CREATE TRIGGER aa_dirty_d_autopsy_causes_of_death AFTER INSERT OR UPDATE OR DELETE ON autopsy_causes_of_death FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_death_info()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_parent_information ON parent_information",
		"DROP TRIGGER IF EXISTS aa_dirty_d_parent_information ON parent_information",
		"CREATE TRIGGER aa_dirty_d_parent_information AFTER INSERT OR UPDATE OR DELETE ON parent_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_patient()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_parent_medical_history ON parent_medical_history",
		"DROP TRIGGER IF EXISTS aa_dirty_d_parent_medical_history ON parent_medical_history",
		"CREATE TRIGGER aa_dirty_d_parent_medical_history AFTER INSERT OR UPDATE OR DELETE ON parent_medical_history FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_parent()",
		"DROP TRIGGER IF EXISTS trg_dirty_d_parent_past_drug_history ON parent_past_drug_history",
		"DROP TRIGGER IF EXISTS aa_dirty_d_parent_past_drug_history ON parent_past_drug_history",
		"CREATE TRIGGER aa_dirty_d_parent_past_drug_history AFTER INSERT OR UPDATE OR DELETE ON parent_past_drug_history FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_d_from_parent()",
		"DROP TRIGGER IF EXISTS trg_dirty_e_reactions ON reactions",
		"DROP TRIGGER IF EXISTS aa_dirty_e_reactions ON reactions",
		"CREATE TRIGGER aa_dirty_e_reactions AFTER INSERT OR UPDATE OR DELETE ON reactions FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_e()",
		"DROP TRIGGER IF EXISTS trg_dirty_f_test_results ON test_results",
		"DROP TRIGGER IF EXISTS aa_dirty_f_test_results ON test_results",
		"CREATE TRIGGER aa_dirty_f_test_results AFTER INSERT OR UPDATE OR DELETE ON test_results FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_f()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_information ON drug_information",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_information ON drug_information",
		"CREATE TRIGGER aa_dirty_g_drug_information AFTER INSERT OR UPDATE OR DELETE ON drug_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_active_substances ON drug_active_substances",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_active_substances ON drug_active_substances",
		"CREATE TRIGGER aa_dirty_g_drug_active_substances AFTER INSERT OR UPDATE OR DELETE ON drug_active_substances FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_dosage_information ON dosage_information",
		"DROP TRIGGER IF EXISTS aa_dirty_g_dosage_information ON dosage_information",
		"CREATE TRIGGER aa_dirty_g_dosage_information AFTER INSERT OR UPDATE OR DELETE ON dosage_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_indications ON drug_indications",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_indications ON drug_indications",
		"CREATE TRIGGER aa_dirty_g_drug_indications AFTER INSERT OR UPDATE OR DELETE ON drug_indications FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_device_characteristics ON drug_device_characteristics",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_device_characteristics ON drug_device_characteristics",
		"CREATE TRIGGER aa_dirty_g_drug_device_characteristics AFTER INSERT OR UPDATE OR DELETE ON drug_device_characteristics FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_recurrence_information ON drug_recurrence_information",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_recurrence_information ON drug_recurrence_information",
		"CREATE TRIGGER aa_dirty_g_drug_recurrence_information AFTER INSERT OR UPDATE OR DELETE ON drug_recurrence_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_drug_reaction_assessments ON drug_reaction_assessments",
		"DROP TRIGGER IF EXISTS aa_dirty_g_drug_reaction_assessments ON drug_reaction_assessments",
		"CREATE TRIGGER aa_dirty_g_drug_reaction_assessments AFTER INSERT OR UPDATE OR DELETE ON drug_reaction_assessments FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_g_relatedness_assessments ON relatedness_assessments",
		"DROP TRIGGER IF EXISTS aa_dirty_g_relatedness_assessments ON relatedness_assessments",
		"CREATE TRIGGER aa_dirty_g_relatedness_assessments AFTER INSERT OR UPDATE OR DELETE ON relatedness_assessments FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_g_from_drug()",
		"DROP TRIGGER IF EXISTS trg_dirty_h_narrative_information ON narrative_information",
		"DROP TRIGGER IF EXISTS aa_dirty_h_narrative_information ON narrative_information",
		"CREATE TRIGGER aa_dirty_h_narrative_information AFTER INSERT OR UPDATE OR DELETE ON narrative_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h()",
		"DROP TRIGGER IF EXISTS trg_dirty_h_sender_diagnoses ON sender_diagnoses",
		"DROP TRIGGER IF EXISTS aa_dirty_h_sender_diagnoses ON sender_diagnoses",
		"CREATE TRIGGER aa_dirty_h_sender_diagnoses AFTER INSERT OR UPDATE OR DELETE ON sender_diagnoses FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h_from_narrative()",
		"DROP TRIGGER IF EXISTS trg_dirty_h_case_summary_information ON case_summary_information",
		"DROP TRIGGER IF EXISTS aa_dirty_h_case_summary_information ON case_summary_information",
		"CREATE TRIGGER aa_dirty_h_case_summary_information AFTER INSERT OR UPDATE OR DELETE ON case_summary_information FOR EACH ROW EXECUTE FUNCTION mark_case_dirty_h_from_narrative()",
	]
}

async fn execute_ignoring_duplicate_policy<'a>(
	tx: &mut sqlx::Transaction<'a, Postgres>,
	sql: &str,
) -> Result<(), sqlx::Error> {
	match sqlx::query(sql).execute(&mut **tx).await {
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

async fn execute_ignoring_duplicate_constraint<'a>(
	tx: &mut sqlx::Transaction<'a, Postgres>,
	sql: &str,
) -> Result<(), sqlx::Error> {
	match sqlx::query(sql).execute(&mut **tx).await {
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
