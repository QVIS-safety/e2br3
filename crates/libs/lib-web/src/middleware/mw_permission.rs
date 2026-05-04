//! Permission checking middleware and extractors for RBAC
//!
//! This module provides:
//! - `RequirePermission` - Extractor that checks for a specific permission
//! - `RequireAnyPermission` - Extractor that checks for any of multiple permissions
//! - `RequireRole` - Extractor that checks for a specific role
//! - Helper functions for permission checking in handlers

use crate::error::{Error, Result};
use crate::middleware::mw_auth::CtxW;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use lib_core::ctx::Ctx;
use lib_core::model::acs::{has_permission, Permission};
use std::marker::PhantomData;

// region:    --- RequirePermission Extractor

/// Extractor that requires a specific permission.
///
/// Usage in handlers:
/// ```rust,ignore
/// async fn create_case(
///     ctx: CtxW,
///     _perm: RequirePermission<CaseCreatePerm>,
///     // ... other extractors
/// ) -> Result<Json<Case>> {
///     // Handler code - permission already verified
/// }
/// ```
pub struct RequirePermission<P: PermissionCheck> {
	_marker: PhantomData<P>,
}

/// Trait for permission markers
pub trait PermissionCheck: Send + Sync + 'static {
	fn permission() -> Permission;
	fn permission_name() -> &'static str;
}

impl<S, P> FromRequestParts<S> for RequirePermission<P>
where
	S: Send + Sync,
	P: PermissionCheck,
{
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		// Get the context from extensions (set by mw_ctx_resolver)
		let ctx_result = parts
			.extensions
			.get::<core::result::Result<CtxW, crate::middleware::mw_auth::CtxExtError>>(
			)
			.ok_or(Error::CtxExt(
				crate::middleware::mw_auth::CtxExtError::CtxNotInRequestExt,
			))?;

		let ctx = ctx_result.as_ref().map_err(|e| Error::CtxExt(e.clone()))?;

		// Check permission
		let permission = P::permission();
		if !has_permission(ctx.0.role(), permission) {
			return Err(Error::PermissionDenied {
				required_permission: P::permission_name().to_string(),
			});
		}

		Ok(Self {
			_marker: PhantomData,
		})
	}
}

// endregion: --- RequirePermission Extractor

// region:    --- Permission Check Function

/// Check if the context has a specific permission.
/// Use this for inline permission checks in handlers.
pub fn check_permission(ctx: &Ctx, permission: Permission) -> Result<()> {
	if !has_permission(ctx.role(), permission) {
		return Err(Error::PermissionDenied {
			required_permission: format!("{permission}"),
		});
	}
	Ok(())
}

/// Check if the context has any of the given permissions.
pub fn check_any_permission(ctx: &Ctx, permissions: &[Permission]) -> Result<()> {
	for perm in permissions {
		if has_permission(ctx.role(), *perm) {
			return Ok(());
		}
	}
	Err(Error::PermissionDenied {
		required_permission: permissions
			.iter()
			.map(|p| format!("{p}"))
			.collect::<Vec<_>>()
			.join(" or "),
	})
}

// endregion: --- Permission Check Function

// region:    --- Organization Check Function

/// Check if the user belongs to the same organization as the resource.
/// Admins can access any organization.
pub fn check_organization_access(
	ctx: &Ctx,
	resource_org_id: uuid::Uuid,
) -> Result<()> {
	// Admins can access any organization
	if ctx.is_admin() {
		return Ok(());
	}

	// Check if user's org matches resource's org
	if ctx.organization_id() != resource_org_id {
		return Err(Error::OrganizationAccessDenied {
			user_org: ctx.organization_id(),
			resource_org: resource_org_id,
		});
	}

	Ok(())
}

// endregion: --- Organization Check Function

// region:    --- RequireRole Extractor

/// Extractor that requires a specific role or higher.
pub struct RequireRole<R: RoleCheck> {
	_marker: PhantomData<R>,
}

