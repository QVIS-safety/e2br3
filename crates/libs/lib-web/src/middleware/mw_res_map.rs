use crate::error::{ClientError, Error, Result};
use crate::log::log_request;
use crate::middleware::mw_auth::CtxW;
use crate::middleware::mw_req_stamp::ReqStamp;

use axum::http::{Method, Uri};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::model;
use lib_rest_core::prelude::StatusCode;
use serde_json::{json, to_value};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

fn normalize_detail_for_client(detail: serde_json::Value) -> serde_json::Value {
	match detail {
		serde_json::Value::Object(map) => {
			if let Some(required_permission) =
				map.get("required_permission").and_then(|v| v.as_str())
			{
				return serde_json::Value::String(format!(
					"Missing permission: {required_permission}"
				));
			}
			serde_json::Value::Object(map)
		}
		other => other,
	}
}

fn map_pg_constraint(code: &str) -> Option<(StatusCode, String)> {
	// Postgres SQLSTATE reference:
	// - 23502: not_null_violation
	// - 23503: foreign_key_violation
	// - 23505: unique_violation
	// - 23514: check_violation
	//
	// We intentionally do not leak table/constraint names here.
	match code {
		"23505" => Some((
			StatusCode::CONFLICT,
			"Duplicate value: a record with the same key already exists."
				.to_string(),
		)),
		"23503" => Some((
			StatusCode::BAD_REQUEST,
			"Invalid reference: one of the related records does not exist."
				.to_string(),
		)),
		"23502" => Some((
			StatusCode::BAD_REQUEST,
			"Missing required field: a required value was not provided.".to_string(),
		)),
		"23514" => Some((
			StatusCode::BAD_REQUEST,
			"Invalid value: one or more fields failed a server-side constraint."
				.to_string(),
		)),
		_ => None,
	}
}

fn map_model_error_to_client(
	model_err: &model::Error,
	debug_errors: bool,
) -> (StatusCode, ClientError, Option<serde_json::Value>) {
	// Prefer stable, user-facing 4xx responses for predictable errors.
	match model_err {
		model::Error::EntityNotFound { entity, id } => (
			StatusCode::NOT_FOUND,
			ClientError::ENTITY_NOT_FOUND { entity, id: *id },
			None,
		),
		model::Error::EntityUuidNotFound { entity, id } => (
			StatusCode::NOT_FOUND,
			ClientError::ENTITY_UUID_NOT_FOUND {
				entity,
				id: id.to_string(),
			},
			None,
		),
		model::Error::UserAlreadyExists { .. } => (
			StatusCode::CONFLICT,
			ClientError::SERVICE_ERROR,
			Some(serde_json::Value::String(
				"A user with that email already exists.".to_string(),
			)),
		),
		model::Error::UniqueViolation { .. } => (
			StatusCode::CONFLICT,
			ClientError::SERVICE_ERROR,
			Some(serde_json::Value::String(
				"Duplicate value: a record with the same key already exists."
					.to_string(),
			)),
		),
		_ => {
			if let Some(db_err) = model_err.as_database_error() {
				let code = db_err.code().map(|c| c.to_string()).unwrap_or_default();
				if let Some((status, detail)) = map_pg_constraint(code.as_str()) {
					return (
						status,
						ClientError::SERVICE_ERROR,
						Some(serde_json::Value::String(detail)),
					);
				}
				if debug_errors {
					let mut detail = format!(
						"db_error code={} message={}",
						code,
						db_err.message()
					);
					if let Some(constraint) = db_err.constraint() {
						detail.push_str(&format!(" constraint={constraint}"));
					}
					if let Some(table) = db_err.table() {
						detail.push_str(&format!(" table={table}"));
					}
					return (
						StatusCode::BAD_REQUEST,
						ClientError::SERVICE_ERROR,
						Some(serde_json::Value::String(detail)),
					);
				}
				return (
					StatusCode::BAD_REQUEST,
					ClientError::SERVICE_ERROR,
					Some(serde_json::Value::String(
						"Invalid input: the server rejected one or more fields."
							.to_string(),
					)),
				);
			}

			let detail = if debug_errors {
				Some(serde_json::Value::String(format!("{model_err:?}")))
			} else {
				None
			};
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				ClientError::SERVICE_ERROR,
				detail,
			)
		}
	}
}

