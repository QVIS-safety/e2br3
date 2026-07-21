use super::shared::*;

#[derive(Debug, Deserialize)]
pub struct StudyRegistrationNumberForRestCreate {
	pub sequence_number: i32,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyRegistrationNumberForRestCreate {
	fn into_core(
		self,
		study_presave_id: Uuid,
	) -> StudyPresaveRegistrationNumberForCreate {
		StudyPresaveRegistrationNumberForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			registration_number: self.registration_number,
			country_code: self.country_code,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct StudyProductForRestCreate {
	pub sequence_number: i32,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyProductForRestCreate {
	fn into_core(self, study_presave_id: Uuid) -> StudyPresaveProductForCreate {
		StudyPresaveProductForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			product_presave_id: self.product_presave_id,
			product_name: self.product_name,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct StudyReporterForRestCreate {
	pub sequence_number: i32,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyReporterForRestCreate {
	fn into_core(self, study_presave_id: Uuid) -> StudyPresaveReporterForCreate {
		StudyPresaveReporterForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			reporter_presave_id: self.reporter_presave_id,
			reporter_organization: self.reporter_organization,
			reporter_given_name: self.reporter_given_name,
			reporter_qualification: self.reporter_qualification,
			deleted: self.deleted,
		}
	}
}

pub async fn create_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<StudyPresaveForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = StudyPresaveBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	Ok(rest_created(entity))
}

pub async fn list_study_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = StudyPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_study_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok(rest_ok(entities))
}

pub async fn get_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn update_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<StudyPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	if data.deleted == Some(true) {
		require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	}
	let current = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &current).await?;
	if data.deleted == Some(true) {
		PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Study, id).await?;
	} else {
		StudyPresaveBmc::update(&ctx, &mm, id, data).await?;
	}
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &entity).await?;
	Ok(rest_ok(entity))
}