/// Trait for role markers
pub trait RoleCheck: Send + Sync + 'static {
	fn check(role: &str) -> bool;
	fn role_name() -> &'static str;
}

/// Marker for admin role requirement
pub struct AdminRole;
impl RoleCheck for AdminRole {
	fn check(role: &str) -> bool {
		let normalized = lib_core::ctx::canonical_role(role);
		matches!(
			normalized.as_str(),
			lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO
				| lib_core::ctx::ROLE_SPONSOR_ADMIN_COMPANY
		)
	}
	fn role_name() -> &'static str {
		"safety_db_admin"
	}
}

/// Marker for manager or above role requirement
pub struct ManagerOrAboveRole;
impl RoleCheck for ManagerOrAboveRole {
	fn check(role: &str) -> bool {
		let normalized = lib_core::ctx::canonical_role(role);
		matches!(
			normalized.as_str(),
			lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO
				| lib_core::ctx::ROLE_SPONSOR_ADMIN_COMPANY
				| lib_core::ctx::ROLE_MANAGER
		)
	}
	fn role_name() -> &'static str {
		"manager or safety_db_admin"
	}
}

/// Marker for any authenticated user (not viewer)
pub struct CanModifyRole;
impl RoleCheck for CanModifyRole {
	fn check(role: &str) -> bool {
		lib_core::ctx::canonical_role(role) != lib_core::ctx::ROLE_VIEWER
	}
	fn role_name() -> &'static str {
		"user, manager, or admin"
	}
}

impl<S, R> FromRequestParts<S> for RequireRole<R>
where
	S: Send + Sync,
	R: RoleCheck,
{
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		let ctx_result = parts
			.extensions
			.get::<core::result::Result<CtxW, crate::middleware::mw_auth::CtxExtError>>(
			)
			.ok_or(Error::CtxExt(
				crate::middleware::mw_auth::CtxExtError::CtxNotInRequestExt,
			))?;

		let ctx = ctx_result.as_ref().map_err(|e| Error::CtxExt(e.clone()))?;

		if !R::check(ctx.0.role()) {
			return Err(Error::AccessDenied {
				required_role: R::role_name().to_string(),
			});
		}

		Ok(Self {
			_marker: PhantomData,
		})
	}
}

// endregion: --- RequireRole Extractor

// region:    --- Permission Marker Types

// These are marker types for common permissions.
// Use with RequirePermission<CaseCreate> etc.

use lib_core::model::acs;

macro_rules! define_permission_marker {
	($name:ident, $perm:expr, $perm_name:expr) => {
		pub struct $name;
		impl PermissionCheck for $name {
			fn permission() -> Permission {
				$perm
			}
			fn permission_name() -> &'static str {
				$perm_name
			}
		}
	};
}

// Case permissions
define_permission_marker!(CaseCreate, acs::CASE_CREATE, "Case.Create");
define_permission_marker!(CaseRead, acs::CASE_READ, "Case.Read");
define_permission_marker!(CaseUpdate, acs::CASE_UPDATE, "Case.Update");
define_permission_marker!(CaseDelete, acs::CASE_DELETE, "Case.Delete");
define_permission_marker!(CaseList, acs::CASE_LIST, "Case.List");
define_permission_marker!(CaseApprove, acs::CASE_APPROVE, "Case.Approve");

// Patient permissions
define_permission_marker!(PatientCreate, acs::PATIENT_CREATE, "Patient.Create");
define_permission_marker!(PatientRead, acs::PATIENT_READ, "Patient.Read");
define_permission_marker!(PatientUpdate, acs::PATIENT_UPDATE, "Patient.Update");
define_permission_marker!(PatientDelete, acs::PATIENT_DELETE, "Patient.Delete");

