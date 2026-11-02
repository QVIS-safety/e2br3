use super::rows::camelize_value;
use super::shared::*;
use lib_core::model::presave::deserialize_patch_option;
use serde_json::{json, Value};

pub async fn create_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<ProductPresaveRowsForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_create(
		&ctx,
		&snapshot,
		&mm,
		"product",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				super::input_contract::product_create(&data.rows.product)?;
				for (index, substance) in
					data.rows.active_substances.iter().enumerate()
				{
					super::input_contract::substance_detail(substance, index)?;
					validate_product_active_substance_detail_create(substance)?;
					if substance.deleted {
						return Err(Error::BadRequest {
							message:
								"new product active substance cannot be deleted"
									.into(),
						});
					}
				}
				let id =
					ProductPresaveBmc::create(ctx, mm, data.rows.product).await?;
				for substance in data.rows.active_substances {
					ProductPresaveActiveSubstanceBmc::create(
						ctx,
						mm,
						substance.into_create(id)?,
					)
					.await?;
				}
				Ok(rest_created(
					load_product_presave_details(ctx, mm, id).await?,
				))
			})
		},
	)
	.await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductPresaveRowsForCreate {
	pub rows: ProductPresaveCreateRows,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductPresaveCreateRows {
	pub product: ProductPresaveForCreate,
	#[serde(default)]
	pub active_substances: Vec<ProductActiveSubstanceDetailsForUpdate>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProductPresaveListQuery {
	pub sender_ids: Option<String>,
}

pub async fn list_product_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(query): Query<ProductPresaveListQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ProductPresave>>>)> {
	let ctx = ctx_w.0;
	let sender_ids = parse_scope_filter(query.sender_ids.as_deref(), "senderIds")?;
	let show_all = can_manage_all_presaves(&snapshot);
	let action_id = if lib_core::authorization::policy_registry()
		.action("info.list")
		.is_some_and(|action| {
			lib_core::authorization::check_eligibility(&action.id, &snapshot)
				.is_eligible()
		}) {
		"info.list"
	} else {
		"import.product.list"
	};
	with_authorized_presave_collection_action(
		&ctx,
		&snapshot,
		&mm,
		action_id,
		|ctx, mm, scope| {
			Box::pin(async move {
				let entities = ProductPresaveBmc::list(ctx, mm, None).await?;
				let studies = StudyPresaveBmc::list(ctx, mm, None).await?;
				let entities = (if show_all {
					entities
				} else {
					filter_product_presaves_for_scope(scope, entities, &studies)
				})
				.into_iter()
				.filter(|product| {
					sender_ids.as_ref().is_none_or(|ids| {
						product
							.sender_presave_id
							.is_some_and(|id| ids.contains(&id))
					})
				})
				.collect();
				Ok(rest_ok(entities))
			})
		},
	)
	.await
}

pub async fn get_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Product,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(ProductPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ProductPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Product,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				super::input_contract::product_update(&data)?;
				if data.deleted == Some(true) {
					PresaveLifecycleService::archive(
						ctx,
						mm,
						PresaveKind::Product,
						id,
					)
					.await?;
				} else {
					ProductPresaveBmc::update(ctx, mm, id, data).await?;
				}
				Ok(rest_ok(ProductPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn delete_product_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	with_authorized_presave_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Product,
		id,
		|ctx, mm| {
			Box::pin(async move {
				PresaveLifecycleService::archive(ctx, mm, PresaveKind::Product, id)
					.await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

#[derive(Debug, Serialize)]
pub struct ProductPresaveDetails {
	pub rows: ProductPresaveRows,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPresaveRows {
	pub product: Value,
	pub active_substances: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductPresaveDetailsForUpdate {
	pub rows: ProductPresaveRowsForUpdate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductPresaveRowsForUpdate {
	pub product: Option<ProductPresaveForUpdate>,
	pub active_substances: Option<Vec<ProductActiveSubstanceDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductActiveSubstanceDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default)]
	pub deleted: bool,
	pub sequence_number: Option<i32>,
	pub substance_name: Option<String>,
	#[serde(rename = "substanceTermIdVersion")]
	pub substance_termid_version: Option<String>,
	#[serde(rename = "substanceTermId")]
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	#[serde(rename = "substanceStrengthValue")]
	#[serde(default, deserialize_with = "deserialize_patch_option")]
	pub strength_value: Option<Option<rust_decimal::Decimal>>,
	#[serde(rename = "substanceStrengthUnit")]
	pub strength_unit: Option<String>,
}

impl ProductActiveSubstanceDetailsForUpdate {
	fn into_update(self) -> ProductPresaveActiveSubstanceForUpdate {
		ProductPresaveActiveSubstanceForUpdate {
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
	) -> Result<ProductPresaveActiveSubstanceForCreate> {
		Ok(ProductPresaveActiveSubstanceForCreate {
			product_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"product active substance details create requires sequence_number"
							.to_string(),
				}
			})?,
			substance_name: self.substance_name,
			substance_termid_version: self.substance_termid_version,
			substance_termid: self.substance_termid,
			mfds_version: self.mfds_version,
			mfds_id: self.mfds_id,
			strength_value: self.strength_value.flatten(),
			strength_unit: self.strength_unit,
		})
	}
}

pub async fn get_product_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Product,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(load_product_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_product_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ProductPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Product,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let rows = data.rows;
				if let Some(product) = &rows.product {
					super::input_contract::product_update(product)?;
				}
				if let Some(substances) = &rows.active_substances {
					for (index, substance) in substances.iter().enumerate() {
						if !substance.deleted {
							super::input_contract::substance_detail(
								substance, index,
							)?;
						}
					}
				}
				if rows
					.product
					.as_ref()
					.is_some_and(|parent| parent.deleted == Some(true))
				{
					if rows.active_substances.is_some() {
						return Err(Error::BadRequest {
							message: "presave deletion cannot include child changes"
								.into(),
						});
					}
					PresaveLifecycleService::archive_in_current_txn(
						ctx,
						mm,
						PresaveKind::Product,
						id,
					)
					.await?;
					return Ok(rest_ok(
						load_product_presave_details(ctx, mm, id).await?,
					));
				}
				preflight_product_presave_details(ctx, mm, id, &rows).await?;
				apply_product_presave_details_inner(ctx, mm, id, rows).await?;
				Ok(rest_ok(load_product_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

async fn apply_product_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	rows: ProductPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(parent) = rows.product {
		ProductPresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(active_substances) = rows.active_substances {
		for substance in active_substances {
			upsert_product_active_substance_detail(ctx, mm, id, substance).await?;
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
	let active_substances =
		ProductPresaveActiveSubstanceBmc::list_by_parent(ctx, mm, id).await?;
	let product = camelize_value(serde_json::to_value(parent).map_err(|err| {
		Error::BadRequest {
			message: format!("product presave serialization failed: {err}"),
		}
	})?);
	let canonical_active_substances = active_substances
		.into_iter()
		.map(|substance| {
			let mut value = camelize_value(
				serde_json::to_value(substance)
					.expect("serializable product substance"),
			);
			if let Some(row) = value.as_object_mut() {
				if let Some(term_id_version) = row.remove("substanceTermidVersion") {
					row.insert("substanceTermIdVersion".into(), term_id_version);
				}
				if let Some(term_id) = row.remove("substanceTermid") {
					row.insert("substanceTermId".into(), term_id);
				}
				if let Some(strength) = row.remove("strengthValue") {
					row.insert("substanceStrengthValue".into(), strength);
				}
				if let Some(unit) = row.remove("strengthUnit") {
					row.insert("substanceStrengthUnit".into(), unit);
				}
				row.insert("deleted".into(), json!(false));
			}
			value
		})
		.collect();
	Ok(ProductPresaveDetails {
		rows: ProductPresaveRows {
			product,
			active_substances: canonical_active_substances,
		},
	})
}

async fn preflight_product_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	rows: &ProductPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(active_substances) = &rows.active_substances {
		for substance in active_substances {
			preflight_product_active_substance_detail(
				ctx, mm, product_id, substance,
			)
			.await?;
		}
	}
	Ok(())
}

async fn preflight_product_active_substance_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	substance: &ProductActiveSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.deleted && substance.id.is_none() {
		return Err(Error::BadRequest {
			message: "product active substance delete requires id".to_string(),
		});
	}
	if let Some(id) = substance.id {
		let entity = ProductPresaveActiveSubstanceBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_active_substances",
		)?;
	} else if !substance.deleted {
		validate_product_active_substance_detail_create(substance)?;
	}
	Ok(())
}

fn validate_product_active_substance_detail_create(
	substance: &ProductActiveSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"product active substance details create requires sequence_number"
					.to_string(),
		});
	}
	Ok(())
}

async fn upsert_product_active_substance_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	substance: ProductActiveSubstanceDetailsForUpdate,
) -> Result<()> {
	if substance.deleted && substance.id.is_none() {
		return Err(Error::BadRequest {
			message: "product active substance delete requires id".to_string(),
		});
	}
	if let Some(id) = substance.id {
		let entity = ProductPresaveActiveSubstanceBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_active_substances",
		)?;
		if substance.deleted {
			ProductPresaveActiveSubstanceBmc::delete(ctx, mm, id).await?;
		} else {
			ProductPresaveActiveSubstanceBmc::update(
				ctx,
				mm,
				id,
				substance.into_update(),
			)
			.await?;
		}
	} else {
		ProductPresaveActiveSubstanceBmc::create(
			ctx,
			mm,
			substance.into_create(product_id)?,
		)
		.await?;
	}
	Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductActiveSubstanceForRestCreate {
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<rust_decimal::Decimal>,
	pub strength_unit: Option<String>,
}

impl ProductActiveSubstanceForRestCreate {
	fn into_core(
		self,
		product_presave_id: Uuid,
	) -> ProductPresaveActiveSubstanceForCreate {
		ProductPresaveActiveSubstanceForCreate {
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
	Bmc: ProductPresaveActiveSubstanceBmc,
	Entity: ProductPresaveActiveSubstance,
	RestCreate: ProductActiveSubstanceForRestCreate,
	ForUpdate: ProductPresaveActiveSubstanceForUpdate,
	CreateFn: create_product_active_substance,
	ListFn: list_product_active_substances,
	GetFn: get_product_active_substance,
	UpdateFn: update_product_active_substance,
	DeleteFn: delete_product_active_substance,
	ParentField: product_presave_id,
	ParentKind: Product,
	EntityName: "product_presave_active_substances",
	DeleteMode: hard,
	ValidateCreate: super::input_contract::substance_create,
	ValidateUpdate: super::input_contract::substance_update
}
