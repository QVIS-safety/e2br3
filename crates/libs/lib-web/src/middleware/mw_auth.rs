use crate::error::{Error, Result};
use crate::utils::token::{set_token_cookie, AUTH_TOKEN};
use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use lib_auth::token::{validate_web_token, Token};
use lib_core::ctx::Ctx;
use lib_core::model::user::{UserBmc, UserForAuth};
use lib_core::model::ModelManager;
use serde::Serialize;
use tower_cookies::{Cookie, Cookies};
use tracing::debug;

pub async fn mw_ctx_require(
	ctx: Result<CtxW>,
	req: Request<Body>,
	next: Next,
) -> Result<Response> {
	debug!("{:<12} - mw_ctx_require - {ctx:?}", "MIDDLEWARE");

	ctx?;

	Ok(next.run(req).await)
}

// IMPORTANT: This resolver must never fail, but rather capture the potential Auth error and put in in the
//            request extension as CtxExtResult.
//            This way it won't prevent downstream middleware to be executed, and will still capture the error
//            for the appropriate middleware (.e.g., mw_ctx_require which forces successful auth) or handler
//            to get the appropriate information.
pub async fn mw_ctx_resolver(
	State(mm): State<ModelManager>,
	cookies: Cookies,
	mut req: Request<Body>,
	next: Next,
) -> Response {
	debug!("{:<12} - mw_ctx_resolve", "MIDDLEWARE");

	let audit_reason = req
		.headers()
		.get("x-e2br3-reason-for-change")
		.and_then(|value| value.to_str().ok())
		.and_then(decode_audit_reason_header)
		.and_then(|value| {
			let trimmed = value.trim().to_string();
			if trimmed.is_empty() {
				None
			} else {
				Some(trimmed)
			}
		});

	let ctx_ext_result = match ctx_resolve(mm, &cookies).await {
		Ok(CtxW(ctx)) => {
			let ctx = if let Some(reason) = audit_reason {
				ctx.with_compliance(Some(reason), ctx.e_signature_id())
			} else {
				ctx
			};
			Ok(CtxW(ctx))
		}
		Err(err) => Err(err),
	};

	if ctx_ext_result.is_err()
		&& !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie))
	{
		cookies.remove(Cookie::from(AUTH_TOKEN))
	}

	// Store the ctx_ext_result in the request extension
	// (for Ctx extractor).
	req.extensions_mut().insert(ctx_ext_result);

	next.run(req).await
}

fn decode_audit_reason_header(value: &str) -> Option<String> {
	let bytes = value.as_bytes();
	let mut decoded = Vec::with_capacity(bytes.len());
	let mut index = 0;
	while index < bytes.len() {
		if bytes[index] == b'%' && index + 2 < bytes.len() {
			let hi = hex_value(bytes[index + 1])?;
			let lo = hex_value(bytes[index + 2])?;
			decoded.push((hi << 4) | lo);
			index += 3;
		} else {
			decoded.push(bytes[index]);
			index += 1;
		}
	}
	String::from_utf8(decoded).ok()
}

fn hex_value(value: u8) -> Option<u8> {
	match value {
		b'0'..=b'9' => Some(value - b'0'),
		b'a'..=b'f' => Some(value - b'a' + 10),
		b'A'..=b'F' => Some(value - b'A' + 10),
		_ => None,
	}
}

async fn ctx_resolve(mm: ModelManager, cookies: &Cookies) -> CtxExtResult {
	// -- Get Token String
	let token = cookies
		.get(AUTH_TOKEN)
		.map(|c| c.value().to_string())
		.ok_or(CtxExtError::TokenNotInCookie)?;

	// -- Parse Token
	let token: Token = token.parse().map_err(|_| CtxExtError::TokenWrongFormat)?;

	// -- Get UserForAuth (now includes role and organization_id)
	let user: UserForAuth = UserBmc::auth_by_email(&mm, &token.ident)
		.await
		.map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?
		.ok_or(CtxExtError::UserNotFound)?;
	// -- Validate Token
	validate_web_token(&token, user.token_salt)
		.map_err(|_| CtxExtError::FailValidate)?;

	// -- Update Token
	set_token_cookie(cookies, &user.email, user.token_salt)
		.map_err(|_| CtxExtError::CannotSetTokenCookie)?;

	// -- Create CtxExtResult with user_id, organization_id, and role
	Ctx::new(user.id, user.organization_id, user.role)
		.map(|ctx| ctx.with_permission_profile(user.permission_profile_id))
		.map(CtxW)
		.map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

// region:    --- Ctx Extractor
#[derive(Debug, Clone)]
pub struct CtxW(pub Ctx);

impl<S: Send + Sync> FromRequestParts<S> for CtxW {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		debug!("{:<12} - Ctx", "EXTRACTOR");

		parts
			.extensions
			.get::<CtxExtResult>()
			.ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
			.clone()
			.map_err(Error::CtxExt)
	}
}
// endregion: --- Ctx Extractor

// region:    --- Ctx Extractor Result/Error
type CtxExtResult = core::result::Result<CtxW, CtxExtError>;

#[derive(Clone, Serialize, Debug)]
pub enum CtxExtError {
	TokenNotInCookie,
	TokenWrongFormat,

	UserNotFound,
	ModelAccessError(String),
	FailValidate,
	CannotSetTokenCookie,

	CtxNotInRequestExt,
	CtxCreateFail(String),
}
// endregion: --- Ctx Extractor Result/Error
