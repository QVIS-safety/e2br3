use super::rows::{camelize_rows, camelize_value};
use super::shared::*;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct StudyRegistrationNumberForRestCreate {
	pub sequence_number: i32,
	pub registration_number: Option<String>,
	pub registration_number_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub country_code_null_flavor: Option<String>,
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
			registration_number_null_flavor: self.registration_number_null_flavor,
			country_code: self.country_code,
			country_code_null_flavor: self.country_code_null_flavor,
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
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<StudyPresaveRowsForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_create(
		&ctx,
		&snapshot,
		&mm,
		"study",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				super::input_contract::study_create(&data.rows.study)?;
				for row in &data.rows.products {
					validate_study_product_detail_create(row)?;
					if row.deleted == Some(true) {
						return Err(Error::BadRequest {
							message: "new study product cannot be deleted".into(),
						});
					}
				}
				for (index, row) in data.rows.registration_numbers.iter().enumerate()
				{
					super::input_contract::registration_detail(row, index)?;
					validate_study_registration_number_detail_create(row)?;
					if row.deleted == Some(true) {
						return Err(Error::BadRequest {
							message:
								"new study registration number cannot be deleted"
									.into(),
						});
					}
				}
				for (index, row) in
					data.rows.fda_cross_reported_inds.iter().enumerate()
				{
					super::input_contract::fda_ind_detail(row, index)?;
					if row.sequence_number.is_none()
						|| (row.ind_number.is_none()
							&& row.ind_number_null_flavor.is_none())
						|| row.deleted == Some(true)
					{
						return Err(Error::BadRequest {
							message: "invalid new FDA cross-reported IND".into(),
						});
					}
				}
				for row in &data.rows.reporters {
					validate_study_reporter_detail_create(row)?;
					if row.deleted == Some(true) {
						return Err(Error::BadRequest {
							message: "new study reporter cannot be deleted".into(),
						});
					}
				}
				let id = StudyPresaveBmc::create(ctx, mm, data.rows.study).await?;
				for row in data.rows.products {
					StudyPresaveProductBmc::create(ctx, mm, row.into_create(id)?)
						.await?;
				}
				for row in data.rows.registration_numbers {
					StudyPresaveRegistrationNumberBmc::create(
						ctx,
						mm,
						row.into_create(id)?,
					)
					.await?;
				}
				for row in data.rows.fda_cross_reported_inds {
					StudyPresaveFdaCrossReportedIndNumberBmc::create(
						ctx,
						mm,
						row.into_create(id)?,
					)
					.await?;
				}
				for row in data.rows.reporters {
					StudyPresaveReporterBmc::create(ctx, mm, row.into_create(id)?)
						.await?;
				}
				Ok(rest_created(load_study_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyPresaveRowsForCreate {
	pub rows: StudyPresaveCreateRows,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyPresaveCreateRows {
	pub study: StudyPresaveForCreate,
	#[serde(default)]
	pub products: Vec<StudyProductDetailsForUpdate>,
	#[serde(default)]
	pub reporters: Vec<StudyReporterDetailsForUpdate>,
	#[serde(default)]
	pub registration_numbers: Vec<StudyRegistrationNumberDetailsForUpdate>,
	#[serde(default)]
	pub fda_cross_reported_inds: Vec<StudyFdaCrossReportedIndNumberDetailsForUpdate>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StudyPresaveListQuery {
	pub product_ids: Option<String>,
}

pub async fn list_study_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Query(query): Query<StudyPresaveListQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyPresave>>>)> {
	let ctx = ctx_w.0;
	let product_ids =
		parse_scope_filter(query.product_ids.as_deref(), "productIds")?;
	let show_all = can_manage_all_presaves(&snapshot);
	with_authorized_presave_collection(&ctx, &snapshot, &mm, |ctx, mm, scope| {
		Box::pin(async move {
			let entities = StudyPresaveBmc::list(ctx, mm, None).await?;
			let products = ProductPresaveBmc::list(ctx, mm, None).await?;
			let entities = (if show_all {
				entities
			} else {
				filter_study_presaves_for_scope(scope, entities, &products)
			})
			.into_iter()
			.filter(|study| {
				product_ids.as_ref().is_none_or(|ids| {
					study.product_presave_id.is_some_and(|id| ids.contains(&id))
				})
			})
			.collect();
			Ok(rest_ok(entities))
		})
	})
	.await
}

pub async fn get_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Study,
		id,
		|ctx, mm| {
			Box::pin(
				async move { Ok(rest_ok(StudyPresaveBmc::get(ctx, mm, id).await?)) },
			)
		},
	)
	.await
}

pub async fn update_study_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<StudyPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Study,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				super::input_contract::study_update(&data)?;
				if data.deleted == Some(true) {
					PresaveLifecycleService::archive(
						ctx,
						mm,
						PresaveKind::Study,
						id,
					)
					.await?;
				} else {
					StudyPresaveBmc::update(ctx, mm, id, data).await?;
				}
				Ok(rest_ok(StudyPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn delete_study_presave(
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
		PresaveAuthorizationKind::Study,
		id,
		|ctx, mm| {
			Box::pin(async move {
				PresaveLifecycleService::archive(ctx, mm, PresaveKind::Study, id)
					.await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

#[derive(Debug, Serialize)]
pub struct StudyPresaveDetails {
	pub rows: StudyPresaveRows,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPresaveRows {
	pub study: Value,
	pub products: Vec<Value>,
	pub registration_numbers: Vec<Value>,
	pub fda_cross_reported_inds: Vec<Value>,
	pub reporters: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyPresaveDetailsForUpdate {
	pub rows: StudyPresaveRowsForUpdate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyPresaveRowsForUpdate {
	pub study: Option<StudyPresaveForUpdate>,
	pub products: Option<Vec<StudyProductDetailsForUpdate>>,
	pub registration_numbers: Option<Vec<StudyRegistrationNumberDetailsForUpdate>>,
	pub fda_cross_reported_inds:
		Option<Vec<StudyFdaCrossReportedIndNumberDetailsForUpdate>>,
	pub reporters: Option<Vec<StudyReporterDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyProductDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
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
			deleted: None,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyRegistrationNumberDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub registration_number: Option<String>,
	pub registration_number_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub country_code_null_flavor: Option<String>,
}

impl StudyRegistrationNumberDetailsForUpdate {
	fn into_update(self) -> StudyPresaveRegistrationNumberForUpdate {
		StudyPresaveRegistrationNumberForUpdate {
			sequence_number: self.sequence_number,
			registration_number: self.registration_number,
			registration_number_null_flavor: self.registration_number_null_flavor,
			country_code: self.country_code,
			country_code_null_flavor: self.country_code_null_flavor,
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
			registration_number_null_flavor: self.registration_number_null_flavor,
			country_code: self.country_code,
			country_code_null_flavor: self.country_code_null_flavor,
			deleted: None,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyFdaCrossReportedIndNumberDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
	pub ind_number_null_flavor: Option<String>,
}

impl StudyFdaCrossReportedIndNumberDetailsForUpdate {
	fn into_update(self) -> StudyPresaveFdaCrossReportedIndNumberForUpdate {
		StudyPresaveFdaCrossReportedIndNumberForUpdate {
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
			ind_number_null_flavor: self.ind_number_null_flavor,
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
			ind_number: self.ind_number,
			ind_number_null_flavor: self.ind_number_null_flavor,
			deleted: None,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyReporterDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
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
			deleted: None,
		})
	}
}

pub async fn get_study_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Study,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(load_study_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_study_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<StudyPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Study,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let rows = data.rows;
				if let Some(study) = &rows.study {
					super::input_contract::study_update(study)?;
				}
				if let Some(registrations) = &rows.registration_numbers {
					for (index, registration) in registrations.iter().enumerate() {
						if registration.deleted != Some(true) {
							super::input_contract::registration_detail(
								registration,
								index,
							)?;
						}
					}
				}
				if let Some(items) = &rows.fda_cross_reported_inds {
					for (index, item) in items.iter().enumerate() {
						if item.deleted != Some(true) {
							super::input_contract::fda_ind_detail(item, index)?;
						}
					}
				}
				if rows
					.study
					.as_ref()
					.is_some_and(|parent| parent.deleted == Some(true))
				{
					if rows.products.is_some()
						|| rows.registration_numbers.is_some()
						|| rows.fda_cross_reported_inds.is_some()
						|| rows.reporters.is_some()
					{
						return Err(Error::BadRequest {
							message: "presave deletion cannot include child changes"
								.into(),
						});
					}
					PresaveLifecycleService::archive_in_current_txn(
						ctx,
						mm,
						PresaveKind::Study,
						id,
					)
					.await?;
					return Ok(rest_ok(
						load_study_presave_details(ctx, mm, id).await?,
					));
				}
				preflight_study_presave_details(ctx, mm, id, &rows).await?;
				apply_study_presave_details_inner(ctx, mm, id, rows).await?;
				Ok(rest_ok(load_study_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

async fn apply_study_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: StudyPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.study {
		StudyPresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(products) = data.products {
		for product in products {
			upsert_study_product_detail(ctx, mm, id, product).await?;
		}
	}
	if let Some(study_registration_numbers) = data.registration_numbers {
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
	if let Some(fda_cross_reported_ind_numbers) = data.fda_cross_reported_inds {
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
	let study = camelize_value(
		serde_json::to_value(StudyPresaveBmc::get(ctx, mm, id).await?).map_err(
			|err| Error::BadRequest {
				message: format!("study presave serialization failed: {err}"),
			},
		)?,
	);
	let products = StudyPresaveProductBmc::list_by_parent(ctx, mm, id).await?;
	let study_registration_numbers =
		StudyPresaveRegistrationNumberBmc::list_by_parent(ctx, mm, id).await?;
	let fda_cross_reported_ind_numbers =
		StudyPresaveFdaCrossReportedIndNumberBmc::list_by_parent(ctx, mm, id)
			.await?;
	let reporters = StudyPresaveReporterBmc::list_by_parent(ctx, mm, id).await?;
	Ok(StudyPresaveDetails {
		rows: StudyPresaveRows {
			study,
			products: camelize_rows(products),
			registration_numbers: camelize_rows(study_registration_numbers),
			fda_cross_reported_inds: camelize_rows(fda_cross_reported_ind_numbers),
			reporters: camelize_rows(reporters),
		},
	})
}

async fn preflight_study_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	data: &StudyPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(products) = &data.products {
		for product in products {
			preflight_study_product_detail(ctx, mm, study_id, product).await?;
		}
	}
	if let Some(study_registration_numbers) = &data.registration_numbers {
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
	if let Some(items) = &data.fda_cross_reported_inds {
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
	if product.deleted == Some(true) && product.id.is_none() {
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
	} else if product.deleted != Some(true) {
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
	if registration_number.deleted == Some(true) && registration_number.id.is_none()
	{
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
	} else if registration_number.deleted != Some(true) {
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
	if item.deleted == Some(true) && item.id.is_none() {
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
	} else if item.deleted != Some(true)
		&& (item.sequence_number.is_none()
			|| (item.ind_number.is_none() && item.ind_number_null_flavor.is_none()))
	{
		return Err(Error::BadRequest {
			message: "FDA cross-reported IND create requires sequence_number and ind_number or nullFlavor".to_string(),
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
	if reporter.deleted == Some(true) && reporter.id.is_none() {
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
	} else if reporter.deleted != Some(true) {
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
	if product.deleted == Some(true) && product.id.is_none() {
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
		if product.deleted == Some(true) {
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
	if registration_number.deleted == Some(true) && registration_number.id.is_none()
	{
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
		if registration_number.deleted == Some(true) {
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
		if item.deleted == Some(true) {
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
	} else if item.deleted != Some(true) {
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
	if reporter.deleted == Some(true) && reporter.id.is_none() {
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
		if reporter.deleted == Some(true) {
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
	ParentKind: Study,
	EntityName: "study_presave_registration_numbers",
	DeleteMode: soft,
	ValidateCreate: super::input_contract::registration_create,
	ValidateUpdate: super::input_contract::registration_update
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
	ParentKind: Study,
	EntityName: "study_presave_products",
	DeleteMode: soft,
	ValidateCreate: super::shared::no_input_contract,
	ValidateUpdate: super::shared::no_input_contract
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
	ParentKind: Study,
	EntityName: "study_presave_reporters",
	DeleteMode: soft,
	ValidateCreate: super::shared::no_input_contract,
	ValidateUpdate: super::shared::no_input_contract
}
