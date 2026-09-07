use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::message_header::{
	MessageHeader, MessageHeaderBmc, MessageHeaderForCreate, MessageHeaderForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{is_unique_violation, Result};
use lib_utils::time::format_e2b_timestamp;
use lib_web::middleware::mw_auth::CtxW;
use serde_json::{Map, Value};
use uuid::Uuid;

use super::case_editor_rest::validate_row_payload;

fn validate_message_header_fields(fields: Map<String, Value>) -> Result<()> {
	validate_row_payload("N", "messageHeader", &fields, None)
}

fn create_constraint_fields(data: &MessageHeaderForCreate) -> Map<String, Value> {
	let mut fields = Map::from_iter([
		(
			"messageNumber".to_string(),
			Value::String(data.message_number.clone()),
		),
		(
			"messageSenderIdentifier".to_string(),
			Value::String(data.message_sender_identifier.clone()),
		),
		(
			"messageReceiverIdentifier".to_string(),
			Value::String(data.message_receiver_identifier.clone()),
		),
		(
			"messageDate".to_string(),
			Value::String(data.message_date.clone()),
		),
	]);
	if let Some(value) = &data.batch_sender_identifier {
		fields.insert(
			"batchSenderIdentifier".to_string(),
			Value::String(value.clone()),
		);
	}
	if let Some(value) = &data.batch_receiver_identifier {
		fields.insert(
			"batchReceiverIdentifier".to_string(),
			Value::String(value.clone()),
		);
	}
	if let Some(value) = data.batch_transmission_date {
		fields.insert(
			"batchTransmissionDate".to_string(),
			Value::String(format_e2b_timestamp(value)),
		);
	}
	fields
}

fn update_constraint_fields(data: &MessageHeaderForUpdate) -> Map<String, Value> {
	let mut fields = Map::new();
	macro_rules! insert_string {
		($name:literal, $value:expr) => {
			if let Some(value) = &$value {
				fields.insert($name.to_string(), Value::String(value.clone()));
			}
		};
	}
	insert_string!("batchNumber", data.batch_number);
	insert_string!("batchSenderIdentifier", data.batch_sender_identifier);
	insert_string!("batchReceiverIdentifier", data.batch_receiver_identifier);
	if let Some(value) = data.batch_transmission_date {
		fields.insert(
			"batchTransmissionDate".to_string(),
			Value::String(format_e2b_timestamp(value)),
		);
	}
	insert_string!("messageNumber", data.message_number);
	insert_string!("messageSenderIdentifier", data.message_sender_identifier);
	insert_string!(
		"messageReceiverIdentifier",
		data.message_receiver_identifier
	);
	insert_string!("messageDate", data.message_date);
	fields
}

pub async fn create_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<MessageHeaderForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"message-header",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				validate_message_header_fields(create_constraint_fields(&data))?;

				match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await {
					Ok(entity) => {
						return Ok((
							StatusCode::OK,
							Json(DataRestResult { data: entity }),
						));
					}
					Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {}
					Err(err) => return Err(err.into()),
				}

				match MessageHeaderBmc::create(ctx, mm, data).await {
					Ok(_) => {
						let entity =
							MessageHeaderBmc::get_by_case(ctx, mm, case_id).await?;
						Ok((
							StatusCode::CREATED,
							Json(DataRestResult { data: entity }),
						))
					}
					Err(err) if is_unique_violation(&err) => {
						match MessageHeaderBmc::get_by_case(ctx, mm, case_id).await {
							Ok(entity) => Ok((
								StatusCode::OK,
								Json(DataRestResult { data: entity }),
							)),
							Err(_) => Err(err.into()),
						}
					}
					Err(err) => Err(err.into()),
				}
			})
		},
	)
	.await
}

pub async fn get_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"message-header",
		move |ctx, mm| {
			Box::pin(async move {
				let entity = MessageHeaderBmc::get_by_case(ctx, mm, case_id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn update_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<MessageHeaderForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<MessageHeader>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"message-header",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				validate_message_header_fields(update_constraint_fields(&data))?;
				MessageHeaderBmc::update_by_case(ctx, mm, case_id, data).await?;
				let entity = MessageHeaderBmc::get_by_case(ctx, mm, case_id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn delete_message_header(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"message-header",
		move |ctx, mm| {
			Box::pin(async move {
				MessageHeaderBmc::delete_by_case(ctx, mm, case_id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}
