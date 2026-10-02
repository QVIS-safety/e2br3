#![allow(dead_code)]

use lib_auth::pwd::{self, ContentToHash};
use lib_core::_dev_utils;
use lib_core::authorization::{
	grant_ids_for_menu_privileges, policy_registry, AdminMenuPrivilege,
	BuiltInIdentityKind,
};
use lib_core::ctx::{
	Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
	ROLE_USER, SYSTEM_ORG_ID, SYSTEM_USER_ID,
};
use lib_core::model::admin_settings::AdminSettingsBmc;
use lib_core::model::authorization::AuthorizationMigrationService;
use lib_core::model::case::{CaseBmc, CaseForUpdate};
use lib_core::model::presave::{
	ProductPresaveBmc, ProductPresaveForCreate, SenderPresaveBmc,
	SenderPresaveForCreate, SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
};
use lib_core::model::store::{
	set_full_context_dbx, set_org_context, set_user_context,
};
use lib_core::model::ModelManager;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use uuid::Uuid;

pub type Result<T> =
	core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn unique_safety_report_id_xml(xml: String) -> String {
	let Some(start) = xml.find("extension=\"US-") else {
		return xml;
	};
	let value_start = start + "extension=\"".len();
	let Some(end_rel) = xml[value_start..].find('"') else {
		return xml;
	};
	let old_id = xml[value_start..value_start + end_rel].to_string();
	let new_id = format!("US-TEST-{}", Uuid::new_v4());
	xml.replace(
		&format!("extension=\"{old_id}\""),
		&format!("extension=\"{new_id}\""),
	)
}

pub const TEST_CUSTOM_MANAGER_ROLE: &str = "11111111-1111-4111-8111-111111111111";
pub const TEST_CUSTOM_VIEWER_ROLE: &str = "22222222-2222-4222-8222-222222222222";

pub struct SeedUser {
	pub id: Uuid,
	pub email: String,
	pub token_salt: Uuid,
}

pub struct SeedOrgUsers {
	pub org_id: Uuid,
	pub admin: SeedUser,
	pub viewer: SeedUser,
}

pub struct SeedOrgAllRoles {
	pub org_id: Uuid,
	pub admin: SeedUser,
	pub manager: SeedUser,
	pub user: SeedUser,
	pub viewer: SeedUser,
}

pub struct SeedOrgsUsersCases {
	pub org1_id: Uuid,
	pub org2_id: Uuid,
	pub user1: SeedUser,
	pub user2: SeedUser,
	pub case_org1: Uuid,
	pub case_org2: Uuid,
}

pub struct SeedOrgsManagerCases {
	pub org1_id: Uuid,
	pub org2_id: Uuid,
	pub manager: SeedUser,
	pub user2: SeedUser,
	pub case_org1: Uuid,
	pub case_org2: Uuid,
}

pub async fn init_test_env() -> Result<()> {
	if std::env::var("E2BR3_TEST_DATABASE_NAME").is_err() {
		return Err(
			"database-backed web-server tests require scripts/test-isolated-db.sh"
				.into(),
		);
	}
	std::env::set_var("SERVICE_WEB_FOLDER", "web-folder");
	std::env::set_var("SERVICE_PWD_KEY", "ZmFrZV9rZXk");
	std::env::set_var("SERVICE_TOKEN_KEY", "ZmFrZV9rZXk");
	std::env::set_var("SERVICE_TOKEN_DURATION_SEC", "3600");
	std::env::set_var("SERVICE_DB_MAX_CONNECTIONS", "5");
	std::env::set_var("E2BR3_DEBUG_ERRORS", "1");
	if std::env::var("E2BR3_DEFAULT_MESSAGE_SENDER").is_err() {
		std::env::set_var("E2BR3_DEFAULT_MESSAGE_SENDER", "TEST-SENDER");
	}
	if std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA").is_err() {
		std::env::set_var("E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA", "TEST-FDA");
	}
	if std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH").is_err() {
		std::env::set_var("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH", "TEST-ICH");
	}
	if std::env::var("E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS").is_err() {
		std::env::set_var("E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS", "TEST-MFDS");
	}
	if std::env::var("E2BR3_EXPORT_VALIDATE").is_err() {
		std::env::set_var("E2BR3_EXPORT_VALIDATE", "0");
	}

	// Keep integration tests deterministic even when direnv isn't loaded.
	if std::env::var("E2BR3_EXAMPLES_DIR").is_err() {
		let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../..")
			.join("docs/exporter/fda");
		std::env::set_var("E2BR3_EXAMPLES_DIR", examples_dir);
	}
	if std::env::var("E2BR3_XSD_PATH").is_err() {
		let xsd_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../..")
			.join("docs/exporter/schema/multicacheschemas/MCCI_IN200100UV01.xsd");
		std::env::set_var("E2BR3_XSD_PATH", xsd_path);
	}
	Ok(())
}

