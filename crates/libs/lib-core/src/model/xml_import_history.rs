// XML Import History BMC — manages the `xml_import_history` table.
//
// Write operations (record) handle their own transaction and RLS context setup.
// Read operations handle their own transaction internally since the import
// list uses a manual RLS read (not the with_rls_read helper).

use crate::ctx::Ctx;
use crate::model::store::{set_full_context_dbx, set_full_context_dbx_or_rollback};
use crate::model::ModelManager;
use crate::model::Result;
use sqlx::types::time::OffsetDateTime;
use sqlx::FromRow;
use uuid::Uuid;

// -- Types

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
		status: &str,
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
			.bind(status)
			.bind(error_message)
			.bind(ctx.user_id()),
		)
		.await?;
		dbx.commit_txn().await?;
		Ok(())
	}

	/// List all import history entries visible to the current user (newest first, limit 200).
	/// Manages its own RLS-scoped read transaction.
	pub async fn list_all(
		mm: &ModelManager,
		ctx: &Ctx,
	) -> Result<Vec<XmlImportHistoryRow>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		set_full_context_dbx(dbx, ctx.user_id(), ctx.organization_id(), ctx.role())
			.await?;
		let rows = dbx
			.fetch_all(sqlx::query_as::<_, XmlImportHistoryRow>(
				"SELECT h.id,
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
				  ORDER BY h.uploaded_at DESC, h.created_at DESC
				  LIMIT 200",
			))
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
