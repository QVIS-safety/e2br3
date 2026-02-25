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
	pub table_name: String,
	pub record_id: Uuid,
	pub action: String, // CREATE, UPDATE, DELETE, SUBMIT, NULLIFY
	pub user_id: Uuid,
	#[sqlx(default)]
	pub reason_for_change: Option<String>,
	#[sqlx(default)]
	pub e_signature_id: Option<Uuid>,
	#[sqlx(default)]
	pub user_display: Option<String>,
	pub old_values: Option<JsonValue>,
	pub new_values: Option<JsonValue>,
	pub ip_address: Option<String>, // Stored as TEXT in DB
	pub user_agent: Option<String>,
	#[serde(with = "time::serde::rfc3339")]
	pub created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct AuditLogForCreate {
	pub table_name: String,
	pub record_id: Uuid,
	pub action: String,
	pub reason_for_change: Option<String>,
	pub e_signature_id: Option<Uuid>,
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

const LIST_LIMIT_DEFAULT: i64 = 1000;
const LIST_LIMIT_MAX: i64 = 5000;

#[derive(Iden)]
enum AuditLogIden {
	Id,
	TableName,
	RecordId,
	Action,
	UserId,
	OldValues,
	NewValues,
	IpAddress,
	UserAgent,
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
		_ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<CaseVersion>> {
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1 ORDER BY version DESC",
			Self::TABLE
		);
		let versions = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, CaseVersion>(&sql).bind(case_id))
			.await?;
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

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		audit_c: AuditLogForCreate,
	) -> Result<i64> {
		let user_id = ctx.user_id();
		let sql = "INSERT INTO audit_logs (table_name, record_id, action, user_id, reason_for_change, e_signature_id, old_values, new_values, ip_address, user_agent) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id";

		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (i64,)>(sql)
					.bind(audit_c.table_name)
					.bind(audit_c.record_id)
					.bind(audit_c.action)
					.bind(user_id)
					.bind(audit_c.reason_for_change)
					.bind(audit_c.e_signature_id)
					.bind(audit_c.old_values)
					.bind(audit_c.new_values)
					.bind(audit_c.ip_address)
					.bind(audit_c.user_agent),
			)
			.await?;

		Ok(id)
	}

	pub async fn list(
		_ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<AuditLogFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<AuditLog>> {
		let mut query = Query::select();
		query
			.from(Self::table_ref())
			.columns([
				AuditLogIden::Id,
				AuditLogIden::TableName,
				AuditLogIden::RecordId,
				AuditLogIden::Action,
				AuditLogIden::UserId,
				AuditLogIden::OldValues,
				AuditLogIden::NewValues,
				AuditLogIden::IpAddress,
				AuditLogIden::UserAgent,
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
		let logs = mm
			.dbx()
			.fetch_all(sqlx::query_as_with::<_, AuditLog, _>(&sql, values))
			.await?;
		Ok(logs
			.into_iter()
			.filter(|log| !Self::is_metadata_only_update(log))
			.collect())
	}

	pub async fn list_by_record(
		_ctx: &Ctx,
		mm: &ModelManager,
		table_name: &str,
		record_id: Uuid,
	) -> Result<Vec<AuditLog>> {
		let logs = if table_name == "cases" {
			let sql = format!(
				"SELECT l.*, audit_user_display(l.user_id) AS user_display
				 FROM {} l
				 WHERE (l.table_name = $1 AND l.record_id = $2)
				    OR COALESCE(l.new_values->>'case_id', l.old_values->>'case_id') = $3
				 ORDER BY l.created_at DESC",
				Self::TABLE
			);
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, AuditLog>(&sql)
						.bind(table_name)
						.bind(record_id)
						.bind(record_id.to_string()),
				)
				.await?
		} else {
			let sql = format!(
				"SELECT l.*, audit_user_display(l.user_id) AS user_display
				 FROM {} l
				 WHERE l.table_name = $1 AND l.record_id = $2
				 ORDER BY l.created_at DESC",
				Self::TABLE
			);
			mm.dbx()
				.fetch_all(
					sqlx::query_as::<_, AuditLog>(&sql)
						.bind(table_name)
						.bind(record_id),
				)
				.await?
		};
		Ok(logs
			.into_iter()
			.filter(|log| !Self::is_metadata_only_update(log))
			.collect())
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