pub async fn init_test_mm() -> Result<ModelManager> {
	init_test_env().await?;
	_dev_utils::init_dev().await;
	apply_test_authorization_isolation_migration().await?;
	let mm = ModelManager::new().await?;
	mm.dbx()
		.execute(sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto"))
		.await?;
	Ok(mm)
}

async fn apply_test_authorization_isolation_migration() -> Result<()> {
	static APPLIED: OnceCell<()> = OnceCell::const_new();
	APPLIED
		.get_or_try_init(|| async {
			let database_url = std::env::var("SERVICE_MIGRATION_DB_URL")
				.or_else(|_| std::env::var("SERVICE_DB_URL"))
				.map_err(|error| sqlx::Error::Configuration(Box::new(error)))?;
			let pool = PgPoolOptions::new()
				.max_connections(1)
				.connect(&database_url)
				.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260720_authorization_kernel.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260720_authorization_revisions.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260724_authorization_fact_locks.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260724_authorization_direct_grant_actions.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260724_authorization_operational_grant_actions.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260724_authorization_workflow_config_action.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260725_authorization_export_history_actions.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260725_authorization_built_in_role_assignment_actions.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260725_authorization_home_workflow_case_read.sql"
			))
			.execute(&pool)
			.await?;
			AuthorizationMigrationService::reconcile_registry_storage(
				&pool,
				policy_registry(),
			)
			.await
			.map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260722_authorization_isolation_audit.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260808_users_org_scoped_identity.sql"
			))
			.execute(&pool)
			.await?;
			sqlx::raw_sql(include_str!(
				"../../../../../db/migrations/20260810_global_user_identity.sql"
			))
			.execute(&pool)
			.await?;
			pool.close().await;
			Ok::<(), sqlx::Error>(())
		})
		.await?;
	Ok(())
}

pub async fn seed_active_test_meddra_term(mm: &ModelManager) -> Result<()> {
	let system_user_id = Uuid::parse_str(SYSTEM_USER_ID)?;
	let system_org_id = Uuid::parse_str(SYSTEM_ORG_ID)?;
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id).await?;
	set_org_context(&mut tx, system_org_id, ROLE_SYSTEM_ADMIN).await?;

	sqlx::query("UPDATE terminology_releases SET status='retired' WHERE dictionary='meddra' AND language='en' AND status='active' AND version <> '26.0'").execute(&mut *tx).await?;
	sqlx::query("INSERT INTO terminology_releases(dictionary,version,language,status,loaded_rows) VALUES ('meddra','26.0','en','active',1) ON CONFLICT(dictionary,version,language) DO UPDATE SET status='active'").execute(&mut *tx).await?;

	sqlx::query(
		"INSERT INTO meddra_terms (code, term, level, version, language, active)
		 VALUES ('10000001', 'Test reaction term', 'LLT', '26.0', 'en', true)
		 ON CONFLICT (code, version, language)
		 DO UPDATE SET term = EXCLUDED.term, level = EXCLUDED.level, active = true",
	)
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;
	Ok(())
}

pub fn system_user_id() -> Uuid {
	Uuid::parse_str(SYSTEM_USER_ID).expect("system user id")
}

pub fn system_org_id() -> Uuid {
	Uuid::parse_str(SYSTEM_ORG_ID).expect("system org id")
}

