#![allow(unexpected_cfgs)]

// region:    --- Modules

mod error;

pub use self::error::{Error, Result};

use crate::authorization::RequestAuthorizationSnapshot;
use crate::model::store::DatabaseIsolationContext;

// endregion: --- Modules

// region:    --- Role Constants

/// Platform/system role. Can provision safety-database access and run internal operations.
pub const ROLE_SYSTEM_ADMIN: &str = "system_admin";
/// Fixed in-database sponsor admin role for CRO deployments.
pub const ROLE_SPONSOR_ADMIN_CRO: &str = "sponsor_admin_cro";
/// Fixed in-database sponsor admin role for pharmaceutical-company deployments.
pub const ROLE_SPONSOR_ADMIN_COMPANY: &str = "sponsor_admin_company";
/// Role for regular user access (case CRUD)
pub const ROLE_USER: &str = "user";

// System UUIDs
pub const SYSTEM_USER_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const SYSTEM_ORG_ID: &str = "00000000-0000-0000-0000-000000000000";

// endregion: --- Role Constants

#[allow(unexpected_cfgs)]
#[cfg_attr(feature = "with-rpc", derive(rpc_router::RpcResource))]
#[derive(Clone, Debug)]
pub struct Ctx {
	user_id: uuid::Uuid,
	organization_id: uuid::Uuid,
	role: String,
	change_reason: Option<String>,
	change_category: Option<String>,
	e_signature_id: Option<uuid::Uuid>,
	authorization_isolation: Option<DatabaseIsolationContext>,
}

// Constructors.
impl Ctx {
	/// Creates a root context with the system user ID.
	/// Used for migrations, background jobs, and system operations.
	pub fn root_ctx() -> Self {
		Ctx {
			user_id: uuid::Uuid::parse_str(SYSTEM_USER_ID)
				.expect("Invalid system UUID"),
			organization_id: uuid::Uuid::parse_str(SYSTEM_ORG_ID)
				.expect("Invalid system org UUID"),
			role: ROLE_SYSTEM_ADMIN.to_string(),
			change_reason: None,
			change_category: None,
			e_signature_id: None,
			authorization_isolation: None,
		}
	}

	/// Creates a new context with the given user UUID, organization ID, and role.
	/// This is the primary constructor for user-initiated operations.
	pub fn new(
		user_id: uuid::Uuid,
		organization_id: uuid::Uuid,
		role: String,
	) -> Result<Self> {
		let role = canonical_role(&role);
		if user_id.is_nil() {
			return Err(Error::CtxCannotNewNilUuid);
		}
		if organization_id.is_nil() && role != ROLE_SYSTEM_ADMIN {
			return Err(Error::CtxCannotNewNilOrgId);
		}
		if role.is_empty() {
			return Err(Error::CtxCannotNewInvalidRole);
		}

		Ok(Self {
			user_id,
			organization_id,
			role,
			change_reason: None,
			change_category: None,
			e_signature_id: None,
			authorization_isolation: None,
		})
	}

	/// Builds a request context from the same validated snapshot used by the
	/// policy kernel. This is the only request path that carries typed database
	/// isolation instead of a caller-provided role label.
	pub fn from_authorization_snapshot(
		snapshot: &RequestAuthorizationSnapshot,
	) -> Result<Self> {
		let mut context = Self::new(
			snapshot.principal_id(),
			snapshot.organization_id(),
			snapshot.legacy_permission_subject().to_string(),
		)?;
		context.authorization_isolation =
			Some(DatabaseIsolationContext::from_snapshot(snapshot));
		Ok(context)
	}
}

// Property Accessors.
impl Ctx {
	pub fn user_id(&self) -> uuid::Uuid {
		self.user_id
	}

	pub fn organization_id(&self) -> uuid::Uuid {
		self.organization_id
	}

	pub fn role(&self) -> &str {
		&self.role
	}

	pub fn permission_subject(&self) -> &str {
		&self.role
	}

	pub fn change_reason(&self) -> Option<&str> {
		self.change_reason.as_deref()
	}

	pub fn change_category(&self) -> Option<&str> {
		self.change_category.as_deref()
	}

	pub fn e_signature_id(&self) -> Option<uuid::Uuid> {
		self.e_signature_id
	}

	pub(crate) fn authorization_isolation(
		&self,
	) -> Option<&DatabaseIsolationContext> {
		self.authorization_isolation.as_ref()
	}

