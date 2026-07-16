// Declare handler modules
pub mod app_branding_rest;
pub mod case_editor_dto;
pub mod case_editor_rest;
pub mod case_export_rest;
pub mod case_intake_rest;
pub mod case_query_catalog_rest;
pub mod case_rest;
pub mod case_validation_rest;
pub mod case_workflow_rest;
pub mod cioms_export_rest;
pub mod compliance;
pub mod organization_rest;
pub mod patient_rest;
pub mod user_rest;

pub mod drug_rest;
pub mod message_header_rest;
pub mod narrative_rest;
pub mod reaction_rest;
pub mod safety_report_rest;
pub mod test_result_rest;

// Newly enabled modules
pub mod admin_settings_rest;
pub mod audit_rest;
pub mod case_identifiers_rest;
pub mod drug_reaction_assessment_rest;
pub mod drug_recurrence_rest;
pub mod drug_sub_rest;
pub mod import_rest;
pub mod narrative_sub_rest;
pub mod parent_history_rest;
pub mod patient_sub_rest;
pub mod permission_contract;
pub mod permission_profile_rest;
pub mod receiver_rest;
pub mod relatedness_assessment_rest;
pub mod safety_report_sub_rest;
pub mod section_presave_rest;
pub mod submission_rest;
pub mod terminology_rest;
pub mod validation_rules_rest;

mod routes;
pub use routes::*;

use axum::routing::get;
use axum::Router;

/// Routes for /api/app
pub fn routes_app() -> Router {
	Router::new().route("/app/branding", get(app_branding_rest::get_app_branding))
}
