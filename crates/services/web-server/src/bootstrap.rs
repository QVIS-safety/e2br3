use crate::Result;
use lib_core::ctx::{Ctx, ROLE_ADMIN};
use lib_core::model::organization::OrganizationBmc;
use lib_core::model::user::{User, UserBmc, UserForCreate, UserForUpdate};
use lib_core::model::ModelManager;
use lib_core::model::Result as ModelResult;
use sqlx::types::Uuid;
use tracing::info;

const DEMO_EMAIL: &str = "demo.user@example.com";
const DEMO_PASSWORD: &str = "welcome";
const DEMO_USERNAME: &str = "demo_user";
const DEMO_ORG_ID: &str = "00000000-0000-0000-0000-000000000001";

pub async fn bootstrap_admin_user(mm: &ModelManager) -> Result<()> {
	let root_ctx = Ctx::root_ctx();
	let org_id = Uuid::parse_str(DEMO_ORG_ID).expect("invalid demo org id");
	if !org_exists(&root_ctx, mm, org_id).await? {
		return Ok(());
	}

	let existing: Option<User> = UserBmc::first_by_email(&root_ctx, mm, DEMO_EMAIL).await?;
	match existing {
		Some(user) => {
			UserBmc::update_pwd_and_clear_must_change(
				&root_ctx,
				mm,
				user.id,
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
			UserBmc::update(&root_ctx, mm, user.id, user_u).await?;
			info!(
				"BOOTSTRAP - synced demo admin user {}",
				DEMO_EMAIL
			);
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

async fn org_exists(
	ctx: &Ctx,
	mm: &ModelManager,
	org_id: Uuid,
) -> ModelResult<bool> {
	match OrganizationBmc::get(ctx, mm, org_id).await {
		Ok(_) => Ok(true),
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => Ok(false),
		Err(err) => Err(err),
	}
}
