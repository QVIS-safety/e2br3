mod common;

#[path = "api_cases/contract/mod.rs"]
mod case_contract_web;
#[path = "api_cases/intake/mod.rs"]
mod case_intake_web;
#[path = "api_cases/case_support.rs"]
mod case_support;
#[path = "api_cases/compliance.rs"]
mod compliance;
#[path = "api_cases/lifecycle.rs"]
mod lifecycle;
#[path = "api_cases/locking.rs"]
mod locking;
#[path = "api_cases/scope/mod.rs"]
mod scope_visibility_web;
#[path = "api_cases/settings.rs"]
mod settings;
#[path = "api_cases/subresources/mod.rs"]
mod subresources_web;
#[path = "api_cases/validation.rs"]
mod validation;
#[path = "api_cases/validation_contract_web.rs"]
mod validation_contract_web;
#[path = "api_cases/workflow.rs"]
mod workflow;

#[allow(dead_code)]
#[path = "api_cases/validation_common.rs"]
mod validation_common;
