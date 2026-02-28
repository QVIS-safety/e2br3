mod common;

#[path = "authz/auth_mw_audit.rs"]
mod auth_mw_audit;
#[path = "authz/rbac_audit.rs"]
mod rbac_audit;
#[path = "authz/rbac_cases.rs"]
mod rbac_cases;
#[path = "authz/rbac_drug.rs"]
mod rbac_drug;
#[path = "authz/rbac_narrative.rs"]
mod rbac_narrative;
#[path = "authz/rbac_organizations.rs"]
mod rbac_organizations;
#[path = "authz/rbac_patient.rs"]
mod rbac_patient;
#[path = "authz/rbac_safety_report.rs"]
mod rbac_safety_report;
#[path = "authz/rbac_subresources.rs"]
mod rbac_subresources;
#[path = "authz/rbac_users.rs"]
mod rbac_users;
#[path = "authz/rls_web.rs"]
mod rls_web;
