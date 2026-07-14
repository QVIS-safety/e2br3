use super::shared::*;

pub async fn create_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<ProductPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = ProductPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	if let Err(err) = ensure_product_presave_scope(&ctx, &mm, &entity).await {
		ProductPresaveBmc::delete(&ctx, &mm, id).await?;
		return Err(err);
	}
	Ok(rest_created(entity))
}

pub async fn list_product_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ProductPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ProductPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_product_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok(rest_ok(entities))
}

pub async fn get_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn update_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ProductPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	let current = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &current).await?;
	if data.deleted == Some(true)
		&& product_presave_used_by_cases(&mm, ctx.organization_id(), id).await?
	{
		return Err(presave_case_link_conflict(
			"product presave is used by cases",
		));
	}
	if data.deleted == Some(true)
		&& presave_scope_assigned_to_users(
			&mm,
			ctx.organization_id(),
			"access_product_ids",
			product_scope_identifiers(&current),
		)
		.await?
	{
		return Err(presave_case_link_conflict(
			"product presave is assigned to users",
		));
	}
	ProductPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &entity).await?;
	if product_presave_used_by_cases(&mm, ctx.organization_id(), id).await? {
		return Err(presave_case_link_conflict(
			"product presave is used by cases",
		));
	}
	if presave_scope_assigned_to_users(
		&mm,
		ctx.organization_id(),
		"access_product_ids",
		product_scope_identifiers(&entity),
	)
	.await?
	{
		return Err(presave_case_link_conflict(
			"product presave is assigned to users",
		));
	}
	ProductPresaveBmc::update(
		&ctx,
		&mm,
		id,
		ProductPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct ProductPresaveDetails {
	pub parent: ProductPresave,
	pub substances: Vec<ProductPresaveSubstance>,
}

#[derive(Deserialize)]
pub struct ProductPresaveDetailsForUpdate {
	pub parent: Option<ProductPresaveForUpdate>,
	pub substances: Option<Vec<ProductSubstanceDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct ProductSubstanceDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<rust_decimal::Decimal>,
	pub strength_unit: Option<String>,
}

impl ProductSubstanceDetailsForUpdate {
	fn into_update(self) -> ProductPresaveSubstanceForUpdate {
		ProductPresaveSubstanceForUpdate {
			sequence_number: self.sequence_number,
			substance_name: self.substance_name,
			substance_termid_version: self.substance_termid_version,
			substance_termid: self.substance_termid,
			mfds_version: self.mfds_version,
			mfds_id: self.mfds_id,
			strength_value: self.strength_value,
			strength_unit: self.strength_unit,
		}
	}

	fn into_create(
		self,
		product_presave_id: Uuid,
	) -> Result<ProductPresaveSubstanceForCreate> {
		Ok(ProductPresaveSubstanceForCreate {
			product_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"product substance details create requires sequence_number"
							.to_string(),
				}
			})?,
			substance_name: self.substance_name,
			substance_termid_version: self.substance_termid_version,
			substance_termid: self.substance_termid,
			mfds_version: self.mfds_version,
			mfds_id: self.mfds_id,
			strength_value: self.strength_value,
			strength_unit: self.strength_unit,
		})
	}
}

pub async fn get_product_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_product_presave_details(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

pub async fn update_product_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ProductPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let current = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &current).await?;

	let ParamsForUpdate { data } = params;
	require_product_detail_operation_permissions(&ctx, &data)?;
	if data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true))
		&& product_presave_used_by_cases(&mm, ctx.organization_id(), id).await?
	{
		return Err(presave_case_link_conflict(
			"product presave is used by cases",
		));
	}
	if data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true))
		&& presave_scope_assigned_to_users(
			&mm,
			ctx.organization_id(),
			"access_product_ids",
			product_scope_identifiers(&current),
		)
		.await?
	{
		return Err(presave_case_link_conflict(
			"product presave is assigned to users",
		));
	}
	preflight_product_presave_details(&ctx, &mm, id, &data).await?;
	apply_product_presave_details(&ctx, &mm, id, data).await?;

	let details = load_product_presave_details(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

async fn apply_product_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: ProductPresaveDetailsForUpdate,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) =
		lib_core::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err.into());
	}

	let apply_result = apply_product_presave_details_inner(ctx, mm, id, data).await;
	if let Err(err) = apply_result {
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}

	dbx.commit_txn().await.map_err(model::Error::from)?;
	Ok(())
}

