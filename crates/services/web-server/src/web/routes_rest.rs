use axum::Router;
use lib_core::model::ModelManager;

use super::rest;

/// Main REST API routes entry point
/// All REST resource routes are composed here
pub fn routes(mm: ModelManager) -> Router {
	Router::new()
		// Application runtime branding/configuration
		.merge(rest::routes_app())
		// Core E2B(R3) entities with nested subresources
		.merge(rest::routes_cases(mm.clone()))
		// Reference data
		.merge(rest::routes_organizations(mm.clone()))
		// System entities
		.merge(rest::routes_users(mm.clone()))
		// Relational INFO section presaves
		.merge(rest::routes_section_presaves(mm.clone()))
		// Terminology search
		.merge(rest::routes_terminology(mm.clone()))
		// XML import/validate
		.merge(rest::routes_import(mm.clone()))
		// Audit logs
		.merge(rest::routes_audit(mm.clone()))
		// Validation rule catalog
		.merge(rest::routes_validation(mm.clone()))
		// Submission tracking + mock ACK ingestion
		.merge(rest::routes_submissions(mm))
}
