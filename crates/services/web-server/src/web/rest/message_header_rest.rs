use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	MESSAGE_HEADER_CREATE, MESSAGE_HEADER_DELETE, MESSAGE_HEADER_READ,
	MESSAGE_HEADER_UPDATE,
};
use lib_core::model::message_header::{
	MessageHeader, MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use std::borrow::Cow;
use uuid::Uuid;

fn is_unique_violation(err: &lib_core::model::Error) -> bool {
	matches!(err, lib_core::model::Error::UniqueViolation { .. })
		|| matches!(
			err.as_database_error().and_then(|db| db.code()),
			Some(Cow::Borrowed("23505"))
		) || {
		let text = format!("{err:?}").to_ascii_lowercase();
		text.contains("duplicate") || text.contains("unique")
	}
}

pub async fn create_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<MessageHeaderForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MESSAGE_HEADER_CREATE)?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	match MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await {
		Ok(entity) => {
			return Ok((StatusCode::OK, Json(DataRestResult { data: entity })));
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {}
		Err(err) => return Err(err.into()),
	}

	match MessageHeaderBmc::create(&ctx, &mm, data).await {
		Ok(_) => {
			let entity = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
			Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
		}
		Err(err) if is_unique_violation(&err) => {
			match MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await {
				Ok(entity) => {
					Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
				}
				Err(_) => Err(err.into()),
			}
		}
		Err(err) => Err(err.into()),
	}
}

pub async fn get_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MESSAGE_HEADER_READ)?;
	let entity = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<MessageHeaderForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MESSAGE_HEADER_UPDATE)?;
	let ParamsForUpdate { data } = params;
	MessageHeaderBmc::update_by_case(&ctx, &mm, case_id, data).await?;
	let entity = MessageHeaderBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, MESSAGE_HEADER_DELETE)?;
	MessageHeaderBmc::delete_by_case(&ctx, &mm, case_id).await?;
	Ok(StatusCode::NO_CONTENT)
}
