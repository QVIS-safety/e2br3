use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST,
	PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
};
use lib_core::model::presave::{
	NarrativePresave, NarrativePresaveBmc, NarrativePresaveCaseSummary,
	NarrativePresaveCaseSummaryBmc, NarrativePresaveCaseSummaryForCreate,
	NarrativePresaveCaseSummaryForUpdate, NarrativePresaveForCreate,
	NarrativePresaveForUpdate, NarrativePresaveSenderDiagnosis,
	NarrativePresaveSenderDiagnosisBmc, NarrativePresaveSenderDiagnosisForCreate,
	NarrativePresaveSenderDiagnosisForUpdate, ProductPresave, ProductPresaveBmc,
	ProductPresaveFdaCrossReportedInd, ProductPresaveFdaCrossReportedIndBmc,
	ProductPresaveFdaCrossReportedIndForCreate,
	ProductPresaveFdaCrossReportedIndForUpdate, ProductPresaveForCreate,
	ProductPresaveForUpdate, ProductPresaveMfdsRegionalItem,
	ProductPresaveMfdsRegionalItemBmc, ProductPresaveMfdsRegionalItemForCreate,
	ProductPresaveMfdsRegionalItemForUpdate, ProductPresaveSubstance,
	ProductPresaveSubstanceBmc, ProductPresaveSubstanceForCreate,
	ProductPresaveSubstanceForUpdate, ReceiverPresave, ReceiverPresaveBmc,
	ReceiverPresaveConsignee, ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsigneeForCreate, ReceiverPresaveConsigneeForUpdate,
	ReceiverPresaveForCreate, ReceiverPresaveForUpdate, ReporterPresave,
	ReporterPresaveBmc, ReporterPresaveForCreate, ReporterPresaveForUpdate,
	SenderPresave, SenderPresaveBmc, SenderPresaveForCreate, SenderPresaveForUpdate,
	SenderPresaveGateway, SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate, SenderPresaveResponsiblePerson,
	SenderPresaveResponsiblePersonBmc, SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresave, StudyPresaveBmc,
	StudyPresaveFdaCrossReportedInd, StudyPresaveFdaCrossReportedIndBmc,
	StudyPresaveFdaCrossReportedIndForCreate,
	StudyPresaveFdaCrossReportedIndForUpdate, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveRegistrationNumber,
	StudyPresaveRegistrationNumberBmc, StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
};
use lib_core::model::user::UserBmc;
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum PresaveScopeSection {
	Sender,
	Product,
	Study,
}

fn normalized_set(values: Vec<String>) -> HashSet<String> {
	values
		.into_iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

fn push_scope_identifier(values: &mut Vec<String>, value: Option<&str>) {
	let Some(value) = value else {
		return;
	};
	let value = value.trim();
	if !value.is_empty() {
		values.push(value.to_ascii_lowercase());
	}
}

async fn allowed_scope_for_section(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	section: PresaveScopeSection,
) -> Result<Option<HashSet<String>>> {
	if lib_rest_core::is_admin(ctx, mm).await? {
		return Ok(None);
	}
	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let values = match section {
		PresaveScopeSection::Sender => {
			lib_rest_core::scope_values_from_raw(user.access_sender_ids.as_deref())
		}
		PresaveScopeSection::Product => {
			lib_rest_core::scope_values_from_raw(user.access_product_ids.as_deref())
		}
		PresaveScopeSection::Study => {
			lib_rest_core::scope_values_from_raw(user.access_study_ids.as_deref())
		}
	};
	Ok(Some(normalized_set(values)))
}

fn product_scope_identifiers(entity: &ProductPresave) -> Vec<String> {
	let mut values = Vec::new();
	push_scope_identifier(&mut values, entity.medicinal_product.as_deref());
	push_scope_identifier(&mut values, entity.brand_name.as_deref());
	push_scope_identifier(&mut values, entity.drug_generic_name.as_deref());
	push_scope_identifier(&mut values, entity.drug_authorization_number.as_deref());
	push_scope_identifier(&mut values, entity.mpid.as_deref());
	push_scope_identifier(&mut values, entity.phpid.as_deref());
	values
}

fn study_scope_identifiers(entity: &StudyPresave) -> Vec<String> {
	let mut values = Vec::new();
	push_scope_identifier(&mut values, entity.sponsor_study_number.as_deref());
	push_scope_identifier(&mut values, entity.study_name.as_deref());
	push_scope_identifier(&mut values, entity.mfds_study_number.as_deref());
	push_scope_identifier(&mut values, entity.mfds_protocol_number.as_deref());
	values
}

async fn sender_scope_identifiers(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entity: &SenderPresave,
) -> Result<Vec<String>> {
	let mut values = Vec::new();
	push_scope_identifier(&mut values, Some(&entity.name));
	push_scope_identifier(&mut values, entity.organization_name.as_deref());
	let gateways =
		SenderPresaveGatewayBmc::list_by_parent(ctx, mm, entity.id).await?;
	for gateway in gateways {
		push_scope_identifier(&mut values, gateway.sender_identifier.as_deref());
		push_scope_identifier(&mut values, gateway.routing_identifier.as_deref());
		push_scope_identifier(&mut values, gateway.cde_sender_identifier.as_deref());
		push_scope_identifier(&mut values, gateway.cdr_sender_identifier.as_deref());
	}
	Ok(values)
}

async fn identifiers_allowed_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	section: PresaveScopeSection,
	identifiers: Vec<String>,
) -> Result<bool> {
	let Some(allowed) = allowed_scope_for_section(ctx, mm, section).await? else {
		return Ok(true);
	};
	if allowed.is_empty() {
		return Ok(false);
	}
	Ok(identifiers
		.iter()
		.any(|identifier| allowed.contains(identifier)))
}

