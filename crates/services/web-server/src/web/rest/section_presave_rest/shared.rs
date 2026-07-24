//! Shared imports, scope guards, and parent-scope helpers
//! used across the presave section entity modules.

pub(super) use axum::extract::{Path, State};
pub(super) use axum::http::StatusCode;
pub(super) use axum::Json;
pub(super) use lib_core::model::acs::{
	PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST,
	PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
};
pub(super) use lib_core::model::presave::{
	NarrativePresave, NarrativePresaveBmc, NarrativePresaveForCreate,
	NarrativePresaveForUpdate, ProductPresave, ProductPresaveBmc,
	ProductPresaveForCreate, ProductPresaveForUpdate, ProductPresaveSubstance,
	ProductPresaveSubstanceBmc, ProductPresaveSubstanceForCreate,
	ProductPresaveSubstanceForUpdate, ReceiverPresave, ReceiverPresaveBmc,
	ReceiverPresaveConsignee, ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsigneeForCreate, ReceiverPresaveConsigneeForUpdate,
	ReceiverPresaveForCreate, ReceiverPresaveForUpdate, ReceiverPresaveRoute,
	ReceiverPresaveRouteBmc, ReceiverPresaveRouteForCreate,
	ReceiverPresaveRouteForUpdate, ReporterPresave, ReporterPresaveBmc,
	ReporterPresaveForCreate, ReporterPresaveForUpdate, SenderPresave,
	SenderPresaveBmc, SenderPresaveForCreate, SenderPresaveForUpdate,
	SenderPresaveGateway, SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate, SenderPresaveResponsiblePerson,
	SenderPresaveResponsiblePersonBmc, SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresave, StudyPresaveBmc,
	StudyPresaveFdaCrossReportedInd, StudyPresaveFdaCrossReportedIndBmc,
	StudyPresaveFdaCrossReportedIndForCreate,
	StudyPresaveFdaCrossReportedIndForUpdate, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveProduct, StudyPresaveProductBmc,
	StudyPresaveProductForCreate, StudyPresaveProductForUpdate,
	StudyPresaveRegistrationNumber, StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate, StudyPresaveReporter,
	StudyPresaveReporterBmc, StudyPresaveReporterForCreate,
	StudyPresaveReporterForUpdate,
};
pub(super) use lib_core::model::presave_lifecycle::{
	PresaveKind, PresaveLifecycleService,
};
pub(super) use lib_core::model::user::UserBmc;
pub(super) use lib_core::model::{self, ModelManager};
pub(super) use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
pub(super) use lib_rest_core::rest_result::DataRestResult;
pub(super) use lib_rest_core::{require_permission, Error, Result};
pub(super) use lib_web::middleware::mw_auth::CtxW;
pub(super) use serde::{Deserialize, Serialize};
pub(super) use std::collections::HashSet;
pub(super) use uuid::Uuid;

macro_rules! generate_simple_presave_rest_fns {
	(
		Bmc: $bmc:ident,
		Entity: $entity:ident,
		ForCreate: $for_create:ident,
		ForUpdate: $for_update:ident,
		CreateFn: $create_fn:ident,
		ListFn: $list_fn:ident,
		GetFn: $get_fn:ident,
		UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident,
		Kind: $kind:ident
	) => {
		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Json(params): Json<ParamsForCreate<$for_create>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
			let ParamsForCreate { data } = params;
			let id = $bmc::create(&ctx, &mm, data).await?;
			Ok(rest_created($bmc::get(&ctx, &mm, id).await?))
		}

		pub async fn $list_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
			Ok(rest_ok($bmc::list(&ctx, &mm, None).await?))
		}

		pub async fn $get_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(id): Path<Uuid>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
			Ok(rest_ok($bmc::get(&ctx, &mm, id).await?))
		}

		pub async fn $update_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(id): Path<Uuid>,
			Json(params): Json<ParamsForUpdate<$for_update>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
			let ParamsForUpdate { data } = params;
			if data.deleted == Some(true) {
				require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
			}
			if data.deleted == Some(true) {
				PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::$kind, id)
					.await?;
			} else {
				$bmc::update(&ctx, &mm, id, data).await?;
			}
			Ok(rest_ok($bmc::get(&ctx, &mm, id).await?))
		}

		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(id): Path<Uuid>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
			PresaveLifecycleService::archive(&ctx, &mm, PresaveKind::$kind, id)
				.await?;
			Ok(StatusCode::NO_CONTENT)
		}
	};
}

