//! This module encompasses errors for all REST handlers.
//! Variants from our application's library errors can be added as required by the handlers.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use derive_more::From;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use std::sync::Arc;

pub type Result<T> = core::result::Result<T, Error>;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub enum XmlClientClassification<'a> {
	InvalidInput(&'static str),
	Model(&'a lib_core::model::Error),
	Internal,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintViolation {
	pub rule_code: String,
	pub path: String,
	pub message: String,
}

#[serde_as]
#[derive(Debug, From, Serialize)]
pub enum Error {
	// -- App Libs
	#[from]
	Model(lib_core::model::Error),
	#[from]
	Xml(xml::Error),

	// -- Authorization
	AccessDenied {
		required_role: String,
	},
	PermissionDenied {
		required_permission: String,
	},
	BadRequest {
		message: String,
	},
	Authentication,
	Configuration {
		key: &'static str,
		message: String,
	},
	Service {
		operation: &'static str,
	},
	Export(xml::Error),
	Internal {
		operation: &'static str,
		#[serde(skip)]
		source: BoxError,
	},
	ConstraintViolation(ConstraintViolation),

	// -- External Modules
	#[from]
	SerdeJson(#[serde_as(as = "DisplayFromStr")] serde_json::Error),
}

impl Error {
	pub fn xml_client_classification(&self) -> Option<XmlClientClassification<'_>> {
		let Self::Xml(error) = self else {
			return None;
		};
		Some(match error {
			xml::Error::InvalidXml { .. }
			| xml::Error::MissingRootElement
			| xml::Error::UnsupportedRoot { .. }
			| xml::Error::InvalidImportRequest { .. } => {
				XmlClientClassification::InvalidInput("Invalid XML input.")
			}
			xml::Error::XsdValidationFailed { .. } => {
				XmlClientClassification::InvalidInput(
					"XML schema validation failed.",
				)
			}
			xml::Error::Model(error) => XmlClientClassification::Model(error),
			xml::Error::NotImplemented { .. }
			| xml::Error::Io(_)
			| xml::Error::SerdeJson(_) => XmlClientClassification::Internal,
		})
	}

	pub fn diagnostic_type(&self) -> String {
		match self {
			Self::Model(_) => "Model".into(),
			Self::Xml(error) => format!("Xml.{}", xml_error_type(error)),
			Self::AccessDenied { .. } => "AccessDenied".into(),
			Self::PermissionDenied { .. } => "PermissionDenied".into(),
			Self::BadRequest { .. } => "BadRequest".into(),
			Self::Authentication => "Authentication".into(),
			Self::Configuration { key, .. } => format!("Configuration.{key}"),
			Self::Service { operation } => format!("Service.{operation}"),
			Self::Export(error) => format!("Export.{}", xml_error_type(error)),
			Self::Internal { operation, .. } => format!("Internal.{operation}"),
			Self::ConstraintViolation(_) => "ConstraintViolation".into(),
			Self::SerdeJson(_) => "SerdeJson".into(),
		}
	}
}

fn xml_error_type(error: &xml::Error) -> String {
	match error {
		xml::Error::InvalidXml { .. } => "InvalidXml".into(),
		xml::Error::XsdValidationFailed { .. } => "XsdValidationFailed".into(),
		xml::Error::MissingRootElement => "MissingRootElement".into(),
		xml::Error::UnsupportedRoot { .. } => "UnsupportedRoot".into(),
		xml::Error::NotImplemented { .. } => "NotImplemented".into(),
		xml::Error::InvalidImportRequest { .. } => "InvalidImportRequest".into(),
		xml::Error::Io(_) => "Io".into(),
		xml::Error::SerdeJson(_) => "SerdeJson".into(),
		xml::Error::Model(_) => "Model".into(),
	}
}

// region:    --- Axum IntoResponse
impl IntoResponse for Error {
	fn into_response(self) -> Response {
		// Create a placeholder Axum response.
		let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

		// Insert the Error into the response.
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

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Model(error) => Some(error),
			Self::Xml(error) | Self::Export(error) => Some(error),
			Self::Internal { source, .. } => Some(source.as_ref()),
			Self::SerdeJson(error) => Some(error),
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Error, XmlClientClassification};

	#[test]
	fn xml_io_is_internal() {
		let error =
			Error::Xml(xml::Error::Io(std::io::Error::other("private path")));
		assert!(matches!(
			error.xml_client_classification(),
			Some(XmlClientClassification::Internal)
		));
	}

	#[test]
	fn xml_model_preserves_the_model_error() {
		let error =
			Error::Xml(xml::Error::Model(lib_core::model::Error::CaseStateConflict));
		assert!(matches!(
			error.xml_client_classification(),
			Some(XmlClientClassification::Model(
				lib_core::model::Error::CaseStateConflict
			))
		));
	}
}
// endregion: --- Error Boilerplate