fn deny_presave_scope() -> Error {
	Error::PermissionDenied {
		required_permission: "PresaveTemplate.Scope".to_string(),
	}
}

async fn ensure_sender_presave_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entity: &SenderPresave,
) -> Result<()> {
	if identifiers_allowed_for_scope(
		ctx,
		mm,
		PresaveScopeSection::Sender,
		sender_scope_identifiers(ctx, mm, entity).await?,
	)
	.await?
	{
		return Ok(());
	}
	Err(deny_presave_scope())
}

async fn ensure_product_presave_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entity: &ProductPresave,
) -> Result<()> {
	if identifiers_allowed_for_scope(
		ctx,
		mm,
		PresaveScopeSection::Product,
		product_scope_identifiers(entity),
	)
	.await?
	{
		return Ok(());
	}
	Err(deny_presave_scope())
}

async fn ensure_study_presave_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entity: &StudyPresave,
) -> Result<()> {
	if identifiers_allowed_for_scope(
		ctx,
		mm,
		PresaveScopeSection::Study,
		study_scope_identifiers(entity),
	)
	.await?
	{
		return Ok(());
	}
	Err(deny_presave_scope())
}

async fn ensure_sender_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
) -> Result<()> {
	let parent = SenderPresaveBmc::get(ctx, mm, sender_id).await?;
	ensure_sender_presave_scope(ctx, mm, &parent).await
}

async fn ensure_product_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
) -> Result<()> {
	let parent = ProductPresaveBmc::get(ctx, mm, product_id).await?;
	ensure_product_presave_scope(ctx, mm, &parent).await
}

async fn ensure_study_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
) -> Result<()> {
	let parent = StudyPresaveBmc::get(ctx, mm, study_id).await?;
	ensure_study_presave_scope(ctx, mm, &parent).await
}

async fn filter_sender_presaves_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entities: Vec<SenderPresave>,
) -> Result<Vec<SenderPresave>> {
	let Some(allowed) =
		allowed_scope_for_section(ctx, mm, PresaveScopeSection::Sender).await?
	else {
		return Ok(entities);
	};
	let mut filtered = Vec::new();
	for entity in entities {
		let identifiers = sender_scope_identifiers(ctx, mm, &entity).await?;
		if !allowed.is_empty()
			&& identifiers
				.iter()
				.any(|identifier| allowed.contains(identifier))
		{
			filtered.push(entity);
		}
	}
	Ok(filtered)
}

async fn filter_product_presaves_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entities: Vec<ProductPresave>,
) -> Result<Vec<ProductPresave>> {
	let Some(allowed) =
		allowed_scope_for_section(ctx, mm, PresaveScopeSection::Product).await?
	else {
		return Ok(entities);
	};
	Ok(entities
		.into_iter()
		.filter(|entity| {
			!allowed.is_empty()
				&& product_scope_identifiers(entity)
					.iter()
					.any(|identifier| allowed.contains(identifier))
		})
		.collect())
}

async fn filter_study_presaves_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entities: Vec<StudyPresave>,
) -> Result<Vec<StudyPresave>> {
	let Some(allowed) =
		allowed_scope_for_section(ctx, mm, PresaveScopeSection::Study).await?
	else {
		return Ok(entities);
	};
	Ok(entities
		.into_iter()
		.filter(|entity| {
			!allowed.is_empty()
				&& study_scope_identifiers(entity)
					.iter()
					.any(|identifier| allowed.contains(identifier))
		})
		.collect())
}

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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_sender_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = SenderPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_sender_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	SenderPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &entity).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	require_sender_detail_operation_permissions(&ctx, &data)?;
	preflight_sender_presave_details(&ctx, &mm, id, &data).await?;

	apply_sender_presave_details(&ctx, &mm, id, data).await?;

	let details = load_sender_presave_details(&ctx, &mm, id).await?;
	ensure_sender_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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

fn require_sender_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
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
	let deletes_child = data
		.gateways
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|gateway| gateway.delete)
		|| data
			.responsible_persons
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|responsible_person| responsible_person.delete);
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
			SenderPresaveGatewayBmc::delete(ctx, mm, id).await?;
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
			SenderPresaveResponsiblePersonBmc::delete(ctx, mm, id).await?;
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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_sender_gateway(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((sender_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderPresaveGatewayForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderPresaveGateway>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_gateways",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let ParamsForUpdate { data } = params;
	SenderPresaveGatewayBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveGatewayBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	SenderPresaveGatewayBmc::delete(&ctx, &mm, id).await?;
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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		sender_id,
		entity.sender_presave_id,
		id,
		"sender_presave_responsible_persons",
	)?;
	ensure_sender_presave_id_scope(&ctx, &mm, sender_id).await?;
	let ParamsForUpdate { data } = params;
	SenderPresaveResponsiblePersonBmc::update(&ctx, &mm, id, data).await?;
	let entity = SenderPresaveResponsiblePersonBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	SenderPresaveResponsiblePersonBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_product_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ProductPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ProductPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_product_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	ProductPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = ProductPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &entity).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	pub fda_cross_reported_inds: Vec<ProductPresaveFdaCrossReportedInd>,
	pub mfds_regional_items: Vec<ProductPresaveMfdsRegionalItem>,
}

