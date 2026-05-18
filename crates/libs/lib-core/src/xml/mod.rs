pub mod export;
mod export_data;
mod export_utils;
pub mod fda;
pub mod ich;
pub mod import;
mod import_runtime;
pub mod import_sections;
pub mod mapping;
pub mod mfds;
pub mod model;
pub mod raw;

pub mod error;
pub mod parser;
pub mod types;
pub mod xml_validation;

pub use error::Error;
pub type Result<T> = core::result::Result<T, Error>;

pub use crate::validation::xml::should_skip_xml_validation;
pub use export::{export_case_xml, export_case_xml_with_options, ExportXmlOptions};
pub use import::{import_e2b_xml, CImportSettings, XmlImportRequest};
pub use import_runtime::c::{
	apply_c_safety_report_import_settings, apply_default_values_to_imported_r2_case,
};
pub use parser::parse_e2b_xml;
pub use types::ParsedE2b;
pub use types::{XmlImportResult, XmlValidationError, XmlValidationReport};
pub use xml_validation::{
	default_xsd_path, validate_e2b_xml, validate_e2b_xml_basic,
	validate_e2b_xml_business, XmlValidatorConfig,
};
