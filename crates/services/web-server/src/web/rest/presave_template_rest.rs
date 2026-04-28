use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE, PRESAVE_TEMPLATE_LIST,
	PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
};
use lib_core::model::presave_template::{
	PresaveEntityType, PresaveTemplate, PresaveTemplateAudit,
	PresaveTemplateAuditBmc, PresaveTemplateBmc, PresaveTemplateForCreate,
	PresaveTemplateForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct PresaveTemplateListQuery {
	#[serde(rename = "entityType")]
	pub entity_type: Option<PresaveEntityType>,
}

/// POST /api/presave-templates
pub async fn create_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<PresaveTemplateForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PresaveTemplate>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_CREATE)?;
	let ParamsForCreate { data } = params;
	let id = PresaveTemplateBmc::create(&ctx, &mm, data).await?;
	let entity = PresaveTemplateBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/presave-templates/{id}
pub async fn get_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<PresaveTemplate>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entity = PresaveTemplateBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// GET /api/presave-templates?entityType=sender
pub async fn list_presave_templates(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(query): Query<PresaveTemplateListQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PresaveTemplate>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_LIST)?;
	let entities = if let Some(entity_type) = query.entity_type {
		PresaveTemplateBmc::list_by_entity_type(&ctx, &mm, entity_type).await?
	} else {
		PresaveTemplateBmc::list(&ctx, &mm).await?
	};
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// PATCH /api/presave-templates/{id}
pub async fn update_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<PresaveTemplateForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PresaveTemplate>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_UPDATE)?;
	let ParamsForUpdate { data } = params;
	PresaveTemplateBmc::update(&ctx, &mm, id, data).await?;
	let entity = PresaveTemplateBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/presave-templates/{id}
pub async fn delete_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_DELETE)?;
	PresaveTemplateBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/presave-templates/{id}/audit
pub async fn list_presave_template_audits(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(template_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PresaveTemplateAudit>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PRESAVE_TEMPLATE_READ)?;
	let entities =
		PresaveTemplateAuditBmc::list_by_template(&ctx, &mm, template_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}