#[derive(Deserialize)]
pub struct ProductPresaveDetailsForUpdate {
	pub parent: Option<ProductPresaveForUpdate>,
	pub substances: Option<Vec<ProductSubstanceDetailsForUpdate>>,
	pub fda_cross_reported_inds:
		Option<Vec<ProductFdaCrossReportedIndDetailsForUpdate>>,
	pub mfds_regional_items: Option<Vec<ProductMfdsRegionalItemDetailsForUpdate>>,
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
			strength_value: self.strength_value,
			strength_unit: self.strength_unit,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct ProductFdaCrossReportedIndDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
}

impl ProductFdaCrossReportedIndDetailsForUpdate {
	fn into_update(self) -> ProductPresaveFdaCrossReportedIndForUpdate {
		ProductPresaveFdaCrossReportedIndForUpdate {
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
		}
	}

	fn into_create(
		self,
		product_presave_id: Uuid,
	) -> Result<ProductPresaveFdaCrossReportedIndForCreate> {
		Ok(ProductPresaveFdaCrossReportedIndForCreate {
			product_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"product FDA cross-reported IND details create requires sequence_number"
							.to_string(),
				}
			})?,
			ind_number: self.ind_number,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct ProductMfdsRegionalItemDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub item_type: Option<String>,
	pub item_value: Option<String>,
}

impl ProductMfdsRegionalItemDetailsForUpdate {
	fn into_update(self) -> ProductPresaveMfdsRegionalItemForUpdate {
		ProductPresaveMfdsRegionalItemForUpdate {
			sequence_number: self.sequence_number,
			item_type: self.item_type,
			item_value: self.item_value,
		}
	}

	fn into_create(
		self,
		product_presave_id: Uuid,
	) -> Result<ProductPresaveMfdsRegionalItemForCreate> {
		Ok(ProductPresaveMfdsRegionalItemForCreate {
			product_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"product MFDS regional item details create requires sequence_number"
							.to_string(),
				}
			})?,
			item_type: self.item_type,
			item_value: self.item_value,
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
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	preflight_product_presave_details(&ctx, &mm, id, &data).await?;
	apply_product_presave_details(&ctx, &mm, id, data).await?;

	let details = load_product_presave_details(&ctx, &mm, id).await?;
	ensure_product_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	if let Some(inds) = data.fda_cross_reported_inds {
		for ind in inds {
			upsert_product_fda_cross_reported_ind_detail(ctx, mm, id, ind).await?;
		}
	}
	if let Some(items) = data.mfds_regional_items {
		for item in items {
			upsert_product_mfds_regional_item_detail(ctx, mm, id, item).await?;
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
	let fda_cross_reported_inds =
		ProductPresaveFdaCrossReportedIndBmc::list_by_parent(ctx, mm, id).await?;
	let mfds_regional_items =
		ProductPresaveMfdsRegionalItemBmc::list_by_parent(ctx, mm, id).await?;
	Ok(ProductPresaveDetails {
		parent,
		substances,
		fda_cross_reported_inds,
		mfds_regional_items,
	})
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
		.any(|item| item.id.is_none() && !item.delete)
		|| data
			.fda_cross_reported_inds
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete)
		|| data
			.mfds_regional_items
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete);
	let deletes_child = data
		.substances
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.delete)
		|| data
			.fda_cross_reported_inds
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.delete)
		|| data
			.mfds_regional_items
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
	if let Some(inds) = &data.fda_cross_reported_inds {
		for ind in inds {
			preflight_product_fda_cross_reported_ind_detail(
				ctx, mm, product_id, ind,
			)
			.await?;
		}
	}
	if let Some(items) = &data.mfds_regional_items {
		for item in items {
			preflight_product_mfds_regional_item_detail(ctx, mm, product_id, item)
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

async fn preflight_product_fda_cross_reported_ind_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	ind: &ProductFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.delete && ind.id.is_none() {
		return Err(Error::BadRequest {
			message: "product FDA cross-reported IND delete requires id".to_string(),
		});
	}
	if let Some(id) = ind.id {
		let entity = ProductPresaveFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_fda_cross_reported_inds",
		)?;
		if !ind.delete {
			let _ = (ctx, mm, product_id);
		}
	} else if !ind.delete {
		validate_product_fda_cross_reported_ind_detail_create(ind)?;
	}
	Ok(())
}

fn validate_product_fda_cross_reported_ind_detail_create(
	ind: &ProductFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"product FDA cross-reported IND details create requires sequence_number"
					.to_string(),
		});
	}
	Ok(())
}

async fn preflight_product_mfds_regional_item_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	item: &ProductMfdsRegionalItemDetailsForUpdate,
) -> Result<()> {
	if item.delete && item.id.is_none() {
		return Err(Error::BadRequest {
			message: "product MFDS regional item delete requires id".to_string(),
		});
	}
	if let Some(id) = item.id {
		let entity = ProductPresaveMfdsRegionalItemBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_mfds_regional_items",
		)?;
		if !item.delete {
			let _ = (ctx, mm, product_id);
		}
	} else if !item.delete {
		validate_product_mfds_regional_item_detail_create(item)?;
	}
	Ok(())
}

fn validate_product_mfds_regional_item_detail_create(
	item: &ProductMfdsRegionalItemDetailsForUpdate,
) -> Result<()> {
	if item.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"product MFDS regional item details create requires sequence_number"
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

async fn upsert_product_fda_cross_reported_ind_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	ind: ProductFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.delete && ind.id.is_none() {
		return Err(Error::BadRequest {
			message: "product FDA cross-reported IND delete requires id".to_string(),
		});
	}
	if let Some(id) = ind.id {
		let entity = ProductPresaveFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_fda_cross_reported_inds",
		)?;
		if ind.delete {
			ProductPresaveFdaCrossReportedIndBmc::delete(ctx, mm, id).await?;
		} else {
			ProductPresaveFdaCrossReportedIndBmc::update(
				ctx,
				mm,
				id,
				ind.into_update(),
			)
			.await?;
		}
	} else {
		ProductPresaveFdaCrossReportedIndBmc::create(
			ctx,
			mm,
			ind.into_create(product_id)?,
		)
		.await?;
	}
	Ok(())
}

