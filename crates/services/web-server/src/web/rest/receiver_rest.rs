use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	RECEIVER_CREATE, RECEIVER_DELETE, RECEIVER_READ, RECEIVER_UPDATE,
};
use lib_core::model::receiver::{
	ReceiverInformation, ReceiverInformationBmc, ReceiverInformationForCreate,
	ReceiverInformationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	is_unique_violation, require_case_write_allowed, require_permission, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

pub async fn create_receiver(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<ReceiverInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RECEIVER_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	match ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await {
		Ok(entity) => {
			return Ok((StatusCode::OK, Json(DataRestResult { data: entity })));
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {}
		Err(err) => return Err(err.into()),
	}

	match ReceiverInformationBmc::create(&ctx, &mm, data).await {
		Ok(_) => {
			let entity =
				ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
			Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
		}
		Err(err) if is_unique_violation(&err) => {
			match ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await {
				Ok(entity) => {
					Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
				}
				Err(_) => Err(err.into()),
			}
		}
		Err(err) => Err(err.into()),
	}
}

pub async fn get_receiver(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Option<ReceiverInformation>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RECEIVER_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let entity =
		ReceiverInformationBmc::get_by_case_optional(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_receiver(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<ReceiverInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<ReceiverInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RECEIVER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	ReceiverInformationBmc::update_by_case(&ctx, &mm, case_id, data).await?;
	let entity = ReceiverInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_receiver(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, RECEIVER_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ReceiverInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	Ok(StatusCode::NO_CONTENT)
}
