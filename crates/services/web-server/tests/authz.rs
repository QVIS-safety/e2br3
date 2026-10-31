mod common;

#[path = "authz/authentication_web.rs"]
mod authentication_web;
#[path = "authz/authorization_snapshot.rs"]
mod authorization_snapshot;
#[path = "authz/protected_route_inventory.rs"]
mod protected_route_inventory;
#[path = "authz/protected_route_parser.rs"]
mod protected_route_parser;
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
#[path = "authz/rbac_users/mod.rs"]
mod rbac_users;
#[path = "authz/rls_web.rs"]
mod rls_web;

#[path = "authz/support.rs"]
mod authz_support;

#[path = "authz/source_gates/mod.rs"]
mod source_gates;