pub fn cookie_header(token: &str) -> String {
	format!("auth-token={token}")
}

pub async fn seed_org_with_users(
	mm: &ModelManager,
	admin_pwd: &str,
	viewer_pwd: &str,
) -> Result<SeedOrgUsers> {
	let org_id = insert_org(mm, system_user_id()).await?;
	let admin = insert_user(
		mm,
		org_id,
		ROLE_SPONSOR_ADMIN_CRO,
		system_user_id(),
		Some(admin_pwd),
	)
	.await?;
	let viewer = insert_user(
		mm,
		org_id,
		TEST_CUSTOM_VIEWER_ROLE,
		system_user_id(),
		Some(viewer_pwd),
	)
	.await?;

	Ok(SeedOrgUsers {
		org_id,
		admin,
		viewer,
	})
}

pub async fn create_test_import_product(
	mm: &ModelManager,
	org_id: Uuid,
	user_id: Uuid,
) -> Result<Uuid> {
	let ctx = lib_core::ctx::Ctx::new(
		user_id,
		org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let sender_id = SenderPresaveBmc::create(
		&ctx,
		mm,
		SenderPresaveForCreate {
			is_default: None,
			sender_type: Some("1".into()),
			organization_name: Some(format!("XML Import Sender {}", Uuid::new_v4())),
			organization_name_notation: None,
			street_address: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			fax: None,
			email: None,
		},
	)
	.await?;
	for (sequence_number, gateway_authority) in
		["ich", "fda", "mfds"].into_iter().enumerate()
	{
		SenderPresaveGatewayBmc::create(
			&ctx,
			mm,
			SenderPresaveGatewayForCreate {
				sender_presave_id: sender_id,
				sequence_number: sequence_number as i32 + 1,
				gateway_authority: gateway_authority.into(),
				sender_identifier: Some("TEST-SENDER".into()),
				routing_identifier: None,
				cde_sender_identifier: None,
				cdr_sender_identifier: None,
				is_default_for_authority: Some(true),
				deleted: None,
			},
		)
		.await?;
	}
	let mut settings = AdminSettingsBmc::get(&ctx, mm, "system")
		.await?
		.ok_or("missing test organization settings")?;
	settings["apply_sender_info_to_imported_cases"] = serde_json::Value::Bool(true);
	AdminSettingsBmc::upsert(&ctx, mm, "system", &settings, Some(user_id)).await?;
	Ok(ProductPresaveBmc::create(
		&ctx,
		mm,
		ProductPresaveForCreate {
			receiver_presave_id: None,
			sender_presave_id: Some(sender_id),
			product_id: Some(format!("XML-IMPORT-{}", Uuid::new_v4())),
			medicinal_product: Some("XML Import Product".into()),
			medicinal_product_notation: None,
			preapproval_ip_name: None,
			brand_name: None,
			original_manufacturer: None,
			product_description: None,
			mpid: None,
			mpid_version: None,
			mfds_mpid: None,
			mfds_mpid_version: None,
			phpid: None,
			phpid_version: None,
			investigational_product_blinded: None,
			obtain_drug_country: None,
			drug_authorization_number: None,
			drug_authorization_country: None,
			drug_authorization_holder: None,
			holder_applicant_name_notation: None,
		},
	)
	.await?)
}

pub async fn prepare_test_fda_export_case(
	mm: &ModelManager,
	org_id: Uuid,
	user_id: Uuid,
	case_id: Uuid,
) -> Result<()> {
	let ctx = Ctx::new(user_id, org_id, ROLE_SPONSOR_ADMIN_CRO.to_string())?;
	CaseBmc::update(
		&ctx,
		mm,
		case_id,
		CaseForUpdate {
			fda_report_type: Some("4".into()),
			..Default::default()
		},
	)
	.await?;
	Ok(())
}

pub async fn seed_company_org_with_users(
	mm: &ModelManager,
	admin_pwd: &str,
	viewer_pwd: &str,
) -> Result<SeedOrgUsers> {
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_SYSTEM_ADMIN)
		.await?;

	let org_id =
		insert_org_with_type(mm, system_user_id(), "pharmaceutical_company").await?;
	let admin = insert_user(
		mm,
		org_id,
		ROLE_SPONSOR_ADMIN_COMPANY,
		system_user_id(),
		Some(admin_pwd),
	)
	.await?;
	let viewer = insert_user(
		mm,
		org_id,
		TEST_CUSTOM_VIEWER_ROLE,
		system_user_id(),
		Some(viewer_pwd),
	)
	.await?;

	Ok(SeedOrgUsers {
		org_id,
		admin,
		viewer,
	})
}

