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
	let assigns_built_in_admin = data
		.role
		.as_deref()
		.map(canonical_role)
		.is_some_and(|role| is_built_in_admin_role(&role));
	let action_id = if assigns_built_in_admin {
		"user.create.built_in_role_assignment"
	} else if assigns_role {
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
	let sender_ids =
		parse_scope_input(data.access_sender_ids.clone()).unwrap_or_default();
	let product_ids =
		parse_scope_input(data.access_product_ids.clone()).unwrap_or_default();
	let study_ids =
		parse_scope_input(data.access_study_ids.clone()).unwrap_or_default();
	validate_scope_hierarchy(
		&db_ctx,
		&mm,
		organization_id,
		&sender_ids,
		&product_ids,
		&study_ids,
	)
	.await?;
	validate_optional_uuid_identifier(
		"active_sender_identifier",
		data.active_sender_identifier.as_deref(),
	)?;
	if !matches!(
		snapshot.identity().built_in_kind(),
		Some(
			BuiltInIdentityKind::SponsorCroAdministrator
				| BuiltInIdentityKind::SponsorCompanyAdministrator
		)
	) && has_sender_scope_assignment(
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
		pwd_clear: initial_password(data.pwd_clear)?,
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
			data: user_view(&db_ctx, &mm, entity).await?,
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
			data: user_view(&db_ctx, &mm, entity).await?,
		}),
	))
}

