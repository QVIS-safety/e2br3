use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlValidationError {
	pub message: String,
	pub line: Option<usize>,
	pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlValidationReport {
	pub ok: bool,
	pub errors: Vec<XmlValidationError>,
	pub root_element: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedE2b {
	pub root_element: String,
	pub json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlImportResult {
	pub case_id: Option<String>,
	pub case_number: Option<String>,
	pub case_version: Option<i64>,
	pub xml_key: Option<String>,
	pub parsed_json_id: Option<String>,
}
