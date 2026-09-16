use super::rows::camelize_value;
use super::shared::*;
use serde_json::Value;

pub async fn create_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<SenderPresaveRowsForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_create(
		&ctx,
		&snapshot,
		&mm,
		"sender",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				super::input_contract::sender_create(&data.rows.sender)?;
				for gateway in &data.rows.gateways {
					validate_sender_gateway_detail_create(gateway)?;
					if gateway.deleted == Some(true) {
						return Err(Error::BadRequest {
							message: "new sender gateway cannot be deleted".into(),
						});
					}
				}
				for (index, person) in
					data.rows.responsible_persons.iter().enumerate()
				{
					super::input_contract::sender_person_detail(person, index)?;
					validate_sender_responsible_person_detail_create(person)?;
					if person.deleted == Some(true) {
						return Err(Error::BadRequest {
							message:
								"new sender responsible person cannot be deleted"
									.into(),
						});
					}
				}
				let id = SenderPresaveBmc::create(ctx, mm, data.rows.sender).await?;
				for gateway in data.rows.gateways {
					SenderPresaveGatewayBmc::create(
						ctx,
						mm,
						gateway.into_create(id)?,
					)
					.await?;
				}
				for person in data.rows.responsible_persons {
					SenderPresaveResponsiblePersonBmc::create(
						ctx,
						mm,
						person.into_create(id)?,
					)
					.await?;
				}
				Ok(rest_created(
					load_sender_presave_details(ctx, mm, id).await?,
				))
			})
		},
	)
	.await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderPresaveRowsForCreate {
	pub rows: SenderPresaveCreateRows,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderPresaveCreateRows {
	pub sender: SenderPresaveForCreate,
	#[serde(default)]
	pub gateways: Vec<SenderGatewayDetailsForUpdate>,
	#[serde(default)]
	pub responsible_persons: Vec<SenderResponsiblePersonDetailsForUpdate>,
}

pub async fn list_sender_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderPresaveDetails>>>)> {
	let ctx = ctx_w.0;
	let show_all = can_manage_all_presaves(&snapshot);
	with_authorized_presave_collection(&ctx, &snapshot, &mm, |ctx, mm, scope| {
		Box::pin(async move {
			let entities = SenderPresaveBmc::list(ctx, mm, None).await?;
			let products = ProductPresaveBmc::list(ctx, mm, None).await?;
			let studies = StudyPresaveBmc::list(ctx, mm, None).await?;
			let visible = if show_all {
				entities
			} else {
				filter_sender_presaves_for_scope(
					scope, entities, &products, &studies,
				)
			};
			let mut details = Vec::with_capacity(visible.len());
			for sender in visible {
				details.push(load_sender_presave_details(ctx, mm, sender.id).await?);
			}
			Ok(rest_ok(details))
		})
	})
	.await
}