/// POST /api/users/me/password
/// Set current user's password and clear first-login password reset requirement.
pub async fn set_my_password(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	cookies: tower_cookies::Cookies,
	Json(params): Json<ParamsForCreate<SetMyPasswordBody>>,
) -> Result<StatusCode> {
	let CtxW(ctx, must_change_password) = ctx_w;
	let ParamsForCreate { data } = params;
	validate_new_password(&data.new_password)?;
	if must_change_password {
		UserBmc::update_pwd_and_clear_must_change(
			&ctx,
			&mm,
			ctx.user_id(),
			&data.new_password,
		)
		.await?;
	} else if data.current_password.is_empty() {
		return Err(Error::BadRequest {
			message: "current_password is required".to_string(),
		});
	} else if !UserBmc::change_password(
		&ctx,
		&mm,
		ctx.user_id(),
		&data.current_password,
		&data.new_password,
	)
	.await?
	{
		return Err(Error::BadRequest {
			message: "current password is invalid".to_string(),
		});
	}
	token::remove_token_cookie(&cookies).map_err(|err| Error::BadRequest {
		message: format!("failed to clear the previous session: {err}"),
	})?;
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
	let include_blinded = raw_query.as_deref().is_some_and(|query| {
		query.split('&').any(|part| {
			matches!(part, "include_blinded=true" | "includeBlinded=true")
		})
	});
	let mut params = ParamsList::<UserFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	if !include_blinded {
		params.filters.get_or_insert_default().push(UserFilter {
			access_blind_allowed: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
	}
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
	let entities = user_views(&db_ctx, &mm, entities).await?;
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
	snapshot: AuthorizationSnapshotW,
	axum::extract::Query(query): axum::extract::Query<WorkflowUserOptionsQuery>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<WorkflowUserOptionView>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_subject_action(
		&ctx,
		&snapshot,
		&mm,
		"case.workflow.config.read",
		move |ctx, mm| {
			Box::pin(async move {
				let users = UserBmc::list_workflow_options(
					ctx,
					mm,
					query.limit.unwrap_or(200),
				)
				.await?;
				let users = users
					.into_iter()
					.map(workflow_user_option_view)
					.collect::<Vec<_>>();
				Ok((StatusCode::OK, Json(DataRestResult { data: users })))
			})
		},
	)
	.await
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
	let assigns_built_in_admin = data
		.role
		.as_deref()
		.map(canonical_role)
		.is_some_and(|role| is_built_in_admin_role(&role));
	let action_id = if assigns_built_in_admin {
		"user.update.built_in_role_assignment"
	} else if data.role.is_some() {
		"user.update.role_assignment"
	} else {
		"user.update"
	};
	let forbids_sender_scope_assignment = !matches!(
		snapshot.identity().built_in_kind(),
		Some(
			BuiltInIdentityKind::SponsorCroAdministrator
				| BuiltInIdentityKind::SponsorCompanyAdministrator
		)
	);
	lib_rest_core::with_authorized_user_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		action_id,
		"user.update.built_in_administrator",
		move |db_ctx, mm| {
			Box::pin(async move {
				let clear_access_start_at =
					matches!(data.access_start_at, Some(None));
				let clear_access_end_at = matches!(data.access_end_at, Some(None));
				validate_uuid_scope("access_sender_ids", &data.access_sender_ids)?;
				validate_uuid_scope("access_product_ids", &data.access_product_ids)?;
				validate_uuid_scope("access_study_ids", &data.access_study_ids)?;
				validate_optional_uuid_identifier(
					"active_sender_identifier",
					data.active_sender_identifier.as_deref(),
				)?;
				if forbids_sender_scope_assignment
					&& has_sender_scope_assignment(
						&data.active_sender_identifier,
						&data.access_sender_ids,
					) {
					return Err(sender_scope_assignment_forbidden());
				}
				let existing: User = UserBmc::get(db_ctx, mm, id).await?;
				let sender_ids = scope_values_for_update(
					data.access_sender_ids.as_ref(),
					existing.access_sender_ids.as_deref(),
				);
				let product_ids = scope_values_for_update(
					data.access_product_ids.as_ref(),
					existing.access_product_ids.as_deref(),
				);
				let study_ids = scope_values_for_update(
					data.access_study_ids.as_ref(),
					existing.access_study_ids.as_deref(),
				);
				validate_scope_hierarchy(
					db_ctx,
					mm,
					existing.organization_id,
					&sender_ids,
					&product_ids,
					&study_ids,
				)
				.await?;
				let role = normalize_user_role(data.role);
				if role.is_some() {
					validate_permission_profile_role_for_org(
						db_ctx,
						mm,
						role.as_deref(),
					)
					.await?;
					validate_sponsor_admin_role_for_org(
						db_ctx,
						mm,
						existing.organization_id,
						role.as_deref(),
					)
					.await?;
					validate_single_sponsor_admin_for_org(
						db_ctx,
						mm,
						existing.organization_id,
						role.as_deref(),
						Some(id),
					)
					.await?;
				}
				let email = normalize_optional_email_input(data.email)?;
				let username = normalize_optional_username_input(data.username)?;
				if username.as_deref().is_some_and(str::is_empty) {
					return Err(Error::BadRequest {
						message: "username is required".to_string(),
					});
				}
				let update = UserForUpdate {
					organization_id: None,
					email,
					username,
					role,
					active: data.active,
					comments: data.comments,
					other_information: data.other_information,
					access_start_at: data.access_start_at.flatten(),
					access_end_at: data.access_end_at.flatten(),
					access_sender_ids: serialize_scope_input(data.access_sender_ids),
					access_product_ids: serialize_scope_input(
						data.access_product_ids,
					),
					access_study_ids: serialize_scope_input(data.access_study_ids),
					access_blind_allowed: data.access_blind_allowed,
					active_sender_identifier: data.active_sender_identifier,
					last_login_at: data.last_login_at,
				};
				UserBmc::update_patch(
					db_ctx,
					mm,
					id,
					update,
					clear_access_start_at,
					clear_access_end_at,
				)
				.await?;
				let entity: User = UserBmc::get(db_ctx, mm, id).await?;
				Ok((
					StatusCode::OK,
					Json(DataRestResult {
						data: user_view(db_ctx, mm, entity).await?,
					}),
				))
			})
		},
	)
	.await
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
			data: user_view(&ctx, &mm, entity).await?,
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
	let privileges =
		current_user_menu_privileges(&ctx, snapshot.identity().built_in_kind(), &mm)
			.await?;
	let eligible_actions = lib_core::authorization::eligible_action_ids(&snapshot);
	let policy_version = snapshot.version().organization_revision();
	Ok((
		StatusCode::OK,
		Json(DataRestResult {
			data: CurrentUserProfileView {
				user: user_view(&ctx, &mm, entity).await?,
				active_organization: organization_selection.active_organization,
				available_organizations: organization_selection
					.available_organizations,
				routing,
				privileges,
				eligible_actions,
				policy_version,
			},
		}),
	))
}

async fn current_user_menu_privileges(
	ctx: &Ctx,
	built_in_kind: Option<BuiltInIdentityKind>,
	mm: &ModelManager,
) -> Result<Vec<AdminMenuPrivilege>> {
	if let Some(kind) = built_in_kind {
		return Ok(built_in_menu_privileges(kind));
	}
	let Ok(profile_id) = Uuid::parse_str(ctx.role()) else {
		return Ok(Vec::new());
	};
	let row = match PermissionProfileBmc::get(ctx, mm, profile_id).await {
		Ok(row) => row,
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {
			return Ok(Vec::new());
		}
		Err(lib_core::model::Error::Store(message))
			if message.contains("RowNotFound") =>
		{
			return Ok(Vec::new());
		}
		Err(error) => return Err(Error::Model(error)),
	};
	if !row.active || row.organization_id != ctx.organization_id() {
		return Ok(Vec::new());
	}
	Ok(normalize_menu_privileges(&row.privileges_json.0).unwrap_or_default())
}

pub async fn update_current_user_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	cookies: tower_cookies::Cookies,
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
	let current_user: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
	let target_account = UserBmc::auth_by_email_and_organization(
		&mm,
		&current_user.email,
		next_organization_id,
	)
	.await?
	.ok_or_else(|| Error::AccessDenied {
		required_role: "active_organization_membership".to_string(),
	})?;
	token::set_token_cookie(
		&cookies,
		&target_account.email,
		target_account.organization_id,
		target_account.token_salt,
	)
	.map_err(|_| Error::BadRequest {
		message: "could not establish organization session".to_string(),
	})?;
	let selected_ctx = Ctx::new(
		target_account.id,
		target_account.organization_id,
		target_account.role.clone(),
	)
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
	axum::extract::Query(query): axum::extract::Query<RoutingProfileQuery>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_rest_core::RoutingProfile>>,
)> {
	let ctx = ctx_w.0;
	let organization_id = query
		.organization_id
		.unwrap_or_else(|| ctx.organization_id());
	let routing_ctx = if organization_id == ctx.organization_id() {
		ctx
	} else {
		let current_user: User = UserBmc::get(&ctx, &mm, ctx.user_id()).await?;
		let target_account = UserBmc::auth_by_email_and_organization(
			&mm,
			&current_user.email,
			organization_id,
		)
		.await?
		.ok_or_else(|| Error::AccessDenied {
			required_role: "active_organization_membership".to_string(),
		})?;
		Ctx::new(
			target_account.id,
			target_account.organization_id,
			target_account.role,
		)
		.map_err(|_| Error::BadRequest {
			message: "valid organization context required".to_string(),
		})?
	};
	let routing = routing_profile_for_user(&routing_ctx, &mm).await?;
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