async fn upsert_product_mfds_regional_item_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
	item: ProductMfdsRegionalItemDetailsForUpdate,
) -> Result<()> {
	if item.delete && item.id.is_none() {
		return Err(Error::BadRequest {
			message: "product MFDS regional item delete requires id".to_string(),
		});
	}
	if let Some(id) = item.id {
		let entity = ProductPresaveMfdsRegionalItemBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			product_id,
			entity.product_presave_id,
			id,
			"product",
			"product_presave_mfds_regional_items",
		)?;
		if item.delete {
			ProductPresaveMfdsRegionalItemBmc::delete(ctx, mm, id).await?;
		} else {
			ProductPresaveMfdsRegionalItemBmc::update(
				ctx,
				mm,
				id,
				item.into_update(),
			)
			.await?;
		}
	} else {
		ProductPresaveMfdsRegionalItemBmc::create(
			ctx,
			mm,
			item.into_create(product_id)?,
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
			strength_value: self.strength_value,
			strength_unit: self.strength_unit,
		}
	}
}

pub async fn create_product_substance(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ProductSubstanceForRestCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveSubstance>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForCreate { data } = params;
	let id =
		ProductPresaveSubstanceBmc::create(&ctx, &mm, data.into_core(product_id))
			.await?;
	let entity = ProductPresaveSubstanceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_product_substances(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<ProductPresaveSubstance>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let entities =
		ProductPresaveSubstanceBmc::list_by_parent(&ctx, &mm, product_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_product_substance(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveSubstance>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ProductPresaveSubstanceBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_substances",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_product_substance(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ProductPresaveSubstanceForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ProductPresaveSubstance>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = ProductPresaveSubstanceBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_substances",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForUpdate { data } = params;
	ProductPresaveSubstanceBmc::update(&ctx, &mm, id, data).await?;
	let entity = ProductPresaveSubstanceBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_product_substance(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = ProductPresaveSubstanceBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_substances",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	ProductPresaveSubstanceBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ProductFdaCrossReportedIndForRestCreate {
	pub sequence_number: i32,
	pub ind_number: Option<String>,
}

impl ProductFdaCrossReportedIndForRestCreate {
	fn into_core(
		self,
		product_presave_id: Uuid,
	) -> ProductPresaveFdaCrossReportedIndForCreate {
		ProductPresaveFdaCrossReportedIndForCreate {
			product_presave_id,
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
		}
	}
}

pub async fn create_product_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ProductFdaCrossReportedIndForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForCreate { data } = params;
	let id = ProductPresaveFdaCrossReportedIndBmc::create(
		&ctx,
		&mm,
		data.into_core(product_id),
	)
	.await
	.map_err(map_product_fda_cross_reported_ind_model_error)?;
	let entity = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_product_fda_cross_reported_inds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<ProductPresaveFdaCrossReportedInd>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let entities =
		ProductPresaveFdaCrossReportedIndBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_product_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_fda_cross_reported_inds",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_product_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ProductPresaveFdaCrossReportedIndForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_fda_cross_reported_inds",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForUpdate { data } = params;
	ProductPresaveFdaCrossReportedIndBmc::update(&ctx, &mm, id, data)
		.await
		.map_err(map_product_fda_cross_reported_ind_model_error)?;
	let entity = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

fn map_product_fda_cross_reported_ind_model_error(err: model::Error) -> Error {
	match err {
		model::Error::Store(message)
			if message.contains(
				"product_presave_fda_cross_reported_inds field `product_presave_id`",
			) =>
		{
			Error::BadRequest { message }
		}
		err => err.into(),
	}
}

pub async fn delete_product_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = ProductPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_fda_cross_reported_inds",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	ProductPresaveFdaCrossReportedIndBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ProductMfdsRegionalItemForRestCreate {
	pub sequence_number: i32,
	pub item_type: Option<String>,
	pub item_value: Option<String>,
}

impl ProductMfdsRegionalItemForRestCreate {
	fn into_core(
		self,
		product_presave_id: Uuid,
	) -> ProductPresaveMfdsRegionalItemForCreate {
		ProductPresaveMfdsRegionalItemForCreate {
			product_presave_id,
			sequence_number: self.sequence_number,
			item_type: self.item_type,
			item_value: self.item_value,
		}
	}
}

pub async fn create_product_mfds_regional_item(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ProductMfdsRegionalItemForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveMfdsRegionalItem>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForCreate { data } = params;
	let id = ProductPresaveMfdsRegionalItemBmc::create(
		&ctx,
		&mm,
		data.into_core(product_id),
	)
	.await
	.map_err(map_product_mfds_regional_item_model_error)?;
	let entity = ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_product_mfds_regional_items(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(product_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<ProductPresaveMfdsRegionalItem>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let entities =
		ProductPresaveMfdsRegionalItemBmc::list_by_parent(&ctx, &mm, product_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_product_mfds_regional_item(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveMfdsRegionalItem>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_mfds_regional_items",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_product_mfds_regional_item(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<ProductPresaveMfdsRegionalItemForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<ProductPresaveMfdsRegionalItem>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_mfds_regional_items",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	let ParamsForUpdate { data } = params;
	ProductPresaveMfdsRegionalItemBmc::update(&ctx, &mm, id, data)
		.await
		.map_err(map_product_mfds_regional_item_model_error)?;
	let entity = ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

fn map_product_mfds_regional_item_model_error(err: model::Error) -> Error {
	match err {
		model::Error::Store(message)
			if message.contains(
				"product_presave_mfds_regional_items field `product_presave_id`",
			) =>
		{
			Error::BadRequest { message }
		}
		err => err.into(),
	}
}

pub async fn delete_product_mfds_regional_item(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((product_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = ProductPresaveMfdsRegionalItemBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		product_id,
		entity.product_presave_id,
		id,
		"product_presave_mfds_regional_items",
	)?;
	ensure_product_presave_id_scope(&ctx, &mm, product_id).await?;
	ProductPresaveMfdsRegionalItemBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

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
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_reporter_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<ReporterPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = ReporterPresaveBmc::list(&ctx, &mm, None).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<ReporterPresave>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = ReporterPresaveBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_reporter_presave(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
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
pub struct StudyFdaCrossReportedIndForRestCreate {
	pub sequence_number: i32,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyFdaCrossReportedIndForRestCreate {
	fn into_core(
		self,
		study_presave_id: Uuid,
	) -> StudyPresaveFdaCrossReportedIndForCreate {
		StudyPresaveFdaCrossReportedIndForCreate {
			study_presave_id,
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct NarrativeSenderDiagnosisForRestCreate {
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeSenderDiagnosisForRestCreate {
	fn into_core(
		self,
		narrative_presave_id: Uuid,
	) -> NarrativePresaveSenderDiagnosisForCreate {
		NarrativePresaveSenderDiagnosisForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number,
			diagnosis_meddra_version: self.diagnosis_meddra_version,
			diagnosis_meddra_code: self.diagnosis_meddra_code,
			deleted: self.deleted,
		}
	}
}

#[derive(Debug, Deserialize)]
pub struct NarrativeCaseSummaryForRestCreate {
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeCaseSummaryForRestCreate {
	fn into_core(
		self,
		narrative_presave_id: Uuid,
	) -> NarrativePresaveCaseSummaryForCreate {
		NarrativePresaveCaseSummaryForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number,
			summary_type: self.summary_type,
			language_code: self.language_code,
			summary_text: self.summary_text,
			deleted: self.deleted,
		}
	}
}

fn text_present(value: Option<&str>) -> bool {
	value.is_some_and(|value| !value.trim().is_empty())
}

fn ensure_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if path_parent_id != entity_parent_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

fn ensure_detail_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	parent_name: &'static str,
	entity: &'static str,
) -> Result<()> {
	ensure_parent_scope(path_parent_id, entity_parent_id, entity_id, entity).map_err(
		|_| Error::BadRequest {
			message: format!(
				"{entity} child does not belong to {parent_name} {path_parent_id}"
			),
		},
	)
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
	if let Err(err) = ensure_study_presave_scope(&ctx, &mm, &entity).await {
		StudyPresaveBmc::delete(&ctx, &mm, id).await?;
		return Err(err);
	}
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_presaves(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyPresave>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = StudyPresaveBmc::list(&ctx, &mm, None).await?;
	let entities = filter_study_presaves_for_scope(&ctx, &mm, entities).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
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
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	StudyPresaveBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveBmc::get(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &entity).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
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
	StudyPresaveBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct StudyPresaveDetails {
	pub parent: StudyPresave,
	#[serde(rename = "registrations")]
	pub registration_numbers: Vec<StudyPresaveRegistrationNumber>,
	pub fda_cross_reported_inds: Vec<StudyPresaveFdaCrossReportedInd>,
}

#[derive(Deserialize)]
pub struct StudyPresaveDetailsForUpdate {
	pub parent: Option<StudyPresaveForUpdate>,
	#[serde(rename = "registrations")]
	pub registration_numbers: Option<Vec<StudyRegistrationNumberDetailsForUpdate>>,
	pub fda_cross_reported_inds:
		Option<Vec<StudyFdaCrossReportedIndDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
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
pub struct StudyFdaCrossReportedIndDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyFdaCrossReportedIndDetailsForUpdate {
	fn into_update(self) -> StudyPresaveFdaCrossReportedIndForUpdate {
		StudyPresaveFdaCrossReportedIndForUpdate {
			sequence_number: self.sequence_number,
			ind_number: self.ind_number,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		study_presave_id: Uuid,
	) -> Result<StudyPresaveFdaCrossReportedIndForCreate> {
		Ok(StudyPresaveFdaCrossReportedIndForCreate {
			study_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"study FDA cross-reported IND details create requires sequence_number"
							.to_string(),
				}
			})?,
			ind_number: self.ind_number,
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
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	preflight_study_presave_details(&ctx, &mm, id, &data).await?;
	apply_study_presave_details(&ctx, &mm, id, data).await?;

	let details = load_study_presave_details(&ctx, &mm, id).await?;
	ensure_study_presave_scope(&ctx, &mm, &details.parent).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
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
	if let Some(registration_numbers) = data.registration_numbers {
		for registration_number in registration_numbers {
			upsert_study_registration_number_detail(
				ctx,
				mm,
				id,
				registration_number,
			)
			.await?;
		}
	}
	if let Some(inds) = data.fda_cross_reported_inds {
		for ind in inds {
			upsert_study_fda_cross_reported_ind_detail(ctx, mm, id, ind).await?;
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
	let registration_numbers =
		StudyPresaveRegistrationNumberBmc::list_by_parent(ctx, mm, id).await?;
	let fda_cross_reported_inds =
		StudyPresaveFdaCrossReportedIndBmc::list_by_parent(ctx, mm, id).await?;
	Ok(StudyPresaveDetails {
		parent,
		registration_numbers,
		fda_cross_reported_inds,
	})
}

fn require_study_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	data: &StudyPresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.registration_numbers
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.id.is_none() && !item.delete)
		|| data
			.fda_cross_reported_inds
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete);
	let deletes_child = data
		.registration_numbers
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.delete || item.deleted == Some(true))
		|| data
			.fda_cross_reported_inds
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
	if let Some(registration_numbers) = &data.registration_numbers {
		for registration_number in registration_numbers {
			preflight_study_registration_number_detail(
				ctx,
				mm,
				study_id,
				registration_number,
			)
			.await?;
		}
	}
	if let Some(inds) = &data.fda_cross_reported_inds {
		for ind in inds {
			preflight_study_fda_cross_reported_ind_detail(ctx, mm, study_id, ind)
				.await?;
		}
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

async fn preflight_study_fda_cross_reported_ind_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	ind: &StudyFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.delete && ind.id.is_none() {
		return Err(Error::BadRequest {
			message: "study FDA cross-reported IND delete requires id".to_string(),
		});
	}
	if let Some(id) = ind.id {
		let entity = StudyPresaveFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_fda_cross_reported_inds",
		)?;
	} else if !ind.delete {
		validate_study_fda_cross_reported_ind_detail_create(ind)?;
	}
	Ok(())
}

fn validate_study_fda_cross_reported_ind_detail_create(
	ind: &StudyFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"study FDA cross-reported IND details create requires sequence_number"
					.to_string(),
		});
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

async fn upsert_study_fda_cross_reported_ind_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
	ind: StudyFdaCrossReportedIndDetailsForUpdate,
) -> Result<()> {
	if ind.delete && ind.id.is_none() {
		return Err(Error::BadRequest {
			message: "study FDA cross-reported IND delete requires id".to_string(),
		});
	}
	if let Some(id) = ind.id {
		let entity = StudyPresaveFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			study_id,
			entity.study_presave_id,
			id,
			"study",
			"study_presave_fda_cross_reported_inds",
		)?;
		if ind.delete {
			StudyPresaveFdaCrossReportedIndBmc::update(
				ctx,
				mm,
				id,
				StudyPresaveFdaCrossReportedIndForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			StudyPresaveFdaCrossReportedIndBmc::update(
				ctx,
				mm,
				id,
				ind.into_update(),
			)
			.await?;
		}
	} else {
		StudyPresaveFdaCrossReportedIndBmc::create(
			ctx,
			mm,
			ind.into_create(study_id)?,
		)
		.await?;
	}
	Ok(())
}

pub async fn create_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyRegistrationNumberForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(study_id);
	let id = StudyPresaveRegistrationNumberBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_registration_numbers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyPresaveRegistrationNumber>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let entities =
		StudyPresaveRegistrationNumberBmc::list_by_parent(&ctx, &mm, study_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyPresaveRegistrationNumberForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveRegistrationNumber>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let ParamsForUpdate { data } = params;
	StudyPresaveRegistrationNumberBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = StudyPresaveRegistrationNumberBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_registration_numbers",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	StudyPresaveRegistrationNumberBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveRegistrationNumberForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyFdaCrossReportedIndForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(study_id);
	let id = StudyPresaveFdaCrossReportedIndBmc::create(&ctx, &mm, data).await?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_study_fda_cross_reported_inds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(study_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyPresaveFdaCrossReportedInd>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let entities =
		StudyPresaveFdaCrossReportedIndBmc::list_by_parent(&ctx, &mm, study_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyPresaveFdaCrossReportedIndForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<StudyPresaveFdaCrossReportedInd>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	let ParamsForUpdate { data } = params;
	StudyPresaveFdaCrossReportedIndBmc::update(&ctx, &mm, id, data).await?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((study_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = StudyPresaveFdaCrossReportedIndBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		study_id,
		entity.study_presave_id,
		id,
		"study_presave_fda_cross_reported_inds",
	)?;
	ensure_study_presave_id_scope(&ctx, &mm, study_id).await?;
	StudyPresaveFdaCrossReportedIndBmc::update(
		&ctx,
		&mm,
		id,
		StudyPresaveFdaCrossReportedIndForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

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

#[derive(Debug, Serialize)]
pub struct NarrativePresaveDetails {
	pub parent: NarrativePresave,
	pub sender_diagnoses: Vec<NarrativePresaveSenderDiagnosis>,
	pub case_summaries: Vec<NarrativePresaveCaseSummary>,
}

#[derive(Deserialize)]
pub struct NarrativePresaveDetailsForUpdate {
	pub parent: Option<NarrativePresaveForUpdate>,
	pub sender_diagnoses: Option<Vec<NarrativeSenderDiagnosisDetailsForUpdate>>,
	pub case_summaries: Option<Vec<NarrativeCaseSummaryDetailsForUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct NarrativeSenderDiagnosisDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeSenderDiagnosisDetailsForUpdate {
	fn into_update(self) -> NarrativePresaveSenderDiagnosisForUpdate {
		NarrativePresaveSenderDiagnosisForUpdate {
			sequence_number: self.sequence_number,
			diagnosis_meddra_version: self.diagnosis_meddra_version,
			diagnosis_meddra_code: self.diagnosis_meddra_code,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		narrative_presave_id: Uuid,
	) -> Result<NarrativePresaveSenderDiagnosisForCreate> {
		Ok(NarrativePresaveSenderDiagnosisForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"narrative sender diagnosis details create requires sequence_number"
							.to_string(),
				}
			})?,
			diagnosis_meddra_version: self.diagnosis_meddra_version,
			diagnosis_meddra_code: self.diagnosis_meddra_code,
			deleted: self.deleted,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct NarrativeCaseSummaryDetailsForUpdate {
	pub id: Option<Uuid>,
	#[serde(default, rename = "_delete")]
	pub delete: bool,
	pub sequence_number: Option<i32>,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: Option<bool>,
}

impl NarrativeCaseSummaryDetailsForUpdate {
	fn into_update(self) -> NarrativePresaveCaseSummaryForUpdate {
		NarrativePresaveCaseSummaryForUpdate {
			sequence_number: self.sequence_number,
			summary_type: self.summary_type,
			language_code: self.language_code,
			summary_text: self.summary_text,
			deleted: self.deleted,
		}
	}

	fn into_create(
		self,
		narrative_presave_id: Uuid,
	) -> Result<NarrativePresaveCaseSummaryForCreate> {
		Ok(NarrativePresaveCaseSummaryForCreate {
			narrative_presave_id,
			sequence_number: self.sequence_number.ok_or_else(|| {
				Error::BadRequest {
					message:
						"narrative case summary details create requires sequence_number"
							.to_string(),
				}
			})?,
			summary_type: self.summary_type,
			language_code: self.language_code,
			summary_text: self.summary_text,
			deleted: self.deleted,
		})
	}
}

pub async fn get_narrative_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let details = load_narrative_presave_details(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
}

pub async fn update_narrative_presave_details(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveDetailsForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativePresaveDetails>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	NarrativePresaveBmc::get(&ctx, &mm, id).await?;

	let ParamsForUpdate { data } = params;
	require_narrative_detail_operation_permissions(&ctx, &data)?;
	preflight_narrative_presave_details(&ctx, &mm, id, &data).await?;
	apply_narrative_presave_details(&ctx, &mm, id, data).await?;

	let details = load_narrative_presave_details(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: details })))
}

async fn apply_narrative_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: NarrativePresaveDetailsForUpdate,
) -> Result<()> {
	let dbx = mm.dbx();
	dbx.begin_txn().await.map_err(model::Error::from)?;
	if let Err(err) =
		lib_core::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
	{
		let _ = dbx.rollback_txn().await;
		return Err(err.into());
	}

	let apply_result =
		apply_narrative_presave_details_inner(ctx, mm, id, data).await;
	if let Err(err) = apply_result {
		let _ = dbx.rollback_txn().await;
		return Err(err);
	}

	dbx.commit_txn().await.map_err(model::Error::from)?;
	Ok(())
}

async fn apply_narrative_presave_details_inner(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
	data: NarrativePresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(parent) = data.parent {
		NarrativePresaveBmc::update(ctx, mm, id, parent).await?;
	}
	if let Some(sender_diagnoses) = data.sender_diagnoses {
		for sender_diagnosis in sender_diagnoses {
			upsert_narrative_sender_diagnosis_detail(ctx, mm, id, sender_diagnosis)
				.await?;
		}
	}
	if let Some(case_summaries) = data.case_summaries {
		for case_summary in case_summaries {
			upsert_narrative_case_summary_detail(ctx, mm, id, case_summary).await?;
		}
	}
	Ok(())
}

async fn load_narrative_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<NarrativePresaveDetails> {
	let parent = NarrativePresaveBmc::get(ctx, mm, id).await?;
	let sender_diagnoses =
		NarrativePresaveSenderDiagnosisBmc::list_by_parent(ctx, mm, id).await?;
	let case_summaries =
		NarrativePresaveCaseSummaryBmc::list_by_parent(ctx, mm, id).await?;
	Ok(NarrativePresaveDetails {
		parent,
		sender_diagnoses,
		case_summaries,
	})
}

fn require_narrative_detail_operation_permissions(
	ctx: &lib_core::ctx::Ctx,
	data: &NarrativePresaveDetailsForUpdate,
) -> Result<()> {
	let creates_child = data
		.sender_diagnoses
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.id.is_none() && !item.delete)
		|| data
			.case_summaries
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|item| item.id.is_none() && !item.delete);
	let deletes_child = data
		.sender_diagnoses
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|item| item.delete || item.deleted == Some(true))
		|| data
			.case_summaries
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

async fn preflight_narrative_presave_details(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	data: &NarrativePresaveDetailsForUpdate,
) -> Result<()> {
	if let Some(sender_diagnoses) = &data.sender_diagnoses {
		for sender_diagnosis in sender_diagnoses {
			preflight_narrative_sender_diagnosis_detail(
				ctx,
				mm,
				narrative_id,
				sender_diagnosis,
			)
			.await?;
		}
	}
	if let Some(case_summaries) = &data.case_summaries {
		for case_summary in case_summaries {
			preflight_narrative_case_summary_detail(
				ctx,
				mm,
				narrative_id,
				case_summary,
			)
			.await?;
		}
	}
	Ok(())
}

async fn preflight_narrative_sender_diagnosis_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	sender_diagnosis: &NarrativeSenderDiagnosisDetailsForUpdate,
) -> Result<()> {
	if sender_diagnosis.delete && sender_diagnosis.id.is_none() {
		return Err(Error::BadRequest {
			message: "narrative sender diagnosis delete requires id".to_string(),
		});
	}
	if let Some(id) = sender_diagnosis.id {
		let entity = NarrativePresaveSenderDiagnosisBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			narrative_id,
			entity.narrative_presave_id,
			id,
			"narrative",
			"narrative_presave_sender_diagnoses",
		)?;
	} else if !sender_diagnosis.delete {
		validate_narrative_sender_diagnosis_detail_create(sender_diagnosis)?;
	}
	Ok(())
}

fn validate_narrative_sender_diagnosis_detail_create(
	sender_diagnosis: &NarrativeSenderDiagnosisDetailsForUpdate,
) -> Result<()> {
	if sender_diagnosis.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"narrative sender diagnosis details create requires sequence_number"
					.to_string(),
		});
	}
	Ok(())
}

async fn preflight_narrative_case_summary_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	case_summary: &NarrativeCaseSummaryDetailsForUpdate,
) -> Result<()> {
	if case_summary.delete && case_summary.id.is_none() {
		return Err(Error::BadRequest {
			message: "narrative case summary delete requires id".to_string(),
		});
	}
	if let Some(id) = case_summary.id {
		let entity = NarrativePresaveCaseSummaryBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			narrative_id,
			entity.narrative_presave_id,
			id,
			"narrative",
			"narrative_presave_case_summaries",
		)?;
	} else if !case_summary.delete {
		validate_narrative_case_summary_detail_create(case_summary)?;
	}
	Ok(())
}

fn validate_narrative_case_summary_detail_create(
	case_summary: &NarrativeCaseSummaryDetailsForUpdate,
) -> Result<()> {
	if case_summary.sequence_number.is_none() {
		return Err(Error::BadRequest {
			message:
				"narrative case summary details create requires sequence_number"
					.to_string(),
		});
	}
	Ok(())
}

async fn upsert_narrative_sender_diagnosis_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	sender_diagnosis: NarrativeSenderDiagnosisDetailsForUpdate,
) -> Result<()> {
	if sender_diagnosis.delete && sender_diagnosis.id.is_none() {
		return Err(Error::BadRequest {
			message: "narrative sender diagnosis delete requires id".to_string(),
		});
	}
	if let Some(id) = sender_diagnosis.id {
		let entity = NarrativePresaveSenderDiagnosisBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			narrative_id,
			entity.narrative_presave_id,
			id,
			"narrative",
			"narrative_presave_sender_diagnoses",
		)?;
		if sender_diagnosis.delete {
			NarrativePresaveSenderDiagnosisBmc::update(
				ctx,
				mm,
				id,
				NarrativePresaveSenderDiagnosisForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			NarrativePresaveSenderDiagnosisBmc::update(
				ctx,
				mm,
				id,
				sender_diagnosis.into_update(),
			)
			.await?;
		}
	} else {
		NarrativePresaveSenderDiagnosisBmc::create(
			ctx,
			mm,
			sender_diagnosis.into_create(narrative_id)?,
		)
		.await?;
	}
	Ok(())
}

async fn upsert_narrative_case_summary_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	narrative_id: Uuid,
	case_summary: NarrativeCaseSummaryDetailsForUpdate,
) -> Result<()> {
	if case_summary.delete && case_summary.id.is_none() {
		return Err(Error::BadRequest {
			message: "narrative case summary delete requires id".to_string(),
		});
	}
	if let Some(id) = case_summary.id {
		let entity = NarrativePresaveCaseSummaryBmc::get(ctx, mm, id).await?;
		ensure_detail_parent_scope(
			narrative_id,
			entity.narrative_presave_id,
			id,
			"narrative",
			"narrative_presave_case_summaries",
		)?;
		if case_summary.delete {
			NarrativePresaveCaseSummaryBmc::update(
				ctx,
				mm,
				id,
				NarrativePresaveCaseSummaryForUpdate {
					deleted: Some(true),
					..Default::default()
				},
			)
			.await?;
		} else {
			NarrativePresaveCaseSummaryBmc::update(
				ctx,
				mm,
				id,
				case_summary.into_update(),
			)
			.await?;
		}
	} else {
		NarrativePresaveCaseSummaryBmc::create(
			ctx,
			mm,
			case_summary.into_create(narrative_id)?,
		)
		.await?;
	}
	Ok(())
}

pub async fn create_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<NarrativeSenderDiagnosisForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(narrative_id);
	let id = NarrativePresaveSenderDiagnosisBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_sender_diagnoses(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<NarrativePresaveSenderDiagnosis>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		NarrativePresaveSenderDiagnosisBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveSenderDiagnosisForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveSenderDiagnosis>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	let ParamsForUpdate { data } = params;
	NarrativePresaveSenderDiagnosisBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_sender_diagnosis(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = NarrativePresaveSenderDiagnosisBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_sender_diagnoses",
	)?;
	NarrativePresaveSenderDiagnosisBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveSenderDiagnosisForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}

pub async fn create_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<NarrativeCaseSummaryForRestCreate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let data = data.into_core(narrative_id);
	let id = NarrativePresaveCaseSummaryBmc::create(&ctx, &mm, data).await?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn list_narrative_case_summaries(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(narrative_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<NarrativePresaveCaseSummary>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities =
		NarrativePresaveCaseSummaryBmc::list_by_parent(&ctx, &mm, narrative_id)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn get_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<NarrativePresaveCaseSummaryForUpdate>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<NarrativePresaveCaseSummary>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	let ParamsForUpdate { data } = params;
	NarrativePresaveCaseSummaryBmc::update(&ctx, &mm, id, data).await?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_narrative_case_summary(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((narrative_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	let entity = NarrativePresaveCaseSummaryBmc::get(&ctx, &mm, id).await?;
	ensure_parent_scope(
		narrative_id,
		entity.narrative_presave_id,
		id,
		"narrative_presave_case_summaries",
	)?;
	NarrativePresaveCaseSummaryBmc::update(
		&ctx,
		&mm,
		id,
		NarrativePresaveCaseSummaryForUpdate {
			deleted: Some(true),
			..Default::default()
		},
	)
	.await?;
	Ok(StatusCode::NO_CONTENT)
}
