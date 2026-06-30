// Audit Logs and Case Versions

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::store::set_full_context_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::filter::{FilterNodes, ListOptions, OpValsString};
use sea_query::{Alias, Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- CaseVersion

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CaseVersion {
	pub id: Uuid,
	pub case_id: Uuid,
	pub version: i32,
	pub snapshot: JsonValue, // Full case data snapshot
	pub changed_by: Uuid,
	pub change_reason: Option<String>,
	pub created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct CaseVersionForCreate {
	pub case_id: Uuid,
	pub version: i32,
	pub snapshot: JsonValue,
	pub change_reason: Option<String>,
}

// -- AuditLog

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AuditLog {
	pub id: i64,
	pub organization_id: Uuid,
	pub table_name: String,
	pub record_id: Uuid,
	pub action: String, // CREATE, UPDATE, DELETE, SUBMIT, NULLIFY
	pub user_id: Uuid,
	#[sqlx(default)]
	pub reason_for_change: Option<String>,
	#[sqlx(default)]
	pub change_category: Option<String>,
	#[sqlx(default)]
	pub e_signature_id: Option<Uuid>,
	#[sqlx(default)]
	pub user_display: Option<String>,
	#[sqlx(default)]
	pub changed_fields: Option<JsonValue>,
	pub old_values: Option<JsonValue>,
	pub new_values: Option<JsonValue>,
	pub ip_address: Option<String>, // Stored as TEXT in DB
	pub user_agent: Option<String>,
	#[sqlx(default)]
	pub prev_hash: Option<String>,
	#[sqlx(default)]
	pub entry_hash: Option<String>,
	#[serde(with = "time::serde::rfc3339")]
	pub created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct AuditLogForCreate {
	pub table_name: String,
	pub record_id: Uuid,
	pub action: String,
	pub reason_for_change: Option<String>,
	pub change_category: Option<String>,
	pub e_signature_id: Option<Uuid>,
	pub changed_fields: Option<JsonValue>,
	pub old_values: Option<JsonValue>,
	pub new_values: Option<JsonValue>,
	pub ip_address: Option<String>, // Stored as TEXT in DB
	pub user_agent: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct AuditLogFilter {
	pub table_name: Option<OpValsString>,
	pub action: Option<OpValsString>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditChainVerificationReport {
	pub total_rows: i64,
	pub verified_ok_rows: i64,
	pub broken_rows: i64,
	pub first_broken_id: Option<i64>,
	pub first_broken_reason: Option<String>,
	#[serde(with = "time::serde::rfc3339")]
	pub checked_at: OffsetDateTime,
}

const LIST_LIMIT_DEFAULT: i64 = 1000;
const LIST_LIMIT_MAX: i64 = 5000;

#[derive(Iden)]
enum AuditLogIden {
	Id,
	OrganizationId,
	TableName,
	RecordId,
	Action,
	UserId,
	ReasonForChange,
	ChangeCategory,
	ESignatureId,
	ChangedFields,
	OldValues,
	NewValues,
	IpAddress,
	UserAgent,
	PrevHash,
	EntryHash,
	CreatedAt,
}

// -- BMCs

pub struct CaseVersionBmc;
impl DbBmc for CaseVersionBmc {
	const TABLE: &'static str = "case_versions";
}

impl CaseVersionBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		version_c: CaseVersionForCreate,
	) -> Result<Uuid> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
		let user_id = ctx.user_id();
		let sql = "INSERT INTO case_versions (case_id, version, snapshot, change_reason, changed_by) VALUES ($1, $2, $3, $4, $5) RETURNING id";

		let res = dbx
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(sql)
					.bind(version_c.case_id)
					.bind(version_c.version)
					.bind(version_c.snapshot)
					.bind(version_c.change_reason)
					.bind(user_id),
			)
			.await;
		let (id,) = match res {
			Ok(val) => val,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;

		Ok(id)
	}

	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<CaseVersion>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1 ORDER BY version DESC",
			Self::TABLE
		);
		let versions = match dbx
			.fetch_all(sqlx::query_as::<_, CaseVersion>(&sql).bind(case_id))
			.await
		{
			Ok(versions) => versions,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;
		Ok(versions)
	}
}

pub struct AuditLogBmc;
impl DbBmc for AuditLogBmc {
	const TABLE: &'static str = "audit_logs";
}

impl AuditLogBmc {
	fn is_hex_hash64(value: &str) -> bool {
		value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
	}

	fn is_metadata_only_update(log: &AuditLog) -> bool {
		if log.action != "UPDATE" {
			return false;
		}

		let Some(mut old_values) = log.old_values.clone() else {
			return false;
		};
		let Some(mut new_values) = log.new_values.clone() else {
			return false;
		};

		let JsonValue::Object(ref mut old_obj) = old_values else {
			return false;
		};
		let JsonValue::Object(ref mut new_obj) = new_values else {
			return false;
		};

		old_obj.remove("updated_at");
		old_obj.remove("updated_by");
		new_obj.remove("updated_at");
		new_obj.remove("updated_by");

		old_values == new_values
	}

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		audit_c: AuditLogForCreate,
	) -> Result<i64> {
		set_full_context_dbx(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let user_id = ctx.user_id();
		let organization_id = ctx.organization_id();
		let sql = "INSERT INTO audit_logs (table_name, record_id, organization_id, action, user_id, reason_for_change, change_category, e_signature_id, changed_fields, old_values, new_values, ip_address, user_agent) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id";

		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (i64,)>(sql)
					.bind(audit_c.table_name)
					.bind(audit_c.record_id)
					.bind(organization_id)
					.bind(audit_c.action)
					.bind(user_id)
					.bind(audit_c.reason_for_change)
					.bind(audit_c.change_category)
					.bind(audit_c.e_signature_id)
					.bind(audit_c.changed_fields)
					.bind(audit_c.old_values)
					.bind(audit_c.new_values)
					.bind(audit_c.ip_address)
					.bind(audit_c.user_agent),
			)
			.await?;

		Ok(id)
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<AuditLogFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<AuditLog>> {
		let mut query = Query::select();
		query
			.from(Self::table_ref())
			.columns([
				AuditLogIden::Id,
				AuditLogIden::OrganizationId,
				AuditLogIden::TableName,
				AuditLogIden::RecordId,
				AuditLogIden::Action,
				AuditLogIden::UserId,
				AuditLogIden::ReasonForChange,
				AuditLogIden::ChangeCategory,
				AuditLogIden::ESignatureId,
				AuditLogIden::ChangedFields,
				AuditLogIden::OldValues,
				AuditLogIden::NewValues,
				AuditLogIden::IpAddress,
				AuditLogIden::UserAgent,
				AuditLogIden::PrevHash,
				AuditLogIden::EntryHash,
				AuditLogIden::CreatedAt,
			])
			.expr_as(
				Expr::cust("audit_user_display(user_id)"),
				Alias::new("user_display"),
			);

		if let Some(filters) = filters {
			let filters: modql::filter::FilterGroups = filters.into();
			let cond: sea_query::Condition = filters.try_into()?;
			query.cond_where(cond);
		}

		let list_options = compute_list_options(list_options)?;
		list_options.apply_to_sea_query(&mut query);

		let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
		let logs = match dbx
			.fetch_all(sqlx::query_as_with::<_, AuditLog, _>(&sql, values))
			.await
		{
			Ok(logs) => logs,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;
		Ok(logs
			.into_iter()
			.filter(|log| !Self::is_metadata_only_update(log))
			.collect())
	}

	pub async fn list_by_record(
		ctx: &Ctx,
		mm: &ModelManager,
		table_name: &str,
		record_id: Uuid,
	) -> Result<Vec<AuditLog>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
		let logs = if table_name == "cases" {
			let sql = format!(
				"SELECT l.*, audit_user_display(l.user_id) AS user_display
				 FROM {} l
				 WHERE (l.table_name = $1 AND l.record_id = $2)
				    OR COALESCE(l.new_values->>'case_id', l.old_values->>'case_id') = $3
				 ORDER BY l.created_at DESC",
				Self::TABLE
			);
			match dbx
				.fetch_all(
					sqlx::query_as::<_, AuditLog>(&sql)
						.bind(table_name)
						.bind(record_id)
						.bind(record_id.to_string()),
				)
				.await
			{
				Ok(logs) => logs,
				Err(err) => {
					dbx.rollback_txn().await?;
					return Err(err.into());
				}
			}
		} else {
			let sql = format!(
				"SELECT l.*, audit_user_display(l.user_id) AS user_display
				 FROM {} l
				 WHERE l.table_name = $1 AND l.record_id = $2
				 ORDER BY l.created_at DESC",
				Self::TABLE
			);
			match dbx
				.fetch_all(
					sqlx::query_as::<_, AuditLog>(&sql)
						.bind(table_name)
						.bind(record_id),
				)
				.await
			{
				Ok(logs) => logs,
				Err(err) => {
					dbx.rollback_txn().await?;
					return Err(err.into());
				}
			}
		};
		dbx.commit_txn().await?;
		Ok(logs
			.into_iter()
			.filter(|log| !Self::is_metadata_only_update(log))
			.collect())
	}

	pub async fn verify_hash_chain(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<AuditChainVerificationReport> {
		Self::verify_hash_chain_since(ctx, mm, None).await
	}

	pub async fn verify_hash_chain_since(
		ctx: &Ctx,
		mm: &ModelManager,
		since_id: Option<i64>,
	) -> Result<AuditChainVerificationReport> {
		#[derive(Debug, FromRow)]
		struct ChainRow {
			id: i64,
			prev_hash: Option<String>,
			entry_hash: Option<String>,
			expected_prev_hash: Option<String>,
			expected_entry_hash: String,
		}

		let sql = r#"
			WITH chain AS (
				SELECT
					id,
					prev_hash,
					entry_hash,
					LAG(entry_hash) OVER (ORDER BY id ASC) AS expected_prev_hash,
					encode(
						digest(
							concat_ws(
								'|',
								COALESCE(id::TEXT, ''),
								COALESCE(prev_hash, ''),
								table_name,
								record_id::TEXT,
								action,
								user_id::TEXT,
								COALESCE(reason_for_change, ''),
								COALESCE(change_category, ''),
								COALESCE(e_signature_id::TEXT, ''),
								COALESCE(old_values::TEXT, 'null'),
								COALESCE(new_values::TEXT, 'null'),
								COALESCE(changed_fields::TEXT, 'null'),
								COALESCE(ip_address::TEXT, ''),
								COALESCE(user_agent, ''),
								to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
							),
							'sha256'
						),
						'hex'
					) AS expected_entry_hash
				FROM audit_logs
			)
			SELECT id, prev_hash, entry_hash, expected_prev_hash, expected_entry_hash
			FROM chain
			WHERE ($1::BIGINT IS NULL OR id >= $1)
			ORDER BY id ASC
		"#;

		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_dbx(
			dbx,
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await
		{
			dbx.rollback_txn().await?;
			return Err(err.into());
		}
		let rows: Vec<ChainRow> =
			match dbx.fetch_all(sqlx::query_as(sql).bind(since_id)).await {
				Ok(rows) => rows,
				Err(err) => {
					dbx.rollback_txn().await?;
					return Err(err.into());
				}
			};
		dbx.commit_txn().await?;
		let mut broken_rows = 0_i64;
		let mut first_broken_id = None;
		let mut first_broken_reason = None;

		for row in &rows {
			let expected_prev = row
				.expected_prev_hash
				.clone()
				.unwrap_or_else(|| "0".repeat(64));
			let prev_hash = row.prev_hash.as_deref().unwrap_or("");
			let entry_hash = row.entry_hash.as_deref().unwrap_or("");

			let reason = if !Self::is_hex_hash64(prev_hash) {
				Some("prev_hash is not a 64-char hex value".to_string())
			} else if !Self::is_hex_hash64(entry_hash) {
				Some("entry_hash is not a 64-char hex value".to_string())
			} else if prev_hash != expected_prev {
				Some("prev_hash does not match previous entry_hash".to_string())
			} else if entry_hash != row.expected_entry_hash {
				Some("entry_hash does not match recomputed payload hash".to_string())
			} else {
				None
			};

			if let Some(reason) = reason {
				broken_rows += 1;
				if first_broken_id.is_none() {
					first_broken_id = Some(row.id);
					first_broken_reason = Some(reason);
				}
			}
		}

		let total_rows = rows.len() as i64;
		Ok(AuditChainVerificationReport {
			total_rows,
			verified_ok_rows: total_rows - broken_rows,
			broken_rows,
			first_broken_id,
			first_broken_reason,
			checked_at: OffsetDateTime::now_utc(),
		})
	}
}

fn compute_list_options(list_options: Option<ListOptions>) -> Result<ListOptions> {
	if let Some(mut list_options) = list_options {
		if let Some(limit) = list_options.limit {
			if limit > LIST_LIMIT_MAX {
				return Err(crate::model::Error::ListLimitOverMax {
					max: LIST_LIMIT_MAX,
					actual: limit,
				});
			}
		} else {
			list_options.limit = Some(LIST_LIMIT_DEFAULT);
		}
		Ok(list_options)
	} else {
		Ok(ListOptions {
			limit: Some(LIST_LIMIT_DEFAULT),
			offset: None,
			order_bys: Some("!created_at".into()),
		})
	}
}