pub async fn delete_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &entity).await?;
	PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Study, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct StudyPresaveDetails {
	pub parent: StudyPresave,
	pub products: Vec<StudyPresaveProduct>,
	pub study_registration_numbers: Vec<StudyPresaveRegistrationNumber>,
	pub fda_cross_reported_ind_numbers: Vec<StudyPresaveFdaCrossReportedIndNumber>,
	pub reporters: Vec<StudyPresaveReporter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyPresaveDetailsForUpdate {
	pub parent: Option<StudyPresaveForUpdate>,
	pub products: Option<Vec<StudyProductDetailsForUpdate>>,
	pub study_registration_numbers:
		Option<Vec<StudyRegistrationNumberDetailsForUpdate>>,
	pub fda_cross_reported_ind_numbers:
		Option<Vec<StudyFdaCrossReportedIndNumberDetailsForUpdate>>,
	pub reporters: Option<Vec<StudyReporterDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct StudyProductDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyProductDetailsForUpdate {
	fn into_update(self) -> StudyPresaveProductForUpdate {
		StudyPresaveProductForUpdate {
			sequence_number: self.sequence_number,
			product_presave_id: self.product_presave_id,
			product_name: self.product_name,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		study_presave_id: Uuid,
	) -> Result<StudyPresaveProductForCreate> {
		Ok(StudyPresaveProductForCreate {
			study_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message: "study product details create requires sequence_number"
						.to_string(),
				}
			})?,
			product_presave_id: self.product_presave_id,
			product_name: self.product_name,
			deleted: self.deleted,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyRegistrationNumberDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyRegistrationNumberDetailsForUpdate {
	fn into_update(self) -> StudyPresaveRegistrationNumberForUpdate {
		StudyPresaveRegistrationNumberForUpdate {
			sequence_number: self.sequence_number,
			registration_number: self.registration_number,
			country_code: self.country_code,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		study_presave_id: Uuid,
	) -> Result<StudyPresaveRegistrationNumberForCreate> {
		Ok(StudyPresaveRegistrationNumberForCreate {
			study_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"study registration number details create requires sequence_number"
							.to_string(),
				}
			})?,
			registration_number: self.registration_number,
			country_code: self.country_code,
			deleted: self.deleted,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyFdaCrossReportedIndNumberDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyFdaCrossReportedIndNumberDetailsForUpdate {
	fn into_update(self) -> StudyPresaveFdaCrossReportedIndNumberForUpdate {
		StudyPresaveFdaCrossReportedIndNumberForUpdate {
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		study_presave_id: Uuid,
	) -> Result<StudyPresaveFdaCrossReportedIndNumberForCreate> {
		Ok(StudyPresaveFdaCrossReportedIndNumberForCreate {
			study_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"FDA cross-reported IND create requires sequence_number"
							.to_string(),
				}
			})?,
			ind_number: self.ind_number.ok_or_else(|| Error::BadRequest {
				message: "FDA cross-reported IND create requires ind_number"
					.to_string(),
			})?,
			deleted: self.deleted,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct StudyReporterDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyReporterDetailsForUpdate {
	fn into_update(self) -> StudyPresaveReporterForUpdate {
		StudyPresaveReporterForUpdate {
			sequence_number: self.sequence_number,
			reporter_presave_id: self.reporter_presave_id,
			reporter_organization: self.reporter_organization,
			reporter_given_name: self.reporter_given_name,
			reporter_qualification: self.reporter_qualification,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		study_presave_id: Uuid,
	) -> Result<StudyPresaveReporterForCreate> {
		Ok(StudyPresaveReporterForCreate {
			study_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"study reporter details create requires sequence_number"
							.to_string(),
				}
			})?,
			reporter_presave_id: self.reporter_presave_id,
			reporter_organization: self.reporter_organization,
			reporter_given_name: self.reporter_given_name,
			reporter_qualification: self.reporter_qualification,
			deleted: self.deleted,
		})
	}
}

pub async fn get_study_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_study_presave_details(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

pub async fn update_study_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<StudyPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let current = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &current).await?;

	let ParamsForUpdate { data } = params;
	require_study_detail_operation_permissions(&ctx, &data)?;
	if data
		.parent
		.as_ref()
		.is_some_and(|parent| parent.deleted == Some(true))
	{
		if data.products.is_some()
			|| data.study_registration_numbers.is_some()
			|| data.fda_cross_reported_ind_numbers.is_some()
			|| data.reporters.is_some()
		{
			return Err(Error::BadRequest {
				message: "presave deletion cannot include child changes".into(),
			});
		}
		PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::Study, id).await?;
		return Ok(rest_ok(load_study_presave_details(&ctx, &mm, id).await?));
	}
	preflight_study_presave_details(&ctx, &mm, id, &data).await?;
	apply_study_presave_details(&ctx, &mm, id, data).await?;

	let details = load_study_presave_details(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok(rest_ok(details))
}

async fn apply_study_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: StudyPresaveDetailsForUpdate,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) =
		lib_core::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err.into());
	}

	let apply_result = apply_study_presave_details_inner(ctx, mm, id, data).await;
	if let Err(err) = apply_result {
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}

	dbx.commit_txn().await.map_err(model::Error::from)?;
	Ok(())
}

async fn apply_study_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: StudyPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.parent {
		StudyPresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(products) = data.products {
		for product in products {
			upsert_study_product_detail(ctx, mm, id, product).await?;
		}
	}
	if let Some(study_registration_numbers) = data.study_registration_numbers {
		for registration_number in study_registration_numbers {
			upsert_study_registration_number_detail(
				ctx,
				mm,
				id,
				registration_number,
			)
			.await?;
		}
	}
	if let Some(fda_cross_reported_ind_numbers) = data.fda_cross_reported_ind_numbers
	{
		for item in fda_cross_reported_ind_numbers {
			upsert_study_fda_cross_reported_ind_number_detail(ctx, mm, id, item)
				.await?;
		}
	}
	if let Some(reporters) = data.reporters {
		for reporter in reporters {
			upsert_study_reporter_detail(ctx, mm, id, reporter).await?;
		}
	}
	Ok(())
}

async fn load_study_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<StudyPresaveDetails> {
	let parent = StudyPresaveBmc::get(ctx, mm, id).await?;
	let products = StudyPresaveProductBmc::list_by_parent(ctx, mm, id).await?;
	let study_registration_numbers =
		StudyPresaveRegistrationNumberBmc::list_by_parent(ctx, mm, id).await?;
	let fda_cross_reported_ind_numbers =
		StudyPresaveFdaCrossReportedIndNumberBmc::list_by_parent(ctx, mm, id)
			.await?;
	let reporters = StudyPresaveReporterBmc::list_by_parent(ctx, mm, id).await?;
	Ok(StudyPresaveDetails {
		parent,
		products,
		study_registration_numbers,
		fda_cross_reported_ind_numbers,
		reporters,
	})
}

fn require_study_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	data: &StudyPresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.products
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.id.is_none() && !item.delete)
		|| data
			.study_registration_numbers
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete)
		|| data
			.fda_cross_reported_ind_numbers
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete)
		|| data
			.reporters
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete);
	let deletes_child = data
		.products
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.delete || item.deleted == Some(true))
		|| data
			.study_registration_numbers
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.delete || item.deleted == Some(true))
		|| data
			.fda_cross_reported_ind_numbers
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.delete || item.deleted == Some(true))
		|| data
			.reporters
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.delete || item.deleted == Some(true));
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