pub(super) use generate_simple_presave_rest_fns;

macro_rules! delete_presave_child {
	(hard, $bmc:ident, $for_update:ident, $ctx:ident, $mm:ident, $id:ident) => {
		$bmc::delete(&$ctx, &$mm, $id).await?;
	};
	(soft, $bmc:ident, $for_update:ident, $ctx:ident, $mm:ident, $id:ident) => {
		$bmc::update(
			&$ctx,
			&$mm,
			$id,
			$for_update {
				deleted: Some(true),
				..Default::default()
			},
		)
		.await?;
	};
}

pub(super) use delete_presave_child;

macro_rules! require_presave_child_update_permission {
	(update, $ctx:ident, $data:ident) => {
		require_permission(&$ctx, PRESAVE_TEMPLATE_UPDATE)?;
	};
	(delete_aware, $ctx:ident, $data:ident) => {
		if $data.deleted == Some(true) {
			require_permission(&$ctx, PRESAVE_TEMPLATE_DELETE)?;
		} else {
			require_permission(&$ctx, PRESAVE_TEMPLATE_UPDATE)?;
		}
	};
}

pub(super) use require_presave_child_update_permission;

macro_rules! generate_presave_child_rest_fns {
	(
		Bmc: $bmc:ident,
		Entity: $entity:ident,
		RestCreate: $rest_create:ident,
		ForUpdate: $for_update:ident,
		CreateFn: $create_fn:ident,
		ListFn: $list_fn:ident,
		GetFn: $get_fn:ident,
		UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident,
		ParentField: $parent_field:ident,
		ParentScopeFn: $parent_scope_fn:ident,
		EntityName: $entity_name:literal,
		UpdatePermission: $update_permission:ident,
		DeleteMode: $delete_mode:ident
	) => {
		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(parent_id): Path<Uuid>,
			Json(params): Json<ParamsForCreate<$rest_create>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
			$parent_scope_fn(&ctx, &mm, parent_id).await?;
			let ParamsForCreate { data } = params;
			let id = $bmc::create(&ctx, &mm, data.into_core(parent_id)).await?;
			Ok(rest_created($bmc::get(&ctx, &mm, id).await?))
		}

		pub async fn $list_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path(parent_id): Path<Uuid>,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
			$parent_scope_fn(&ctx, &mm, parent_id).await?;
			Ok(rest_ok($bmc::list_by_parent(&ctx, &mm, parent_id).await?))
		}

		pub async fn $get_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			ensure_parent_scope(parent_id, entity.$parent_field, id, $entity_name)?;
			$parent_scope_fn(&ctx, &mm, parent_id).await?;
			Ok(rest_ok(entity))
		}

		pub async fn $update_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
			Json(params): Json<ParamsForUpdate<$for_update>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			let ParamsForUpdate { data } = params;
			require_presave_child_update_permission!($update_permission, ctx, data);
			let entity = $bmc::get(&ctx, &mm, id).await?;
			ensure_parent_scope(parent_id, entity.$parent_field, id, $entity_name)?;
			$parent_scope_fn(&ctx, &mm, parent_id).await?;
			$bmc::update(&ctx, &mm, id, data).await?;
			Ok(rest_ok($bmc::get(&ctx, &mm, id).await?))
		}

		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			ensure_parent_scope(parent_id, entity.$parent_field, id, $entity_name)?;
			$parent_scope_fn(&ctx, &mm, parent_id).await?;
			delete_presave_child!($delete_mode, $bmc, $for_update, ctx, mm, id);
			Ok(StatusCode::NO_CONTENT)
		}
	};
}

pub(super) use generate_presave_child_rest_fns;

