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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_receiver_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReceiverPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ReceiverPresaveBmc::list(&ctx, &mm, None).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ReceiverPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	ReceiverPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = ReceiverPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	ReceiverPresaveBmc::update(
		&ctx,
		&mm,
		id,
		ReceiverPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct ReceiverPresaveDetails {
	pub parent: ReceiverPresave,
	pub consignees: Vec<ReceiverPresaveConsignee>,
}

#[derive(Deserialize)]
pub struct ReceiverPresaveDetailsForUpdate {
	pub parent: Option<ReceiverPresaveForUpdate>,
	pub consignees: Option<Vec<ReceiverConsigneeDetailsForUpdate>>,
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

pub async fn get_receiver_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_receiver_presave_details(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	preflight_receiver_presave_details(&ctx, &mm, id, &data).await?;
	apply_receiver_presave_details(&ctx, &mm, id, data).await?;

	let details = load_receiver_presave_details(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	Ok(ReceiverPresaveDetails { parent, consignees })
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
	let deletes_child = data
		.consignees
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|consignee| consignee.delete);
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

pub async fn create_receiver_consignee(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(receiver_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ReceiverConsigneeForRestCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveConsignee>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id =
		ReceiverPresaveConsigneeBmc::create(&ctx, &mm, data.into_core(receiver_id))
			.await?;
	let entity = ReceiverPresaveConsigneeBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_receiver_consignees(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(receiver_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<ReceiverPresaveConsignee>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		ReceiverPresaveConsigneeBmc::list_by_parent(&ctx, &mm, receiver_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_receiver_consignee(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((receiver_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveConsignee>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ReceiverPresaveConsigneeBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		receiver_id,
		entity.receiver_presave_id,
		id,
		"receiver_presave_consignees",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_receiver_consignee(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((receiver_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ReceiverPresaveConsigneeForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveConsignee>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = ReceiverPresaveConsigneeBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		receiver_id,
		entity.receiver_presave_id,
		id,
		"receiver_presave_consignees",
	)?;
	let ParamsForUpdate { data } = params;
	ReceiverPresaveConsigneeBmc::update(&ctx, &mm, id, data).await?;
	let entity = ReceiverPresaveConsigneeBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_receiver_consignee(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((receiver_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = ReceiverPresaveConsigneeBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		receiver_id,
		entity.receiver_presave_id,
		id,
		"receiver_presave_consignees",
	)?;
	ReceiverPresaveConsigneeBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