// Drug permissions
define_permission_marker!(DrugCreate, acs::DRUG_CREATE, "Drug.Create");
define_permission_marker!(DrugRead, acs::DRUG_READ, "Drug.Read");
define_permission_marker!(DrugUpdate, acs::DRUG_UPDATE, "Drug.Update");
define_permission_marker!(DrugDelete, acs::DRUG_DELETE, "Drug.Delete");
define_permission_marker!(DrugList, acs::DRUG_LIST, "Drug.List");

// Reaction permissions
define_permission_marker!(ReactionCreate, acs::REACTION_CREATE, "Reaction.Create");
define_permission_marker!(ReactionRead, acs::REACTION_READ, "Reaction.Read");
define_permission_marker!(ReactionUpdate, acs::REACTION_UPDATE, "Reaction.Update");
define_permission_marker!(ReactionDelete, acs::REACTION_DELETE, "Reaction.Delete");
define_permission_marker!(ReactionList, acs::REACTION_LIST, "Reaction.List");

// Test Result permissions
define_permission_marker!(
	TestResultCreate,
	acs::TEST_RESULT_CREATE,
	"TestResult.Create"
);
define_permission_marker!(TestResultRead, acs::TEST_RESULT_READ, "TestResult.Read");
define_permission_marker!(
	TestResultUpdate,
	acs::TEST_RESULT_UPDATE,
	"TestResult.Update"
);
define_permission_marker!(
	TestResultDelete,
	acs::TEST_RESULT_DELETE,
	"TestResult.Delete"
);
define_permission_marker!(TestResultList, acs::TEST_RESULT_LIST, "TestResult.List");

// Narrative permissions
define_permission_marker!(
	NarrativeCreate,
	acs::NARRATIVE_CREATE,
	"Narrative.Create"
);
define_permission_marker!(NarrativeRead, acs::NARRATIVE_READ, "Narrative.Read");
define_permission_marker!(
	NarrativeUpdate,
	acs::NARRATIVE_UPDATE,
	"Narrative.Update"
);
define_permission_marker!(
	NarrativeDelete,
	acs::NARRATIVE_DELETE,
	"Narrative.Delete"
);

// MessageHeader permissions
define_permission_marker!(
	MessageHeaderCreate,
	acs::MESSAGE_HEADER_CREATE,
	"MessageHeader.Create"
);
define_permission_marker!(
	MessageHeaderRead,
	acs::MESSAGE_HEADER_READ,
	"MessageHeader.Read"
);
define_permission_marker!(
	MessageHeaderUpdate,
	acs::MESSAGE_HEADER_UPDATE,
	"MessageHeader.Update"
);
define_permission_marker!(
	MessageHeaderDelete,
	acs::MESSAGE_HEADER_DELETE,
	"MessageHeader.Delete"
);

// SafetyReport permissions
define_permission_marker!(
	SafetyReportCreate,
	acs::SAFETY_REPORT_CREATE,
	"SafetyReport.Create"
);
define_permission_marker!(
	SafetyReportRead,
	acs::SAFETY_REPORT_READ,
	"SafetyReport.Read"
);
define_permission_marker!(
	SafetyReportUpdate,
	acs::SAFETY_REPORT_UPDATE,
	"SafetyReport.Update"
);
define_permission_marker!(
	SafetyReportDelete,
	acs::SAFETY_REPORT_DELETE,
	"SafetyReport.Delete"
);

// User permissions
define_permission_marker!(UserCreate, acs::USER_CREATE, "User.Create");
define_permission_marker!(UserRead, acs::USER_READ, "User.Read");
define_permission_marker!(UserUpdate, acs::USER_UPDATE, "User.Update");
define_permission_marker!(UserDelete, acs::USER_DELETE, "User.Delete");
define_permission_marker!(UserList, acs::USER_LIST, "User.List");

