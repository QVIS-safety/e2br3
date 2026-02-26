use crate::error::{Error, Result};
use crate::utils::token;
use axum::extract::State;
use axum::Json;
use lib_auth::pwd::{self, ContentToHash, SchemeStatus};
use lib_core::ctx::{Ctx, ROLE_ADMIN};
use lib_core::model::user::{UserBmc, UserForAuth, UserForLogin};
use lib_core::model::ModelManager;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_cookies::Cookies;
use tracing::debug;

// region:    --- Login
pub async fn api_login_handler(
	State(mm): State<ModelManager>,
	cookies: Cookies,
	Json(payload): Json<LoginPayload>,
) -> Result<Json<Value>> {
	debug!("{:<12} - api_login_handler", "HANDLER");

	let LoginPayload {
		email,
		pwd: pwd_clear,
	} = payload;
	// -- Get the user (set auth email in-session to satisfy RLS).
	let user: UserForLogin = UserBmc::auth_login_by_email(&mm, &email)
		.await
		.map_err(Error::Model)?
		.ok_or(Error::LoginFailEmailNotFound)?;
	let user_id = user.id;

	// Reject malformed/legacy accounts that cannot build an authenticated context.
	Ctx::new(user.id, user.organization_id, user.role.clone())
		.map_err(|_| Error::LoginFailUserCtxCreate { user_id })?;

	// -- Validate the password.
	let Some(pwd) = user.pwd.clone() else {
		return Err(Error::LoginFailUserHasNoPwd { user_id });
	};

	let scheme_status = pwd::validate_pwd(
		ContentToHash {
			salt: user.pwd_salt,
			content: pwd_clear.clone(),
		},
		pwd,
	)
	.await
	.map_err(|_| Error::LoginFailPwdNotMatching { user_id })?;

	// -- Update password scheme if needed
	if let SchemeStatus::Outdated = scheme_status {
		debug!("pwd encrypt scheme outdated, upgrading.");
		// Upgrade legacy hashes as a privileged internal operation, while
		// keeping audit attribution to the authenticating user.
		let upgrade_ctx =
			Ctx::new(user.id, user.organization_id, ROLE_ADMIN.to_string())
				.map_err(|_| Error::LoginFailUserCtxCreate { user_id })?;
		UserBmc::update_pwd(&upgrade_ctx, &mm, user.id, &pwd_clear).await?;
	}

	// -- Set web token.
	token::set_token_cookie(&cookies, &user.email, user.token_salt)?;

	// Create the success body.
	let body = Json(json!({
		"result": {
			"success": true,
			"must_change_password": user.must_change_password
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
	email: String,
	pwd: String,
}
// endregion: --- Login

// region:    --- Logoff
pub async fn api_logoff_handler(
	cookies: Cookies,
	Json(payload): Json<LogoffPayload>,
) -> Result<Json<Value>> {
	debug!("{:<12} - api_logoff_handler", "HANDLER");
	let should_logoff = payload.logoff;

	if should_logoff {
		token::remove_token_cookie(&cookies)?;
	}

	// Create the success body.
	let body = Json(json!({
		"result": {
			"logged_off": should_logoff
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
pub struct LogoffPayload {
	logoff: bool,
}
// endregion: --- Logoff

// region:    --- Token Refresh
pub async fn api_refresh_handler(
	State(mm): State<ModelManager>,
	ctx_w: crate::middleware::mw_auth::CtxW,
	cookies: Cookies,
) -> Result<Json<Value>> {
	debug!("{:<12} - api_refresh_handler", "HANDLER");

	let ctx = ctx_w.0;
	let user_id = ctx.user_id();

	// Get the user to refresh token
	let user: UserForAuth = UserBmc::get(&ctx, &mm, user_id)
		.await
		.map_err(Error::Model)?;

	// Set new web token
	token::set_token_cookie(&cookies, &user.email, user.token_salt)?;

	// Calculate expiration time (15 minutes from now)
	let expires_at = time::OffsetDateTime::now_utc() + time::Duration::minutes(15);

	// Create the success body with expiration info
	let body = Json(json!({
		"data": {
			"expiresAt": expires_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
		}
	}));

	Ok(body)
}
// endregion: --- Token Refresh
