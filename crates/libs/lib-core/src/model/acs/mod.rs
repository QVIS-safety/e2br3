//! Access Control System (ACS) based on PBAC (Privilege Based Access Control)
//!
//! This module provides the permission framework for SafetyDB:
//!
//! - **Resources**: Entities that can be accessed (Case, User, Drug, etc.)
//! - **Actions**: Operations on resources (Create, Read, Update, Delete, etc.)
//! - **Permissions**: Resource + Action combinations
//! - **Roles**: Built-in role permissions plus dynamic custom role privileges
//!
//! # Usage
//!
//! ```rust,ignore
//! use lib_core::model::acs::{has_permission, Permission, Resource, Action, CASE_CREATE};
//!
//! // Check if a role has a specific permission
//! if has_permission("user", CASE_CREATE) {
//!     // Allow the operation
//! }
//!
//! // Or use the permission constants directly
//! let perm = Permission::new(Resource::Case, Action::Create);
//! if has_permission(ctx.role(), perm) {
//!     // Allow
//! }
//! ```
//!
//! # Role Hierarchy
//!
//! | Role                  | Description                                      |
//! |-----------------------|--------------------------------------------------|
//! | system_admin          | Platform admin; no built-in operational access   |
//! | sponsor_admin_cro     | Sponsor admin with sender-scope assignment       |
//! | sponsor_admin_company | Sponsor admin without sender-scope assignment    |
//! | user                  | Default operational user permissions             |

mod types;
pub use types::*;

mod catalog;
pub use catalog::*;

mod builtin_roles;
pub use builtin_roles::role_permissions;
pub(crate) use builtin_roles::{case_view_permissions, profile_edit_permissions};

mod dynamic_roles;
pub(crate) use dynamic_roles::with_dynamic_role_permissions;
pub use dynamic_roles::{
	remove_dynamic_role, replace_dynamic_roles, upsert_dynamic_role_permissions,
};

mod check;
pub use check::{has_all_permissions, has_any_permission, has_permission};

mod registry_adapter;
pub use registry_adapter::{
	built_in_menu_privileges, normalize_menu_privileges,
	permissions_for_menu_privileges, AdminMenuPrivilege, PrivilegeAdapterError,
};

#[cfg(test)]
mod tests;