pub async fn get_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Sender,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(SenderPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_sender_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<SenderPresaveForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresave>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Sender,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				super::input_contract::sender_update(&data)?;
				if data.deleted == Some(true) {
					PresaveLifecycleService::archive(
						ctx,
						mm,
						PresaveKind::Sender,
						id,
					)
					.await?;
				} else {
					SenderPresaveBmc::update(ctx, mm, id, data).await?;
				}
				Ok(rest_ok(SenderPresaveBmc::get(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn delete_sender_presave(
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
		PresaveAuthorizationKind::Sender,
		id,
		|ctx, mm| {
			Box::pin(async move {
				PresaveLifecycleService::archive(ctx, mm, PresaveKind::Sender, id)
					.await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

#[derive(Debug, Serialize)]
pub struct SenderPresaveDetails {
	pub rows: SenderPresaveRows,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenderPresaveRows {
	pub sender: Value,
	pub gateways: Vec<Value>,
	pub responsible_persons: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderPresaveDetailsForUpdate {
	pub rows: SenderPresaveRowsForUpdate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderPresaveRowsForUpdate {
	pub sender: Option<SenderPresaveForUpdate>,
	pub gateways: Option<Vec<SenderGatewayDetailsForUpdate>>,
	pub responsible_persons: Option<Vec<SenderResponsiblePersonDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderGatewayDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub gateway_authority: Option<String>,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: Option<bool>,
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
			deleted: None,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SenderResponsiblePersonDetailsForUpdate {
	pub id: Option<Uuid>,
	pub deleted: Option<bool>,
	pub sequence_number: Option<i32>,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: Option<bool>,
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
			deleted: None,
		})
	}
}

pub async fn get_sender_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_read(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Sender,
		id,
		|ctx, mm| {
			Box::pin(async move {
				Ok(rest_ok(load_sender_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

pub async fn update_sender_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<SenderPresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveDetails>>)> {
	let ctx = ctx_w.0;
	with_authorized_presave_atomic_update(
		&ctx,
		&snapshot,
		&mm,
		PresaveAuthorizationKind::Sender,
		id,
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let rows = data.rows;
				if let Some(sender) = &rows.sender {
					super::input_contract::sender_update(sender)?;
				}
				if let Some(persons) = &rows.responsible_persons {
					for (index, person) in persons.iter().enumerate() {
						if person.deleted != Some(true) {
							super::input_contract::sender_person_detail(
								person, index,
							)?;
						}
					}
				}
				if rows
					.sender
					.as_ref()
					.is_some_and(|parent| parent.deleted == Some(true))
				{
					if rows.gateways.is_some() || rows.responsible_persons.is_some()
					{
						return Err(Error::BadRequest {
							message: "presave deletion cannot include child changes"
								.into(),
						});
					}
					PresaveLifecycleService::archive_in_current_txn(
						ctx,
						mm,
						PresaveKind::Sender,
						id,
					)
					.await?;
					return Ok(rest_ok(
						load_sender_presave_details(ctx, mm, id).await?,
					));
				}
				preflight_sender_presave_details(ctx, mm, id, &rows).await?;
				apply_sender_presave_details_inner(ctx, mm, id, rows).await?;
				Ok(rest_ok(load_sender_presave_details(ctx, mm, id).await?))
			})
		},
	)
	.await
}

async fn apply_sender_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: SenderPresaveRowsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.sender {
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
	let sender = camelize_value(
		serde_json::to_value(SenderPresaveBmc::get(ctx, mm, id).await?).map_err(
			|err| Error::BadRequest {
				message: format!("sender presave serialization failed: {err}"),
			},
		)?,
	);
	let gateways = SenderPresaveGatewayBmc::list_by_parent(ctx, mm, id).await?;
	let responsible_persons =
		SenderPresaveResponsiblePersonBmc::list_by_parent(ctx, mm, id).await?;
	Ok(SenderPresaveDetails {
		rows: SenderPresaveRows {
			sender,
			gateways: gateways
				.into_iter()
				.map(|row| {
					camelize_value(
						serde_json::to_value(row)
							.expect("serializable sender gateway"),
					)
				})
				.collect(),
			responsible_persons: responsible_persons
				.into_iter()
				.map(|row| {
					camelize_value(
						serde_json::to_value(row)
							.expect("serializable sender responsible person"),
					)
				})
				.collect(),
		},
	})
}

async fn preflight_sender_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
	data: &SenderPresaveRowsForUpdate,
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
	if gateway.deleted == Some(true) && gateway.id.is_none() {
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
	} else if gateway.deleted != Some(true) {
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
	if responsible_person.deleted == Some(true) && responsible_person.id.is_none() {
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
	} else if responsible_person.deleted != Some(true) {
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
	if gateway.deleted == Some(true) && gateway.id.is_none() {
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
		if gateway.deleted == Some(true) {
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
	if responsible_person.deleted == Some(true) && responsible_person.id.is_none() {
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
		if responsible_person.deleted == Some(true) {
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

generate_presave_child_rest_fns! {
	Bmc: SenderPresaveGatewayBmc,
	Entity: SenderPresaveGateway,
	RestCreate: SenderGatewayForRestCreate,
	ForUpdate: SenderPresaveGatewayForUpdate,
	CreateFn: create_sender_gateway_from_path,
	ListFn: list_sender_gateways,
	GetFn: get_sender_gateway,
	UpdateFn: update_sender_gateway,
	DeleteFn: delete_sender_gateway,
	ParentField: sender_presave_id,
	ParentKind: Sender,
	EntityName: "sender_presave_gateways",
	DeleteMode: soft,
	ValidateCreate: super::shared::no_input_contract,
	ValidateUpdate: super::shared::no_input_contract
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

generate_presave_child_rest_fns! {
	Bmc: SenderPresaveResponsiblePersonBmc,
	Entity: SenderPresaveResponsiblePerson,
	RestCreate: SenderResponsiblePersonForRestCreate,
	ForUpdate: SenderPresaveResponsiblePersonForUpdate,
	CreateFn: create_sender_responsible_person,
	ListFn: list_sender_responsible_persons,
	GetFn: get_sender_responsible_person,
	UpdateFn: update_sender_responsible_person,
	DeleteFn: delete_sender_responsible_person,
	ParentField: sender_presave_id,
	ParentKind: Sender,
	EntityName: "sender_presave_responsible_persons",
	DeleteMode: soft,
	ValidateCreate: super::input_contract::sender_person_create,
	ValidateUpdate: super::input_contract::sender_person_update
}
