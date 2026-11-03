use super::rows::camelize_value;
use super::shared::*;
use serde_json::{json, Value};

pub async fn create_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<ReceiverPresaveRowsForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_create(
		&ctx,
		&snapshot,
		&mm,
		"receiver",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				super::input_contract::receiver_create(&data.rows.receiver)?;
				for consignee in &data.rows.consignees {
					validate_receiver_consignee_detail_create(consignee)?;
					if consignee.deleted {
						return Err(Error::BadRequest {
							message: "new receiver consignee cannot be deleted"
								.into(),
						});
					}
				}
				for route in &data.rows.routes {
					validate_receiver_route_detail_create(route)?;
					if route.deleted {
						return Err(Error::BadRequest {
							message: "new receiver route cannot be deleted".into(),
						});
					}
				}
				let id =
					ReceiverPresaveBmc::create(ctx, mm, data.rows.receiver).await?;
				for consignee in data.rows.consignees {
					ReceiverPresaveConsigneeBmc::create(
						ctx,
						mm,
						consignee.into_create(id)?,
					)
					.await?;
				}
				for route in data.rows.routes {
					ReceiverPresaveRouteBmc::create(ctx, mm, route.into_create(id)?)
						.await?;
				}
				Ok(rest_created(
					load_receiver_presave_details(ctx, mm, id).await?,
				))
			})
		},
	)
	.await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverPresaveRowsForCreate {
	pub rows: ReceiverPresaveCreateRows,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverPresaveCreateRows {
	pub receiver: ReceiverPresaveForCreate,
	#[serde(default)]
	pub consignees: Vec<ReceiverConsigneeDetailsForUpdate>,
	#[serde(default)]
	pub routes: Vec<ReceiverRouteDetailsForUpdate>,
}

pub async fn list_receiver_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReceiverPresave>>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_collection(&ctx, &snapshot, &mm, |ctx, mm, _scope| {
		Box::pin(async move {
			Ok(rest_ok(ReceiverPresaveBmc::list(ctx, mm, None).await?))
		})
	})
	.await
}

pub async fn get_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Receiver,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(ReceiverPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_receiver_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReceiverPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Receiver,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				super::input_contract::receiver_update(&data)?;
				if data.deleted == Some(true) {
					PresaveLifecycleService::archive(
						ctx,
						mm,
						PresaveKind::Receiver,
						id,
					)
					.await?;
				} else {
					ReceiverPresaveBmc::update(ctx, mm, id, data).await?;
				}
				Ok(rest_ok(ReceiverPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn delete_receiver_presave(
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
		PresaveAuthorizationKind::Receiver,
		id,
		|ctx, mm| {
			Box::pin(async move {
				PresaveLifecycleService::archive(ctx, mm, PresaveKind::Receiver, id)
					.await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

#[derive(Debug, Serialize)]
pub struct ReceiverPresaveDetails {
	pub rows: ReceiverPresaveRows,
}

#[derive(Debug, Serialize)]
pub struct ReceiverPresaveRows {
	pub receiver: Value,
	pub consignees: Vec<Value>,
	pub routes: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverPresaveDetailsForUpdate {
	pub rows: ReceiverPresaveRowsForUpdate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverPresaveRowsForUpdate {
	pub receiver: Option<ReceiverPresaveForUpdate>,
	pub consignees: Option<Vec<ReceiverConsigneeDetailsForUpdate>>,
	pub routes: Option<Vec<ReceiverRouteDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverConsigneeDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default)]
	pub deleted: bool,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiverRouteDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default)]
	pub deleted: bool,
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
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Receiver,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(load_receiver_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_receiver_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReceiverPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Receiver,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let rows = data.rows;
				if rows
					.receiver
					.as_ref()
					.is_some_and(|parent| parent.deleted == Some(true))
				{
					if rows.consignees.is_some() || rows.routes.is_some() {
						return Err(Error::BadRequest {
							message: "presave deletion cannot include child changes"
								.into(),
						});
					}
					PresaveLifecycleService::archive_in_current_txn(
						ctx,
						mm,
						PresaveKind::Receiver,
						id,
					)
					.await?;
					return Ok(rest_ok(
						load_receiver_presave_details(ctx, mm, id).await?,
					));
				}
				preflight_receiver_presave_details(ctx, mm, id, &rows).await?;
				apply_receiver_presave_details_inner(ctx, mm, id, rows).await?;
				Ok(rest_ok(load_receiver_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

async fn apply_receiver_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: ReceiverPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.receiver {
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
	Ok(())
}

async fn load_receiver_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<ReceiverPresaveDetails> {
	let receiver = camelize_value(
		serde_json::to_value(ReceiverPresaveBmc::get(ctx, mm, id).await?).map_err(
			|err| Error::BadRequest {
				message: format!("receiver presave serialization failed: {err}"),
			},
		)?,
	);
	let consignees =
		ReceiverPresaveConsigneeBmc::list_by_parent(ctx, mm, id).await?;
	let routes = ReceiverPresaveRouteBmc::list_by_parent(ctx, mm, id).await?;
	let canonicalize_child = |row| {
		let mut value = camelize_value(row);
		if let Some(object) = value.as_object_mut() {
			object.insert("deleted".into(), json!(false));
		}
		value
	};
	Ok(ReceiverPresaveDetails {
		rows: ReceiverPresaveRows {
			receiver,
			consignees: consignees
				.into_iter()
				.map(|row| {
					canonicalize_child(
						serde_json::to_value(row)
							.expect("serializable receiver consignee"),
					)
				})
				.collect(),
			routes: routes
				.into_iter()
				.map(|row| {
					canonicalize_child(
						serde_json::to_value(row)
							.expect("serializable receiver route"),
					)
				})
				.collect(),
		},
	})
}

async fn preflight_receiver_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	data: &ReceiverPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(receiver) = &data.receiver {
		super::input_contract::receiver_update(receiver)?;
	}
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
	Ok(())
}

async fn preflight_receiver_consignee_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	receiver_id: Uuid,
	consignee: &ReceiverConsigneeDetailsForUpdate,
) -> Result<()> {
	if consignee.deleted && consignee.id.is_none() {
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
	} else if !consignee.deleted {
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
	if route.deleted && route.id.is_none() {
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
	} else if !route.deleted {
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
	if consignee.deleted && consignee.id.is_none() {
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
		if consignee.deleted {
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
	if route.deleted && route.id.is_none() {
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
		if route.deleted {
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
	ParentKind: Receiver,
	EntityName: "receiver_presave_consignees",
	DeleteMode: hard,
	ValidateCreate: super::shared::no_input_contract,
	ValidateUpdate: super::shared::no_input_contract
}
