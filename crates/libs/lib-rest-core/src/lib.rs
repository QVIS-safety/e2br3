// region:    --- Modules

mod error;
pub mod rest_params;
pub mod rest_result;
mod utils;

pub use self::error::{Error, Result};
pub use rest_params::*;
pub use rest_result::*;

use lib_core::ctx::Ctx;
use lib_core::model::acs::{has_permission, Permission};
use lib_core::model::case::CaseBmc;
use lib_core::model::ModelManager;
use uuid::Uuid;

pub fn require_permission(ctx: &Ctx, permission: Permission) -> Result<()> {
	if !has_permission(ctx.role(), permission) {
		return Err(Error::PermissionDenied {
			required_permission: format!("{permission}"),
		});
	}
	Ok(())
}

pub async fn require_case_write_allowed(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let status = case.status.trim();
	if status.eq_ignore_ascii_case("reviewed")
		|| status.eq_ignore_ascii_case("locked")
	{
		return Err(Error::BadRequest {
			message: "reviewed and locked cases are read-only".to_string(),
		});
	}
	Ok(())
}

pub mod prelude;

// endregion: --- Modules
