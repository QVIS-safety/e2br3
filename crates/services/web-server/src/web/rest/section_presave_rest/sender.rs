use super::shared::*;

pub async fn create_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<SenderPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = SenderPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	if let Err(err) = ensure_sender_presave_scope(&ctx, &mm, &entity).await {
		SenderPresaveBmc::delete(&ctx, &mm, id).await?;
		return Err(err);
	}
	Ok(rest_created(entity))
}

pub async fn list_sender_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = SenderPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_sender_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok(rest_ok(entities))
}

pub async fn get_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn update_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<SenderPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	let current = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &current).await?;
	if data.deleted == Some(true) {
		let identifiers = sender_scope_identifiers(&ctx, &mm, &current).await?;
		if presave_scope_assigned_to_users(
			&mm,
			ctx.organization_id(),
			"access_sender_ids",
			identifiers,
		)
		.await?
		{
			return Err(presave_case_link_conflict(
				"sender presave is assigned to users",
			));
		}
	}
	SenderPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &entity).await?;
	if sender_presave_used_by_cases(&mm, ctx.organization_id(), id).await? {
		return Err(presave_case_link_conflict(
			"sender presave is used by cases",
		));
	}
	let identifiers = sender_scope_identifiers(&ctx, &mm, &entity).await?;
	if presave_scope_assigned_to_users(
		&mm,
		ctx.organization_id(),
		"access_sender_ids",
		identifiers,
	)
	.await?
	{
		return Err(presave_case_link_conflict(
			"sender presave is assigned to users",
		));
	}
	SenderPresaveBmc::update(
		&ctx,
		&mm,
		id,
		SenderPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct SenderPresaveDetails {
	pub parent: SenderPresave,
	pub gateways: Vec<SenderPresaveGateway>,
	pub responsible_persons: Vec<SenderPresaveResponsiblePerson>,
}

#[derive(Deserialize)]
pub struct SenderPresaveDetailsForUpdate {
	pub parent: Option<SenderPresaveForUpdate>,
	pub gateways: Option<Vec<SenderGatewayDetailsForUpdate>>,
	pub responsible_persons: Option<Vec<SenderResponsiblePersonDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct SenderGatewayDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub gateway_authority: Option<String>,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: Option<bool>,
	pub deleted: Option<bool>,
}

impl SenderGatewayDetailsForUpdate {
	fn into_update(self) -> SenderPresaveGatewayForUpdate {
		SenderPresaveGatewayForUpdate {
			sequence_number: self.sequence_number,
			gateway_authority: self.gateway_authority,
			sender_identifier: self.sender_identifier,
			routing_identifier: self.routing_identifier,
			cde_sender_identifier: self.cde_sender_identifier,
			cdr_sender_identifier: self.cdr_sender_identifier,
			is_default_for_authority: self.is_default_for_authority,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		sender_presave_id: Uuid,
	) -> Result<SenderPresaveGatewayForCreate> {
		Ok(SenderPresaveGatewayForCreate {
			sender_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"sender gateway details create requires sequence_number"
							.to_string(),
				}
			})?,
			gateway_authority: self.gateway_authority.ok_or_else(|| {
				Error::BadRequest {
					message:
						"sender gateway details create requires gateway_authority"
							.to_string(),
				}
			})?,
			sender_identifier: self.sender_identifier,
			routing_identifier: self.routing_identifier,
			cde_sender_identifier: self.cde_sender_identifier,
			cdr_sender_identifier: self.cdr_sender_identifier,
			is_default_for_authority: self.is_default_for_authority,
			deleted: self.deleted,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct SenderResponsiblePersonDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: Option<bool>,
	pub deleted: Option<bool>,
}

impl SenderResponsiblePersonDetailsForUpdate {
	fn into_update(self) -> SenderPresaveResponsiblePersonForUpdate {
		SenderPresaveResponsiblePersonForUpdate {
			sequence_number: self.sequence_number,
			department: self.department,
			person_title: self.person_title,
			person_given_name: self.person_given_name,
			person_middle_name: self.person_middle_name,
			person_family_name: self.person_family_name,
			is_default: self.is_default,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		sender_presave_id: Uuid,
	) -> Result<SenderPresaveResponsiblePersonForCreate> {
		Ok(SenderPresaveResponsiblePersonForCreate {
			sender_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
				message:
					"sender responsible person details create requires sequence_number"
						.to_string(),
			}
			})?,
			department: self.department,
			person_title: self.person_title,
			person_given_name: self.person_given_name,
			person_middle_name: self.person_middle_name,
			person_family_name: self.person_family_name,
			is_default: self.is_default,
			deleted: self.deleted,
		})
	}
}

