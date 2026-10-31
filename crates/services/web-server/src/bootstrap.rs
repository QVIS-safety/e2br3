use crate::Result;
use lib_core::ctx::{
	Ctx, ROLE_SPONSOR_ADMIN_COMPANY, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN,
	SYSTEM_ORG_ID,
};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::user::{UserBmc, UserForCreate, UserForUpdate};
use lib_core::model::ModelManager;
use lib_core::model::Result as ModelResult;
use sqlx::query;
use sqlx::query_as;
use sqlx::types::Uuid;
use tracing::info;

const INITIAL_ADMIN_EMAIL: &str = "hdh4063@gmail.com";
const INITIAL_ADMIN_USERNAME: &str = "hdh4063";
const INITIAL_PASSWORD: &str = "welcome";
const LEGACY_DEMO_EMAIL: &str = "demo.user@example.com";
const DEMO_CRO_ORG_ID: &str = "00000000-0000-0000-0000-000000000001";
const DEMO_COMPANY_ORG_ID: &str = "00000000-0000-0000-0000-000000000002";
const DEMO_CRO_SENDER_ID: &str = "00000000-0000-0000-0000-000000000003";
const DEMO_CRO_ADMIN_EMAIL: &str = "demo.cro.admin@example.com";
const DEMO_CRO_ADMIN_USERNAME: &str = "demo_cro_admin";
const DEMO_COMPANY_ADMIN_EMAIL: &str = "demo.company.admin@example.com";
const DEMO_COMPANY_ADMIN_USERNAME: &str = "demo_company_admin";

pub fn admin_bootstrap_enabled(
	value: std::result::Result<String, std::env::VarError>,
) -> Result<bool> {
	let value = value.map_err(|err| {
		crate::Error::Config(format!("SERVICE_BOOTSTRAP_ENABLED is required: {err}"))
	})?;
	value.parse::<bool>().map_err(|_| {
		crate::Error::Config(
			"SERVICE_BOOTSTRAP_ENABLED must be true or false".to_string(),
		)
	})
}

#[test]
fn admin_bootstrap_requires_an_explicit_boolean() {
	assert!(matches!(
		admin_bootstrap_enabled(Ok("false".into())),
		Ok(false)
	));
	assert!(matches!(
		admin_bootstrap_enabled(Ok("true".into())),
		Ok(true)
	));
	assert!(admin_bootstrap_enabled(Err(std::env::VarError::NotPresent)).is_err());
	assert!(admin_bootstrap_enabled(Ok("yes".into())).is_err());
}

pub async fn bootstrap_admin_user(mm: &ModelManager) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	migrate_legacy_demo_user(mm).await?;
	sync_org(
		mm,
		Uuid::parse_str(SYSTEM_ORG_ID).expect("invalid system org id"),
		"System",
		"Internal",
		"system@e2br3.local",
	)
	.await?;
	let cro_org_id = Uuid::parse_str(DEMO_CRO_ORG_ID).expect("invalid CRO org id");
	let company_org_id =
		Uuid::parse_str(DEMO_COMPANY_ORG_ID).expect("invalid company org id");

	sync_org(
		mm,
		cro_org_id,
		"Demo CRO Organization",
		"cro",
		"demo-cro@example.com",
	)
	.await?;
	sync_org(
		mm,
		company_org_id,
		"Demo Pharmaceutical Company",
		"pharmaceutical_company",
		"demo-company@example.com",
	)
	.await?;
	sync_sender_presave(
		mm,
		cro_org_id,
		Uuid::parse_str(DEMO_CRO_SENDER_ID).expect("invalid CRO sender id"),
	)
	.await?;
	sync_user(
		&root_ctx,
		mm,
		Uuid::parse_str(SYSTEM_ORG_ID).expect("invalid system org id"),
		INITIAL_ADMIN_EMAIL,
		INITIAL_ADMIN_USERNAME,
		ROLE_SYSTEM_ADMIN,
		"Bootstrap platform system administrator",
	)
	.await?;
	sync_user(
		&root_ctx,
		mm,
		cro_org_id,
		DEMO_CRO_ADMIN_EMAIL,
		DEMO_CRO_ADMIN_USERNAME,
		ROLE_SPONSOR_ADMIN_CRO,
		"Bootstrap demo CRO sponsor administrator",
	)
	.await?;
	sync_user(
		&root_ctx,
		mm,
		company_org_id,
		DEMO_COMPANY_ADMIN_EMAIL,
		DEMO_COMPANY_ADMIN_USERNAME,
		ROLE_SPONSOR_ADMIN_COMPANY,
		"Bootstrap demo company sponsor administrator",
	)
	.await?;

	Ok(())
}

