use crate::error::{Error, Result};
use crate::middleware::mw_auth::CtxExtError;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use lib_core::authorization::RequestAuthorizationSnapshot;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthorizationSnapshotW(Arc<RequestAuthorizationSnapshot>);

impl AuthorizationSnapshotW {
	pub fn new(snapshot: RequestAuthorizationSnapshot) -> Self {
		Self(Arc::new(snapshot))
	}
}

impl Deref for AuthorizationSnapshotW {
	type Target = RequestAuthorizationSnapshot;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<S: Send + Sync> FromRequestParts<S> for AuthorizationSnapshotW {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		parts.extensions.get::<Self>().cloned().ok_or(Error::CtxExt(
			CtxExtError::AuthorizationSnapshotNotInRequestExt,
		))
	}
}
