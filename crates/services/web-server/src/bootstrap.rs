use crate::Result;
use lib_core::ctx::{Ctx, ROLE_ADMIN};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::user::{UserBmc, UserForCreate, UserForUpdate};
use lib_core::model::ModelManager;
use lib_core::model::Result as ModelResult;
use sqlx::query;
use sqlx::query_as;
use sqlx::types::Uuid;
use tracing::info;

const DEMO_EMAIL: &str = "demo.user@example.com";
const DEMO_PASSWORD: &str = "welcome";
const DEMO_USERNAME: &str = "demo_user";
const DEMO_ORG_ID: &str = "00000000-0000-0000-0000-000000000001";

pub async fn bootstrap_admin_user(mm: &ModelManager) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	let org_id = Uuid::parse_str(DEMO_ORG_ID).expect("invalid demo org id");
	mm.dbx().begin_txn().await.map_err(dbx_into_web)?;
	if let Err(err) = set_full_context_dbx(
		mm.dbx(),
		root_ctx.user_id(),
		root_ctx.organization_id(),
		root_ctx.role(),
	)
	.await
	{
		mm.dbx().rollback_txn().await.map_err(dbx_into_web)?;
		return Err(err.into());
	}
	if !org_exists(mm, org_id).await? {
		let insert = query(
			r#"
			INSERT INTO organizations (
				id, name, org_type, address, city, state, postcode, country_code,
				contact_email, contact_phone, active, created_by, created_at, updated_at
			) VALUES (
				$1, $2, $3, $4, $5, $6, $7, $8,
				$9, $10, $11, $12, NOW(), NOW()
			)
			ON CONFLICT (id) DO NOTHING
			"#,
		)
		.bind(org_id)
		.bind("Demo Organization")
		.bind("internal")
		.bind("123 Demo St")
		.bind("Metropolis")
		.bind("CA")
		.bind("12345")
		.bind("US")
		.bind("demo@example.com")
		.bind("555-1234")
		.bind(true)
		.bind(root_ctx.user_id());
		if let Err(err) = mm.dbx().execute(insert).await {
			mm.dbx().rollback_txn().await.map_err(dbx_into_web)?;
			return Err(dbx_into_web(err));
		}
		mm.dbx().commit_txn().await.map_err(dbx_into_web)?;
		info!(
			"BOOTSTRAP - created demo organization {} with id {}",
			"Demo Organization", org_id
		);
	}
	let existing_user_id: Option<(Uuid,)> = mm
		.dbx()
		.fetch_optional(
			query_as::<_, (Uuid,)>(
				"SELECT id FROM users WHERE lower(email) = lower($1) LIMIT 1",
			)
			.bind(DEMO_EMAIL),
		)
		.await
		.map_err(|err| crate::Error::Model(lib_core::model::Error::Dbx(err)))?;
	mm.dbx().commit_txn().await.map_err(dbx_into_web)?;

	match existing_user_id {
		Some((user_id,)) => {
			UserBmc::update_pwd_and_clear_must_change(
				&root_ctx,
				mm,
				user_id,
				DEMO_PASSWORD,
			)
			.await?;

			let user_u = UserForUpdate {
				email: Some(DEMO_EMAIL.to_string()),
				username: Some(DEMO_USERNAME.to_string()),
				role: Some(ROLE_ADMIN.to_string()),
				first_name: None,
				last_name: None,
				comments: None,
				other_information: None,
				access_start_at: None,
				access_end_at: None,
				access_sender_ids: None,
				access_product_ids: None,
				access_study_ids: None,
				active: Some(true),
				last_login_at: None,
			};
			UserBmc::update(&root_ctx, mm, user_id, user_u).await?;
			info!("BOOTSTRAP - synced demo admin user {}", DEMO_EMAIL);
		}
		None => {
			let create = UserForCreate {
				organization_id: org_id,
				email: DEMO_EMAIL.to_string(),
				username: Some(DEMO_USERNAME.to_string()),
				pwd_clear: DEMO_PASSWORD.to_string(),
				role: Some(ROLE_ADMIN.to_string()),
				first_name: None,
				last_name: None,
				comments: Some("Bootstrap demo admin user".to_string()),
				other_information: None,
				access_start_at: None,
				access_end_at: None,
				access_sender_ids: None,
				access_product_ids: None,
				access_study_ids: None,
			};
			let user_id = UserBmc::create(&root_ctx, mm, create).await?;
			info!(
				"BOOTSTRAP - created demo admin user {} with id {}",
				DEMO_EMAIL, user_id
			);
		}
	}

	Ok(())
}

async fn org_exists(mm: &ModelManager, org_id: Uuid) -> ModelResult<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			query_as::<_, (bool,)>(
				"SELECT EXISTS (SELECT 1 FROM organizations WHERE id = $1)",
			)
			.bind(org_id),
		)
		.await?;
	Ok(exists)
}

fn dbx_into_web<E: core::fmt::Display>(err: E) -> crate::Error {
	crate::Error::Config(err.to_string())
}