// Organization permissions
define_permission_marker!(OrgCreate, acs::ORG_CREATE, "Organization.Create");
define_permission_marker!(OrgRead, acs::ORG_READ, "Organization.Read");
define_permission_marker!(OrgUpdate, acs::ORG_UPDATE, "Organization.Update");
define_permission_marker!(OrgDelete, acs::ORG_DELETE, "Organization.Delete");
define_permission_marker!(OrgList, acs::ORG_LIST, "Organization.List");

// Audit permissions
define_permission_marker!(AuditRead, acs::AUDIT_READ, "AuditLog.Read");
define_permission_marker!(AuditList, acs::AUDIT_LIST, "AuditLog.List");

// Terminology permissions
define_permission_marker!(
	TerminologyRead,
	acs::TERMINOLOGY_READ,
	"Terminology.Read"
);

// XML permissions
define_permission_marker!(XmlExport, acs::XML_EXPORT, "Xml.Export");
define_permission_marker!(XmlImport, acs::XML_IMPORT, "Xml.Import");

// endregion: --- Permission Marker Types

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::middleware::mw_auth::{CtxExtError, CtxW};
	use axum::http::Request;
	use lib_core::ctx::{
		ROLE_MANAGER, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER, ROLE_VIEWER,
	};
	use uuid::Uuid;

	fn parts_with_ctx(role: &str) -> Parts {
		let ctx =
			Ctx::new(Uuid::new_v4(), Uuid::new_v4(), role.to_string()).expect("ctx");
		let (mut parts, _) = Request::new(()).into_parts();
		parts
			.extensions
			.insert::<core::result::Result<CtxW, CtxExtError>>(Ok(CtxW(ctx)));
		parts
	}

	#[tokio::test]
	async fn require_permission_allows_sponsor_admin() {
		let mut parts = parts_with_ctx(ROLE_SPONSOR_ADMIN_CRO);
		let res =
			RequirePermission::<CaseCreate>::from_request_parts(&mut parts, &())
				.await;
		assert!(res.is_ok());
	}

	#[tokio::test]
	async fn require_permission_denies_viewer() {
		let mut parts = parts_with_ctx(ROLE_VIEWER);
		let res =
			RequirePermission::<CaseCreate>::from_request_parts(&mut parts, &())
				.await;
		assert!(matches!(res, Err(Error::PermissionDenied { .. })));
	}

	#[tokio::test]
	async fn require_role_admin_only() {
		let mut parts = parts_with_ctx(ROLE_MANAGER);
		let res =
			RequireRole::<AdminRole>::from_request_parts(&mut parts, &()).await;
		assert!(matches!(res, Err(Error::AccessDenied { .. })));
	}

	#[tokio::test]
	async fn require_role_accepts_sponsor_admin() {
		let mut parts = parts_with_ctx(ROLE_SPONSOR_ADMIN_CRO);
		let res =
			RequireRole::<AdminRole>::from_request_parts(&mut parts, &()).await;
		assert!(res.is_ok());
	}

	#[tokio::test]
	async fn require_role_manager_or_above() {
		let mut parts = parts_with_ctx(ROLE_MANAGER);
		let res =
			RequireRole::<ManagerOrAboveRole>::from_request_parts(&mut parts, &())
				.await;
		assert!(res.is_ok());
	}

	#[test]
	fn check_organization_access_enforces_org() {
		let ctx = Ctx::new(Uuid::new_v4(), Uuid::new_v4(), ROLE_USER.to_string())
			.expect("ctx");
		let res = check_organization_access(&ctx, Uuid::new_v4());
		assert!(matches!(res, Err(Error::OrganizationAccessDenied { .. })));
	}

	#[test]
	fn check_organization_access_allows_sponsor_admin() {
		let ctx = Ctx::new(
			Uuid::new_v4(),
			Uuid::new_v4(),
			ROLE_SPONSOR_ADMIN_CRO.to_string(),
		)
		.expect("ctx");
		let res = check_organization_access(&ctx, Uuid::new_v4());
		assert!(res.is_ok());
	}
}

// endregion: --- Tests
