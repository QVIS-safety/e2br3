use crate::error::{Error, Result};
use crate::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use crate::utils::token::{set_token_cookie, AUTH_TOKEN};
use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use lib_auth::token::{validate_web_token, Token};
use lib_core::authorization::policy_registry;
use lib_core::ctx::Ctx;
use lib_core::model::authorization::SnapshotRepository;
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
		.and_then(trim_non_empty);
	let audit_category = req
		.headers()
		.get("x-e2br3-change-category")
		.and_then(|value| value.to_str().ok())
		.and_then(decode_audit_reason_header)
		.and_then(trim_non_empty);

	let resolved = ctx_resolve(mm.clone(), &cookies).await;
	let snapshot = resolved
		.as_ref()
		.ok()
		.map(|resolved| resolved.snapshot.clone());
	let ctx_ext_result = match resolved {
		Ok(ResolvedRequest { ctx, .. }) => {
			let mut ctx = if let Some(reason) = audit_reason {
				ctx.with_compliance(Some(reason), ctx.e_signature_id())
			} else {
				ctx
			};
			if let Some(category) = audit_category {
				ctx = ctx.with_change_category(Some(category));
			}
			Ok(CtxW(ctx))
		}
		Err(err) => Err(err),
	};
	if let Some(snapshot) = snapshot {
		req.extensions_mut().insert(RbacPolicyVersion(
			snapshot.version().organization_revision(),
		));
		req.extensions_mut().insert(snapshot);
	}

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

fn trim_non_empty(value: String) -> Option<String> {
	let trimmed = value.trim().to_string();
	if trimmed.is_empty() {
		None
	} else {
		Some(trimmed)
	}
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

struct ResolvedRequest {
	ctx: Ctx,
	snapshot: AuthorizationSnapshotW,
}

async fn ctx_resolve(
	mm: ModelManager,
	cookies: &Cookies,
) -> core::result::Result<ResolvedRequest, CtxExtError> {
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
	let authentication_expires_at = lib_utils::time::parse_utc(&token.exp)
		.map_err(|_| CtxExtError::TokenExpiryWrongFormat)?;
	let snapshot = SnapshotRepository::load_repeatable_read(
		mm.dbx().db(),
		policy_registry(),
		user.id,
		user.organization_id,
		time::OffsetDateTime::now_utc(),
		Some(authentication_expires_at),
	)
	.await
	.map_err(|error| CtxExtError::AuthorizationSnapshotLoad(error.to_string()))?;

	// -- Update Token
	set_token_cookie(cookies, &user.email, user.token_salt)
		.map_err(|_| CtxExtError::CannotSetTokenCookie)?;

	let ctx = Ctx::new(
		snapshot.principal_id(),
		snapshot.organization_id(),
		snapshot.legacy_permission_subject().to_string(),
	)
	.map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))?;
	Ok(ResolvedRequest {
		ctx,
		snapshot: AuthorizationSnapshotW::new(snapshot),
	})
}

// region:    --- Ctx Extractor
#[derive(Debug, Clone)]
pub struct CtxW(pub Ctx);

#[derive(Debug, Clone, Copy)]
pub struct RbacPolicyVersion(pub i64);

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
	TokenExpiryWrongFormat,

	UserNotFound,
	ModelAccessError(String),
	FailValidate,
	CannotSetTokenCookie,
	AuthorizationSnapshotLoad(String),

	CtxNotInRequestExt,
	AuthorizationSnapshotNotInRequestExt,
	CtxCreateFail(String),
}
// endregion: --- Ctx Extractor Result/Error