async fn sync_sender_presave(
	mm: &ModelManager,
	organization_id: Uuid,
	sender_id: Uuid,
) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	mm.dbx().begin_txn().await.map_err(dbx_into_web)?;
	set_full_context_dbx(
		mm.dbx(),
		root_ctx.user_id(),
		root_ctx.organization_id(),
		root_ctx.role(),
	)
	.await
	.map_err(dbx_into_web)?;
	let result = mm
		.dbx()
		.execute(
			query(
				r#"
				INSERT INTO sender_presaves (
					id, organization_id, deleted, is_default, sender_type,
					organization_name, created_by, updated_by
				) VALUES (
					$1, $2, false, true, '1', 'Demo Sender Organization', $3, $3
				)
				ON CONFLICT (id) DO UPDATE
				SET organization_id = EXCLUDED.organization_id,
					deleted = false,
					is_default = true,
					sender_type = EXCLUDED.sender_type,
					organization_name = EXCLUDED.organization_name,
					updated_by = EXCLUDED.updated_by,
					updated_at = NOW()
				"#,
			)
			.bind(sender_id)
			.bind(organization_id)
			.bind(root_ctx.user_id()),
		)
		.await;
	match result {
		Ok(_) => {
			mm.dbx().commit_txn().await.map_err(dbx_into_web)?;
			info!(
				"BOOTSTRAP - synced demo sender {} with id {}",
				"Demo Sender Organization", sender_id
			);
		}
		Err(err) => {
			mm.dbx().rollback_txn().await.map_err(dbx_into_web)?;
			return Err(dbx_into_web(err));
		}
	}
	Ok(())
}

async fn migrate_legacy_demo_user(mm: &ModelManager) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	mm.dbx().begin_txn().await.map_err(dbx_into_web)?;
	set_full_context_dbx(
		mm.dbx(),
		root_ctx.user_id(),
		root_ctx.organization_id(),
		root_ctx.role(),
	)
	.await?;
	let result = mm
		.dbx()
		.execute(
			query(
				r#"
				UPDATE users
				SET email = CONCAT('removed-demo-user+', id::text, '@example.invalid'),
					username = CONCAT('removed_demo_user_', REPLACE(id::text, '-', '_')),
					active = false,
					updated_by = $1,
					updated_at = NOW()
				WHERE lower(email) = lower($2)
				"#,
			)
			.bind(root_ctx.user_id())
			.bind(LEGACY_DEMO_EMAIL),
		)
		.await;
	match result {
		Ok(_) => mm.dbx().commit_txn().await.map_err(dbx_into_web)?,
		Err(err) => {
			mm.dbx().rollback_txn().await.map_err(dbx_into_web)?;
			return Err(dbx_into_web(err));
		}
	}
	Ok(())
}

async fn sync_org(
	mm: &ModelManager,
	org_id: Uuid,
	name: &str,
	org_type: &str,
	contact_email: &str,
) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	mm.dbx().begin_txn().await.map_err(dbx_into_web)?;
	set_full_context_dbx(
		mm.dbx(),
		root_ctx.user_id(),
		root_ctx.organization_id(),
		root_ctx.role(),
	)
	.await?;
	let result = mm
		.dbx()
		.execute(
			query(
				r#"
				WITH synced_org AS (
					INSERT INTO organizations (
						id, name, org_type, country_code, contact_email, active,
						created_by, created_at, updated_by, updated_at
					) VALUES (
						$1, $2, $3, 'KR', $4, true, $5, NOW(), $5, NOW()
					)
					ON CONFLICT (id) DO UPDATE
					SET name = EXCLUDED.name,
						org_type = EXCLUDED.org_type,
						contact_email = EXCLUDED.contact_email,
						active = true,
						updated_by = EXCLUDED.updated_by,
						updated_at = NOW()
					RETURNING id
				)
				INSERT INTO app_settings (
					organization_id, key, value, updated_by
				)
				SELECT id, 'system', app_settings_default_value(), $5
				FROM synced_org
				ON CONFLICT (organization_id, key) DO NOTHING
				"#,
			)
			.bind(org_id)
			.bind(name)
			.bind(org_type)
			.bind(contact_email)
			.bind(root_ctx.user_id()),
		)
		.await;
	match result {
		Ok(_) => {
			mm.dbx().commit_txn().await.map_err(dbx_into_web)?;
			info!(
				"BOOTSTRAP - synced organization {} with id {}",
				name, org_id
			);
		}
		Err(err) => {
			mm.dbx().rollback_txn().await.map_err(dbx_into_web)?;
			return Err(dbx_into_web(err));
		}
	}
	Ok(())
}

