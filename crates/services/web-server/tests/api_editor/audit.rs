pub(super) mod assessment;
pub(super) mod projection;
use super::fields::read_path;
use crate::common::Result;
use lib_core::ctx::Ctx;
use lib_core::model::audit::{AuditLog, AuditLogBmc};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

pub(super) async fn case_audit_ids(
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: &str,
) -> Result<Vec<i64>> {
	let mut ids =
		AuditLogBmc::list_by_record(ctx, mm, "cases", Uuid::parse_str(case_id)?)
			.await?
			.into_iter()
			.map(|log| log.id)
			.collect::<Vec<_>>();
	ids.sort_unstable();
	Ok(ids)
}

pub(super) struct Snapshot {
	logs: HashSet<i64>,
	record_id: Option<Uuid>,
	value: Option<Value>,
}

fn record_id(
	spec: &Value,
	projection: &Value,
	case_id: Uuid,
) -> Result<Option<Uuid>> {
	match spec["record"]["source"].as_str() {
		Some("case") => Ok(Some(case_id)),
		Some("projection") => {
			let path = spec["record"]["path"]
				.as_str()
				.ok_or("missing audit record path")?;
			match read_path(projection, path) {
				None | Some(Value::Null) => Ok(None),
				Some(value) => Ok(Some(Uuid::parse_str(
					value.as_str().ok_or("audit record ID is not a string")?,
				)?)),
			}
		}
		_ => Err("unknown audit record source".into()),
	}
}

async fn stored_value(
	mm: &ModelManager,
	ctx: &Ctx,
	spec: &Value,
	id: Uuid,
) -> Result<Value> {
	let table = spec["table"].as_str().ok_or("missing audit table")?;
	let column = spec["column"].as_str().ok_or("missing audit column")?;
	for identifier in [table, column] {
		assert!(
			!identifier.is_empty()
				&& identifier.bytes().all(|c| c.is_ascii_lowercase()
					|| c.is_ascii_digit()
					|| c == b'_'),
			"invalid contract SQL identifier: {identifier}"
		);
	}
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role())
		.await?;
	let result = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (Value,)>(&format!(
				"SELECT to_jsonb(t) FROM {table} t WHERE id = $1"
			))
			.bind(id),
		)
		.await;
	let row = match result {
		Ok((row,)) => row,
		Err(error) => {
			mm.dbx().rollback_txn().await?;
			return Err(error.into());
		}
	};
	mm.dbx().commit_txn().await?;
	row.get(column)
		.cloned()
		.ok_or_else(|| format!("missing stored column {table}.{column}").into())
}

pub(super) async fn capture(
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
	spec: &Value,
	projection: &Value,
) -> Result<Snapshot> {
	let logs = AuditLogBmc::list_by_record(ctx, mm, "cases", case_id).await?;
	let record_id = record_id(spec, projection, case_id)?;
	let value = match record_id {
		Some(id) => Some(stored_value(mm, ctx, spec, id).await?),
		None => None,
	};
	Ok(Snapshot {
		logs: logs.into_iter().map(|log| log.id).collect(),
		record_id,
		value,
	})
}

pub(super) async fn assert_mutation(
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
	spec: &Value,
	before: Snapshot,
	projection: &Value,
	reason: &str,
) -> Result<()> {
	let id = record_id(spec, projection, case_id)?
		.ok_or("mutation did not expose an audit record ID")?;
	if let Some(previous) = before.record_id {
		assert_eq!(id, previous, "mutation replaced the audited row ID");
	}
	let table = spec["table"].as_str().ok_or("missing audit table")?;
	let column = spec["column"].as_str().ok_or("missing audit column")?;
	let after = stored_value(mm, ctx, spec, id).await?;
	let logs = AuditLogBmc::list_by_record(ctx, mm, "cases", case_id).await?;
	let mut changes: Vec<&AuditLog> = logs
		.iter()
		.filter(|log| {
			!before.logs.contains(&log.id)
				&& log.table_name == table
				&& log.record_id == id
				&& log.new_values.as_ref().and_then(|value| value.get(column))
					!= log.old_values.as_ref().and_then(|value| value.get(column))
		})
		.collect();
	changes.sort_by_key(|log| log.id);
	if before.value.as_ref() == Some(&after) {
		assert!(
			changes.is_empty(),
			"no-op wrote a false field change for {table}.{column}: {changes:?}"
		);
		return Ok(());
	}
	assert!(
		!changes.is_empty(),
		"missing audit change for {table}/{id}/{column}"
	);
	let first = changes.first().ok_or("missing first audit change")?;
	let last = changes.last().ok_or("missing last audit change")?;
	if let Some(old) = before.value {
		assert_eq!(
			first
				.old_values
				.as_ref()
				.and_then(|value| value.get(column)),
			Some(&old),
			"audit old value"
		);
	}
	assert_eq!(
		last.new_values.as_ref().and_then(|value| value.get(column)),
		Some(&after),
		"audit new value"
	);
	for change in changes {
		assert_eq!(change.user_id, ctx.user_id(), "audit actor");
		assert_eq!(
			change.organization_id,
			ctx.organization_id(),
			"audit organization"
		);
		assert_eq!(
			change.reason_for_change.as_deref(),
			Some(reason),
			"audit reason"
		);
	}
	Ok(())
}

pub(super) async fn assert_rejected(
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
	before: Snapshot,
) -> Result<()> {
	let after = AuditLogBmc::list_by_record(ctx, mm, "cases", case_id).await?;
	assert!(
		after.iter().all(|log| before.logs.contains(&log.id)),
		"rejected request wrote a case audit entry"
	);
	Ok(())
}
