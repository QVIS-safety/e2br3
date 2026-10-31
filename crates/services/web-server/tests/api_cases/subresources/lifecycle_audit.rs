use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::model::audit::AuditLogBmc;
use lib_core::model::ModelManager;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AuditSnapshot {
	pub id: i64,
	pub organization_id: Uuid,
	pub user_id: Uuid,
	pub action: String,
	pub reason: Option<String>,
	pub category: Option<String>,
	pub changed_fields: Option<Value>,
	pub old_values: Option<Value>,
	pub new_values: Option<Value>,
}

pub(super) async fn audit_rows(
	mm: &ModelManager,
	ctx: &Ctx,
	table: &str,
	record_id: Uuid,
) -> Result<Vec<AuditSnapshot>> {
	assert!(
		!table.is_empty()
			&& table.bytes().all(|byte| byte.is_ascii_lowercase()
				|| byte.is_ascii_digit()
				|| byte == b'_'),
		"invalid audit table name"
	);
	let mut rows = AuditLogBmc::list_by_record(ctx, mm, table, record_id).await?;
	rows.sort_by_key(|row| row.id);
	Ok(rows
		.into_iter()
		.map(|row| AuditSnapshot {
			id: row.id,
			organization_id: row.organization_id,
			user_id: row.user_id,
			action: row.action,
			reason: row.reason_for_change,
			category: row.change_category,
			changed_fields: row.changed_fields,
			old_values: row.old_values,
			new_values: row.new_values,
		})
		.collect())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn assert_audit_appended(
	mm: &ModelManager,
	ctx: &Ctx,
	table: &str,
	record_id: Uuid,
	before: &[AuditSnapshot],
	action: &str,
	reason: &str,
	deleted_transition: Option<(bool, bool)>,
	field_transition: Option<(&str, Value, Value)>,
) -> Result<Vec<AuditSnapshot>> {
	let after = audit_rows(mm, ctx, table, record_id).await?;
	let appended = after
		.iter()
		.filter(|row| !before.iter().any(|previous| previous.id == row.id))
		.collect::<Vec<_>>();
	assert!(
		!appended.is_empty(),
		"missing {action} audit for {table}/{record_id}"
	);
	let matching = appended.iter().find(|row| {
		row.action == action
			&& row.reason.as_deref() == Some(reason)
			&& row.user_id == ctx.user_id()
			&& row.organization_id == ctx.organization_id()
			&& row.category.is_none()
	});
	assert!(
		matching.is_some(),
		"missing {action} audit reason for {table}/{record_id}: {appended:?}"
	);
	let matching = matching.ok_or("matching audit row disappeared")?;
	if let Some((old_deleted, new_deleted)) = deleted_transition {
		assert_eq!(
			matching
				.old_values
				.as_ref()
				.and_then(|value| value.get("deleted")),
			Some(&Value::Bool(old_deleted)),
			"missing deleted transition for {table}/{record_id}: {appended:?}"
		);
		assert_eq!(
			matching
				.new_values
				.as_ref()
				.and_then(|value| value.get("deleted")),
			Some(&Value::Bool(new_deleted)),
			"missing deleted transition for {table}/{record_id}: {appended:?}"
		);
		assert_eq!(
			matching
				.changed_fields
				.as_ref()
				.and_then(|value| value.get("deleted"))
				.and_then(|value| value.get("old")),
			Some(&Value::Bool(old_deleted)),
			"missing changed_fields.deleted.old for {table}/{record_id}"
		);
		assert_eq!(
			matching
				.changed_fields
				.as_ref()
				.and_then(|value| value.get("deleted"))
				.and_then(|value| value.get("new")),
			Some(&Value::Bool(new_deleted)),
			"missing changed_fields.deleted.new for {table}/{record_id}"
		);
	}
	if let Some((field, old_value, new_value)) = field_transition {
		assert_eq!(
			matching
				.old_values
				.as_ref()
				.and_then(|value| value.get(field)),
			Some(&old_value),
			"wrong audit old value for {table}.{field}/{record_id}"
		);
		assert_eq!(
			matching
				.new_values
				.as_ref()
				.and_then(|value| value.get(field)),
			Some(&new_value),
			"wrong audit new value for {table}.{field}/{record_id}"
		);
		assert_eq!(
			matching
				.changed_fields
				.as_ref()
				.and_then(|value| value.get(field))
				.and_then(|value| value.get("old")),
			Some(&old_value),
			"wrong changed_fields old value for {table}.{field}/{record_id}"
		);
		assert_eq!(
			matching
				.changed_fields
				.as_ref()
				.and_then(|value| value.get(field))
				.and_then(|value| value.get("new")),
			Some(&new_value),
			"wrong changed_fields new value for {table}.{field}/{record_id}"
		);
	}
	if action == "DELETE" {
		assert!(matching.old_values.is_some());
		assert!(matching.new_values.is_none());
		assert!(matching.changed_fields.is_some());
	}
	Ok(after)
}

pub(super) async fn assert_cascade_delete_audit(
	mm: &ModelManager,
	ctx: &Ctx,
	table: &str,
	record_id: Uuid,
	reason: &str,
) -> Result<()> {
	let rows = audit_rows(mm, ctx, table, record_id).await?;
	assert!(
		rows.iter().any(|row| {
			row.action == "DELETE"
				&& row.reason.as_deref() == Some(reason)
				&& row.user_id == ctx.user_id()
				&& row.organization_id == ctx.organization_id()
				&& row.category.is_none()
				&& row.old_values.is_some()
				&& row.new_values.is_none()
				&& row.changed_fields.is_some()
		}),
		"missing cascaded DELETE audit for {table}/{record_id}: {rows:?}"
	);
	Ok(())
}
