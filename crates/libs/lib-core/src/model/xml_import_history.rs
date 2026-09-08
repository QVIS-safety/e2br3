// XML Import History BMC — manages the `xml_import_history` table.
//
// Write operations (record) handle their own transaction and RLS context setup.
// Read operations handle their own transaction internally since the import
// list uses a manual RLS read (not the with_rls_read helper).

use crate::authorization::EnforcedScopeFilter;
use crate::ctx::Ctx;
use crate::model::case::case_scope_where;
use crate::model::store::{set_full_context_dbx, set_full_context_dbx_or_rollback};
use crate::model::ModelManager;
use crate::model::Result;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use sqlx::FromRow;
use uuid::Uuid;

// -- Types

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum XmlImportHistoryStatus {
	Success,
	Warning,
	Skipped,
	Error,
}

impl XmlImportHistoryStatus {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Success => "success",
			Self::Warning => "warning",
			Self::Skipped => "skipped",
			Self::Error => "error",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::XmlImportHistoryStatus;

	#[test]
	fn status_values_match_the_database_contract() {
		assert_eq!(XmlImportHistoryStatus::Success.as_str(), "success");
		assert_eq!(XmlImportHistoryStatus::Warning.as_str(), "warning");
		assert_eq!(XmlImportHistoryStatus::Skipped.as_str(), "skipped");
		assert_eq!(XmlImportHistoryStatus::Error.as_str(), "error");
	}
}

/// Full row returned by the history list query (includes uploader email via JOIN).
#[derive(Debug, FromRow)]
pub struct XmlImportHistoryRow {
	pub id: Uuid,
	pub uploaded_file_name: String,
	pub source_file_name: String,
	pub case_id: Option<Uuid>,
	pub case_number: Option<String>,
	pub status: String,
	pub error_message: Option<String>,
	pub uploaded_by: Uuid,
	pub uploader_email: Option<String>,
	pub uploaded_at: OffsetDateTime,
}

/// Minimal row used for error-log download access checks.
#[derive(Debug, FromRow)]
pub struct XmlImportHistoryErrorRow {
	pub case_id: Option<Uuid>,
	pub source_file_name: String,
	pub error_message: Option<String>,
}

// -- XmlImportHistoryBmc

pub struct XmlImportHistoryBmc;

impl XmlImportHistoryBmc {
	/// Record a single XML import audit entry (begins and commits its own transaction).
	pub async fn record(
		mm: &ModelManager,
		ctx: &Ctx,
		uploaded_file_name: &str,
		source_file_name: &str,
		case_id: Option<Uuid>,
		case_number: Option<&str>,
		status: XmlImportHistoryStatus,
		error_message: Option<&str>,
	) -> Result<()> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		set_full_context_dbx_or_rollback(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		dbx.execute(
			sqlx::query(
				"INSERT INTO xml_import_history (
					uploaded_file_name,
					source_file_name,
					case_id,
					case_number,
					status,
					error_message,
					uploaded_by
				) VALUES ($1, $2, $3, $4, $5, $6, $7)",
			)
			.bind(uploaded_file_name)
			.bind(source_file_name)
			.bind(case_id)
			.bind(case_number)
			.bind(status.as_str())
			.bind(error_message)
			.bind(ctx.user_id()),
		)
		.await?;
		dbx.commit_txn().await?;
		Ok(())
	}

	pub async fn list_all_scoped(
		mm: &ModelManager,
		ctx: &Ctx,
		scope: &EnforcedScopeFilter,
		include_unscoped: bool,
	) -> Result<Vec<XmlImportHistoryRow>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await?;
		let scope_where = case_scope_where(1);
		let rows = dbx
			.fetch_all(
				sqlx::query_as::<_, XmlImportHistoryRow>(&format!(
					r#"
					SELECT h.id,
					       h.uploaded_file_name,
					       h.source_file_name,
					       h.case_id,
					       h.case_number,
					       h.status,
					       h.error_message,
					       h.uploaded_by,
					       u.email AS uploader_email,
					       h.uploaded_at
					  FROM xml_import_history h
					  LEFT JOIN users u ON u.id = h.uploaded_by
					  LEFT JOIN cases c ON c.id = h.case_id
					 WHERE (
						(
							h.case_id IS NULL
							AND (h.uploaded_by = $4 OR $5)
						)
						OR (
							h.case_id IS NOT NULL
							AND ({scope_where})
						)
					 )
					 ORDER BY h.uploaded_at DESC, h.created_at DESC
					 LIMIT 200
					"#
				))
				.bind(scope.sender_ids())
				.bind(scope.product_ids())
				.bind(scope.study_ids())
				.bind(ctx.user_id())
				.bind(include_unscoped),
			)
			.await?;
		dbx.commit_txn().await?;
		Ok(rows)
	}

	/// Fetch the error details row for a single import history entry.
	/// Manages its own RLS-scoped read transaction.
	pub async fn get_error_row(
		mm: &ModelManager,
		ctx: &Ctx,
		id: Uuid,
	) -> Result<Option<XmlImportHistoryErrorRow>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await?;
		let row = dbx
			.fetch_optional(
				sqlx::query_as::<_, XmlImportHistoryErrorRow>(
					"SELECT case_id, source_file_name, error_message
					   FROM xml_import_history
					  WHERE id = $1",
				)
				.bind(id),
			)
			.await?;
		dbx.commit_txn().await?;
		Ok(row)
	}
}
