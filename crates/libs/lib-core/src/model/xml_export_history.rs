// XML Export History BMC — manages the `xml_export_history` table.
//
// Write operations (record) handle their own transaction and RLS context setup.
// Read operations accept a `&Dbx` because callers embed them inside an
// existing RLS-scoped read transaction via `lib_rest_core::with_rls_read`.

use crate::ctx::Ctx;
use crate::model::store::dbx::Dbx;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use uuid::Uuid;

// -- Types

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct XmlExportHistoryRecord {
	pub id: Uuid,
	pub case_id: Uuid,
	pub case_number: Option<String>,
	pub file_name: String,
	pub status: String,
	pub error_message: Option<String>,
	pub validation_profile: Option<String>,
	pub exported_by: Uuid,
	pub exporter_email: Option<String>,
	pub exported_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
pub struct XmlExportHistoryErrorRow {
	pub case_id: Uuid,
	pub file_name: String,
	pub error_message: Option<String>,
}

// -- XmlExportHistoryBmc

pub struct XmlExportHistoryBmc;

impl XmlExportHistoryBmc {
	/// Record a single XML export audit entry (begins and commits its own transaction).
	pub async fn record(
		mm: &ModelManager,
		ctx: &Ctx,
		case_id: Uuid,
		case_number: Option<&str>,
		file_name: &str,
		validation_profile: Option<&str>,
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
				"INSERT INTO xml_export_history (
					case_id,
					case_number,
					file_name,
					status,
					error_message,
					validation_profile,
					exported_by
				) VALUES ($1, $2, $3, $4, $5, $6, $7)",
			)
			.bind(case_id)
			.bind(case_number)
			.bind(file_name)
			.bind(status)
			.bind(error_message)
			.bind(validation_profile)
			.bind(ctx.user_id()),
		)
		.await?;
		dbx.commit_txn().await?;
		Ok(())
	}

	/// List all export history entries visible to the current user (newest first, limit 200).
	/// Must be called from inside an RLS-scoped read context (e.g. `with_rls_read`).
	pub async fn list_all(dbx: &Dbx) -> Result<Vec<XmlExportHistoryRecord>> {
		dbx.fetch_all(sqlx::query_as::<_, XmlExportHistoryRecord>(
			"SELECT h.id,
			        h.case_id,
			        h.case_number,
			        h.file_name,
			        h.status,
			        h.error_message,
			        h.validation_profile,
			        h.exported_by,
			        u.email AS exporter_email,
			        h.exported_at
			   FROM xml_export_history h
			   LEFT JOIN users u ON u.id = h.exported_by
			  ORDER BY h.exported_at DESC, h.created_at DESC
			  LIMIT 200",
		))
		.await
		.map_err(crate::model::Error::from)
	}

	/// List export history entries for a specific case (newest first, limit 200).
	/// Must be called from inside an RLS-scoped read context.
	pub async fn list_by_case(
		dbx: &Dbx,
		case_id: Uuid,
	) -> Result<Vec<XmlExportHistoryRecord>> {
		dbx.fetch_all(
			sqlx::query_as::<_, XmlExportHistoryRecord>(
				"SELECT h.id,
				        h.case_id,
				        h.case_number,
				        h.file_name,
				        h.status,
				        h.error_message,
				        h.validation_profile,
				        h.exported_by,
				        u.email AS exporter_email,
				        h.exported_at
				   FROM xml_export_history h
				   LEFT JOIN users u ON u.id = h.exported_by
				  WHERE h.case_id = $1
				  ORDER BY h.exported_at DESC, h.created_at DESC
				  LIMIT 200",
			)
			.bind(case_id),
		)
		.await
		.map_err(crate::model::Error::from)
	}

	/// Fetch the error details row for a single export history entry.
	/// Must be called from inside an RLS-scoped read context.
	pub async fn get_error_row(
		dbx: &Dbx,
		id: Uuid,
	) -> Result<Option<XmlExportHistoryErrorRow>> {
		dbx.fetch_optional(
			sqlx::query_as::<_, XmlExportHistoryErrorRow>(
				"SELECT case_id, file_name, error_message
				   FROM xml_export_history
				  WHERE id = $1",
			)
			.bind(id),
		)
		.await
		.map_err(crate::model::Error::from)
	}
}
