use crate::error::Result;
use crate::error::{ClientError, Error};
use crate::middleware::mw_req_stamp::ReqStamp;
use axum::http::{Method, StatusCode, Uri};
use lib_core::ctx::Ctx;
use lib_utils::time::{format_time, now_utc};
use time::Duration;
use tracing::{info, warn};

pub async fn log_request(
	http_method: Method,
	uri: Uri,
	req_stamp: ReqStamp,
	ctx: Option<Ctx>,
	status: StatusCode,
	web_error: Option<&Error>,
	rest_error: Option<&lib_rest_core::Error>,
	client_error: Option<ClientError>,
) -> Result<()> {
	let error_type = error_type(web_error, rest_error);
	let sqlstate = database_error(web_error, rest_error)
		.and_then(|error| error.code())
		.map(|code| code.to_string());
	let ReqStamp { uuid, time_in } = req_stamp;
	let now = now_utc();
	let duration: Duration = now - time_in;
	let duration_ms = (duration.as_seconds_f64() * 1_000_000.).floor() / 1_000.;
	let user_id = ctx.map(|ctx| ctx.user_id().to_string());
	let client_error_type = client_error.map(|error| error.as_ref().to_string());
	let http_path = sanitized_http_path(&uri);
	if status.is_client_error() || status.is_server_error() {
		warn!(
			event = "http_request",
			request_id = %uuid,
			timestamp = %format_time(now),
			time_in = %format_time(time_in),
			duration_ms,
			http_method = %http_method,
			http_path,
			status = status.as_u16(),
			user_id = ?user_id.as_deref(),
			client_error_type = ?client_error_type.as_deref(),
			error_type = ?error_type.as_deref(),
			sqlstate = ?sqlstate.as_deref(),
			"request completed with error"
		);
	} else {
		info!(
			event = "http_request",
			request_id = %uuid,
			timestamp = %format_time(now),
			time_in = %format_time(time_in),
			duration_ms,
			http_method = %http_method,
			http_path,
			status = status.as_u16(),
			user_id = ?user_id.as_deref(),
			"request completed"
		);
	}

	Ok(())
}

fn error_type(
	web_error: Option<&Error>,
	rest_error: Option<&lib_rest_core::Error>,
) -> Option<String> {
	match web_error {
		Some(Error::Model(error)) => Some(model_error_type(error)),
		Some(Error::Rest(error)) => Some(rest_error_type(error).to_string()),
		Some(Error::CtxExt(error)) => {
			Some(format!("CtxExt.{}", ctx_error_type(error)))
		}
		Some(error) => Some(error.as_ref().to_string()),
		None => rest_error.map(rest_error_type),
	}
}

fn rest_error_type(error: &lib_rest_core::Error) -> String {
	match error {
		lib_rest_core::Error::Model(error) => model_error_type(error),
		_ => error.diagnostic_type(),
	}
}

fn model_error_type(error: &lib_core::model::Error) -> String {
	use lib_core::model::Error::*;
	match error {
		EntityNotFound { .. } => "Model.EntityNotFound".into(),
		EntityUuidNotFound { .. } => "Model.EntityUuidNotFound".into(),
		ListLimitOverMax { .. } => "Model.ListLimitOverMax".into(),
		CountFail => "Model.CountFail".into(),
		UserAlreadyExists { .. } => "Model.UserAlreadyExists".into(),
		UniqueViolation { .. } => "Model.UniqueViolation".into(),
		Validation { .. } => "Model.Validation".into(),
		Conflict { .. } => "Model.Conflict".into(),
		CaseStateConflict => "Model.CaseStateConflict".into(),
		CantCreateModelManagerProvider(_) => {
			"Model.CantCreateModelManagerProvider".into()
		}
		Store(_) => "Model.Store".into(),
		Pwd(_) => "Model.Pwd".into(),
		Dbx(lib_core::model::store::dbx::Error::Sqlx(_)) => "Model.Dbx.Sqlx".into(),
		Dbx(lib_core::model::store::dbx::Error::TxnCantCommitNoOpenTxn) => {
			"Model.Dbx.TxnCantCommitNoOpenTxn".into()
		}
		Dbx(lib_core::model::store::dbx::Error::TxnAborted) => {
			"Model.Dbx.TxnAborted".into()
		}
		SeaQuery(_) => "Model.SeaQuery".into(),
		ModqlIntoSea(_) => "Model.ModqlIntoSea".into(),
	}
}

fn ctx_error_type(error: &crate::middleware::mw_auth::CtxExtError) -> &'static str {
	use crate::middleware::mw_auth::CtxExtError::*;
	match error {
		TokenNotInCookie => "TokenNotInCookie",
		TokenWrongFormat => "TokenWrongFormat",
		TokenExpiryWrongFormat => "TokenExpiryWrongFormat",
		UserNotFound => "UserNotFound",
		ModelAccessError(_) => "ModelAccessError",
		FailValidate => "FailValidate",
		CannotSetTokenCookie => "CannotSetTokenCookie",
		AuthorizationSnapshotLoad(_) => "AuthorizationSnapshotLoad",
		PasswordChangeRequired => "PasswordChangeRequired",
		CtxNotInRequestExt => "CtxNotInRequestExt",
		AuthorizationSnapshotNotInRequestExt => {
			"AuthorizationSnapshotNotInRequestExt"
		}
		CtxCreateFail(_) => "CtxCreateFail",
	}
}

fn database_error<'a>(
	web_error: Option<&'a Error>,
	rest_error: Option<&'a lib_rest_core::Error>,
) -> Option<&'a (dyn sqlx::error::DatabaseError + 'static)> {
	let model_error = match web_error {
		Some(Error::Model(error)) => Some(error),
		Some(Error::Rest(lib_rest_core::Error::Model(error))) => Some(error),
		_ => match rest_error {
			Some(lib_rest_core::Error::Model(error)) => Some(error),
			_ => None,
		},
	};
	model_error.and_then(lib_core::model::Error::as_database_error)
}

fn sanitized_http_path(uri: &Uri) -> &str {
	uri.path()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn request_log_path_excludes_query_data() {
		let uri: Uri = "/api/cases?patient=secret".parse().unwrap();
		assert_eq!(sanitized_http_path(&uri), "/api/cases");
	}

	#[test]
	fn diagnostics_are_typed_without_private_values() {
		let rest_error = lib_rest_core::Error::Configuration {
			key: "AS2_CALLBACK_TOKEN",
			message: "private-token-value".into(),
		};
		let diagnostic = error_type(None, Some(&rest_error));
		assert_eq!(
			diagnostic.as_deref(),
			Some("Configuration.AS2_CALLBACK_TOKEN")
		);
		assert!(!diagnostic
			.as_deref()
			.is_some_and(|value| { value.contains("private-token-value") }));

		let web_error = Error::Rest(rest_error);
		let diagnostic = error_type(Some(&web_error), None);
		assert_eq!(
			diagnostic.as_deref(),
			Some("Configuration.AS2_CALLBACK_TOKEN")
		);
	}

	#[test]
	fn internal_source_value_is_not_logged() {
		let rest_error = lib_rest_core::Error::Internal {
			operation: "export.zip.write",
			source: Box::new(std::io::Error::other("private filesystem path")),
		};
		let diagnostic = error_type(None, Some(&rest_error));
		assert_eq!(diagnostic.as_deref(), Some("Internal.export.zip.write"));
		assert!(!diagnostic
			.as_deref()
			.is_some_and(|value| { value.contains("private filesystem path") }));
	}
}
