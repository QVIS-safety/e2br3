use crate::{Error, Result};
use lib_core::authorization::{
	AuthorizationContext, AuthorizationDenial, AuthorizedMutation, AuthorizedRead,
	RequestAuthorizationSnapshot,
};
use lib_core::ctx::{Ctx, ROLE_SYSTEM_ADMIN};
use uuid::Uuid;

pub fn denied(denial: AuthorizationDenial) -> Error {
	Error::PermissionDenied {
		required_permission: format!(
			"{} ({:?})",
			denial.action_id(),
			denial.reason()
		),
	}
}

trait PermitEvidence {
	fn principal_id(&self) -> Uuid;
	fn organization_id(&self) -> Uuid;
	fn target_organization_id(&self) -> Option<Uuid>;
}

impl<C: AuthorizationContext> PermitEvidence for AuthorizedRead<'_, C> {
	fn principal_id(&self) -> Uuid {
		self.principal_id()
	}
	fn organization_id(&self) -> Uuid {
		self.organization_id()
	}
	fn target_organization_id(&self) -> Option<Uuid> {
		self.target_organization_id()
	}
}

impl<C: AuthorizationContext> PermitEvidence for AuthorizedMutation<'_, C> {
	fn principal_id(&self) -> Uuid {
		self.principal_id()
	}
	fn organization_id(&self) -> Uuid {
		self.organization_id()
	}
	fn target_organization_id(&self) -> Option<Uuid> {
		self.target_organization_id()
	}
}

pub fn rls_ctx_for_authorized_read<C: AuthorizationContext>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &AuthorizedRead<'_, C>,
) -> Result<Ctx> {
	rls_ctx_from_permit(request_ctx, snapshot, permit)
}

pub fn rls_ctx_for_authorized_mutation<C: AuthorizationContext>(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &AuthorizedMutation<'_, C>,
) -> Result<Ctx> {
	rls_ctx_from_permit(request_ctx, snapshot, permit)
}

fn rls_ctx_from_permit(
	request_ctx: &Ctx,
	snapshot: &RequestAuthorizationSnapshot,
	permit: &impl PermitEvidence,
) -> Result<Ctx> {
	if permit.principal_id() != request_ctx.user_id()
		|| permit.principal_id() != snapshot.principal_id()
		|| permit.organization_id() != snapshot.organization_id()
	{
		return Err(Error::AccessDenied {
			required_role: "authorization permit bound to this request".to_string(),
		});
	}
	let target_organization_id =
		permit
			.target_organization_id()
			.ok_or_else(|| Error::AccessDenied {
				required_role: "authorization permit with target organization"
					.to_string(),
			})?;
	if target_organization_id == request_ctx.organization_id() {
		return Ok(request_ctx.clone());
	}
	if !snapshot.identity().is_platform_administrator() {
		return Err(Error::AccessDenied {
			required_role: "platform administrator cross-organization permit"
				.to_string(),
		});
	}
	Ctx::new(
		request_ctx.user_id(),
		target_organization_id,
		ROLE_SYSTEM_ADMIN.to_string(),
	)
	.map(|ctx| {
		ctx.with_compliance(
			request_ctx.change_reason().map(ToString::to_string),
			request_ctx.e_signature_id(),
		)
	})
	.map_err(|_| Error::BadRequest {
		message: "invalid authorized target organization context".to_string(),
	})
}
