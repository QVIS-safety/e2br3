pub mod export;
mod export_data;
mod export_postprocess;
mod export_runtime;
mod export_utils;
pub mod export_sections;
pub mod fda;
pub mod ich;
pub mod import;
pub mod import_sections;
pub mod mapping;
pub mod mfds;
pub mod model;
pub mod raw;
pub mod validate;

pub mod error;
pub mod parser;
pub mod types;
pub mod xml_validation;
mod xml_validation_fda;
mod xml_validation_ich;

pub use error::Error;
pub type Result<T> = core::result::Result<T, Error>;

pub use export::export_case_xml;
pub use import::{import_e2b_xml, XmlImportRequest};
pub use parser::parse_e2b_xml;
pub use types::ParsedE2b;
pub use types::{XmlImportResult, XmlValidationError, XmlValidationReport};
pub use xml_validation::{
	should_skip_xml_validation, validate_e2b_xml, validate_e2b_xml_basic,
	validate_e2b_xml_business, XmlValidatorConfig,
};
