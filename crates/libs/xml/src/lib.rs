pub mod error;
pub mod export;
mod export_data;
mod export_utils;
pub mod fda;
pub mod ich;
pub mod import;
mod import_constraint;
pub mod import_sections;
pub mod mapping;
pub mod mfds;
pub mod model;
pub mod parser;
pub mod raw;
pub mod types;
pub mod validation;

pub use error::Error;
pub type Result<T> = core::result::Result<T, Error>;

pub use export::{
	export_case_xml_with_options, serialize_case_xml_for_authority,
	ExportXmlOptions, OutboundMessageHeader,
};
pub use import::{
	extract_safety_report_id_from_xml, import_e2b_xml, import_e2b_xml_in_txn,
	CImportSettings, XmlImportRequest,
};
pub use import_sections::c_safety_report::apply_c_safety_report_import_settings;
pub use parser::parse_e2b_xml;
pub use types::ParsedE2b;
pub use types::{XmlImportResult, XmlValidationError, XmlValidationReport};
pub use validation::{
	default_xsd_path, validate_e2b_xml, validate_e2b_xml_basic,
	validate_e2b_xml_for_import, XmlValidatorConfig,
};
