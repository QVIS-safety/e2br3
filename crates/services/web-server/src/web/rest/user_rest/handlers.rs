use super::*;

fn user_target_organization(
	ctx: &Ctx,
	snapshot: &AuthorizationSnapshotW,
) -> Option<Uuid> {
	if snapshot.identity().is_platform_administrator() {
		None
	} else {
		Some(ctx.organization_id())
	}
}

/// POST /api/users
/// Create a new user
/// **Requires User.Create permission (admin only)**
pub async fn create_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<UserForCreateAdminPayload>>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let ParamsForCreate { data } = params;
	let organization_id = if snapshot.identity().is_platform_administrator() {
		data.organization_id.ok_or_else(|| Error::BadRequest {
			message: "organization_id is required".to_string(),
		})?
	} else {
		ctx.organization_id()
	};
	let assigns_role = data
		.role
		.as_deref()
		.is_some_and(|role| canonical_role(role) != ROLE_USER);
	let action_id = if assigns_role {
		"user.create.role_assignment"
	} else {
		"user.create"
	};
	let create_action = policy_registry()
		.context_action::<Proposed<UserCreateProposal>>(action_id)
		.expect("registered user create policy");
	let permit = authorize_contextual_mutation(
		create_action,
		&snapshot,
		proposed_user_context(organization_id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	validate_uuid_scope("access_sender_ids", &data.access_sender_ids)?;
	validate_uuid_scope("access_product_ids", &data.access_product_ids)?;
	validate_uuid_scope("access_study_ids", &data.access_study_ids)?;
	validate_optional_uuid_identifier(
		"active_sender_identifier",
		data.active_sender_identifier.as_deref(),
	)?;
	if sender_scope_assignment_forbidden_for_ctx(&ctx)
		&& has_sender_scope_assignment(
			&data.active_sender_identifier,
			&data.access_sender_ids,
		) {
		return Err(sender_scope_assignment_forbidden());
	}
	if organization_id.is_nil() {
		return Err(Error::BadRequest {
			message: "organization context is required".to_string(),
		});
	}
	// New users are provisioned with a temporary password and must reset it on first login.
	let role = normalize_user_role(data.role);
	let email = normalize_email_input(data.email)?;
	let username = normalize_optional_username_input(data.username)?
		.filter(|value| !value.is_empty())
		.unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());
	validate_username(&username)?;
	validate_permission_profile_role_for_org(&db_ctx, &mm, role.as_deref()).await?;
	validate_sponsor_admin_assignment_authority(&ctx, role.as_deref())?;
	validate_sponsor_admin_role_for_org(
		&db_ctx,
		&mm,
		organization_id,
		role.as_deref(),
	)
	.await?;
	validate_single_sponsor_admin_for_org(
		&db_ctx,
		&mm,
		organization_id,
		role.as_deref(),
		None,
	)
	.await?;
	let create = UserForCreate {
		organization_id,
		email,
		username: Some(username),
		pwd_clear: initial_password(data.pwd_clear),
		role,
		comments: data.comments,
		other_information: data.other_information,
		access_start_at: data.access_start_at,
		access_end_at: data.access_end_at,
		active_sender_identifier: data.active_sender_identifier,
		access_sender_ids: parse_scope_input(data.access_sender_ids),
		access_product_ids: parse_scope_input(data.access_product_ids),
		access_study_ids: parse_scope_input(data.access_study_ids),
		access_blind_allowed: data.access_blind_allowed,
	};
	let id = UserBmc::create(&db_ctx, &mm, create).await?;
	UserBmc::set_must_change_password(&db_ctx, &mm, id, true).await?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::CREATED,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

/// GET /api/users/:id
/// Get a user by ID
/// **Requires User.Read permission (all authenticated users)**
pub async fn get_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let target_organization_id = user_target_organization(&ctx, &snapshot);
	let action = policy_registry()
		.context_action::<Existing<UserResource>>("user.read")
		.expect("user.read policy");
	let permit = authorize_contextual_read(
		action,
		&snapshot,
		existing_user_read_context(id, target_organization_id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_read(&ctx, &snapshot, &permit)?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

/// POST /api/users/me/password
/// Set current user's password and clear first-login password reset requirement.
pub async fn set_my_password(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<SetMyPasswordBody>>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	let ParamsForCreate { data } = params;
	let new_password = data.new_password.trim();
	if new_password.is_empty() {
		return Err(Error::BadRequest {
			message: "new_password is required".to_string(),
		});
	}
	let privileged_ctx = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "valid user context required".to_string(),
	})?;
	UserBmc::update_pwd_and_clear_must_change(
		&privileged_ctx,
		&mm,
		ctx.user_id(),
		new_password,
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users
/// List all users with optional filtering
/// **Requires User.List permission (all authenticated users can list users in their org)**
pub async fn list_users(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<UserView>>>)> {
	let ctx = ctx_w.0;
	let target_organization_id = user_target_organization(&ctx, &snapshot);
	let params = ParamsList::<UserFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	let action = policy_registry()
		.context_action("user.list")
		.expect("user.list policy");
	let permit = authorize_contextual_read(
		action,
		&snapshot,
		user_collection_context(target_organization_id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_read(&ctx, &snapshot, &permit)?;
	let entities =
		UserBmc::list(&db_ctx, &mm, params.filters, params.list_options).await?;
	let entities = entities.into_iter().map(user_view).collect::<Vec<_>>();
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowUserOptionsQuery {
	pub limit: Option<i64>,
}

/// GET /api/users/workflow-options
/// Lightweight active user options for workflow assignment selectors.
pub async fn list_workflow_user_options(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::Query(query): axum::extract::Query<WorkflowUserOptionsQuery>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<WorkflowUserOptionView>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	let users =
		UserBmc::list_workflow_options(&ctx, &mm, query.limit.unwrap_or(200))
			.await?;
	let users = users
		.into_iter()
		.map(workflow_user_option_view)
		.collect::<Vec<_>>();
	Ok((StatusCode::OK, Json(DataRestResult { data: users })))
}

/// PUT /api/users/:id
/// Update a user
/// **Requires User.Update permission (admin only)**
pub async fn update_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<UserForUpdateAdminPayload>>,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let ParamsForUpdate { data } = params;
	let target_organization_id = user_target_organization(&ctx, &snapshot);
	let action_id = if data.role.is_some() {
		"user.update.role_assignment"
	} else {
		"user.update"
	};
	let action = policy_registry()
		.context_action::<Existing<UserResource>>(action_id)
		.expect("registered user update policy");
	let permit = authorize_contextual_mutation(
		action,
		&snapshot,
		existing_user_mutation_context(id, target_organization_id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	validate_uuid_scope("access_sender_ids", &data.access_sender_ids)?;
	validate_uuid_scope("access_product_ids", &data.access_product_ids)?;
	validate_uuid_scope("access_study_ids", &data.access_study_ids)?;
	validate_optional_uuid_identifier(
		"active_sender_identifier",
		data.active_sender_identifier.as_deref(),
	)?;
	if sender_scope_assignment_forbidden_for_ctx(&ctx)
		&& has_sender_scope_assignment(
			&data.active_sender_identifier,
			&data.access_sender_ids,
		) {
		return Err(sender_scope_assignment_forbidden());
	}
	let existing: User = UserBmc::get(&db_ctx, &mm, id).await?;
	validate_existing_sponsor_admin_mutation(&ctx, &existing)?;
	let role = normalize_user_role(data.role);
	if role.is_some() {
		validate_permission_profile_role_for_org(&db_ctx, &mm, role.as_deref())
			.await?;
		validate_sponsor_admin_assignment_authority(&ctx, role.as_deref())?;
		validate_sponsor_admin_role_for_org(
			&db_ctx,
			&mm,
			existing.organization_id,
			role.as_deref(),
		)
		.await?;
		validate_single_sponsor_admin_for_org(
			&db_ctx,
			&mm,
			existing.organization_id,
			role.as_deref(),
			Some(id),
		)
		.await?;
	}
	let email = normalize_optional_email_input(data.email)?;
	let username = normalize_optional_username_input(data.username)?;
	let update = UserForUpdate {
		organization_id: None,
		email,
		username,
		role,
		comments: data.comments,
		other_information: data.other_information,
		access_start_at: data.access_start_at,
		access_end_at: data.access_end_at,
		access_sender_ids: serialize_scope_input(data.access_sender_ids),
		access_product_ids: serialize_scope_input(data.access_product_ids),
		access_study_ids: serialize_scope_input(data.access_study_ids),
		access_blind_allowed: data.access_blind_allowed,
		active_sender_identifier: data.active_sender_identifier,
		active: data.active,
		last_login_at: data.last_login_at,
	};
	UserBmc::update(&db_ctx, &mm, id, update).await?;
	let entity: User = UserBmc::get(&db_ctx, &mm, id).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

/// DELETE /api/users/:id
/// Delete a user
/// **Requires User.Delete permission (admin only)**
pub async fn delete_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	let target_organization_id = user_target_organization(&ctx, &snapshot);
	let action = policy_registry()
		.context_action::<Existing<UserResource>>("user.delete")
		.expect("user.delete policy");
	let permit = authorize_contextual_mutation(
		action,
		&snapshot,
		existing_user_mutation_context(id, target_organization_id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	if id == ctx.user_id() {
		return Err(Error::BadRequest {
			message: "cannot delete yourself".to_string(),
		});
	}
	let existing: User = UserBmc::get(&db_ctx, &mm, id).await?;
	validate_existing_sponsor_admin_mutation(&ctx, &existing)?;
	UserBmc::update(
		&db_ctx,
		&mm,
		id,
		UserForUpdate {
			organization_id: None,
			email: None,
			username: None,
			role: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
			active: Some(false),
			last_login_at: None,
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/me
/// Get current user's profile
/// **Any authenticated user**
pub async fn get_current_user(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<UserView>>)> {
	let ctx = ctx_w.0;
	let entity: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: user_view(entity),
		}),
	))
}

pub async fn get_current_user_profile(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<CurrentUserProfileView>>)> {
	let ctx = ctx_w.0;
	let entity: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
	let organization_selection =
		current_user_organization_selection_view(&ctx, &mm).await?;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	let privileges = current_user_menu_privileges(&ctx, &mm).await?;
	let mut permissions = all_permissions()
		.iter()
		.copied()
		.filter(|permission| {
			legacy_permission_allowed(ctx.permission_subject(), *permission)
		})
		.map(|permission| permission.to_string())
		.collect::<Vec<_>>();
	permissions.sort_unstable();
	permissions.dedup();
	let policy_version = snapshot.version().organization_revision();
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CurrentUserProfileView {
				user: user_view(entity),
				active_organization: organization_selection.active_organization,
				available_organizations: organization_selection
					.available_organizations,
				routing,
				privileges,
				permissions,
				policy_version,
			},
		}),
	))
}

async fn current_user_menu_privileges(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<Vec<AdminMenuPrivilege>> {
	let built_in = built_in_menu_privileges(ctx.role());
	if !built_in.is_empty() {
		return Ok(built_in);
	}
	let Ok(profile_id) = Uuid::parse_str(ctx.role()) else {
		return Ok(Vec::new());
	};
	let row = PermissionProfileBmc::get(ctx, mm, profile_id)
		.await
		.map_err(Error::Model)?;
	if !row.active || row.organization_id != ctx.organization_id() {
		return Ok(Vec::new());
	}
	Ok(normalize_menu_privileges(&row.privileges_json.0).unwrap_or_default())
}

pub async fn update_current_user_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForUpdate<OrganizationSelectionBody>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<CurrentUserOrganizationSelectionView>>,
)> {
	let ctx = ctx_w.0;
	let next_organization_id = params.data.organization_id;
	if next_organization_id.is_nil() {
		return Err(Error::BadRequest {
			message: "organization_id is required".to_string(),
		});
	}
	let is_member = UserBmc::user_has_organization_membership(
		&ctx,
		&mm,
		ctx.user_id(),
		next_organization_id,
	)
	.await?;
	if !is_member {
		return Err(Error::AccessDenied {
			required_role: "organization_membership".to_string(),
		});
	}
	let update_ctx = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SYSTEM_ADMIN.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "valid organization update context required".to_string(),
	})?
	.with_compliance(
		ctx.change_reason().map(ToString::to_string),
		ctx.e_signature_id(),
	);
	UserBmc::update(
		&update_ctx,
		&mm,
		ctx.user_id(),
		UserForUpdate {
			organization_id: Some(next_organization_id),
			email: None,
			username: None,
			role: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: None,
			active: None,
			last_login_at: None,
		},
	)
	.await?;
	let selected_ctx =
		Ctx::new(ctx.user_id(), next_organization_id, ctx.role().to_string())
			.map_err(|_| Error::BadRequest {
				message: "valid selected organization context required".to_string(),
			})?
			.with_compliance(
				ctx.change_reason().map(ToString::to_string),
				ctx.e_signature_id(),
			);
	let selection =
		current_user_organization_selection_view(&selected_ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: selection })))
}

pub async fn get_current_user_routing(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_rest_core::RoutingProfile>>,
)> {
	let ctx = ctx_w.0;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: routing })))
}

pub async fn update_current_user_routing(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForUpdate<RoutingSelectionBody>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_rest_core::RoutingProfile>>,
)> {
	let ctx = ctx_w.0;
	let next_sender = validate_active_sender_selection(
		&ctx,
		&mm,
		params.data.active_sender_identifier.as_deref(),
	)
	.await?;
	let routing_update_ctx = Ctx::new(
		ctx.user_id(),
		ctx.organization_id(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)
	.map_err(|_| Error::BadRequest {
		message: "valid routing update context required".to_string(),
	})?;
	UserBmc::update(
		&routing_update_ctx,
		&mm,
		ctx.user_id(),
		UserForUpdate {
			organization_id: None,
			email: None,
			username: None,
			role: None,
			comments: None,
			other_information: None,
			access_start_at: None,
			access_end_at: None,
			access_sender_ids: None,
			access_product_ids: None,
			access_study_ids: None,
			access_blind_allowed: None,
			active_sender_identifier: next_sender,
			active: None,
			last_login_at: None,
		},
	)
	.await?;
	let routing = routing_profile_for_user(&ctx, &mm).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: routing })))
}
