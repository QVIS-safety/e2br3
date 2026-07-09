use super::*;

pub(super) fn user_view(user: User) -> UserView {
	let active = user_is_effectively_active(&user);
	let access_sender_ids = user.access_sender_ids.clone();
	let access_product_ids = user.access_product_ids.clone();
	let access_study_ids = user.access_study_ids.clone();
	let access_blind_allowed = user.access_blind_allowed;
	let active_sender_identifier = user.active_sender_identifier.clone();
	UserView {
		id: user.id,
		organization_id: user.organization_id,
		email: user.email,
		username: user.username,
		role: user.role.clone(),
		role_meta: role_metadata(&user.role, None),
		comments: user.comments,
		other_information: user.other_information,
		scope: UserScopeView {
			assigned_sender_ids: lib_rest_core::scope_values_from_raw(
				access_sender_ids.as_deref(),
			),
			assigned_product_ids: lib_rest_core::scope_values_from_raw(
				access_product_ids.as_deref(),
			),
			assigned_study_ids: lib_rest_core::scope_values_from_raw(
				access_study_ids.as_deref(),
			),
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
	}
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
	let mut available =
		UserBmc::list_member_organizations(ctx, mm, ctx.user_id()).await?;
	if available.is_empty() {
		available.push(active.clone());
	}
	Ok(CurrentUserOrganizationSelectionView {
		active_organization: organization_option_view(active),
		available_organizations: available
			.into_iter()
			.map(organization_option_view)
			.collect(),
	})
}
