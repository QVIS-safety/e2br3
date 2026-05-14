use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::organization::{
	Organization, OrganizationBmc, OrganizationFilter, OrganizationForCreate,
	OrganizationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

fn require_system_admin(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_system_admin() {
		return Err(Error::AccessDenied {
			required_role: "system_admin".to_string(),
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
	require_system_admin(&ctx)?;
	let ParamsForCreate { data } = params;
	let id = OrganizationBmc::create(&ctx, &mm, data).await?;
	let entity = OrganizationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn get_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	require_system_admin(&ctx)?;
	let entity = OrganizationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn list_organizations(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<Organization>>>)> {
	let ctx = ctx_w.0;
	require_system_admin(&ctx)?;
	let params =
		ParamsList::<OrganizationFilter>::from_raw_query(raw_query.as_deref())
			.map_err(|message| Error::BadRequest { message })?;
	let entities =
		OrganizationBmc::list(&ctx, &mm, params.filters, params.list_options)
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
	require_system_admin(&ctx)?;
	let ParamsForUpdate { data } = params;
	OrganizationBmc::update(&ctx, &mm, id, data).await?;
	let entity = OrganizationBmc::get(&ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_system_admin(&ctx)?;
	OrganizationBmc::delete(&ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
