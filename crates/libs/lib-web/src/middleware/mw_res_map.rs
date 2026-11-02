use crate::error::{ClientError, Error, Result};
use crate::log::log_request;
use crate::middleware::mw_auth::{CtxW, RbacPolicyVersion};
use crate::middleware::mw_req_stamp::ReqStamp;

use axum::extract::Extension;
use axum::http::{HeaderValue, Method, Uri};
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::model;
use lib_rest_core::prelude::StatusCode;
use serde_json::{json, to_value};
use std::sync::Arc;
use tracing::{debug, error};

fn normalize_detail_for_client(detail: serde_json::Value) -> serde_json::Value {
	match detail {
		serde_json::Value::Object(map) => {
			if let Some(required_permission) =
				map.get("required_permission").and_then(|v| v.as_str())
			{
				return json!({ "requiredPermission": required_permission });
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
) -> (StatusCode, ClientError, Option<serde_json::Value>) {
	// Prefer stable, user-facing 4xx responses for predictable errors.
	match model_err {
		model::Error::EntityNotFound { .. } => {
			(StatusCode::NOT_FOUND, ClientError::ENTITY_NOT_FOUND, None)
		}
		model::Error::EntityUuidNotFound { .. } => (
			StatusCode::NOT_FOUND,
			ClientError::ENTITY_UUID_NOT_FOUND,
			None,
		),
		model::Error::UserAlreadyExists { .. } => (
			StatusCode::CONFLICT,
			ClientError::CONFLICT,
			Some(serde_json::Value::String(
				"A user with that email already exists.".to_string(),
			)),
		),
		model::Error::UniqueViolation { .. } => (
			StatusCode::CONFLICT,
			ClientError::CONFLICT,
			Some(serde_json::Value::String(
				"Duplicate value: a record with the same key already exists."
					.to_string(),
			)),
		),
		model::Error::Validation { message } => (
			StatusCode::BAD_REQUEST,
			ClientError::INVALID_REQUEST,
			Some(serde_json::Value::String(message.clone())),
		),
		model::Error::Conflict { message } => (
			StatusCode::CONFLICT,
			ClientError::CONFLICT,
			Some(serde_json::Value::String(message.clone())),
		),
		model::Error::CaseStateConflict => {
			(StatusCode::CONFLICT, ClientError::CASE_STATE_CONFLICT, None)
		}
		_ => {
			if let Some(db_err) = model_err.as_database_error() {
				if let Some((status, detail)) = db_err
					.code()
					.and_then(|code| map_pg_constraint(code.as_ref()))
				{
					return (
						status,
						if status == StatusCode::CONFLICT {
							ClientError::CONFLICT
						} else {
							ClientError::INVALID_REQUEST
						},
						Some(serde_json::Value::String(detail)),
					);
				}
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					ClientError::SERVICE_ERROR,
					None,
				);
			}
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				ClientError::SERVICE_ERROR,
				None,
			)
		}
	}
}

fn map_rest_error_to_client(
	error: &lib_rest_core::Error,
) -> (StatusCode, ClientError, Option<serde_json::Value>) {
	match error {
		lib_rest_core::Error::AccessDenied { required_role } => (
			StatusCode::FORBIDDEN,
			ClientError::ACCESS_DENIED {
				required_role: required_role.clone(),
			},
			None,
		),
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
			ClientError::INVALID_REQUEST,
			Some(serde_json::Value::String(message.clone())),
		),
		lib_rest_core::Error::Authentication => {
			(StatusCode::UNAUTHORIZED, ClientError::NO_AUTH, None)
		}
		lib_rest_core::Error::Configuration { .. } => (
			StatusCode::SERVICE_UNAVAILABLE,
			ClientError::CONFIGURATION_ERROR,
			None,
		),
		lib_rest_core::Error::Service { .. }
		| lib_rest_core::Error::Export(_)
		| lib_rest_core::Error::Internal { .. } => (
			StatusCode::INTERNAL_SERVER_ERROR,
			ClientError::SERVICE_ERROR,
			None,
		),
		lib_rest_core::Error::ConstraintViolation(detail) => (
			StatusCode::UNPROCESSABLE_ENTITY,
			ClientError::CONSTRAINT_VIOLATION,
			Some(
				to_value(detail)
					.expect("constraint violation detail must serialize"),
			),
		),
		lib_rest_core::Error::Xml(_) => match error.xml_client_classification() {
			Some(lib_rest_core::XmlClientClassification::InvalidInput(message)) => (
				StatusCode::BAD_REQUEST,
				ClientError::INVALID_REQUEST,
				Some(json!(message)),
			),
			Some(lib_rest_core::XmlClientClassification::Model(model_error)) => {
				map_model_error_to_client(model_error)
			}
			Some(lib_rest_core::XmlClientClassification::Internal) | None => (
				StatusCode::INTERNAL_SERVER_ERROR,
				ClientError::SERVICE_ERROR,
				None,
			),
		},
		lib_rest_core::Error::Model(model_error) => {
			map_model_error_to_client(model_error)
		}
		lib_rest_core::Error::SerdeJson(_) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			ClientError::SERVICE_ERROR,
			None,
		),
	}
}

