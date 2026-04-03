#![allow(dead_code)]

use lib_auth::pwd::{self, ContentToHash};
use lib_core::_dev_utils;
use lib_core::ctx::{
	ROLE_ADMIN, ROLE_MANAGER, ROLE_USER, ROLE_VIEWER, SYSTEM_ORG_ID, SYSTEM_USER_ID,
};
use lib_core::model::store::{
	set_full_context_dbx, set_org_context, set_user_context,
};
use lib_core::model::ModelManager;
use std::path::PathBuf;
use uuid::Uuid;

pub type Result<T> =
	core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

pub async fn init_test_env() {
	std::env::set_var(
		"SERVICE_DB_URL",
		"postgres://app_user:dev_only_pwd@localhost/app_db",
	);
	std::env::set_var("SERVICE_WEB_FOLDER", "web-folder");
	std::env::set_var("SERVICE_PWD_KEY", "ZmFrZV9rZXk");
	std::env::set_var("SERVICE_TOKEN_KEY", "ZmFrZV9rZXk");
	std::env::set_var("SERVICE_TOKEN_DURATION_SEC", "3600");
	std::env::set_var("E2BR3_DEBUG_ERRORS", "1");

	// Keep integration tests deterministic even when direnv isn't loaded.
	if std::env::var("E2BR3_EXAMPLES_DIR").is_err() {
		let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../..")
			.join("docs/refs/instances");
		std::env::set_var("E2BR3_EXAMPLES_DIR", examples_dir);
	}
	if std::env::var("E2BR3_XSD_PATH").is_err() {
		let xsd_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../..")
			.join("deploy/ec2/schemas/multicacheschemas/MCCI_IN200100UV01.xsd");
		std::env::set_var("E2BR3_XSD_PATH", xsd_path);
	}
}

pub async fn init_test_mm() -> Result<ModelManager> {
	init_test_env().await;
	_dev_utils::init_dev().await;
	_dev_utils::ensure_dev_schema_compatibility()
		.await
		.map_err(|err| format!("{err}"))?;
	let mm = ModelManager::new().await?;
	mm.dbx()
		.execute(sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto"))
		.await?;
	Ok(mm)
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
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_ADMIN).await?;

	let org_id = insert_org(mm, system_user_id()).await?;
	let admin =
		insert_user(mm, org_id, ROLE_ADMIN, system_user_id(), Some(admin_pwd))
			.await?;
	let viewer =
		insert_user(mm, org_id, ROLE_VIEWER, system_user_id(), Some(viewer_pwd))
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
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_ADMIN).await?;

	let org_id = insert_org(mm, system_user_id()).await?;
	let admin =
		insert_user(mm, org_id, ROLE_ADMIN, system_user_id(), Some(admin_pwd))
			.await?;
	let viewer =
		insert_user(mm, org_id, ROLE_VIEWER, system_user_id(), Some(viewer_pwd))
			.await?;

	Ok(SeedOrgUsers {
		org_id,
		admin,
		viewer,
	})
}

pub async fn seed_org_with_all_roles(mm: &ModelManager) -> Result<SeedOrgAllRoles> {
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_ADMIN).await?;

	let org_id = insert_org(mm, system_user_id()).await?;
	let admin = insert_user(mm, org_id, ROLE_ADMIN, system_user_id(), None).await?;
	let manager =
		insert_user(mm, org_id, ROLE_MANAGER, system_user_id(), None).await?;
	let user = insert_user(mm, org_id, ROLE_USER, system_user_id(), None).await?;
	let viewer =
		insert_user(mm, org_id, ROLE_VIEWER, system_user_id(), None).await?;

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
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_ADMIN).await?;

	let org1_id = insert_org(mm, system_user_id()).await?;
	let org2_id = insert_org(mm, system_user_id()).await?;
	let user1 =
		insert_user(mm, org1_id, ROLE_VIEWER, system_user_id(), None).await?;
	let user2 =
		insert_user(mm, org2_id, ROLE_VIEWER, system_user_id(), None).await?;
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
	let dbx = mm.dbx();
	set_full_context_dbx(dbx, system_user_id(), system_org_id(), ROLE_ADMIN).await?;

	let org1_id = insert_org(mm, system_user_id()).await?;
	let org2_id = insert_org(mm, system_user_id()).await?;
	let manager =
		insert_user(mm, org1_id, "manager", system_user_id(), None).await?;
	let user2 =
		insert_user(mm, org2_id, ROLE_VIEWER, system_user_id(), None).await?;
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
	mm.dbx()
		.execute(
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
	Ok(())
}

async fn insert_org(mm: &ModelManager, created_by: Uuid) -> Result<Uuid> {
	let org_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, created_by).await?;
	set_org_context(&mut tx, system_org_id(), ROLE_ADMIN).await?;
	sqlx::query(
		"INSERT INTO organizations (id, name, org_type, address, contact_email, created_by, updated_by)
		 VALUES ($1, $2, $3, $4, $5, $6, $6)",
	)
	.bind(org_id)
	.bind(format!("RLS Org {org_id}"))
	.bind("internal")
	.bind("123 RLS St")
	.bind(format!("rls-org-{org_id}@example.com"))
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(org_id)
}

async fn insert_user(
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
	set_org_context(&mut tx, system_org_id(), ROLE_ADMIN).await?;
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
	.bind(role)
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	Ok(SeedUser {
		id: user_id,
		email,
		token_salt,
	})
}

async fn insert_case(
	mm: &ModelManager,
	org_id: Uuid,
	created_by: Uuid,
) -> Result<Uuid> {
	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, created_by).await?;
	set_org_context(&mut tx, org_id, ROLE_ADMIN).await?;
	sqlx::query(
		"INSERT INTO cases (id, organization_id, safety_report_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $4, $4)",
	)
	.bind(case_id)
	.bind(org_id)
	.bind(format!("SR-TEST-{case_id}"))
	.bind(created_by)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(case_id)
}
