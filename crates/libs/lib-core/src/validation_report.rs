use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
	pub code: String,
	pub message: String,
	pub path: String,
	pub field_path: Option<String>,
	pub section: String,
	pub subsection: String,
	pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSectionSummary {
	pub section: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSubsectionSummary {
	pub section: String,
	pub subsection: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseValidationReport {
	#[serde(default)]
	pub authority: String,
	pub case_id: Uuid,
	pub ok: bool,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
	pub section_summaries: Vec<ValidationSectionSummary>,
	pub subsection_summaries: Vec<ValidationSubsectionSummary>,
	pub issues: Vec<ValidationIssue>,
}
