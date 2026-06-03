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
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

fn require_audit_permission(ctx: &lib_core::ctx::Ctx) -> Result<()> {
	if !ctx.is_system_admin()
		&& !has_permission(ctx.permission_subject(), AUDIT_LIST)
	{
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseAuditTrailRow {
	pub no: i64,
	pub audit_log_id: i64,
	pub date_time: OffsetDateTime,
	pub user_display: Option<String>,
	pub page: String,
	pub item: String,
	pub row_no1: String,
	pub row_no2: String,
	pub row_no3: String,
	pub value: String,
	pub notation: String,
	pub null_flavor: String,
	pub reason: String,
	pub e_signature_id: Option<Uuid>,
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

fn audit_field_label(table_name: &str, field: &str) -> (&'static str, String) {
	match (table_name, field) {
		("cases", "report_year") => {
			("CI (C.1)", "Report Year (REPORT_YEAR)".to_string())
		}
		("cases", "report_type") => {
			("CI (C.1)", "Type of Report (C.1.3)".to_string())
		}
		("cases", "date_of_most_recent_information") => (
			"CI (C.1)",
			"Date of Most Recent Information for This Report (C.1.5)".to_string(),
		),
		("cases", "safety_report_id") => (
			"CI (C.1)",
			"Sender's Safety Report Unique Identifier (C.1.1)".to_string(),
		),
		("cases", "mfds_report_type") => (
			"CI (C.1)",
			"MFDS Report Type (MFDS_REPORT_TYPE)".to_string(),
		),
		("safety_report_identification", "report_type") => {
			("CI (C.1)", "Type of Report (C.1.3)".to_string())
		}
		("safety_report_identification", "date_of_most_recent_information") => (
			"CI (C.1)",
			"Date of Most Recent Information for This Report (C.1.5)".to_string(),
		),
		_ => ("Case", field.replace('_', " ")),
	}
}

fn changed_field_entries(log: &AuditLog) -> Vec<(String, JsonValue)> {
	log.changed_fields
		.as_ref()
		.and_then(JsonValue::as_object)
		.map(|object| {
			object
				.iter()
				.map(|(key, value)| (key.clone(), value.clone()))
				.collect()
		})
		.unwrap_or_default()
}

fn json_display_value(value: &JsonValue) -> String {
	match value {
		JsonValue::String(value) => value.clone(),
		JsonValue::Null => String::new(),
		other => other.to_string(),
	}
}

fn audit_display_value(action: &str, diff: &JsonValue) -> String {
	let preferred = if action == "DELETE" { "old" } else { "new" };
	diff.get(preferred)
		.or_else(|| diff.get("new"))
		.or_else(|| diff.get("old"))
		.map(json_display_value)
		.unwrap_or_default()
}

fn audit_notation_value(field: &str, diff: &JsonValue) -> String {
	if field.ends_with("_notation") {
		audit_display_value("UPDATE", diff)
	} else {
		String::new()
	}
}

fn audit_null_flavor_value(field: &str, diff: &JsonValue) -> String {
	if field.ends_with("_null_flavor") {
		audit_display_value("UPDATE", diff)
	} else {
		String::new()
	}
}

fn audit_reason_value(action: &str, reason: Option<String>) -> String {
	reason.unwrap_or_else(|| {
		if action == "CREATE" {
			"Initial Data".to_string()
		} else {
			String::new()
		}
	})
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

/// GET /api/cases/{case_id}/audit-trail
/// Reference-style field-level audit projection for a case.
/// **Requires AuditLog.List permission**
pub async fn list_case_audit_trail(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<CaseAuditTrailRow>>>)> {
	let ctx = ctx_w.0;
	tracing::debug!(
		"{:<12} - rest list_case_audit_trail case_id={}",
		"HANDLER",
		case_id
	);

	require_audit_permission(&ctx)?;

	let logs = AuditLogBmc::list_by_record(&ctx, &mm, "cases", case_id)
		.await
		.map_err(WebError::Model)?;
	let mut rows = Vec::new();
	for log in logs {
		for (field, diff) in changed_field_entries(&log) {
			let (page, item) = audit_field_label(&log.table_name, &field);
			rows.push(CaseAuditTrailRow {
				no: log.id,
				audit_log_id: log.id,
				date_time: log.created_at,
				user_display: log.user_display.clone(),
				page: page.to_string(),
				item,
				row_no1: "0".to_string(),
				row_no2: "0".to_string(),
				row_no3: "0".to_string(),
				value: audit_display_value(&log.action, &diff),
				notation: audit_notation_value(&field, &diff),
				null_flavor: audit_null_flavor_value(&field, &diff),
				reason: audit_reason_value(
					&log.action,
					log.reason_for_change.clone(),
				),
				e_signature_id: log.e_signature_id,
			});
		}
	}

	Ok((StatusCode::OK, Json(DataRestResult { data: rows })))
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

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn audit_field_label_maps_reference_ci_fields() {
		assert_eq!(
			audit_field_label("cases", "report_type"),
			("CI (C.1)", "Type of Report (C.1.3)".to_string())
		);
		assert_eq!(
			audit_field_label("cases", "mfds_report_type"),
			(
				"CI (C.1)",
				"MFDS Report Type (MFDS_REPORT_TYPE)".to_string()
			)
		);
	}

	#[test]
	fn audit_display_value_prefers_new_value_except_delete() {
		let diff = json!({ "old": "1", "new": "2" });
		assert_eq!(audit_display_value("UPDATE", &diff), "2");
		assert_eq!(audit_display_value("CREATE", &diff), "2");
		assert_eq!(audit_display_value("DELETE", &diff), "1");
	}

	#[test]
	fn audit_reason_defaults_create_to_initial_data() {
		assert_eq!(audit_reason_value("CREATE", None), "Initial Data");
		assert_eq!(
			audit_reason_value("CREATE", Some("Imported row".to_string())),
			"Imported row"
		);
		assert_eq!(audit_reason_value("UPDATE", None), "");
	}
}