pub async fn seed_org_with_admin_and_viewer(
	mm: &ModelManager,
	admin_pwd: &str,
	viewer_pwd: &str,
) -> Result<SeedOrgUsers> {
	let org_id = insert_org(mm, system_user_id()).await?;
	let admin = insert_user(
		mm,
		org_id,
		ROLE_SPONSOR_ADMIN_CRO,
		system_user_id(),
		Some(admin_pwd),
	)
	.await?;
	let viewer = insert_user(
		mm,
		org_id,
		TEST_CUSTOM_VIEWER_ROLE,
		system_user_id(),
		Some(viewer_pwd),
	)
	.await?;

	Ok(SeedOrgUsers {
		org_id,
		admin,
		viewer,
	})
}

pub async fn seed_org_with_all_roles(mm: &ModelManager) -> Result<SeedOrgAllRoles> {
	let org_id = insert_org(mm, system_user_id()).await?;
	let admin =
		insert_user(mm, org_id, ROLE_SPONSOR_ADMIN_CRO, system_user_id(), None)
			.await?;
	let manager =
		insert_user(mm, org_id, TEST_CUSTOM_MANAGER_ROLE, system_user_id(), None)
			.await?;
	let user = insert_user(mm, org_id, ROLE_USER, system_user_id(), None).await?;
	let viewer =
		insert_user(mm, org_id, TEST_CUSTOM_VIEWER_ROLE, system_user_id(), None)
			.await?;

	Ok(SeedOrgAllRoles {
		org_id,
		admin,
		manager,
		user,
		viewer,
	})
}

pub async fn seed_two_orgs_users_cases(
	mm: &ModelManager,
) -> Result<SeedOrgsUsersCases> {
	let org1_id = insert_org(mm, system_user_id()).await?;
	let org2_id = insert_org(mm, system_user_id()).await?;
	let user1 =
		insert_user(mm, org1_id, TEST_CUSTOM_VIEWER_ROLE, system_user_id(), None)
			.await?;
	let user2 =
		insert_user(mm, org2_id, TEST_CUSTOM_VIEWER_ROLE, system_user_id(), None)
			.await?;
	let case_org1 = insert_case(mm, org1_id, system_user_id()).await?;
	let case_org2 = insert_case(mm, org2_id, system_user_id()).await?;

	Ok(SeedOrgsUsersCases {
		org1_id,
		org2_id,
		user1,
		user2,
		case_org1,
		case_org2,
	})
}

pub async fn seed_two_orgs_manager_cases(
	mm: &ModelManager,
) -> Result<SeedOrgsManagerCases> {
	let org1_id = insert_org(mm, system_user_id()).await?;
	let org2_id = insert_org(mm, system_user_id()).await?;
	let manager = insert_user(
		mm,
		org1_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		None,
	)
	.await?;
	let user2 =
		insert_user(mm, org2_id, TEST_CUSTOM_VIEWER_ROLE, system_user_id(), None)
			.await?;
	let case_org1 = insert_case(mm, org1_id, system_user_id()).await?;
	let case_org2 = insert_case(mm, org2_id, system_user_id()).await?;

	Ok(SeedOrgsManagerCases {
		org1_id,
		org2_id,
		manager,
		user2,
		case_org1,
		case_org2,
	})
}

