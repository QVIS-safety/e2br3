pub use crate::error::ClientError;
pub use crate::error::{Error, Result};
use lib_auth::token::generate_web_token;
use tower_cookies::{cookie::SameSite, Cookie, Cookies};
use uuid::Uuid;

// endregion: --- Modules

pub(crate) const AUTH_TOKEN: &str = "auth-token";

pub fn set_token_cookie(
	cookies: &Cookies,
	email: &str,
	organization_id: Uuid,
	salt: Uuid,
) -> Result<()> {
	// Bind the selected organization to the global account's signed session.
	let identity = format!("{email}|{organization_id}");
	let token = generate_web_token(&identity, salt)?;

	let mut cookie = Cookie::new(AUTH_TOKEN, token.to_string());
	cookie.set_http_only(true);
	cookie.set_path("/");
	cookie.set_same_site(SameSite::Lax);
	cookie.set_secure(secure_auth_cookie()?);

	cookies.add(cookie);

	Ok(())
}

fn secure_auth_cookie() -> Result<bool> {
	let config_error = |message| {
		Error::CtxExt(crate::middleware::mw_auth::CtxExtError::Configuration(
			message,
		))
	};
	let value = std::env::var("E2BR3_COOKIE_SECURE").map_err(|error| {
		config_error(format!("E2BR3_COOKIE_SECURE is required: {error}"))
	})?;
	match value.trim().to_ascii_lowercase().as_str() {
		"1" | "true" | "yes" | "on" => Ok(true),
		"0" | "false" | "no" | "off" => Ok(false),
		_ => Err(config_error(
			"E2BR3_COOKIE_SECURE must be a boolean value".into(),
		)),
	}
}

pub fn remove_token_cookie(cookies: &Cookies) -> Result<()> {
	let mut cookie = Cookie::from(AUTH_TOKEN);
	cookie.set_path("/");

	cookies.remove(cookie);

	Ok(())
}