pub async fn get_sender_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_sender_presave_details(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

pub async fn update_sender_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<SenderPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let current = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &current).await?;

	let ParamsForUpdate { data } = params;
	require_sender_detail_operation_permissions(&ctx, &mm, id, &data).await?;
	if data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true))
	{
		let identifiers = sender_scope_identifiers(&ctx, &mm, &current).await?;
		if presave_scope_assigned_to_users(
			&mm,
			ctx.organization_id(),
			"access_sender_ids",
			identifiers,
		)
		.await?
		{
			return Err(presave_case_link_conflict(
				"sender presave is assigned to users",
			));
		}
	}
	preflight_sender_presave_details(&ctx, &mm, id, &data).await?;

	apply_sender_presave_details(&ctx, &mm, id, data).await?;

	let details = load_sender_presave_details(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

async fn apply_sender_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: SenderPresaveDetailsForUpdate,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) =
		lib_core::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err.into());
	}

	let apply_result = apply_sender_presave_details_inner(ctx, mm, id, data).await;
	if let Err(err) = apply_result {
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}

	dbx.commit_txn().await.map_err(model::Error::from)?;
	Ok(())
}

async fn apply_sender_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: SenderPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.parent {
		SenderPresaveBmc::update(ctx, mm, id, parent).await?;
	}

	if let Some(gateways) = data.gateways {
		for gateway in gateways {
			upsert_sender_gateway_detail(ctx, mm, id, gateway).await?;
		}
	}

	if let Some(responsible_persons) = data.responsible_persons {
		for responsible_person in responsible_persons {
			upsert_sender_responsible_person_detail(ctx, mm, id, responsible_person)
				.await?;
		}
	}

	Ok(())
}

async fn load_sender_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<SenderPresaveDetails> {
	let parent = SenderPresaveBmc::get(ctx, mm, id).await?;
	let gateways = SenderPresaveGatewayBmc::list_by_parent(ctx, mm, id).await?;
	let responsible_persons =
		SenderPresaveResponsiblePersonBmc::list_by_parent(ctx, mm, id).await?;
	Ok(SenderPresaveDetails {
		parent,
		gateways,
		responsible_persons,
	})
}

async fn require_sender_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	data: &SenderPresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.gateways
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|gateway| gateway.id.is_none() && !gateway.delete)
		|| data
			.responsible_persons
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|responsible_person| {
				responsible_person.id.is_none() && !responsible_person.delete
			});
	let deletes_child =
		sender_detail_payload_deletes_child(ctx, mm, sender_id, data).await?;
	let deletes_parent = data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true));

	if creates_child {
		require_permission(ctx, PRESAVE_TEMPLATE_CREATE)?;
	}
	if deletes_child || deletes_parent {
		require_permission(ctx, PRESAVE_TEMPLATE_DELETE)?;
	}

	Ok(())
}

async fn sender_detail_payload_deletes_child(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	data: &SenderPresaveDetailsForUpdate,
) -> Result<bool> {
	for gateway in data.gateways.as_deref().unwrap_or_default() {
		if gateway.delete {
			return Ok(true);
		}
		if gateway.deleted == Some(true)
			&& sender_gateway_deleted_transition(ctx, mm, sender_id, gateway).await?
		{
			return Ok(true);
		}
	}

	for responsible_person in data.responsible_persons.as_deref().unwrap_or_default()
	{
		if responsible_person.delete {
			return Ok(true);
		}
		if responsible_person.deleted == Some(true)
			&& sender_responsible_person_deleted_transition(
				ctx,
				mm,
				sender_id,
				responsible_person,
			)
			.await?
		{
			return Ok(true);
		}
	}

	Ok(false)
}

async fn sender_gateway_deleted_transition(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	gateway: &SenderGatewayDetailsForUpdate,
) -> Result<bool> {
	let Some(id) = gateway.id else {
		return Ok(true);
	};
	let entity = SenderPresaveGatewayBmc::get(ctx, mm, id).await?;
	Ok(entity.sender_presave_id != sender_id || !entity.deleted)
}

async fn sender_responsible_person_deleted_transition(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	responsible_person: &SenderResponsiblePersonDetailsForUpdate,
) -> Result<bool> {
	let Some(id) = responsible_person.id else {
		return Ok(true);
	};
	let entity = SenderPresaveResponsiblePersonBmc::get(ctx, mm, id).await?;
	Ok(entity.sender_presave_id != sender_id || !entity.deleted)
}

