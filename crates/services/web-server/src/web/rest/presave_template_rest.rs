use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	has_permission, PRESAVE_TEMPLATE_CREATE, PRESAVE_TEMPLATE_DELETE,
	PRESAVE_TEMPLATE_LIST, PRESAVE_TEMPLATE_READ, PRESAVE_TEMPLATE_UPDATE,
};
use lib_core::model::presave_template::{
	PresaveEntityType, PresaveTemplate, PresaveTemplateAudit,
	PresaveTemplateAuditBmc, PresaveTemplateBmc, PresaveTemplateForCreate,
	PresaveTemplateForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
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
	tracing::debug!("{:<12} - rest create_presave_template", "HANDLER");

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_CREATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.Create".to_string(),
		});
	}

	let ParamsForCreate { data } = params;
	let id = PresaveTemplateBmc::create(&ctx, &mm, data)
		.await
		.map_err(WebError::Model)?;
	let entity = PresaveTemplateBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/presave-templates/{id}
pub async fn get_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<PresaveTemplate>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest get_presave_template id={}", "HANDLER", id);

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_READ) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.Read".to_string(),
		});
	}

	let entity = PresaveTemplateBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// GET /api/presave-templates?entityType=sender
pub async fn list_presave_templates(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Query(query): Query<PresaveTemplateListQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PresaveTemplate>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_presave_templates entity_type={:?}",
		"HANDLER",
		query.entity_type
	);

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_LIST) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.List".to_string(),
		});
	}

	let entities = if let Some(entity_type) = query.entity_type {
		PresaveTemplateBmc::list_by_entity_type(&ctx, &mm, entity_type)
			.await
			.map_err(WebError::Model)?
	} else {
		PresaveTemplateBmc::list(&ctx, &mm)
			.await
			.map_err(WebError::Model)?
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
	tracing::debug!("{:<12} - rest update_presave_template id={}", "HANDLER", id);

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_UPDATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.Update".to_string(),
		});
	}

	let ParamsForUpdate { data } = params;
	PresaveTemplateBmc::update(&ctx, &mm, id, data)
		.await
		.map_err(WebError::Model)?;
	let entity = PresaveTemplateBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/presave-templates/{id}
pub async fn delete_presave_template(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest delete_presave_template id={}", "HANDLER", id);

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_DELETE) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.Delete".to_string(),
		});
	}

	PresaveTemplateBmc::delete(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;
	Ok(StatusCode::NO_CONTENT)
}

/// GET /api/presave-templates/{id}/audit
pub async fn list_presave_template_audits(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(template_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PresaveTemplateAudit>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_presave_template_audits template_id={}",
		"HANDLER",
		template_id
	);

	if !has_permission(ctx.role(), PRESAVE_TEMPLATE_READ) {
		return Err(WebError::PermissionDenied {
			required_permission: "PresaveTemplate.Read".to_string(),
		});
	}

	let entities = PresaveTemplateAuditBmc::list_by_template(&ctx, &mm, template_id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}
