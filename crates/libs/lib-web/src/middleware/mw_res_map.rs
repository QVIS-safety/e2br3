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
				if let Some(detail) = test_result_constraint(
					db_err.code().as_deref(),
					db_err.table(),
					db_err.constraint(),
				) {
					return map_rest_error_to_client(
						&lib_rest_core::Error::ConstraintViolation(detail),
					);
				}
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

fn test_result_constraint(
	code: Option<&str>,
	table: Option<&str>,
	constraint: Option<&str>,
) -> Option<lib_rest_core::ConstraintViolation> {
	match (code, table, constraint) {
		(
			Some("23514"),
			Some("test_results"),
			Some(
				"ck_test_result_qualifier_value"
				| "ck_nfv_test_results_3f503f260cf6",
			),
		) => Some(lib_rest_core::ConstraintViolation {
			rule_code: "LOCAL.F.r.testResultQualifier.REQUIRES.VALUE".into(),
			path: "testResults[].testResultQualifier".into(),
			message: "a test result qualifier requires a numeric test result".into(),
		}),
		_ => None,
	}
}

pub fn map_rest_error_to_client(
	error: &lib_rest_core::Error,
) -> (StatusCode, ClientError, Option<serde_json::Value>) {
	match error {
		lib_rest_core::Error::CaseEditorRow {
			collection_path,
			client_row_index,
			source,
		} => {
			let (status, code, mut detail) = map_rest_error_to_client(source);
			if matches!(&code, ClientError::CONSTRAINT_VIOLATION) {
				if let Some(serde_json::Value::String(path)) =
					detail.as_mut().and_then(|detail| detail.get_mut("path"))
				{
					for prefix in [
						format!("{collection_path}.0."),
						format!("{collection_path}[]."),
					] {
						if let Some(field) = path.strip_prefix(&prefix) {
							*path = format!(
								"{collection_path}.{client_row_index}.{field}"
							);
							break;
						}
					}
				}
			}
			(status, code, detail)
		}
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
		lib_rest_core::Error::Export(
			lib_rest_core::XmlError::MissingExportField { field },
		) => (
			StatusCode::BAD_REQUEST,
			ClientError::INVALID_REQUEST,
			Some(json!(format!("{field} is required for export."))),
		),
		lib_rest_core::Error::Service { .. }
		| lib_rest_core::Error::Export(_)
		| lib_rest_core::Error::Internal { .. } => (
			StatusCode::INTERNAL_SERVER_ERROR,
			ClientError::SERVICE_ERROR,
			None,
		),
		lib_rest_core::Error::ConstraintViolation(detail) => {
			match to_value(detail) {
				Ok(value) => (
					StatusCode::UNPROCESSABLE_ENTITY,
					ClientError::CONSTRAINT_VIOLATION,
					Some(value),
				),
				Err(error) => {
					tracing::error!(%error, "failed to serialize constraint violation");
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						ClientError::SERVICE_ERROR,
						None,
					)
				}
			}
		}
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
	let ctx = match ctx {
		Ok(ctx) => Some(ctx.0),
		Err(_) => None,
	};

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
				let client_error = match to_value(client_error) {
					Ok(value) => Some(value),
					Err(error) => {
						tracing::error!(%error, "failed to serialize client error");
						None
					}
				};
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
					let mut permission_detail = match detail
						.take()
						.and_then(|value| value.as_object().cloned())
					{
						Some(detail) => detail,
						None => serde_json::Map::new(),
					};
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
	let response_status = match client_status_error.as_ref() {
		Some((status, _)) => *status,
		None => res.status(),
	};
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

	let mut response = match error_response {
		Some(response) => response,
		None => res,
	};
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
	fn batch_constraint_context_changes_only_the_outer_row_index() {
		for (collection, path, expected) in [
			(
				"literatureReferences",
				"literatureReferences.0.referenceText",
				"literatureReferences.4.referenceText",
			),
			(
				"patientInformation.pastDrugHistory",
				"patientInformation.pastDrugHistory.0.mpidVersion",
				"patientInformation.pastDrugHistory.4.mpidVersion",
			),
			(
				"reactions",
				"reactions.0.reactionStartDate",
				"reactions.4.reactionStartDate",
			),
			(
				"testResults",
				"testResults[].testResultQualifier",
				"testResults.4.testResultQualifier",
			),
			(
				"drugs",
				"drugs.0.activeSubstances.2.substanceStrengthValue",
				"drugs.4.activeSubstances.2.substanceStrengthValue",
			),
			(
				"testResults",
				"reactions.0.reactionStartDate",
				"reactions.0.reactionStartDate",
			),
			(
				"testResults",
				"testResults.7.testName",
				"testResults.7.testName",
			),
		] {
			let error = lib_rest_core::Error::CaseEditorRow {
				collection_path: collection,
				client_row_index: 4,
				source: Box::new(lib_rest_core::Error::ConstraintViolation(
					lib_rest_core::ConstraintViolation {
						rule_code: "INPUT.TYPE".into(),
						path: path.into(),
						message: "must be a JSON string".into(),
					},
				)),
			};
			let (status, code, detail) = map_rest_error_to_client(&error);
			assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
			assert_eq!(code.as_ref(), "CONSTRAINT_VIOLATION");
			assert_eq!(
				detail,
				Some(
					json!({ "ruleCode": "INPUT.TYPE", "path": expected, "message": "must be a JSON string" })
				)
			);
			assert!(std::error::Error::source(&error).is_some());
			assert_eq!(error.diagnostic_type(), "CaseEditorRow.ConstraintViolation");
		}
	}

	#[test]
	fn batch_context_preserves_permission_not_found_and_service_classification() {
		for source in [
			lib_rest_core::Error::PermissionDenied {
				required_permission: "case.child.update".into(),
			},
			lib_rest_core::Error::BadRequest {
				message: "Invalid row".into(),
			},
			lib_rest_core::Error::Service {
				operation: "private.operation",
			},
			lib_rest_core::Error::Model(model::Error::EntityUuidNotFound {
				entity: "test_results",
				id: Uuid::nil(),
			}),
		] {
			let (expected_status, expected_code, expected_detail) =
				map_rest_error_to_client(&source);
			let wrapped = lib_rest_core::Error::CaseEditorRow {
				collection_path: "testResults",
				client_row_index: 1,
				source: Box::new(source),
			};
			let (status, code, detail) = map_rest_error_to_client(&wrapped);
			assert_eq!(status, expected_status);
			assert_eq!(code.as_ref(), expected_code.as_ref());
			assert_eq!(detail, expected_detail);
		}
	}

	#[test]
	fn qualifier_dependency_has_a_typed_field_error_only_for_its_constraints() {
		for name in [
			"ck_test_result_qualifier_value",
			"ck_nfv_test_results_3f503f260cf6",
		] {
			let detail = test_result_constraint(
				Some("23514"),
				Some("test_results"),
				Some(name),
			)
			.expect("known qualifier dependency");
			let (status, error, detail) = map_rest_error_to_client(
				&lib_rest_core::Error::ConstraintViolation(detail),
			);
			assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
			assert_eq!(error.as_ref(), "CONSTRAINT_VIOLATION");
			assert_eq!(
				detail,
				Some(json!({
					"ruleCode": "LOCAL.F.r.testResultQualifier.REQUIRES.VALUE",
					"path": "testResults[].testResultQualifier",
					"message": "a test result qualifier requires a numeric test result"
				}))
			);
		}
		for (code, table, name) in [
			(
				Some("23514"),
				Some("test_results"),
				Some("ck_test_result_qualifier"),
			),
			(
				Some("23505"),
				Some("test_results"),
				Some("ck_test_result_qualifier_value"),
			),
			(
				Some("23514"),
				Some("other_table"),
				Some("ck_test_result_qualifier_value"),
			),
			(None, None, None),
		] {
			assert!(test_result_constraint(code, table, name).is_none());
		}
	}

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

	#[test]
	fn missing_export_fields_are_safe_client_errors() {
		let (status, error, detail) =
			map_rest_error_to_client(&lib_rest_core::Error::Export(
				lib_rest_core::XmlError::MissingExportField {
					field: "N.1.4 Batch Receiver Identifier",
				},
			));
		assert_eq!(status, StatusCode::BAD_REQUEST);
		assert_eq!(error.as_ref(), "INVALID_REQUEST");
		assert_eq!(
			detail,
			Some(json!(
				"N.1.4 Batch Receiver Identifier is required for export."
			))
		);
	}

	#[test]
	fn internal_export_xml_errors_remain_server_errors() {
		let (status, error, detail) = map_rest_error_to_client(
			&lib_rest_core::Error::Export(lib_rest_core::XmlError::InvalidXml {
				message: "private template failure".into(),
				line: None,
				column: None,
			}),
		);
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