async fn preflight_sender_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	data: &SenderPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(gateways) = &data.gateways {
		for gateway in gateways {
			preflight_sender_gateway_detail(ctx, mm, sender_id, gateway).await?;
		}
	}

	if let Some(responsible_persons) = &data.responsible_persons {
		for responsible_person in responsible_persons {
			preflight_sender_responsible_person_detail(
				ctx,
				mm,
				sender_id,
				responsible_person,
			)
			.await?;
		}
	}

	Ok(())
}

async fn preflight_sender_gateway_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	gateway: &SenderGatewayDetailsForUpdate,
) -> Result<()> {
	if gateway.delete && gateway.id.is_none() {
		return Err(Error::BadRequest {
			message: "sender gateway delete requires id".to_string(),
		});
	}

	if let Some(id) = gateway.id {
		let entity = SenderPresaveGatewayBmc::get(ctx, mm, id).await?;
		ensure_sender_detail_parent_scope(
			sender_id,
			entity.sender_presave_id,
			id,
			"sender_presave_gateways",
		)?;
	} else if !gateway.delete {
		validate_sender_gateway_detail_create(gateway)?;
	}

	Ok(())
}

fn validate_sender_gateway_detail_create(
	gateway: &SenderGatewayDetailsForUpdate,
) -> Result<()> {
	if gateway.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message: "sender gateway details create requires sequence_number"
				.to_string(),
		});
	}
	if gateway.gateway_authority.is_none() {
		return Err(Error::BadRequest {
			message: "sender gateway details create requires gateway_authority"
				.to_string(),
		});
	}

	Ok(())
}

async fn preflight_sender_responsible_person_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	responsible_person: &SenderResponsiblePersonDetailsForUpdate,
) -> Result<()> {
	if responsible_person.delete && responsible_person.id.is_none() {
		return Err(Error::BadRequest {
			message: "sender responsible person delete requires id".to_string(),
		});
	}

	if let Some(id) = responsible_person.id {
		let entity = SenderPresaveResponsiblePersonBmc::get(ctx, mm, id).await?;
		ensure_sender_detail_parent_scope(
			sender_id,
			entity.sender_presave_id,
			id,
			"sender_presave_responsible_persons",
		)?;
	} else if !responsible_person.delete {
		validate_sender_responsible_person_detail_create(responsible_person)?;
	}

	Ok(())
}

fn validate_sender_responsible_person_detail_create(
	responsible_person: &SenderResponsiblePersonDetailsForUpdate,
) -> Result<()> {
	if responsible_person.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"sender responsible person details create requires sequence_number"
					.to_string(),
		});
	}

	Ok(())
}

async fn upsert_sender_gateway_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	gateway: SenderGatewayDetailsForUpdate,
) -> Result<()> {
	if gateway.delete && gateway.id.is_none() {
		return Err(Error::BadRequest {
			message: "sender gateway delete requires id".to_string(),
		});
	}

	if let Some(id) = gateway.id {
		let entity = SenderPresaveGatewayBmc::get(ctx, mm, id).await?;
		ensure_sender_detail_parent_scope(
			sender_id,
			entity.sender_presave_id,
			id,
			"sender_presave_gateways",
		)?;
		if gateway.delete {
			SenderPresaveGatewayBmc::update(
				ctx,
				mm,
				id,
				SenderPresaveGatewayForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			SenderPresaveGatewayBmc::update(ctx, mm, id, gateway.into_update())
				.await?;
		}
	} else {
		SenderPresaveGatewayBmc::create(ctx, mm, gateway.into_create(sender_id)?)
			.await?;
	}

	Ok(())
}

async fn upsert_sender_responsible_person_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	responsible_person: SenderResponsiblePersonDetailsForUpdate,
) -> Result<()> {
	if responsible_person.delete && responsible_person.id.is_none() {
		return Err(Error::BadRequest {
			message: "sender responsible person delete requires id".to_string(),
		});
	}

	if let Some(id) = responsible_person.id {
		let entity = SenderPresaveResponsiblePersonBmc::get(ctx, mm, id).await?;
		ensure_sender_detail_parent_scope(
			sender_id,
			entity.sender_presave_id,
			id,
			"sender_presave_responsible_persons",
		)?;
		if responsible_person.delete {
			SenderPresaveResponsiblePersonBmc::update(
				ctx,
				mm,
				id,
				SenderPresaveResponsiblePersonForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			SenderPresaveResponsiblePersonBmc::update(
				ctx,
				mm,
				id,
				responsible_person.into_update(),
			)
			.await?;
		}
	} else {
		SenderPresaveResponsiblePersonBmc::create(
			ctx,
			mm,
			responsible_person.into_create(sender_id)?,
		)
		.await?;
	}

	Ok(())
}

fn ensure_sender_detail_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	ensure_parent_scope(path_parent_id, entity_parent_id, entity_id, entity).map_err(
		|_| Error::BadRequest {
			message: format!(
				"{entity} child does not belong to sender {path_parent_id}"
			),
		},
	)
}

