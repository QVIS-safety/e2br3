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
	let origin = std::env::var("E2BR3_PUBLIC_ORIGIN").map_err(|error| {
		cookie_config_error(format!("E2BR3_PUBLIC_ORIGIN is required: {error}"))
	})?;
	secure_cookie_for_origin(&origin)
}

fn cookie_config_error(message: String) -> Error {
	Error::CtxExt(crate::middleware::mw_auth::CtxExtError::Configuration(
		message,
	))
}

fn secure_cookie_for_origin(origin: &str) -> Result<bool> {
	let invalid = || {
		cookie_config_error("E2BR3_PUBLIC_ORIGIN must be an HTTP(S) origin without credentials, path, query or fragment".into())
	};
	let uri: axum::http::Uri = origin.parse().map_err(|_| invalid())?;
	if uri.host().is_none()
		|| origin.contains('@')
		|| origin.contains('#')
		|| !matches!(uri.path(), "" | "/")
		|| uri.query().is_some()
	{
		return Err(invalid());
	}
	match uri.scheme_str() {
		Some("https") => Ok(true),
		Some("http") => Ok(false),
		_ => Err(invalid()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cookie_security_follows_only_the_validated_public_origin() {
		assert!(secure_cookie_for_origin("https://safety.example").expect("HTTPS"));
		assert!(
			!secure_cookie_for_origin("http://127.0.0.1:19212").expect("local HTTP")
		);
		for invalid in [
			"",
			"safety.example",
			"ftp://safety.example",
			"https://user:pass@host",
			"https://host/path",
			"https://host?query",
			"https://host/#fragment",
		] {
			assert!(secure_cookie_for_origin(invalid).is_err(), "{invalid}");
		}
	}
}

pub fn remove_token_cookie(cookies: &Cookies) -> Result<()> {
	let mut cookie = Cookie::from(AUTH_TOKEN);
	cookie.set_path("/");

	cookies.remove(cookie);

	Ok(())
}