async fn sync_user(
	ctx: &Ctx,
	mm: &ModelManager,
	organization_id: Uuid,
	email: &str,
	username: &str,
	role: &str,
	comments: &str,
) -> ModelResult<()> {
	let existing_user_id =
		find_user_id_by_email(ctx, mm, email, organization_id).await?;

	match existing_user_id {
		Some(user_id) => {
			sync_user_organization(ctx, mm, user_id, organization_id).await?;
			let user_u = UserForUpdate {
				organization_id: None,
				email: Some(email.to_string()),
				username: Some(username.to_string()),
				role: Some(role.to_string()),
				comments: Some(comments.to_string()),
				other_information: None,
				access_start_at: None,
				access_end_at: None,
				active_sender_identifier: None,
				access_sender_ids: None,
				access_product_ids: None,
				access_study_ids: None,
				access_blind_allowed: None,
				active: Some(true),
				last_login_at: None,
			};
			UserBmc::update(ctx, mm, user_id, user_u).await?;
			UserBmc::update_pwd_and_clear_must_change(
				ctx,
				mm,
				user_id,
				INITIAL_PASSWORD,
			)
			.await?;
			info!("BOOTSTRAP - synced initial user {}", email);
		}
		None => {
			let create = UserForCreate {
				organization_id,
				email: email.to_string(),
				username: Some(username.to_string()),
				pwd_clear: INITIAL_PASSWORD.to_string(),
				role: Some(role.to_string()),
				comments: Some(comments.to_string()),
				other_information: None,
				access_start_at: None,
				access_end_at: None,
				active_sender_identifier: None,
				access_sender_ids: None,
				access_product_ids: None,
				access_study_ids: None,
				access_blind_allowed: None,
			};
			let user_id = UserBmc::create(ctx, mm, create).await?;
			info!(
				"BOOTSTRAP - created initial user {} with id {}",
				email, user_id
			);
		}
	}
	Ok(())
}

async fn find_user_id_by_email(
	ctx: &Ctx,
	mm: &ModelManager,
	email: &str,
	organization_id: Uuid,
) -> ModelResult<Option<Uuid>> {
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_dbx(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		mm.dbx().rollback_txn().await?;
		return Err(err);
	}
	let result = mm
		.dbx()
		.fetch_optional(
			query_as::<_, (Uuid,)>(
				"SELECT id FROM users
				 WHERE lower(email) = lower($1)
				   AND organization_id = $2
				 LIMIT 1",
			)
			.bind(email)
			.bind(organization_id),
		)
		.await;
	match result {
		Ok(row) => {
			mm.dbx().commit_txn().await?;
			Ok(row.map(|(id,)| id))
		}
		Err(err) => {
			mm.dbx().rollback_txn().await?;
			Err(err.into())
		}
	}
}

async fn sync_user_organization(
	ctx: &Ctx,
	mm: &ModelManager,
	user_id: Uuid,
	organization_id: Uuid,
) -> ModelResult<()> {
	mm.dbx().begin_txn().await?;
	if let Err(err) = set_full_context_dbx(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		mm.dbx().rollback_txn().await?;
		return Err(err);
	}
	let result = mm
		.dbx()
		.execute(
			query(
				"UPDATE users SET organization_id = $1, updated_by = $2, updated_at = NOW() WHERE id = $3",
			)
			.bind(organization_id)
			.bind(ctx.user_id())
			.bind(user_id),
		)
		.await;
	match result {
		Ok(_) => {
			UserBmc::ensure_organization_membership(
				ctx,
				mm,
				user_id,
				organization_id,
			)
			.await?;
			mm.dbx().commit_txn().await?
		}
		Err(err) => {
			mm.dbx().rollback_txn().await?;
			return Err(err.into());
		}
	}
	Ok(())
}