pub async fn mw_response_map(
	ctx: Result<CtxW>, // Axum 0.8 does not seem to support Option anymore
	uri: Uri,
	req_method: Method,
	req_stamp: ReqStamp,
	policy_version: Option<Extension<RbacPolicyVersion>>,
	res: Response,
) -> Response {
	let ctx = ctx.map(|ctx| ctx.0).ok();

	debug!("{:<12} - mw_response_map", "RES_MAPPER");
	let uuid = req_stamp.uuid;

	// -- Get the eventual response error.
	let web_error = res.extensions().get::<Arc<Error>>().map(Arc::as_ref);
	let rest_error = res
		.extensions()
		.get::<Arc<lib_rest_core::Error>>()
		.map(Arc::as_ref);
	let mut debug_detail: Option<serde_json::Value> = None;
	let client_status_error = if let Some(err) = web_error {
		// web::Error often wraps model errors directly (e.g. /api/users).
		// Map predictable DB/model failures to stable 4xx responses with a safe detail.
		let (status_code, client_error, detail_override) = match err {
			Error::Model(model_err) => map_model_error_to_client(model_err),
			Error::Rest(rest_err) => map_rest_error_to_client(rest_err),
			_ => {
				let (status, client) = err.client_status_and_error();
				(status, client, None)
			}
		};

		if detail_override.is_some() {
			debug_detail = detail_override;
		}
		Some((status_code, client_error))
	} else if let Some(err) = rest_error {
		let (status_code, client_error, detail) = map_rest_error_to_client(err);
		debug_detail = detail;
		Some((status_code, client_error))
	} else {
		None
	};

	// -- If client error, build the new reponse.
	let policy_version_value = policy_version.map(|Extension(value)| value.0);
	let error_response =
		client_status_error
			.as_ref()
			.map(|(status_code, client_error)| {
				let client_error = to_value(client_error).ok();
				let message = client_error.as_ref().and_then(|v| v.get("message"));
				let mut detail = debug_detail
					.clone()
					.or_else(|| {
						client_error.as_ref().and_then(|v| v.get("detail")).cloned()
					})
					.map(normalize_detail_for_client);
				if message.and_then(|value| value.as_str())
					== Some("PERMISSION_DENIED")
				{
					let mut permission_detail = detail
						.take()
						.and_then(|value| value.as_object().cloned())
						.unwrap_or_default();
					if let Some(version) = policy_version_value {
						permission_detail
							.insert("policyVersion".to_string(), json!(version));
					}
					detail = Some(serde_json::Value::Object(permission_detail));
				}

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
	let response_status = client_status_error
		.as_ref()
		.map(|(status, _)| *status)
		.unwrap_or_else(|| res.status());
	let client_error = client_status_error.unzip().1;

	if let Err(err) = log_request(
		req_method,
		uri,
		req_stamp,
		ctx,
		response_status,
		web_error,
		rest_error,
		client_error,
	)
	.await
	{
		error!("log_request failed: {err}");
	}

	debug!("\n");

	let mut response = error_response.unwrap_or(res);
	if let Ok(value) = HeaderValue::from_str(&uuid.to_string()) {
		response.headers_mut().insert("x-request-id", value);
	}
	if let Some(version) = policy_version_value {
		if let Ok(value) = HeaderValue::from_str(&version.to_string()) {
			response
				.headers_mut()
				.insert("x-rbac-policy-version", value);
		}
	}
	response
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_utils::time::now_utc;
	use uuid::Uuid;

	#[test]
	fn validation_errors_are_identifiable_client_errors() {
		let (status, error, detail) =
			map_model_error_to_client(&model::Error::Validation {
				message: "invalid field".into(),
			});
		assert_eq!(status, StatusCode::BAD_REQUEST);
		assert_eq!(error.as_ref(), "INVALID_REQUEST");
		assert_eq!(detail, Some(json!("invalid field")));
	}

	#[test]
	fn conflicts_have_stable_public_codes() {
		let (status, error, detail) =
			map_model_error_to_client(&model::Error::Conflict {
				message: "presave is in use".into(),
			});
		assert_eq!(status, StatusCode::CONFLICT);
		assert_eq!(error.as_ref(), "CONFLICT");
		assert_eq!(detail, Some(json!("presave is in use")));

		let (status, error, detail) =
			map_model_error_to_client(&model::Error::CaseStateConflict);
		assert_eq!(status, StatusCode::CONFLICT);
		assert_eq!(error.as_ref(), "CASE_STATE_CONFLICT");
		assert_eq!(detail, None);
	}

	#[test]
	fn infrastructure_errors_do_not_expose_details() {
		let (status, error, detail) =
			map_rest_error_to_client(&lib_rest_core::Error::Configuration {
				key: "SECRET_SETTING",
				message: "SECRET_SETTING is missing".into(),
			});
		assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
		assert_eq!(error.as_ref(), "CONFIGURATION_ERROR");
		assert_eq!(detail, None);

		let (status, error, detail) =
			map_rest_error_to_client(&lib_rest_core::Error::Service {
				operation: "test.operation",
			});
		assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(error.as_ref(), "SERVICE_ERROR");
		assert_eq!(detail, None);
	}

	#[tokio::test]
	async fn response_exposes_the_request_stamp_id() {
		let request_id = Uuid::new_v4();
		let response = mw_response_map(
			Err(Error::ReqStampNotInReqExt),
			"/health".parse().unwrap(),
			Method::GET,
			ReqStamp {
				uuid: request_id,
				time_in: now_utc(),
			},
			None,
			StatusCode::OK.into_response(),
		)
		.await;
		let expected = request_id.to_string();
		assert_eq!(
			response
				.headers()
				.get("x-request-id")
				.and_then(|value| value.to_str().ok()),
			Some(expected.as_str())
		);
	}
}