async fn preflight_study_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	data: &StudyPresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(products) = &data.products {
		for product in products {
			preflight_study_product_detail(ctx, mm, study_id, product).await?;
		}
	}
	if let Some(study_registration_numbers) = &data.study_registration_numbers {
		for registration_number in study_registration_numbers {
			preflight_study_registration_number_detail(
				ctx,
				mm,
				study_id,
				registration_number,
			)
			.await?;
		}
	}
	if let Some(items) = &data.fda_cross_reported_ind_numbers {
		for item in items {
			preflight_study_fda_cross_reported_ind_number_detail(
				ctx, mm, study_id, item,
			)
			.await?;
		}
	}
	if let Some(reporters) = &data.reporters {
		for reporter in reporters {
			preflight_study_reporter_detail(ctx, mm, study_id, reporter).await?;
		}
	}
	Ok(())
}

async fn preflight_study_product_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	product: &StudyProductDetailsForUpdate,
) -> Result<()> {
	if product.delete && product.id.is_none() {
		return Err(Error::BadRequest {
			message: "study product delete requires id".to_string(),
		});
	}
	if let Some(id) = product.id {
		let entity = StudyPresaveProductBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_products",
		)?;
	} else if !product.delete {
		validate_study_product_detail_create(product)?;
	}
	Ok(())
}

fn validate_study_product_detail_create(
	product: &StudyProductDetailsForUpdate,
) -> Result<()> {
	if product.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message: "study product details create requires sequence_number"
				.to_string(),
		});
	}
	Ok(())
}

async fn preflight_study_registration_number_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	registration_number: &StudyRegistrationNumberDetailsForUpdate,
) -> Result<()> {
	if registration_number.delete && registration_number.id.is_none() {
		return Err(Error::BadRequest {
			message: "study registration number delete requires id".to_string(),
		});
	}
	if let Some(id) = registration_number.id {
		let entity = StudyPresaveRegistrationNumberBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_registration_numbers",
		)?;
	} else if !registration_number.delete {
		validate_study_registration_number_detail_create(registration_number)?;
	}
	Ok(())
}

fn validate_study_registration_number_detail_create(
	registration_number: &StudyRegistrationNumberDetailsForUpdate,
) -> Result<()> {
	if registration_number.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"study registration number details create requires sequence_number"
					.to_string(),
		});
	}
	Ok(())
}

async fn preflight_study_fda_cross_reported_ind_number_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	item: &StudyFdaCrossReportedIndNumberDetailsForUpdate,
) -> Result<()> {
	if item.delete && item.id.is_none() {
		return Err(Error::BadRequest {
			message: "FDA cross-reported IND delete requires id".to_string(),
		});
	}
	if let Some(id) = item.id {
		let entity =
			StudyPresaveFdaCrossReportedIndNumberBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_fda_cross_reported_ind_numbers",
		)?;
	} else if !item.delete
		&& (item.sequence_number.is_none() || item.ind_number.is_none())
	{
		return Err(Error::BadRequest {
			message: "FDA cross-reported IND create requires sequence_number and ind_number".to_string(),
		});
	}
	Ok(())
}

async fn preflight_study_reporter_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	reporter: &StudyReporterDetailsForUpdate,
) -> Result<()> {
	if reporter.delete && reporter.id.is_none() {
		return Err(Error::BadRequest {
			message: "study reporter delete requires id".to_string(),
		});
	}
	if let Some(id) = reporter.id {
		let entity = StudyPresaveReporterBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_reporters",
		)?;
	} else if !reporter.delete {
		validate_study_reporter_detail_create(reporter)?;
	}
	Ok(())
}

fn validate_study_reporter_detail_create(
	reporter: &StudyReporterDetailsForUpdate,
) -> Result<()> {
	if reporter.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message: "study reporter details create requires sequence_number"
				.to_string(),
		});
	}
	Ok(())
}

async fn upsert_study_product_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	product: StudyProductDetailsForUpdate,
) -> Result<()> {
	if product.delete && product.id.is_none() {
		return Err(Error::BadRequest {
			message: "study product delete requires id".to_string(),
		});
	}
	if let Some(id) = product.id {
		let entity = StudyPresaveProductBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_products",
		)?;
		if product.delete {
			StudyPresaveProductBmc::update(
				ctx,
				mm,
				id,
				StudyPresaveProductForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			StudyPresaveProductBmc::update(ctx, mm, id, product.into_update())
				.await?;
		}
	} else {
		StudyPresaveProductBmc::create(ctx, mm, product.into_create(study_id)?)
			.await?;
	}
	Ok(())
}