#[derive(Debug, Deserialize)]
pub struct SenderGatewayForRestCreate {
	pub sequence_number: i32,
	pub gateway_authority: String,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: Option<bool>,
}

impl SenderGatewayForRestCreate {
	fn into_core(self, sender_presave_id: Uuid) -> SenderPresaveGatewayForCreate {
		SenderPresaveGatewayForCreate {
			sender_presave_id,
			sequence_number: self.sequence_number,
			gateway_authority: self.gateway_authority,
			sender_identifier: self.sender_identifier,
			routing_identifier: self.routing_identifier,
			cde_sender_identifier: self.cde_sender_identifier,
			cdr_sender_identifier: self.cdr_sender_identifier,
			is_default_for_authority: self.is_default_for_authority,
			deleted: None,
		}
	}
}

pub async fn create_sender_gateway_from_path(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(sender_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SenderGatewayForRestCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveGateway>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let ParamsForCreate { data } = params;
	let id = SenderPresaveGatewayBmc::create(&ctx, &mm, data.into_core(sender_id))
		.await?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	Ok(rest_created(entity))
}

pub async fn list_sender_gateways(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(sender_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderPresaveGateway>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let entities =
		SenderPresaveGatewayBmc::list_by_parent(&ctx, &mm, sender_id).await?;
	Ok(rest_ok(entities))
}

pub async fn get_sender_gateway(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveGateway>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_gateways",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	Ok(rest_ok(entity))
}

pub async fn update_sender_gateway(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderPresaveGatewayForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveGateway>>)> {
	let ctx = ctx_w.0;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	} else {
		require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	}
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_gateways",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	SenderPresaveGatewayBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_sender_gateway(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_gateways",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	SenderPresaveGatewayBmc::update(
		&ctx,
		&mm,
		id,
		SenderPresaveGatewayForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SenderResponsiblePersonForRestCreate {
	pub sequence_number: i32,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: Option<bool>,
}

impl SenderResponsiblePersonForRestCreate {
	fn into_core(
		self,
		sender_presave_id: Uuid,
	) -> SenderPresaveResponsiblePersonForCreate {
		SenderPresaveResponsiblePersonForCreate {
			sender_presave_id,
			sequence_number: self.sequence_number,
			department: self.department,
			person_title: self.person_title,
			person_given_name: self.person_given_name,
			person_middle_name: self.person_middle_name,
			person_family_name: self.person_family_name,
			is_default: self.is_default,
			deleted: None,
		}
	}
}

pub async fn create_sender_responsible_person(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(sender_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SenderResponsiblePersonForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SenderPresaveResponsiblePerson>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let ParamsForCreate { data } = params;
	let id = SenderPresaveResponsiblePersonBmc::create(
		&ctx,
		&mm,
		data.into_core(sender_id),
	)
	.await?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	Ok(rest_created(entity))
}

pub async fn list_sender_responsible_persons(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(sender_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<SenderPresaveResponsiblePerson>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let entities =
		SenderPresaveResponsiblePersonBmc::list_by_parent(&ctx, &mm, sender_id)
			.await?;
	Ok(rest_ok(entities))
}

pub async fn get_sender_responsible_person(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SenderPresaveResponsiblePerson>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_responsible_persons",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	Ok(rest_ok(entity))
}

pub async fn update_sender_responsible_person(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderPresaveResponsiblePersonForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<SenderPresaveResponsiblePerson>>,
)> {
	let ctx = ctx_w.0;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	} else {
		require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	}
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_responsible_persons",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	SenderPresaveResponsiblePersonBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_sender_responsible_person(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_responsible_persons",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	SenderPresaveResponsiblePersonBmc::update(
		&ctx,
		&mm,
		id,
		SenderPresaveResponsiblePersonForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}
