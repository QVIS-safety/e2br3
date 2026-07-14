use super::shared::*;

pub async fn create_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<ReceiverPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = ReceiverPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = ReceiverPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_created(entity))
}

pub async fn list_receiver_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReceiverPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ReceiverPresaveBmc::list(&ctx, &mm, None).await?;
	Ok(rest_ok(entities))
}

pub async fn get_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ReceiverPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn update_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReceiverPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	if data.deleted == Some(true) {
		PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Receiver, id)
			.await?;
	} else {
		ReceiverPresaveBmc::update(&ctx, &mm, id, data).await?;
	}
	let entity = ReceiverPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Receiver, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct ReceiverPresaveDetails {
	pub parent: ReceiverPresave,
	pub consignees: Vec<ReceiverPresaveConsignee>,
	pub routes: Vec<ReceiverPresaveRoute>,
	pub children: ReceiverPresaveDetailsChildren,
}

#[derive(Debug, Serialize)]
pub struct ReceiverPresaveDetailsChildren {
	pub consignees: Vec<ReceiverPresaveConsignee>,
	pub routes: Vec<ReceiverPresaveRoute>,
}

#[derive(Deserialize)]
pub struct ReceiverPresaveDetailsForUpdate {
	pub parent: Option<ReceiverPresaveForUpdate>,
	pub consignees: Option<Vec<ReceiverConsigneeDetailsForUpdate>>,
	pub routes: Option<Vec<ReceiverRouteDetailsForUpdate>>,
	pub children: Option<ReceiverPresaveChildrenDetailsForUpdate>,
}

#[derive(Deserialize)]
pub struct ReceiverPresaveChildrenDetailsForUpdate {
	pub consignees: Option<Vec<ReceiverConsigneeDetailsForUpdate>>,
	pub routes: Option<Vec<ReceiverRouteDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiverConsigneeDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub name: Option<String>,
	pub phone: Option<String>,
	pub email: Option<String>,
}

impl ReceiverConsigneeDetailsForUpdate {
	fn into_update(self) -> ReceiverPresaveConsigneeForUpdate {
		ReceiverPresaveConsigneeForUpdate {
			sequence_number: self.sequence_number,
			name: self.name,
			phone: self.phone,
			email: self.email,
		}
	}

	fn into_create(
		self,
		receiver_presave_id: Uuid,
	) -> Result<ReceiverPresaveConsigneeForCreate> {
		Ok(ReceiverPresaveConsigneeForCreate {
			receiver_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver consignee details create requires sequence_number"
							.to_string(),
				}
			})?,
			name: self.name,
			phone: self.phone,
			email: self.email,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct ReceiverRouteDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub authority: Option<String>,
	pub receiver_label: Option<String>,
	pub batch_receiver_identifier: Option<String>,
	pub message_receiver_identifier: Option<String>,
	pub condition_page: Option<String>,
	pub condition_field_code: Option<String>,
	pub condition_operator: Option<String>,
	pub condition_value_code: Option<String>,
	pub condition_value_label: Option<String>,
}

impl ReceiverRouteDetailsForUpdate {
	fn into_update(self) -> ReceiverPresaveRouteForUpdate {
		ReceiverPresaveRouteForUpdate {
			sequence_number: self.sequence_number,
			authority: self.authority,
			receiver_label: self.receiver_label,
			batch_receiver_identifier: self.batch_receiver_identifier,
			message_receiver_identifier: self.message_receiver_identifier,
			condition_page: self.condition_page,
			condition_field_code: self.condition_field_code,
			condition_operator: self.condition_operator,
			condition_value_code: self.condition_value_code,
			condition_value_label: self.condition_value_label,
		}
	}