async fn upsert_study_registration_number_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	registration_number: StudyRegistrationNumberDetailsForUpdate,
) -> Result<()> {
	if registration_number.delete && registration_number.id.is_none() {
		return Err(Error::BadRequest {
			message: "study registration number delete requires id".to_string(),
		});
	}
	if let Some(id) = registration_number.id {
		let entity = StudyPresaveRegistrationNumberBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_registration_numbers",
		)?;
		if registration_number.delete {
			StudyPresaveRegistrationNumberBmc::update(
				ctx,
				mm,
				id,
				StudyPresaveRegistrationNumberForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			StudyPresaveRegistrationNumberBmc::update(
				ctx,
				mm,
				id,
				registration_number.into_update(),
			)
			.await?;
		}
	} else {
		StudyPresaveRegistrationNumberBmc::create(
			ctx,
			mm,
			registration_number.into_create(study_id)?,
		)
		.await?;
	}
	Ok(())
}

async fn upsert_study_fda_cross_reported_ind_number_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	item: StudyFdaCrossReportedIndNumberDetailsForUpdate,
) -> Result<()> {
	if let Some(id) = item.id {
		let entity =
			StudyPresaveFdaCrossReportedIndNumberBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_fda_cross_reported_ind_numbers",
		)?;
		if item.delete {
			StudyPresaveFdaCrossReportedIndNumberBmc::update(
				ctx,
				mm,
				id,
				StudyPresaveFdaCrossReportedIndNumberForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			StudyPresaveFdaCrossReportedIndNumberBmc::update(
				ctx,
				mm,
				id,
				item.into_update(),
			)
			.await?;
		}
	} else if !item.delete {
		StudyPresaveFdaCrossReportedIndNumberBmc::create(
			ctx,
			mm,
			item.into_create(study_id)?,
		)
		.await?;
	}
	Ok(())
}

async fn upsert_study_reporter_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	reporter: StudyReporterDetailsForUpdate,
) -> Result<()> {
	if reporter.delete && reporter.id.is_none() {
		return Err(Error::BadRequest {
			message: "study reporter delete requires id".to_string(),
		});
	}
	if let Some(id) = reporter.id {
		let entity = StudyPresaveReporterBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_reporters",
		)?;
		if reporter.delete {
			StudyPresaveReporterBmc::update(
				ctx,
				mm,
				id,
				StudyPresaveReporterForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			StudyPresaveReporterBmc::update(ctx, mm, id, reporter.into_update())
				.await?;
		}
	} else {
		StudyPresaveReporterBmc::create(ctx, mm, reporter.into_create(study_id)?)
			.await?;
	}
	Ok(())
}

generate_presave_child_rest_fns! {
	Bmc: StudyPresaveRegistrationNumberBmc,
	Entity: StudyPresaveRegistrationNumber,
	RestCreate: StudyRegistrationNumberForRestCreate,
	ForUpdate: StudyPresaveRegistrationNumberForUpdate,
	CreateFn: create_study_registration_number,
	ListFn: list_study_registration_numbers,
	GetFn: get_study_registration_number,
	UpdateFn: update_study_registration_number,
	DeleteFn: delete_study_registration_number,
	ParentField: study_presave_id,
	ParentScopeFn: ensure_study_presave_id_scope,
	EntityName: "study_presave_registration_numbers",
	UpdatePermission: update,
	DeleteMode: soft
}

generate_presave_child_rest_fns! {
	Bmc: StudyPresaveProductBmc,
	Entity: StudyPresaveProduct,
	RestCreate: StudyProductForRestCreate,
	ForUpdate: StudyPresaveProductForUpdate,
	CreateFn: create_study_product,
	ListFn: list_study_products,
	GetFn: get_study_product,
	UpdateFn: update_study_product,
	DeleteFn: delete_study_product,
	ParentField: study_presave_id,
	ParentScopeFn: ensure_study_presave_id_scope,
	EntityName: "study_presave_products",
	UpdatePermission: update,
	DeleteMode: soft
}

generate_presave_child_rest_fns! {
	Bmc: StudyPresaveReporterBmc,
	Entity: StudyPresaveReporter,
	RestCreate: StudyReporterForRestCreate,
	ForUpdate: StudyPresaveReporterForUpdate,
	CreateFn: create_study_reporter,
	ListFn: list_study_reporters,
	GetFn: get_study_reporter,
	UpdateFn: update_study_reporter,
	DeleteFn: delete_study_reporter,
	ParentField: study_presave_id,
	ParentScopeFn: ensure_study_presave_id_scope,
	EntityName: "study_presave_reporters",
	UpdatePermission: update,
	DeleteMode: soft
}