pub(super) async fn allow_presave_parent_scope(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	_parent_id: Uuid,
) -> Result<()> {
	Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum PresaveScopeSection {
	Sender,
	Product,
	Study,
}

pub(super) fn normalized_set(values: Vec<String>) -> HashSet<String> {
	values
		.into_iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

pub(super) async fn allowed_scope_for_section(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	section: PresaveScopeSection,
) -> Result<Option<HashSet<String>>> {
	if ctx.is_system_admin() || ctx.is_sponsor_admin() {
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

pub(super) fn product_scope_identifiers(entity: &ProductPresave) -> Vec<String> {
	vec![entity.id.to_string()]
}

pub(super) fn study_scope_identifiers(entity: &StudyPresave) -> Vec<String> {
	vec![entity.id.to_string()]
}

pub(super) async fn sender_scope_identifiers(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	entity: &SenderPresave,
) -> Result<Vec<String>> {
	Ok(vec![entity.id.to_string()])
}

pub(super) async fn identifiers_allowed_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	section: PresaveScopeSection,
	identifiers: Vec<String>,
) -> Result<bool> {
	let Some(allowed) = allowed_scope_for_section(ctx, mm, section).await? else {
		return Ok(true);
	};
	if allowed.is_empty() {
		return Ok(true);
	}
	Ok(identifiers
		.iter()
		.any(|identifier| allowed.contains(identifier)))
}

pub(super) fn deny_presave_scope() -> Error {
	Error::PermissionDenied {
		required_permission: "PresaveTemplate.Scope".to_string(),
	}
}

pub(super) fn presave_case_link_conflict(message: &str) -> Error {
	model::Error::Conflict {
		message: message.to_string(),
	}
	.into()
}

pub(super) fn rest_ok<T: Serialize>(
	data: T,
) -> (StatusCode, Json<DataRestResult<T>>) {
	(StatusCode::OK, Json(DataRestResult { data }))
}

pub(super) fn rest_created<T: Serialize>(
	data: T,
) -> (StatusCode, Json<DataRestResult<T>>) {
	(StatusCode::CREATED, Json(DataRestResult { data }))
}

pub(super) async fn presave_scope_assigned_to_users(
	mm: &ModelManager,
	organization_id: Uuid,
	scope_column: &str,
	identifiers: Vec<String>,
) -> Result<bool> {
	if identifiers.is_empty() {
		return Ok(false);
	}
	let sql = match scope_column {
		"access_sender_ids" => {
			r#"
			SELECT EXISTS (
				SELECT 1
				FROM users u
				CROSS JOIN LATERAL jsonb_array_elements_text(
					CASE
						WHEN u.access_sender_ids IS NULL OR btrim(u.access_sender_ids) = ''
							THEN '[]'::jsonb
						ELSE u.access_sender_ids::jsonb
					END
				) AS scope_value(value)
				WHERE u.organization_id = $1
				  AND u.active = true
				  AND lower(btrim(scope_value.value)) = ANY($2)
			)
			"#
		}
		"access_product_ids" => {
			r#"
			SELECT EXISTS (
				SELECT 1
				FROM users u
				CROSS JOIN LATERAL jsonb_array_elements_text(
					CASE
						WHEN u.access_product_ids IS NULL OR btrim(u.access_product_ids) = ''
							THEN '[]'::jsonb
						ELSE u.access_product_ids::jsonb
					END
				) AS scope_value(value)
				WHERE u.organization_id = $1
				  AND u.active = true
				  AND lower(btrim(scope_value.value)) = ANY($2)
			)
			"#
		}
		"access_study_ids" => {
			r#"
			SELECT EXISTS (
				SELECT 1
				FROM users u
				CROSS JOIN LATERAL jsonb_array_elements_text(
					CASE
						WHEN u.access_study_ids IS NULL OR btrim(u.access_study_ids) = ''
							THEN '[]'::jsonb
						ELSE u.access_study_ids::jsonb
					END
				) AS scope_value(value)
				WHERE u.organization_id = $1
				  AND u.active = true
				  AND lower(btrim(scope_value.value)) = ANY($2)
			)
			"#
		}
		_ => return Ok(false),
	};
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(sql)
				.bind(organization_id)
				.bind(identifiers),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn sender_presave_used_by_cases(
	mm: &ModelManager,
	organization_id: Uuid,
	id: Uuid,
) -> Result<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT EXISTS (
					SELECT 1
					FROM sender_information sender
					JOIN cases c ON c.id = sender.case_id
					WHERE c.organization_id = $1
					  AND sender.source_sender_presave_id = $2
				)
				"#,
			)
			.bind(organization_id)
			.bind(id),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn product_presave_used_by_cases(
	mm: &ModelManager,
	organization_id: Uuid,
	id: Uuid,
) -> Result<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT EXISTS (
					SELECT 1
					FROM drug_information drug
					JOIN cases c ON c.id = drug.case_id
					WHERE c.organization_id = $1
					  AND drug.source_product_presave_id = $2
				)
				"#,
			)
			.bind(organization_id)
			.bind(id),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn study_presave_used_by_cases(
	mm: &ModelManager,
	organization_id: Uuid,
	id: Uuid,
) -> Result<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT EXISTS (
					SELECT 1
					FROM study_information study
					JOIN cases c ON c.id = study.case_id
					WHERE c.organization_id = $1
					  AND study.source_study_presave_id = $2
				)
				"#,
			)
			.bind(organization_id)
			.bind(id),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn reporter_presave_used_by_cases(
	mm: &ModelManager,
	organization_id: Uuid,
	id: Uuid,
) -> Result<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT EXISTS (
					SELECT 1
					FROM primary_sources source
					JOIN cases c ON c.id = source.case_id
					WHERE c.organization_id = $1
					  AND source.source_reporter_presave_id = $2
				)
				"#,
			)
			.bind(organization_id)
			.bind(id),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn narrative_presave_used_by_cases(
	mm: &ModelManager,
	organization_id: Uuid,
	id: Uuid,
) -> Result<bool> {
	let (exists,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				r#"
				SELECT EXISTS (
					SELECT 1
					FROM narrative_information narrative
					JOIN cases c ON c.id = narrative.case_id
					WHERE c.organization_id = $1
					  AND narrative.source_narrative_presave_id = $2
				)
				"#,
			)
			.bind(organization_id)
			.bind(id),
		)
		.await
		.map_err(|err| Error::from(model::Error::from(err)))?;
	Ok(exists)
}