fn dbx_into_web<E: core::fmt::Display>(err: E) -> crate::Error {
	crate::Error::Config(err.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::_dev_utils;
	use serial_test::serial;

	async fn init_bootstrap_test_mm() -> ModelManager {
		let database_url = std::env::var("SERVICE_DB_URL").unwrap_or_else(|_| {
			"postgres://app_user:dev_only_pwd@localhost/app_db".to_string()
		});
		std::env::set_var("SERVICE_DB_URL", &database_url);
		let migration_database_url = std::env::var("SERVICE_MIGRATION_DB_URL")
			.unwrap_or_else(|_| database_url.clone());
		std::env::set_var("SERVICE_MIGRATION_DB_URL", migration_database_url);
		std::env::set_var("SERVICE_WEB_FOLDER", "web-folder");
		std::env::set_var("SERVICE_PWD_KEY", "ZmFrZV9rZXk");
		std::env::set_var("SERVICE_TOKEN_KEY", "ZmFrZV9rZXk");
		std::env::set_var("SERVICE_TOKEN_DURATION_SEC", "3600");
		_dev_utils::init_dev().await;
		let mm = ModelManager::new().await.expect("test model manager");
		web_server::initialize_authorization_storage()
			.await
			.expect("test authorization storage");
		mm
	}

	#[tokio::test]
	#[serial]
	async fn bootstrap_admin_user_is_idempotent_when_demo_user_exists() {
		let mm = init_bootstrap_test_mm().await;

		bootstrap_admin_user(&mm)
			.await
			.expect("bootstrap should sync an existing demo user");
	}

	#[tokio::test]
	#[serial]
	async fn bootstrap_admin_user_creates_platform_and_demo_org_admins() {
		let mm = init_bootstrap_test_mm().await;

		bootstrap_admin_user(&mm)
			.await
			.expect("bootstrap should sync initial users");

		let root_ctx = Ctx::root_ctx();
		mm.dbx().begin_txn().await.expect("begin query transaction");
		set_full_context_dbx(
			mm.dbx(),
			root_ctx.user_id(),
			root_ctx.organization_id(),
			root_ctx.role(),
		)
		.await
		.expect("set root query context");
		let rows = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, (String, String, String, String)>(
				r#"
			SELECT u.email, u.role, u.organization_id::text, COALESCE(o.org_type, '')
			FROM users u
			LEFT JOIN organizations o ON o.id = u.organization_id
			WHERE lower(u.email) IN (
				'hdh4063@gmail.com',
				'demo.cro.admin@example.com',
				'demo.company.admin@example.com',
				'demo.user@example.com'
			)
			ORDER BY u.email
			"#,
			))
			.await
			.expect("query seeded users");
		mm.dbx()
			.commit_txn()
			.await
			.expect("commit query transaction");

		assert!(
			!rows
				.iter()
				.any(|(email, _, _, _)| email == "demo.user@example.com"),
			"legacy demo user should not be bootstrapped"
		);
		assert!(
			rows.iter().any(|(email, role, org_id, org_type)| {
				email == "hdh4063@gmail.com"
					&& role == "system_admin"
					&& org_id.as_str() == lib_core::ctx::SYSTEM_ORG_ID
					&& org_type == "Internal"
			}),
			"{rows:?}"
		);
		assert!(
			rows.iter().any(|(email, role, _, org_type)| {
				email == "demo.cro.admin@example.com"
					&& role == "sponsor_admin_cro"
					&& org_type == "cro"
			}),
			"{rows:?}"
		);
		assert!(
			rows.iter().any(|(email, role, _, org_type)| {
				email == "demo.company.admin@example.com"
					&& role == "sponsor_admin_company"
					&& org_type == "pharmaceutical_company"
			}),
			"{rows:?}"
		);

		mm.dbx().begin_txn().await.expect("begin sender query");
		set_full_context_dbx(
			mm.dbx(),
			root_ctx.user_id(),
			root_ctx.organization_id(),
			root_ctx.role(),
		)
		.await
		.expect("set sender query context");
		let sender = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, (Uuid, Uuid, String)>(
				"SELECT id, organization_id, organization_name FROM sender_presaves WHERE id = $1",
			)
			.bind(Uuid::parse_str(DEMO_CRO_SENDER_ID).expect("invalid CRO sender id")))
			.await
			.expect("query bootstrapped sender");
		mm.dbx().commit_txn().await.expect("commit sender query");
		assert_eq!(
			sender,
			Some((
				Uuid::parse_str(DEMO_CRO_SENDER_ID).expect("invalid CRO sender id"),
				Uuid::parse_str(DEMO_CRO_ORG_ID).expect("invalid CRO org id"),
				"Demo Sender Organization".to_string(),
			))
		);

		mm.dbx().begin_txn().await.expect("begin settings query");
		set_full_context_dbx(
			mm.dbx(),
			root_ctx.user_id(),
			root_ctx.organization_id(),
			root_ctx.role(),
		)
		.await
		.expect("set settings query context");
		let settings = mm
			.dbx()
			.fetch_all(
				sqlx::query_as::<_, (Uuid,)>(
					r#"
				SELECT organization_id
				FROM app_settings
				WHERE key = 'system'
				  AND organization_id IN ($1, $2, $3)
				ORDER BY organization_id
				"#,
				)
				.bind(Uuid::parse_str(SYSTEM_ORG_ID).expect("invalid system org id"))
				.bind(Uuid::parse_str(DEMO_CRO_ORG_ID).expect("invalid CRO org id"))
				.bind(
					Uuid::parse_str(DEMO_COMPANY_ORG_ID)
						.expect("invalid company org id"),
				),
			)
			.await
			.expect("query bootstrapped settings");
		mm.dbx().commit_txn().await.expect("commit settings query");
		assert_eq!(
			settings.len(),
			3,
			"all bootstrap organizations need settings"
		);
	}
}