	fn into_create(
		self,
		receiver_presave_id: Uuid,
	) -> Result<ReceiverPresaveRouteForCreate> {
		Ok(ReceiverPresaveRouteForCreate {
			receiver_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver route details create requires sequence_number"
							.to_string(),
				}
			})?,
			authority: self.authority.ok_or_else(|| Error::BadRequest {
				message: "receiver route details create requires authority"
					.to_string(),
			})?,
			receiver_label: self.receiver_label.ok_or_else(|| {
				Error::BadRequest {
					message: "receiver route details create requires receiver_label"
						.to_string(),
				}
			})?,
			batch_receiver_identifier: self.batch_receiver_identifier,
			message_receiver_identifier: self
				.message_receiver_identifier
				.ok_or_else(|| {
					Error::BadRequest {
					message:
						"receiver route details create requires message_receiver_identifier"
							.to_string(),
				}
				})?,
			condition_page: self.condition_page.ok_or_else(|| {
				Error::BadRequest {
					message: "receiver route details create requires condition_page"
						.to_string(),
				}
			})?,
			condition_field_code: self.condition_field_code.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver route details create requires condition_field_code"
							.to_string(),
				}
			})?,
			condition_operator: self.condition_operator.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver route details create requires condition_operator"
							.to_string(),
				}
			})?,
			condition_value_code: self.condition_value_code.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver route details create requires condition_value_code"
							.to_string(),
				}
			})?,
			condition_value_label: self.condition_value_label.ok_or_else(|| {
				Error::BadRequest {
					message:
						"receiver route details create requires condition_value_label"
							.to_string(),
				}
			})?,
		})
	}
}

pub async fn get_receiver_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_receiver_presave_details(&ctx, &mm, id).await?;
	Ok(rest_ok(details))
}

pub async fn update_receiver_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReceiverPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	ReceiverPresaveBmc::get(&ctx, &mm, id).await?;

	let ParamsForUpdate { data } = params;
	require_receiver_detail_operation_permissions(&ctx, &data)?;
	if data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true))
	{
		if data.consignees.is_some()
			|| data.routes.is_some()
			|| data.children.is_some()
		{
			return Err(Error::BadRequest {
				message: "presave deletion cannot include child changes".into(),
			});
		}
		PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Receiver, id)
			.await?;
		return Ok(rest_ok(load_receiver_presave_details(&ctx, &mm, id).await?));
	}
	preflight_receiver_presave_details(&ctx, &mm, id, &data).await?;
	apply_receiver_presave_details(&ctx, &mm, id, data).await?;

	let details = load_receiver_presave_details(&ctx, &mm, id).await?;
	Ok(rest_ok(details))
}

async fn apply_receiver_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: ReceiverPresaveDetailsForUpdate,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) =
		lib_core::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err.into());
	}

	let apply_result = apply_receiver_presave_details_inner(ctx, mm, id, data).await;
	if let Err(err) = apply_result {
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}

	dbx.commit_txn().await.map_err(model::Error::from)?;
	Ok(())
}

async fn apply_receiver_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: ReceiverPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.parent {
		ReceiverPresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(consignees) = data.consignees {
		for consignee in consignees {
			upsert_receiver_consignee_detail(ctx, mm, id, consignee).await?;
		}
	}
	if let Some(routes) = data.routes {
		for route in routes {
			upsert_receiver_route_detail(ctx, mm, id, route).await?;
		}
	}
	if let Some(children) = data.children {
		if let Some(consignees) = children.consignees {
			for consignee in consignees {
				upsert_receiver_consignee_detail(ctx, mm, id, consignee).await?;
			}
		}
		if let Some(routes) = children.routes {
			for route in routes {
				upsert_receiver_route_detail(ctx, mm, id, route).await?;
			}
		}
	}
	Ok(())
}

async fn load_receiver_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<ReceiverPresaveDetails> {
	let parent = ReceiverPresaveBmc::get(ctx, mm, id).await?;
	let consignees =
		ReceiverPresaveConsigneeBmc::list_by_parent(ctx, mm, id).await?;
	let routes = ReceiverPresaveRouteBmc::list_by_parent(ctx, mm, id).await?;
	let children = ReceiverPresaveDetailsChildren {
		consignees: consignees.clone(),
		routes: routes.clone(),
	};
	Ok(ReceiverPresaveDetails {
		parent,
		consignees,
		routes,
		children,
	})
}

