mod common;

#[allow(dead_code)]
#[path = "validation/validation_common.rs"]
mod validation_common;

#[path = "api/admin_settings/mod.rs"]
mod admin_settings;
#[path = "api/app_branding_web.rs"]
mod app_branding_web;
#[path = "api/audit_web.rs"]
mod audit_web;
#[path = "api/case_contract_web.rs"]
mod case_contract_web;
#[path = "api/case_editor_contract_web.rs"]
mod case_editor_contract_web;
#[path = "api/case_intake_web.rs"]
mod case_intake_web;
#[path = "api/case_validation_web.rs"]
mod case_validation_web;
#[path = "api/error_mapping_web.rs"]
mod error_mapping_web;
#[cfg(any())]
#[path = "api/export_contract_web.rs"]
mod export_contract_web;
#[cfg(any())]
#[path = "api/import_contract_web.rs"]
mod import_contract_web;
#[path = "api/import_history_web.rs"]
mod import_history_web;
#[path = "api/middleware_ctx.rs"]
mod middleware_ctx;
#[path = "api/presave/mod.rs"]
mod presave;
#[path = "api/role_admin/mod.rs"]
mod role_admin;
#[path = "api/scope_visibility_web.rs"]
mod scope_visibility_web;
#[path = "api/submission_lifecycle_web.rs"]
mod submission_lifecycle_web;
#[path = "api/submission_schema_guard_web.rs"]
mod submission_schema_guard_web;
#[path = "api/subresources_web.rs"]
mod subresources_web;
#[path = "api/terminology_contract_web.rs"]
mod terminology_contract_web;
#[path = "api/user_organization_membership_web.rs"]
mod user_organization_membership_web;
#[path = "api/validation_contract_web.rs"]
mod validation_contract_web;
#[path = "api/validation_rules_web.rs"]
mod validation_rules_web;
