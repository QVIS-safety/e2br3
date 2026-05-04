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
use lib_core::model::user::UserBmc;
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct PresaveTemplateListQuery {
	#[serde(rename = "entityType")]
	pub entity_type: Option<PresaveEntityType>,
}

fn normalized_set(values: Vec<String>) -> HashSet<String> {
	values
		.into_iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

fn collect_json_strings_for_keys(
	value: &Value,
	keys: &[&str],
	out: &mut Vec<String>,
) {
	match value {
		Value::Object(map) => {
			for (key, value) in map {
				if keys
					.iter()
					.any(|candidate| key.eq_ignore_ascii_case(candidate))
				{
					if let Some(text) = value.as_str() {
						let text = text.trim();
						if !text.is_empty() {
							out.push(text.to_ascii_lowercase());
						}
					}
				}
				collect_json_strings_for_keys(value, keys, out);
			}
		}
		Value::Array(items) => {
			for item in items {
				collect_json_strings_for_keys(item, keys, out);
			}
		}
		_ => {}
	}
}

fn template_scope_identifiers(template: &PresaveTemplate) -> Vec<String> {
	let keys = match template.entity_type {
		PresaveEntityType::Sender => &[
			"senderIdentifier",
			"messageSenderIdentifier",
			"batchSenderIdentifier",
			"senderOrganization",
		][..],
		PresaveEntityType::Product => &[
			"productId",
			"productIdentifier",
			"medicinalProduct",
			"drugGenericName",
			"drugBrandName",
			"drugAuthorizationNumber",
			"mpid",
			"phpid",
		][..],
		PresaveEntityType::Study => &[
			"studyId",
			"sponsorStudyNumber",
			"studyName",
			"studyRegistrationNumber",
		][..],
		_ => &[][..],
	};
	let mut values = Vec::new();
	collect_json_strings_for_keys(&template.data, keys, &mut values);
	values
}

async fn allowed_scope_for_entity(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	entity_type: PresaveEntityType,
) -> Result<Option<HashSet<String>>> {
	if lib_rest_core::is_safety_db_admin(ctx, mm).await? {
		return Ok(None);
	}
	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let values = match entity_type {
		PresaveEntityType::Sender => {
			lib_rest_core::scope_values_from_raw(user.access_sender_ids.as_deref())
		}
		PresaveEntityType::Product => {
			lib_rest_core::scope_values_from_raw(user.access_product_ids.as_deref())
		}
		PresaveEntityType::Study => {
			lib_rest_core::scope_values_from_raw(user.access_study_ids.as_deref())
		}
		_ => return Ok(None),
	};
	Ok(Some(normalized_set(values)))
}

async fn presave_template_allowed_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	template: &PresaveTemplate,
) -> Result<bool> {
	let Some(allowed) =
		allowed_scope_for_entity(ctx, mm, template.entity_type).await?
	else {
		return Ok(true);
	};
	if allowed.is_empty() {
		return Ok(false);
	}
	let identifiers = template_scope_identifiers(template);
	Ok(identifiers
		.iter()
		.any(|identifier| allowed.contains(identifier)))
}

async fn filter_presave_templates_for_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	templates: Vec<PresaveTemplate>,
) -> Result<Vec<PresaveTemplate>> {
	let mut filtered = Vec::with_capacity(templates.len());
	for template in templates {
		if presave_template_allowed_for_scope(ctx, mm, &template).await? {
			filtered.push(template);
		}
	}
	Ok(filtered)
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
	if !presave_template_allowed_for_scope(&ctx, &mm, &entity).await? {
		return Err(Error::PermissionDenied {
			required_permission: "PresaveTemplate.Scope".to_string(),
		});
	}
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
	let entities = filter_presave_templates_for_scope(&ctx, &mm, entities).await?;
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