pub async fn insert_case_version(
	mm: &ModelManager,
	case_id: Uuid,
	version: i32,
	changed_by: Uuid,
) -> Result<()> {
	let snapshot = serde_json::json!({
		"id": case_id,
		"version": version,
	});
	let dbx = mm.dbx();
	dbx.begin_txn().await?;
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_SYSTEM_ADMIN)
		.await?;
	dbx.execute(
		sqlx::query(
			"INSERT INTO case_versions (case_id, version, snapshot, changed_by)
			 VALUES ($1, $2, $3, $4)",
		)
		.bind(case_id)
		.bind(version)
		.bind(snapshot)
		.bind(changed_by),
	)
	.await?;
	dbx.commit_txn().await?;
	Ok(())
}

async fn insert_org(mm: &ModelManager, created_by: Uuid) -> Result<Uuid> {
	insert_org_with_type(mm, created_by, "cro").await
}

async fn insert_org_with_type(
	mm: &ModelManager,
	created_by: Uuid,
	organization_type: &str,
) -> Result<Uuid> {
	let org_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, created_by).await?;
	set_org_context(&mut tx, system_org_id(), ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO organizations (id, name, org_type, address, contact_email, created_by, updated_by)
		 VALUES ($1, $2, $3, $4, $5, $6, $6)",
	)
	.bind(org_id)
	.bind(format!("RLS Org {org_id}"))
	.bind(organization_type)
	.bind("123 RLS St")
	.bind(format!("rls-org-{org_id}@example.com"))
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	lib_core::model::admin_settings::AdminSettingsBmc::ensure_default_for_org(
		&Ctx::root_ctx(),
		mm,
		org_id,
	)
	.await?;
	Ok(org_id)
}

async fn seed_normalized_role_assignment(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	user_id: Uuid,
	organization_id: Uuid,
	legacy_role: &str,
) -> Result<()> {
	let registry = policy_registry();
	let built_in_kind = match legacy_role {
		ROLE_SYSTEM_ADMIN => Some(BuiltInIdentityKind::PlatformAdministrator),
		ROLE_SPONSOR_ADMIN_CRO => Some(BuiltInIdentityKind::SponsorCroAdministrator),
		ROLE_SPONSOR_ADMIN_COMPANY => {
			Some(BuiltInIdentityKind::SponsorCompanyAdministrator)
		}
		ROLE_USER => Some(BuiltInIdentityKind::OperationalUser),
		_ => None,
	};
	let built_in = registry
		.built_in_identities()
		.iter()
		.find(|identity| Some(identity.kind) == built_in_kind);
	let role_id = if let Some(identity) = built_in {
		identity.id
	} else {
		let preferred =
			Uuid::parse_str(legacy_role).unwrap_or_else(|_| Uuid::new_v4());
		let preferred_role = sqlx::query_as::<_, (Option<Uuid>, bool)>(
			"SELECT organization_id, built_in FROM authorization_roles WHERE id = $1",
		)
		.bind(preferred)
		.fetch_optional(&mut **tx)
		.await?;
		let role_id = match preferred_role {
			Some((Some(owner), false)) if owner == organization_id => preferred,
			None => preferred,
			_ => Uuid::new_v4(),
		};
		let privileges =
			test_role_privileges(tx, organization_id, legacy_role).await?;
		let grant_ids = grant_ids_for_menu_privileges(&privileges, false)
			.map_err(|error| format!("invalid test role privileges: {error:?}"))?;
		let grant_ids = grant_ids
			.into_iter()
			.map(|grant| grant.to_string())
			.collect::<Vec<_>>();
		sqlx::query("SELECT authz_upsert_custom_role($1, $2, $3, true, $4)")
			.bind(role_id)
			.bind(organization_id)
			.bind(format!("test-role-{role_id}"))
			.bind(grant_ids)
			.execute(&mut **tx)
			.await?;
		role_id
	};

	sqlx::query("SELECT authz_assign_user_role($1, $2, $3)")
		.bind(user_id)
		.bind(organization_id)
		.bind(role_id)
		.execute(&mut **tx)
		.await?;
	Ok(())
}