fn require_receiver_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	data: &ReceiverPresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.consignees
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|consignee| consignee.id.is_none() && !consignee.delete);
	let creates_route = data
		.routes
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|route| route.id.is_none() && !route.delete);
	let creates_nested_child = data
		.children
		.as_ref()
		.and_then(|children| children.consignees.as_deref())
		.unwrap_or_default()
		.iter()
		.any(|consignee| consignee.id.is_none() && !consignee.delete)
		|| data
			.children
			.as_ref()
			.and_then(|children| children.routes.as_deref())
			.unwrap_or_default()
			.iter()
			.any(|route| route.id.is_none() && !route.delete);
	let deletes_child = data
		.consignees
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|consignee| consignee.delete);
	let deletes_route = data
		.routes
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|route| route.delete);
	let deletes_nested_child = data
		.children
		.as_ref()
		.and_then(|children| children.consignees.as_deref())
		.unwrap_or_default()
		.iter()
		.any(|consignee| consignee.delete)
		|| data
			.children
			.as_ref()
			.and_then(|children| children.routes.as_deref())
			.unwrap_or_default()
			.iter()
			.any(|route| route.delete);
	let deletes_parent = data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true));

	if creates_child || creates_route || creates_nested_child {
		require_permission(ctx, PRESAVE_TEMPLATE_CREATE)?;
	}
	if deletes_child || deletes_route || deletes_nested_child || deletes_parent {
		require_permission(ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	Ok(())
}

async fn preflight_receiver_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	data: &ReceiverPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(consignees) = &data.consignees {
		for consignee in consignees {
			preflight_receiver_consignee_detail(ctx, mm, receiver_id, consignee)
				.await?;
		}
	}
	if let Some(routes) = &data.routes {
		for route in routes {
			preflight_receiver_route_detail(ctx, mm, receiver_id, route).await?;
		}
	}
	if let Some(children) = &data.children {
		if let Some(consignees) = &children.consignees {
			for consignee in consignees {
				preflight_receiver_consignee_detail(ctx, mm, receiver_id, consignee)
					.await?;
			}
		}
		if let Some(routes) = &children.routes {
			for route in routes {
				preflight_receiver_route_detail(ctx, mm, receiver_id, route).await?;
			}
		}
	}
	Ok(())
}

async fn preflight_receiver_consignee_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	consignee: &ReceiverConsigneeDetailsForUpdate,
) -> Result<()> {
	if consignee.delete && consignee.id.is_none() {
		return Err(Error::BadRequest {
			message: "receiver consignee delete requires id".to_string(),
		});
	}

	if let Some(id) = consignee.id {
		let entity = ReceiverPresaveConsigneeBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			receiver_id,
			entity.receiver_presave_id,
			id,
			"receiver",
			"receiver_presave_consignees",
		)?;
	} else if !consignee.delete {
		validate_receiver_consignee_detail_create(consignee)?;
	}
	Ok(())
}

fn validate_receiver_consignee_detail_create(
	consignee: &ReceiverConsigneeDetailsForUpdate,
) -> Result<()> {
	if consignee.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message: "receiver consignee details create requires sequence_number"
				.to_string(),
		});
	}
	Ok(())
}

async fn preflight_receiver_route_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	route: &ReceiverRouteDetailsForUpdate,
) -> Result<()> {
	if route.delete && route.id.is_none() {
		return Err(Error::BadRequest {
			message: "receiver route delete requires id".to_string(),
		});
	}

	if let Some(id) = route.id {
		let entity = ReceiverPresaveRouteBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			receiver_id,
			entity.receiver_presave_id,
			id,
			"receiver",
			"receiver_presave_routes",
		)?;
	} else if !route.delete {
		validate_receiver_route_detail_create(route)?;
	}
	Ok(())
}

