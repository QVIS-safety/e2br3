use super::shared::*;

pub async fn create_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<NarrativePresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = NarrativePresaveBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<NarrativePresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = NarrativePresaveBmc::list(&ctx, &mm, None).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	NarrativePresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	NarrativePresaveBmc::get(&ctx, &mm, id).await?;
	if narrative_presave_used_by_cases(&mm, ctx.organization_id(), id).await? {
		return Err(presave_case_link_conflict(
			"narrative presave is used by cases",
		));
	}
	NarrativePresaveBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}
