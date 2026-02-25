#![allow(unexpected_cfgs)]

// region:    --- Modules

mod error;

pub use self::error::{Error, Result};

// endregion: --- Modules

// region:    --- Role Constants

/// System role for full administrative access
pub const ROLE_ADMIN: &str = "admin";
/// Role for management-level access (case approval, user viewing)
pub const ROLE_MANAGER: &str = "manager";
/// Role for regular user access (case CRUD)
pub const ROLE_USER: &str = "user";
/// Role for read-only access
pub const ROLE_VIEWER: &str = "viewer";

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
	e_signature_id: Option<uuid::Uuid>,
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
			role: ROLE_ADMIN.to_string(),
			change_reason: None,
			e_signature_id: None,
		}
	}

	/// Creates a new context with the given user UUID, organization ID, and role.
	/// This is the primary constructor for user-initiated operations.
	pub fn new(
		user_id: uuid::Uuid,
		organization_id: uuid::Uuid,
		role: String,
	) -> Result<Self> {
		if user_id.is_nil() {
			return Err(Error::CtxCannotNewNilUuid);
		}
		if organization_id.is_nil() && role != ROLE_ADMIN {
			return Err(Error::CtxCannotNewNilOrgId);
		}
		if !matches!(
			role.as_str(),
			ROLE_ADMIN | ROLE_MANAGER | ROLE_USER | ROLE_VIEWER
		) {
			return Err(Error::CtxCannotNewInvalidRole);
		}

		Ok(Self {
			user_id,
			organization_id,
			role,
			change_reason: None,
			e_signature_id: None,
		})
	}

	/// Creates a new context with just user_id (legacy support).
	/// Uses system organization and user role as defaults.
	#[deprecated(
		since = "0.3.0",
		note = "Use `Ctx::new(user_id, org_id, role)` instead"
	)]
	pub fn new_with_user_id_only(user_id: uuid::Uuid) -> Result<Self> {
		if user_id.is_nil() {
			return Err(Error::CtxCannotNewNilUuid);
		}

		Err(Error::CtxCannotNewNilOrgId)
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

	pub fn change_reason(&self) -> Option<&str> {
		self.change_reason.as_deref()
	}

	pub fn e_signature_id(&self) -> Option<uuid::Uuid> {
		self.e_signature_id
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

	// Role check helpers
	pub fn is_admin(&self) -> bool {
		self.role == ROLE_ADMIN
	}

	pub fn is_manager(&self) -> bool {
		self.role == ROLE_MANAGER
	}

	pub fn is_user(&self) -> bool {
		self.role == ROLE_USER
	}

	pub fn is_viewer(&self) -> bool {
		self.role == ROLE_VIEWER
	}

	/// Returns true if the user has at least manager-level access (admin or manager)
	pub fn is_manager_or_above(&self) -> bool {
		self.is_admin() || self.is_manager()
	}

	/// Returns true if the user can modify data (not a viewer)
	pub fn can_modify(&self) -> bool {
		self.is_admin() || self.is_manager() || self.is_user()
	}
}
