//! Model Layer
//!
//! Design:
//!
//! - The Model layer normalizes the application's data type
//!   structures and access.
//! - All application code data access must go through the Model layer.
//! - The `ModelManager` holds the internal states/resources
//!   needed by ModelControllers to access data.
//!   (e.g., db_pool, S3 client, redis client).
//! - Model Controllers (e.g., `CaseBmc`, `UserBmc`) implement
//!   CRUD and other data access methods on a given "entity"
#![allow(unexpected_cfgs)]
//!   (e.g., `Case`, `User`).
//!   (`Bmc` is short for Backend Model Controller).
//! - In frameworks like Axum, Tauri, `ModelManager` are typically used as App State.
//! - ModelManager are designed to be passed as an argument
//!   to all Model Controllers functions.
//!

// region:    --- Modules

pub mod acs;
mod base;
mod error;
pub mod store;

// E2B(R3) SafetyDB Core Models
pub mod case;
pub mod case_numbering;
pub mod case_query;
pub mod case_query_catalog;
pub mod case_validation_report_cache;
pub mod case_validation_summary;
pub mod organization;
pub mod user; // E2B users table (UUID-based) // Organizations table // Core cases table

// E2B(R3) Section C - Safety Report Identification
pub mod safety_report; // Safety report ID, sender info, primary sources, literature refs, study info

// E2B(R3) Section D - Patient Information
pub mod patient; // Patient info, medical history, past drugs, death info, parent info

// E2B(R3) Section E - Reaction/Event
pub mod reaction; // Adverse event reactions

// E2B(R3) Section F - Tests and Procedures
pub mod test_result; // Lab results and diagnostic tests

// E2B(R3) Section G - Drug Information
pub mod drug; // Drug info, active substances, dosage, indications

// E2B(R3) Section H - Narrative
pub mod narrative; // Case narrative, sender diagnoses, case summaries

// E2B(R3) Section N - Message Headers
pub mod message_header; // Batch/message transmission headers

// E2B(R3) Section A - Receiver Information
pub mod receiver; // Receiver details for routing to regulatory authorities

// E2B(R3) G.k.9.i - Drug-Reaction Assessment
pub mod drug_reaction_assessment; // Causality assessment linking drugs to reactions

// E2B(R3) G.k.8.r - Drug Recurrence Information
pub mod drug_recurrence; // Structured rechallenge/recurrence data

// E2B(R3) C.1.9.r / C.1.10.r - Case Identifiers
pub mod case_identifiers; // Other case identifiers and linked report numbers

// E2B(R3) D.10.7 / D.10.8 - Parent History
pub mod parent_history; // Parent medical history and past drug history

// Controlled Terminologies
pub mod terminology; // MedDRA, WHODrug, ISO countries, E2B code lists
pub mod terminology_import; // Import/stage/activate pipeline for MedDRA and WHODrug

// Audit and Versioning
pub mod audit; // Audit logs and case versions
pub mod e_signature; // Electronic signatures for Part 11 critical actions

// Presave Templates
pub mod presave; // Section-specific INFO presave master data
pub mod submission_receiver_option; // Submission workflow receiver routing options

// Admin
pub mod admin_settings;
pub mod permission_profile; // Dynamic permission profiles and permission cache // System settings (app_settings table)

// Export Audit
pub mod xml_export_history; // XML export audit trail

// Case Intake / Duplicate Detection
pub mod case_duplicate; // Duplicate matching logic and LATERAL JOIN scan query

// Import Audit
pub mod xml_import_decision; // XML import skip/follow-up/new decision logic
pub mod xml_import_history; // XML import audit trail

// Utilities
pub mod modql_utils;

pub use self::error::{Error, Result};

use crate::model::store::dbx::Dbx;
use crate::model::store::new_db_pool;

// endregion: --- Modules

// region:    --- ModelManager

#[allow(unexpected_cfgs)]
#[cfg_attr(feature = "with-rpc", derive(rpc_router::RpcResource))]
pub struct ModelManager {
	dbx: Dbx,
}

impl Clone for ModelManager {
	fn clone(&self) -> Self {
		let dbx = Dbx::new(self.dbx.db().clone(), true).expect(
			"cloning ModelManager should create a Dbx over the existing pool",
		);
		ModelManager { dbx }
	}
}

impl ModelManager {
	/// Constructor
	pub async fn new() -> Result<Self> {
		let db_pool = new_db_pool()
			.await
			.map_err(|ex| Error::CantCreateModelManagerProvider(ex.to_string()))?;
		let dbx = Dbx::new(db_pool, true)?;
		Ok(ModelManager { dbx })
	}

	pub fn new_with_txn(&self) -> Result<ModelManager> {
		let dbx = Dbx::new(self.dbx.db().clone(), true)?;
		Ok(ModelManager { dbx })
	}

	pub fn dbx(&self) -> &Dbx {
		&self.dbx
	}
}

// endregion: --- ModelManager
