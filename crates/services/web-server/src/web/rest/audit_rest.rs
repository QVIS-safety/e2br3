// Audit Log REST endpoints

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{has_permission, AUDIT_LIST};
use lib_core::model::audit::{
	AuditChainVerificationReport, AuditLog, AuditLogBmc, AuditLogFilter,
	CaseVersion, CaseVersionBmc,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::ParamsList;
use lib_rest_core::rest_result::DataRestResult;
use lib_web::middleware::mw_auth::CtxW;
use lib_web::{Error as WebError, Result};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;

fn require_audit_permission(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_system_admin() && !has_permission(ctx.role(), AUDIT_LIST) {
		return Err(WebError::PermissionDenied {
			required_permission: "AuditLog.List".to_string(),
		});
	}
	Ok(())
}

#[derive(Debug, Deserialize)]
pub struct AuditRecordQuery {
	pub field: Option<String>,
}

fn json_object_has_key(value: &Option<JsonValue>, field: &str) -> bool {
	value
		.as_ref()
		.and_then(JsonValue::as_object)
		.is_some_and(|object| object.contains_key(field))
}

fn audit_log_touches_field(log: &AuditLog, field: &str) -> bool {
	if log.action == "UPDATE" {
		return json_object_has_key(&log.changed_fields, field);
	}
	json_object_has_key(&log.changed_fields, field)
		|| json_object_has_key(&log.old_values, field)
		|| json_object_has_key(&log.new_values, field)
}

/// GET /api/audit-logs
/// List all audit logs with optional filtering
/// **Requires AuditLog.List permission**
pub async fn list_audit_logs(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<AuditLog>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest list_audit_logs", "HANDLER");

	// Verify audit permission
	require_audit_permission(&ctx)?;

	let params = ParamsList::<AuditLogFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| {
			WebError::from(lib_rest_core::Error::BadRequest { message })
		})?;

	let logs = AuditLogBmc::list(&ctx, &mm, params.filters, params.list_options)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: logs })))
}

/// GET /api/audit-logs/by-record/{table_name}/{record_id}
/// List audit logs for a specific record
/// **Requires AuditLog.List permission**
pub async fn list_audit_logs_by_record(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((table_name, record_id)): Path<(String, Uuid)>,
	Query(query): Query<AuditRecordQuery>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<AuditLog>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_audit_logs_by_record table={} id={}",
		"HANDLER",
		table_name,
		record_id
	);

	// Verify audit permission
	require_audit_permission(&ctx)?;

	let mut logs = AuditLogBmc::list_by_record(&ctx, &mm, &table_name, record_id)
		.await
		.map_err(WebError::Model)?;
	if let Some(field) = query
		.field
		.as_deref()
		.map(str::trim)
		.filter(|field| !field.is_empty())
	{
		logs.retain(|log| audit_log_touches_field(log, field));
	}

	Ok((StatusCode::OK, Json(DataRestResult { data: logs })))
}

/// GET /api/cases/{case_id}/versions
/// List all versions for a specific case
/// **Requires AuditLog.Read permission**
pub async fn list_case_versions(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CaseVersion>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_case_versions case_id={}",
		"HANDLER",
		case_id
	);

	// Verify audit permission
	require_audit_permission(&ctx)?;

	let versions = CaseVersionBmc::list_by_case(&ctx, &mm, case_id)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: versions })))
}

/// GET /api/audit-logs/verify-integrity
/// Verifies the append-only audit hash chain integrity.
/// **Requires AuditLog.List permission**
pub async fn verify_audit_log_integrity(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
) -> Result<(
	StatusCode,
	Json<DataRestResult<AuditChainVerificationReport>>,
)> {
	let ctx = ctx_w.0;
	tracing::debug!("{:<12} - rest verify_audit_log_integrity", "HANDLER");

	require_audit_permission(&ctx)?;

	let report = AuditLogBmc::verify_hash_chain(&ctx, &mm)
		.await
		.map_err(WebError::Model)?;

	Ok((StatusCode::OK, Json(DataRestResult { data: report })))
}