pub(super) async fn ensure_sender_presave_scope(
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

pub(crate) async fn ensure_product_presave_scope(
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

pub(super) async fn ensure_study_presave_scope(
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

pub(super) async fn ensure_sender_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	sender_id: Uuid,
) -> Result<()> {
	let parent = SenderPresaveBmc::get(ctx, mm, sender_id).await?;
	ensure_sender_presave_scope(ctx, mm, &parent).await
}

pub(super) async fn ensure_product_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	product_id: Uuid,
) -> Result<()> {
	let parent = ProductPresaveBmc::get(ctx, mm, product_id).await?;
	ensure_product_presave_scope(ctx, mm, &parent).await
}

pub(super) async fn ensure_study_presave_id_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	study_id: Uuid,
) -> Result<()> {
	let parent = StudyPresaveBmc::get(ctx, mm, study_id).await?;
	ensure_study_presave_scope(ctx, mm, &parent).await
}

pub(super) async fn filter_sender_presaves_for_scope(
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
		if allowed.is_empty()
			|| identifiers
				.iter()
				.any(|identifier| allowed.contains(identifier))
		{
			filtered.push(entity);
		}
	}
	Ok(filtered)
}

pub(super) async fn filter_product_presaves_for_scope(
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
			allowed.is_empty()
				|| product_scope_identifiers(entity)
					.iter()
					.any(|identifier| allowed.contains(identifier))
		})
		.collect())
}

pub(super) async fn filter_study_presaves_for_scope(
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
			allowed.is_empty()
				|| study_scope_identifiers(entity)
					.iter()
					.any(|identifier| allowed.contains(identifier))
		})
		.collect())
}

pub(super) fn text_present(value: Option<&str>) -> bool {
	value.is_some_and(|value| !value.trim().is_empty())
}

pub(super) fn ensure_parent_scope(
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

pub(super) fn ensure_detail_parent_scope(
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