pub async fn mw_response_map(
	ctx: Result<CtxW>, // Axum 0.8 does not seem to support Option anymore
	uri: Uri,
	req_method: Method,
	req_stamp: ReqStamp,
	res: Response,
) -> Response {
	let ctx = ctx.map(|ctx| ctx.0).ok();

	debug!("{:<12} - mw_response_map", "RES_MAPPER");
	let uuid = Uuid::new_v4();

	// -- Get the eventual response error.
	let web_error = res.extensions().get::<Arc<Error>>().map(Arc::as_ref);
	let rest_error = res
		.extensions()
		.get::<Arc<lib_rest_core::Error>>()
		.map(Arc::as_ref);
	let debug_errors = std::env::var("E2BR3_DEBUG_ERRORS")
		.map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
		.unwrap_or(false);
	let mut debug_detail: Option<serde_json::Value> = None;
	let client_status_error = if let Some(err) = web_error {
		// web::Error often wraps model errors directly (e.g. /api/users).
		// Map predictable DB/model failures to stable 4xx responses with a safe detail.
		let (status_code, client_error, detail_override) = match err {
			Error::Model(model_err) => {
				map_model_error_to_client(model_err, debug_errors)
			}
			Error::Rest(rest_err) => {
				// Prefer the same mapping logic as the rest_error branch below.
				// We keep this lightweight: status and client_error come from existing
				// patterns; detail is populated for BadRequest/Xml similarly.
				match rest_err {
					lib_rest_core::Error::PermissionDenied {
						required_permission,
					} => (
						StatusCode::FORBIDDEN,
						ClientError::PERMISSION_DENIED {
							required_permission: required_permission.clone(),
						},
						None,
					),
					lib_rest_core::Error::BadRequest { message } => (
						StatusCode::BAD_REQUEST,
						ClientError::SERVICE_ERROR,
						Some(serde_json::Value::String(message.clone())),
					),
					lib_rest_core::Error::Xml(err) => (
						StatusCode::BAD_REQUEST,
						ClientError::SERVICE_ERROR,
						Some(serde_json::Value::String(format!("{err:?}"))),
					),
					lib_rest_core::Error::Model(model_err) => {
						map_model_error_to_client(model_err, debug_errors)
					}
					_ => (
						StatusCode::INTERNAL_SERVER_ERROR,
						ClientError::SERVICE_ERROR,
						if debug_errors {
							Some(serde_json::Value::String(format!("{rest_err:?}")))
						} else {
							None
						},
					),
				}
			}
			_ => {
				// Keep existing behavior for auth/permission/etc.
				let (status, client) = err.client_status_and_error();
				let detail = if debug_errors {
					Some(serde_json::Value::String(format!("{err:?}")))
				} else {
					None
				};
				(status, client, detail)
			}
		};

		if detail_override.is_some() {
			debug_detail = detail_override;
		}
		Some((status_code, client_error))
	} else if let Some(err) = rest_error {
		if debug_errors {
			debug_detail = Some(serde_json::Value::String(format!("{err:?}")));
		}
		let (status_code, client_error) = match err {
			lib_rest_core::Error::PermissionDenied {
				required_permission,
			} => (
				StatusCode::FORBIDDEN,
				ClientError::PERMISSION_DENIED {
					required_permission: required_permission.clone(),
				},
			),
			lib_rest_core::Error::BadRequest { message } => {
				debug_detail = Some(serde_json::Value::String(message.clone()));
				(StatusCode::BAD_REQUEST, ClientError::SERVICE_ERROR)
			}
			lib_rest_core::Error::Xml(err) => {
				debug_detail = Some(serde_json::Value::String(format!("{err:?}")));
				(StatusCode::BAD_REQUEST, ClientError::SERVICE_ERROR)
			}
			lib_rest_core::Error::Model(model::Error::EntityNotFound {
				entity,
				id,
			}) => (
				StatusCode::NOT_FOUND,
				ClientError::ENTITY_NOT_FOUND { entity, id: *id },
			),
			lib_rest_core::Error::Model(model::Error::EntityUuidNotFound {
				entity,
				id,
			}) => (
				StatusCode::NOT_FOUND,
				ClientError::ENTITY_UUID_NOT_FOUND {
					entity,
					id: id.to_string(),
				},
			),
			lib_rest_core::Error::Model(model_err) => {
				let (status, client, detail) =
					map_model_error_to_client(model_err, debug_errors);
				if detail.is_some() {
					debug_detail = detail;
				}
				(status, client)
			}
			lib_rest_core::Error::SerdeJson(_) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				ClientError::SERVICE_ERROR,
			),
		};
		Some((status_code, client_error))
	} else {
		None
	};

	// -- If client error, build the new reponse.
	let error_response =
		client_status_error
			.as_ref()
			.map(|(status_code, client_error)| {
				let client_error = to_value(client_error).ok();
				let message = client_error.as_ref().and_then(|v| v.get("message"));
				let detail = debug_detail
					.clone()
					.or_else(|| {
						client_error.as_ref().and_then(|v| v.get("detail")).cloned()
					})
					.map(normalize_detail_for_client);

				let client_error_body = json!({
						"error": {
						"message": message, // Variant name
						"data": {
							"req_uuid": uuid.to_string(),
							"detail": detail
						},
					}
				});

				debug!("CLIENT ERROR BODY:\n{client_error_body}");

				// Build the new response from the client_error_body
				(*status_code, Json(client_error_body)).into_response()
			});

	// -- Build and log the server log line.
	let client_error = client_status_error.unzip().1;

	if let Err(err) =
		log_request(req_method, uri, req_stamp, ctx, web_error, client_error).await
	{
		error!("log_request failed: {err}");
	}

	debug!("\n");

	error_response.unwrap_or(res)
}
