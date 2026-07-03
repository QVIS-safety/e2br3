use super::shared::*;

pub async fn create_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<ReporterPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReporterPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = ReporterPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_created(entity))
}

pub async fn list_reporter_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReporterPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ReporterPresaveBmc::list(&ctx, &mm, None).await?;
	Ok(rest_ok(entities))
}

pub async fn get_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReporterPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn update_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReporterPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReporterPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	ReporterPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	if reporter_presave_used_by_cases(&mm, ctx.organization_id(), id).await? {
		return Err(presave_case_link_conflict(
			"reporter presave is used by cases",
		));
	}
	ReporterPresaveBmc::update(
		&ctx,
		&mm,
		id,
		ReporterPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}
