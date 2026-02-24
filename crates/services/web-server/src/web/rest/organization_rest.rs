use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	has_permission, ORG_CREATE, ORG_DELETE, ORG_LIST, ORG_READ, ORG_UPDATE,
};
use lib_core::model::organization::{
	Organization, OrganizationBmc, OrganizationFilter, OrganizationForCreate,
	OrganizationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
use uuid::Uuid;

fn require_admin_role(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_admin() {
		return Err(WebError::AccessDenied {
			required_role: "admin".to_string(),
		});
	}
	Ok(())
}

pub async fn create_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<OrganizationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin_role(&ctx)?;

	if !has_permission(ctx.role(), ORG_CREATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "Organization.Create".to_string(),
		});
	}

	let ParamsForCreate { data } = params;
	let id = OrganizationBmc::create(&ctx, &mm, data)
		.await
		.map_err(WebError::Model)?;
	let entity = OrganizationBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn get_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin_role(&ctx)?;

	if !has_permission(ctx.role(), ORG_READ) {
		return Err(WebError::PermissionDenied {
			required_permission: "Organization.Read".to_string(),
		});
	}

	let entity = OrganizationBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn list_organizations(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<Organization>>>)> {
	let ctx = ctx_w.0;
	require_admin_role(&ctx)?;

	if !has_permission(ctx.role(), ORG_LIST) {
		return Err(WebError::PermissionDenied {
			required_permission: "Organization.List".to_string(),
		});
	}

	let params =
		ParamsList::<OrganizationFilter>::from_raw_query(raw_query.as_deref())
			.map_err(|message| {
				WebError::from(lib_rest_core::Error::BadRequest { message })
			})?;

	let entities =
		OrganizationBmc::list(&ctx, &mm, params.filters, params.list_options)
			.await
			.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn update_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<OrganizationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin_role(&ctx)?;

	if !has_permission(ctx.role(), ORG_UPDATE) {
		return Err(WebError::PermissionDenied {
			required_permission: "Organization.Update".to_string(),
		});
	}

	let ParamsForUpdate { data } = params;
	OrganizationBmc::update(&ctx, &mm, id, data)
		.await
		.map_err(WebError::Model)?;
	let entity = OrganizationBmc::get(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_admin_role(&ctx)?;

	if !has_permission(ctx.role(), ORG_DELETE) {
		return Err(WebError::PermissionDenied {
			required_permission: "Organization.Delete".to_string(),
		});
	}

	OrganizationBmc::delete(&ctx, &mm, id)
		.await
		.map_err(WebError::Model)?;

	Ok(StatusCode::NO_CONTENT)
}
