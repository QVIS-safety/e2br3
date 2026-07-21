use crate::middleware;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use derive_more::From;
use lib_auth::{pwd, token};
use lib_core::model;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

pub type Result<T> = core::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Serialize, From, strum_macros::AsRefStr)]
#[serde(tag = "type", content = "data")]
pub enum Error {
	// -- Login
	LoginFailUsernameNotFound,
	LoginFailEmailNotFound,
	LoginFailUserHasNoPwd {
		user_id: Uuid,
	},
	LoginFailPwdNotMatching {
		user_id: Uuid,
	},
	LoginFailUserCtxCreate {
		user_id: Uuid,
	},

	// -- Authorization
	AccessDenied {
		required_role: String,
	},
	PermissionDenied {
		required_permission: String,
	},
	OrganizationAccessDenied {
		user_org: Uuid,
		resource_org: Uuid,
	},

	// -- CtxExtError
	#[from]
	CtxExt(middleware::mw_auth::CtxExtError),

	// -- Extractors
	ReqStampNotInReqExt,

	// -- Modules
	#[from]
	Model(model::Error),
	#[from]
	Pwd(pwd::Error),
	#[from]
	Token(token::Error),
	#[from]
	Rest(lib_rest_core::Error),

	// -- External Modules
	#[from]
	SerdeJson(#[serde_as(as = "DisplayFromStr")] serde_json::Error),
}

// region:    --- Axum IntoResponse
impl IntoResponse for Error {
	fn into_response(self) -> Response {
		debug!("{:<12} - model::Error {self:?}", "INTO_RES");

		// Create a placeholder Axum reponse.
		let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

		// Insert the Error into the reponse.
		response.extensions_mut().insert(Arc::new(self));

		response
	}
}
// endregion: --- Axum IntoResponse

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
	fn fmt(
		&self,
		fmt: &mut core::fmt::Formatter,
	) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}
// endregion: --- Error Boilerplate

// region:    --- Client Error

/// From the root error to the http status code and ClientError
impl Error {
	pub fn client_status_and_error(&self) -> (StatusCode, ClientError) {
		use Error::*; // TODO: should change to `use web::Error as E`

		match self {
			// -- Login
			LoginFailUsernameNotFound
			| LoginFailEmailNotFound
			| LoginFailUserHasNoPwd { .. }
			| LoginFailPwdNotMatching { .. }
			| LoginFailUserCtxCreate { .. } => {
				(StatusCode::FORBIDDEN, ClientError::LOGIN_FAIL)
			}

			// -- Auth
			CtxExt(_) => (StatusCode::FORBIDDEN, ClientError::NO_AUTH),

			// -- Authorization
			AccessDenied { required_role } => (
				StatusCode::FORBIDDEN,
				ClientError::ACCESS_DENIED {
					required_role: required_role.clone(),
				},
			),
			PermissionDenied {
				required_permission,
			} => (
				StatusCode::FORBIDDEN,
				ClientError::PERMISSION_DENIED {
					required_permission: required_permission.clone(),
				},
			),
			OrganizationAccessDenied { .. } => (
				StatusCode::FORBIDDEN,
				ClientError::ORGANIZATION_ACCESS_DENIED,
			),

			// -- Model
			Model(model::Error::EntityNotFound { entity, id }) => (
				StatusCode::BAD_REQUEST,
				ClientError::ENTITY_NOT_FOUND { entity, id: *id },
			),
			Model(model::Error::EntityUuidNotFound { entity, id }) => (
				StatusCode::BAD_REQUEST,
				ClientError::ENTITY_UUID_NOT_FOUND {
					entity,
					id: id.to_string(),
				},
			),

			// -- Fallback.
			_ => (
				StatusCode::INTERNAL_SERVER_ERROR,
				ClientError::SERVICE_ERROR,
			),
		}
	}
}

#[derive(Debug, Serialize, strum_macros::AsRefStr)]
#[serde(tag = "message", content = "detail")]
#[allow(non_camel_case_types)]
pub enum ClientError {
	LOGIN_FAIL,
	NO_AUTH,
	ACCESS_DENIED { required_role: String },
	PERMISSION_DENIED { required_permission: String },
	ORGANIZATION_ACCESS_DENIED,
	ENTITY_NOT_FOUND { entity: &'static str, id: i64 },
	ENTITY_UUID_NOT_FOUND { entity: &'static str, id: String },
	CONSTRAINT_VIOLATION,
	SERVICE_ERROR,
}
// endregion: --- Client Error
