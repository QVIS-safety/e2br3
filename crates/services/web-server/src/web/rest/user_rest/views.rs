use super::*;

pub(super) async fn user_view(
	ctx: &Ctx,
	mm: &ModelManager,
	user: User,
) -> Result<UserView> {
	let mut views = user_views(ctx, mm, vec![user]).await?;
	views.pop().ok_or_else(|| Error::Internal {
		operation: "build user view",
		source: std::io::Error::other("single user view was not produced").into(),
	})
}

pub(super) async fn user_views(
	ctx: &Ctx,
	mm: &ModelManager,
	users: Vec<User>,
) -> Result<Vec<UserView>> {
	let user_ids = users.iter().map(|user| user.id).collect::<Vec<_>>();
	let role_assignments =
		UserBmc::role_assignments_for_users(ctx, mm, &user_ids).await?;
	let role_names = PermissionProfileBmc::list(ctx, mm)
		.await?
		.into_iter()
		.map(|profile| (profile.id.to_string(), profile.name))
		.collect::<std::collections::HashMap<_, _>>();
	users
		.into_iter()
		.map(|mut user| {
			if user.id == ctx.user_id() {
				user.organization_id = ctx.organization_id();
			}
			let role_id = role_assignments
				.get(&(user.id, user.organization_id))
				.copied()
				.ok_or_else(|| Error::Internal {
					operation: "load user role assignment",
					source: std::io::Error::other(format!(
						"user {} has no role assignment for organization {}",
						user.id, user.organization_id
					))
					.into(),
				})?;
			let role = role_for_assignment(role_id)?;
			let role_name = role_names.get(&role).map(String::as_str);
			user_view_with_role(user, role, role_name)
		})
		.collect()
}

fn user_view_with_role(
	user: User,
	role: String,
	role_name: Option<&str>,
) -> Result<UserView> {
	let active = user_is_effectively_active(&user);
	let access_sender_ids = user.access_sender_ids.clone();
	let access_product_ids = user.access_product_ids.clone();
	let access_study_ids = user.access_study_ids.clone();
	let access_blind_allowed = user.access_blind_allowed;
	let active_sender_identifier = user.active_sender_identifier.clone();
	Ok(UserView {
		id: user.id,
		organization_id: user.organization_id,
		email: user.email,
		username: user.username,
		role: role.clone(),
		role_meta: role_metadata(&role, role_name)?,
		comments: user.comments,
		other_information: user.other_information,
		scope: UserScopeView {
			assigned_sender_ids: lib_rest_core::scope_values_from_raw(
				access_sender_ids.as_deref(),
			)
			.map_err(|error| Error::Model(lib_core::model::Error::Store(error)))?,
			assigned_product_ids: lib_rest_core::scope_values_from_raw(
				access_product_ids.as_deref(),
			)
			.map_err(|error| Error::Model(lib_core::model::Error::Store(error)))?,
			assigned_study_ids: lib_rest_core::scope_values_from_raw(
				access_study_ids.as_deref(),
			)
			.map_err(|error| Error::Model(lib_core::model::Error::Store(error)))?,
			access_blind_allowed: access_blind_allowed == Some(true),
			active_sender_identifier: active_sender_identifier.clone(),
			access_start_at: user.access_start_at,
			access_end_at: user.access_end_at,
		},
		active,
		must_change_password: user.must_change_password,
		last_login_at: user.last_login_at,
		created_at: user.created_at,
		updated_at: user.updated_at,
		created_by: user.created_by,
		updated_by: user.updated_by,
	})
}

fn role_for_assignment(role_id: Uuid) -> Result<String> {
	Ok(
		match super::super::authorization_registry()?
			.built_in_identities()
			.iter()
			.find(|identity| identity.id == role_id)
			.map(|identity| identity.kind)
		{
			Some(BuiltInIdentityKind::PlatformAdministrator) => {
				ROLE_SYSTEM_ADMIN.to_string()
			}
			Some(BuiltInIdentityKind::SponsorCroAdministrator) => {
				ROLE_SPONSOR_ADMIN_CRO.to_string()
			}
			Some(BuiltInIdentityKind::SponsorCompanyAdministrator) => {
				ROLE_SPONSOR_ADMIN_COMPANY.to_string()
			}
			Some(BuiltInIdentityKind::OperationalUser) => ROLE_USER.to_string(),
			Some(BuiltInIdentityKind::InternalServicePrincipal) | None => {
				role_id.to_string()
			}
		},
	)
}

pub(super) fn workflow_user_option_view(
	user: WorkflowUserOption,
) -> WorkflowUserOptionView {
	WorkflowUserOptionView {
		id: user.id,
		email: user.email.clone(),
		display_name: if user.username.trim().is_empty() {
			user.email
		} else {
			user.username
		},
	}
}

pub(super) fn organization_option_view(
	organization: Organization,
) -> OrganizationOptionView {
	OrganizationOptionView {
		id: organization.id,
		name: organization.name,
		org_type: organization.org_type,
	}
}

pub(super) async fn current_user_organization_selection_view(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<CurrentUserOrganizationSelectionView> {
	let active: Organization =
		OrganizationBmc::get(ctx, mm, ctx.organization_id()).await?;
	let available =
		UserBmc::list_member_organizations(ctx, mm, ctx.user_id()).await?;
	if available.is_empty() {
		return Err(Error::Internal {
			operation: "load user organizations",
			source: std::io::Error::other(
				"active user has no organization memberships",
			)
			.into(),
		});
	}
	Ok(CurrentUserOrganizationSelectionView {
		active_organization: organization_option_view(active),
		available_organizations: available
			.into_iter()
			.map(organization_option_view)
			.collect(),
	})
}