async fn test_role_privileges(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	organization_id: Uuid,
	legacy_role: &str,
) -> Result<Vec<AdminMenuPrivilege>> {
	if legacy_role == TEST_CUSTOM_VIEWER_ROLE {
		return Ok(vec![AdminMenuPrivilege {
			menu_key: "case".to_string(),
			can_read: true,
			can_edit: false,
			can_review: false,
			can_lock: false,
		}]);
	}
	if legacy_role == TEST_CUSTOM_MANAGER_ROLE {
		return Ok(["case", "info", "import", "export_submission"]
			.into_iter()
			.map(|menu_key| AdminMenuPrivilege {
				menu_key: menu_key.to_string(),
				can_read: true,
				can_edit: false,
				can_review: false,
				can_lock: false,
			})
			.collect());
	}
	let Some(role_id) = Uuid::parse_str(legacy_role).ok() else {
		return Ok(Vec::new());
	};
	let raw = sqlx::query_scalar::<_, serde_json::Value>(
		"SELECT privileges_json FROM permission_profiles WHERE id = $1 AND organization_id = $2 AND active",
	)
	.bind(role_id)
	.bind(organization_id)
	.fetch_optional(&mut **tx)
	.await?;
	match raw {
		Some(raw) => Ok(serde_json::from_value(raw)?),
		None => Ok(Vec::new()),
	}
}

pub async fn insert_user(
	mm: &ModelManager,
	org_id: Uuid,
	role: &str,
	created_by: Uuid,
	pwd_clear: Option<&str>,
) -> Result<SeedUser> {
	let user_id = Uuid::new_v4();
	let token_salt = Uuid::new_v4();
	let pwd_salt = Uuid::new_v4();
	let email = format!("rls-user-{user_id}@example.com");
	let username = format!("rls_user_{user_id}");
	let pwd = match pwd_clear {
		Some(clear) => Some(
			pwd::hash_pwd(ContentToHash {
				content: clear.to_string(),
				salt: pwd_salt,
			})
			.await?,
		),
		None => None,
	};

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, created_by).await?;
	set_org_context(&mut tx, system_org_id(), ROLE_SYSTEM_ADMIN).await?;
	let normalized_role = role;
	sqlx::query(
		"INSERT INTO users (id, organization_id, email, username, pwd, pwd_salt, token_salt, role, active, created_by, updated_by)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9, $9)",
	)
	.bind(user_id)
	.bind(org_id)
	.bind(&email)
	.bind(username)
	.bind(pwd)
	.bind(pwd_salt)
	.bind(token_salt)
	.bind(normalized_role)
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (user_id, organization_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $3)
		 ON CONFLICT (user_id, organization_id) DO NOTHING",
	)
	.bind(user_id)
	.bind(org_id)
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	seed_normalized_role_assignment(&mut tx, user_id, org_id, role).await?;
	tx.commit().await?;

	Ok(SeedUser {
		id: user_id,
		email,
		token_salt,
	})
}

pub async fn insert_user_organization_membership(
	mm: &ModelManager,
	user_id: Uuid,
	org_id: Uuid,
) -> Result<()> {
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, system_user_id()).await?;
	set_org_context(&mut tx, system_org_id(), ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO user_organization_memberships (user_id, organization_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $3)
		 ON CONFLICT (user_id, organization_id) DO NOTHING",
	)
	.bind(user_id)
	.bind(org_id)
	.bind(system_user_id())
	.execute(&mut *tx)
	.await?;
	let legacy_role =
		sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE id = $1")
			.bind(user_id)
			.fetch_one(&mut *tx)
			.await?;
	seed_normalized_role_assignment(&mut tx, user_id, org_id, &legacy_role).await?;
	tx.commit().await?;
	Ok(())
}

async fn insert_case(
	mm: &ModelManager,
	org_id: Uuid,
	created_by: Uuid,
) -> Result<Uuid> {
	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, created_by).await?;
	set_org_context(&mut tx, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let safety_report_id = format!("SR-TEST-{case_id}");
	sqlx::query(
		"INSERT INTO cases (id, organization_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $3)",
	)
	.bind(case_id)
	.bind(org_id)
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO safety_report_identification (case_id, safety_report_id, version, created_by, updated_by)
		 VALUES ($1, $2, 1, $3, $3)",
	)
	.bind(case_id)
	.bind(safety_report_id)
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(case_id)
}