fn validate_receiver_route_detail_create(
	route: &ReceiverRouteDetailsForUpdate,
) -> Result<()> {
	let required = [
		(route.sequence_number.is_some(), "sequence_number"),
		(route.authority.is_some(), "authority"),
		(route.receiver_label.is_some(), "receiver_label"),
		(
			route.message_receiver_identifier.is_some(),
			"message_receiver_identifier",
		),
		(route.condition_page.is_some(), "condition_page"),
		(route.condition_field_code.is_some(), "condition_field_code"),
		(route.condition_operator.is_some(), "condition_operator"),
		(route.condition_value_code.is_some(), "condition_value_code"),
		(
			route.condition_value_label.is_some(),
			"condition_value_label",
		),
	];
	for (present, field) in required {
		if !present {
			return Err(Error::BadRequest {
				message: format!("receiver route details create requires {field}"),
			});
		}
	}
	Ok(())
}

async fn upsert_receiver_consignee_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	consignee: ReceiverConsigneeDetailsForUpdate,
) -> Result<()> {
	if consignee.delete && consignee.id.is_none() {
		return Err(Error::BadRequest {
			message: "receiver consignee delete requires id".to_string(),
		});
	}

	if let Some(id) = consignee.id {
		let entity = ReceiverPresaveConsigneeBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			receiver_id,
			entity.receiver_presave_id,
			id,
			"receiver",
			"receiver_presave_consignees",
		)?;
		if consignee.delete {
			ReceiverPresaveConsigneeBmc::delete(ctx, mm, id).await?;
		} else {
			ReceiverPresaveConsigneeBmc::update(
				ctx,
				mm,
				id,
				consignee.into_update(),
			)
			.await?;
		}
	} else {
		ReceiverPresaveConsigneeBmc::create(
			ctx,
			mm,
			consignee.into_create(receiver_id)?,
		)
		.await?;
	}
	Ok(())
}

async fn upsert_receiver_route_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	route: ReceiverRouteDetailsForUpdate,
) -> Result<()> {
	if route.delete && route.id.is_none() {
		return Err(Error::BadRequest {
			message: "receiver route delete requires id".to_string(),
		});
	}

	if let Some(id) = route.id {
		let entity = ReceiverPresaveRouteBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			receiver_id,
			entity.receiver_presave_id,
			id,
			"receiver",
			"receiver_presave_routes",
		)?;
		if route.delete {
			ReceiverPresaveRouteBmc::delete(ctx, mm, id).await?;
		} else {
			ReceiverPresaveRouteBmc::update(ctx, mm, id, route.into_update())
				.await?;
		}
	} else {
		ReceiverPresaveRouteBmc::create(ctx, mm, route.into_create(receiver_id)?)
			.await?;
	}
	Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ReceiverConsigneeForRestCreate {
	pub sequence_number: i32,
	pub name: Option<String>,
	pub phone: Option<String>,
	pub email: Option<String>,
}

impl ReceiverConsigneeForRestCreate {
	fn into_core(
		self,
		receiver_presave_id: Uuid,
	) -> ReceiverPresaveConsigneeForCreate {
		ReceiverPresaveConsigneeForCreate {
			receiver_presave_id,
			sequence_number: self.sequence_number,
			name: self.name,
			phone: self.phone,
			email: self.email,
		}
	}
}

generate_presave_child_rest_fns! {
	Bmc: ReceiverPresaveConsigneeBmc,
	Entity: ReceiverPresaveConsignee,
	RestCreate: ReceiverConsigneeForRestCreate,
	ForUpdate: ReceiverPresaveConsigneeForUpdate,
	CreateFn: create_receiver_consignee,
	ListFn: list_receiver_consignees,
	GetFn: get_receiver_consignee,
	UpdateFn: update_receiver_consignee,
	DeleteFn: delete_receiver_consignee,
	ParentField: receiver_presave_id,
	ParentScopeFn: allow_presave_parent_scope,
	EntityName: "receiver_presave_consignees",
	UpdatePermission: update,
	DeleteMode: hard
}
