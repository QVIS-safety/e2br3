use crate::types::XmlValidationError;
use derive_more::From;
use lib_core::model;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, Serialize, From)]
pub enum Error {
	InvalidXml {
		message: String,
		line: Option<usize>,
		column: Option<usize>,
	},
	XsdValidationFailed {
		errors: Vec<XmlValidationError>,
	},
	MissingRootElement,
	UnsupportedRoot {
		found: String,
	},
	NotImplemented {
		feature: &'static str,
	},
	InvalidImportRequest {
		message: String,
	},
	MissingExportField {
		field: &'static str,
	},
	#[from]
	Io(#[serde_as(as = "DisplayFromStr")] std::io::Error),
	#[from]
	SerdeJson(#[serde_as(as = "DisplayFromStr")] serde_json::Error),
	#[from]
	Model(#[serde_as(as = "DisplayFromStr")] model::Error),
}

impl core::fmt::Display for Error {
	fn fmt(
		&self,
		fmt: &mut core::fmt::Formatter,
	) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}