	pub fn with_compliance(
		&self,
		change_reason: Option<String>,
		e_signature_id: Option<uuid::Uuid>,
	) -> Self {
		let mut next = self.clone();
		next.change_reason = change_reason;
		next.e_signature_id = e_signature_id;
		next
	}

	pub fn with_change_category(&self, change_category: Option<String>) -> Self {
		let mut next = self.clone();
		next.change_category = change_category;
		next
	}

	// Role check helpers
	pub fn is_admin(&self) -> bool {
		self.is_system_admin() || self.is_sponsor_admin()
	}

	pub fn is_system_admin(&self) -> bool {
		self.role == ROLE_SYSTEM_ADMIN
	}

	pub fn is_sponsor_admin(&self) -> bool {
		self.role == ROLE_SPONSOR_ADMIN_CRO
			|| self.role == ROLE_SPONSOR_ADMIN_COMPANY
	}

	pub fn is_cro_sponsor_admin(&self) -> bool {
		self.role == ROLE_SPONSOR_ADMIN_CRO
	}

	pub fn is_company_sponsor_admin(&self) -> bool {
		self.role == ROLE_SPONSOR_ADMIN_COMPANY
	}

	pub fn is_operational_admin(&self) -> bool {
		self.is_sponsor_admin()
	}

	pub fn is_user(&self) -> bool {
		self.role == ROLE_USER
	}
}

pub fn canonical_role(role: &str) -> String {
	match role.trim().to_ascii_lowercase().as_str() {
		"system-admin" => ROLE_SYSTEM_ADMIN.to_string(),
		"system_admin" => ROLE_SYSTEM_ADMIN.to_string(),
		"sponsor administrator(cro)" => ROLE_SPONSOR_ADMIN_CRO.to_string(),
		"sponsor administrator (cro)" => ROLE_SPONSOR_ADMIN_CRO.to_string(),
		"sponsor_admin_cro" => ROLE_SPONSOR_ADMIN_CRO.to_string(),
		"sponsor administrator(pharmaceutical company)" => {
			ROLE_SPONSOR_ADMIN_COMPANY.to_string()
		}
		"sponsor administrator (pharmaceutical company)" => {
			ROLE_SPONSOR_ADMIN_COMPANY.to_string()
		}
		"sponsor_admin_company" => ROLE_SPONSOR_ADMIN_COMPANY.to_string(),
		other => other.to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn canonicalizes_client_role_labels() {
		assert_eq!(canonical_role("system_admin"), ROLE_SYSTEM_ADMIN);
		assert_eq!(
			canonical_role("Sponsor Administrator (CRO)"),
			ROLE_SPONSOR_ADMIN_CRO
		);
		assert_eq!(
			canonical_role("Sponsor Administrator (Pharmaceutical Company)"),
			ROLE_SPONSOR_ADMIN_COMPANY
		);
	}

	#[test]
	fn removed_admin_string_is_not_promoted_to_sponsor_admin() {
		assert_eq!(canonical_role("admin"), "admin");
		let ctx = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::new_v4(),
			"admin".to_string(),
		)
		.expect("ctx");
		assert!(!ctx.is_operational_admin());
		assert!(!ctx.is_system_admin());
	}

	#[test]
	fn operational_admin_roles_are_distinct_from_system_admin() {
		let sponsor_admin = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::new_v4(),
			ROLE_SPONSOR_ADMIN_CRO.to_string(),
		)
		.expect("ctx");
		assert!(sponsor_admin.is_operational_admin());
		assert!(!sponsor_admin.is_system_admin());

		let system_admin = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::nil(),
			ROLE_SYSTEM_ADMIN.to_string(),
		)
		.expect("ctx");
		assert!(!system_admin.is_operational_admin());
		assert!(system_admin.is_system_admin());
	}

	#[test]
	fn is_admin_includes_system_and_sponsor_admins_only() {
		let system_admin = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::nil(),
			ROLE_SYSTEM_ADMIN.to_string(),
		)
		.expect("system ctx");
		let cro_admin = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::new_v4(),
			ROLE_SPONSOR_ADMIN_CRO.to_string(),
		)
		.expect("cro ctx");
		let company_admin = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::new_v4(),
			ROLE_SPONSOR_ADMIN_COMPANY.to_string(),
		)
		.expect("company ctx");
		let user = Ctx::new(
			uuid::Uuid::new_v4(),
			uuid::Uuid::new_v4(),
			ROLE_USER.to_string(),
		)
		.expect("user ctx");

		assert!(system_admin.is_admin());
		assert!(cro_admin.is_admin());
		assert!(company_admin.is_admin());
		assert!(!user.is_admin());
	}
}
