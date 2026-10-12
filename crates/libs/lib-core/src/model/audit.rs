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

#[derive(FilterNodes, Deserialize, Default)]
pub struct AuditLogFilter {
	pub table_name: Option<OpValsString>,
	pub action: Option<OpValsString>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
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
			return Err(err);
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
			return Err(err);
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
			return Err(err);
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
			return Err(err);
		}
		let logs = if table_name == "cases" {
			let sql = format!(
				"WITH RECURSIVE case_org AS (
					SELECT DISTINCT organization_id
					  FROM {0}
					 WHERE table_name = 'cases'
					   AND record_id = $1
				 ),
				 records(record_id) AS (
					VALUES ($1::uuid)
					UNION
					SELECT child.record_id
					  FROM {0} child
					 CROSS JOIN case_org
					  JOIN records parent ON child.parent_record_ids
						@> ARRAY[parent.record_id]
					 WHERE child.organization_id = case_org.organization_id
				 )
				 SELECT l.*, audit_user_display(l.user_id) AS user_display
				   FROM {0} l
				  CROSS JOIN case_org
				  WHERE l.organization_id = case_org.organization_id
				    AND l.record_id IN (SELECT record_id FROM records)
				  ORDER BY l.created_at DESC",
				Self::TABLE
			);
			match dbx
				.fetch_all(sqlx::query_as::<_, AuditLog>(&sql).bind(record_id))
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
			return Err(err);
		}
		let report = match dbx
			.fetch_one(
				sqlx::query_as::<_, AuditChainVerificationReport>(
					"SELECT * FROM verify_audit_log_hash_chain($1)",
				)
				.bind(since_id),
			)
			.await
		{
			Ok(report) => report,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;
		Ok(report)
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
