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
pub(crate) use builtin_roles::{
	admin_permissions, profile_edit_permissions, viewer_permissions,
};

mod permission;

pub use permission::*;

#[cfg(test)]
mod tests;
