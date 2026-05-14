use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{ORG_CREATE, ORG_DELETE, ORG_LIST, ORG_READ, ORG_UPDATE};
use lib_core::model::organization::{
	Organization, OrganizationBmc, OrganizationFilter, OrganizationForCreate,
	OrganizationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	admin_db_ctx, require_admin, require_permission, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

pub async fn create_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<OrganizationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, ORG_CREATE)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let ParamsForCreate { data } = params;
	let id = OrganizationBmc::create(&db_ctx, &mm, data).await?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn get_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, ORG_READ)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn list_organizations(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<Organization>>>)> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, ORG_LIST)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let params =
		ParamsList::<OrganizationFilter>::from_raw_query(raw_query.as_deref())
			.map_err(|message| Error::BadRequest { message })?;
	let entities =
		OrganizationBmc::list(&db_ctx, &mm, params.filters, params.list_options)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn update_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<OrganizationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, ORG_UPDATE)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	let ParamsForUpdate { data } = params;
	OrganizationBmc::update(&db_ctx, &mm, id, data).await?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_admin(&ctx, &mm).await?;
	if !ctx.is_system_admin() {
		require_permission(&ctx, ORG_DELETE)?;
	}
	let db_ctx = admin_db_ctx(&ctx, &mm).await?;
	OrganizationBmc::delete(&db_ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
