use super::{role_permissions, with_dynamic_role_permissions, Permission};
use crate::ctx::canonical_role;

/// Checks if a role has a specific permission.
pub fn has_permission(role: &str, permission: Permission) -> bool {
	let normalized = canonical_role(role);
	with_dynamic_role_permissions(&normalized, |dynamic| match dynamic {
		Some(permissions) => permissions.contains(&permission),
		None => role_permissions(&normalized).contains(&permission),
	})
}

/// Checks if a role has any of the given permissions.
pub fn has_any_permission(role: &str, permissions: &[Permission]) -> bool {
	let normalized = canonical_role(role);
	with_dynamic_role_permissions(&normalized, |dynamic| {
		let role_permissions =
			dynamic.unwrap_or_else(|| role_permissions(&normalized));
		permissions
			.iter()
			.any(|permission| role_permissions.contains(permission))
	})
}

/// Checks if a role has all of the given permissions.
pub fn has_all_permissions(role: &str, permissions: &[Permission]) -> bool {
	let normalized = canonical_role(role);
	with_dynamic_role_permissions(&normalized, |dynamic| {
		let role_permissions =
			dynamic.unwrap_or_else(|| role_permissions(&normalized));
		permissions
			.iter()
			.all(|permission| role_permissions.contains(permission))
	})
}