async fn apply_product_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: ProductPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.parent {
		ProductPresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(substances) = data.substances {
		for substance in substances {
			upsert_product_substance_detail(ctx, mm, id, substance).await?;
		}
	}
	Ok(())
}

async fn load_product_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<ProductPresaveDetails> {
	let parent = ProductPresaveBmc::get(ctx, mm, id).await?;
	let substances = ProductPresaveSubstanceBmc::list_by_parent(ctx, mm, id).await?;
	Ok(ProductPresaveDetails { parent, substances })
}

fn require_product_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	data: &ProductPresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.substances
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.id.is_none() && !item.delete);
	let deletes_child = data
		.substances
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.delete);
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

async fn preflight_product_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	data: &ProductPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(substances) = &data.substances {
		for substance in substances {
			preflight_product_substance_detail(ctx, mm, product_id, substance)
				.await?;
		}
	}
	Ok(())
}

async fn preflight_product_substance_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	substance: &ProductSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.delete && substance.id.is_none() {
		return Err(Error::BadRequest {
			message: "product substance delete requires id".to_string(),
		});
	}
	if let Some(id) = substance.id {
		let entity = ProductPresaveSubstanceBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_substances",
		)?;
	} else if !substance.delete {
		validate_product_substance_detail_create(substance)?;
	}
	Ok(())
}

fn validate_product_substance_detail_create(
	substance: &ProductSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message: "product substance details create requires sequence_number"
				.to_string(),
		});
	}
	Ok(())
}

async fn upsert_product_substance_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	substance: ProductSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.delete && substance.id.is_none() {
		return Err(Error::BadRequest {
			message: "product substance delete requires id".to_string(),
		});
	}
	if let Some(id) = substance.id {
		let entity = ProductPresaveSubstanceBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_substances",
		)?;
		if substance.delete {
			ProductPresaveSubstanceBmc::delete(ctx, mm, id).await?;
		} else {
			ProductPresaveSubstanceBmc::update(ctx, mm, id, substance.into_update())
				.await?;
		}
	} else {
		ProductPresaveSubstanceBmc::create(
			ctx,
			mm,
			substance.into_create(product_id)?,
		)
		.await?;
	}
	Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ProductSubstanceForRestCreate {
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<rust_decimal::Decimal>,
	pub strength_unit: Option<String>,
}

impl ProductSubstanceForRestCreate {
	fn into_core(
		self,
		product_presave_id: Uuid,
	) -> ProductPresaveSubstanceForCreate {
		ProductPresaveSubstanceForCreate {
			product_presave_id,
			sequence_number: self.sequence_number,
			substance_name: self.substance_name,
			substance_termid_version: self.substance_termid_version,
			substance_termid: self.substance_termid,
			mfds_version: self.mfds_version,
			mfds_id: self.mfds_id,
			strength_value: self.strength_value,
			strength_unit: self.strength_unit,
		}
	}
}

generate_presave_child_rest_fns! {
	Bmc: ProductPresaveSubstanceBmc,
	Entity: ProductPresaveSubstance,
	RestCreate: ProductSubstanceForRestCreate,
	ForUpdate: ProductPresaveSubstanceForUpdate,
	CreateFn: create_product_substance,
	ListFn: list_product_substances,
	GetFn: get_product_substance,
	UpdateFn: update_product_substance,
	DeleteFn: delete_product_substance,
	ParentField: product_presave_id,
	ParentScopeFn: ensure_product_presave_id_scope,
	EntityName: "product_presave_substances",
	UpdatePermission: update,
	DeleteMode: hard
}
