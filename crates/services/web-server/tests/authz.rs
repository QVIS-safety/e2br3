mod common;

#[path = "authz/auth_mw_audit.rs"]
mod auth_mw_audit;
#[path = "authz/authorization_audit.rs"]
mod authorization_audit;
#[path = "authz/authorization_isolation.rs"]
mod authorization_isolation;
#[path = "authz/authorization_revisions.rs"]
mod authorization_revisions;
#[path = "authz/authorization_snapshot.rs"]
mod authorization_snapshot;
#[path = "authz/authorization_startup.rs"]
mod authorization_startup;
#[path = "authz/authorization_storage.rs"]
mod authorization_storage;
#[path = "authz/authorization_test_support.rs"]
mod authorization_test_support;
#[path = "authz/platform_admin_policy.rs"]
mod platform_admin_policy;
#[path = "authz/policy_kernel_characterization.rs"]
mod policy_kernel_characterization;
#[path = "authz/protected_route_inventory.rs"]
mod protected_route_inventory;
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
