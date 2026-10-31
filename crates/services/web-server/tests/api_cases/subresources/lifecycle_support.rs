use super::lifecycle_audit::*;
use super::lifecycle_policy::assert_rejected_mutation_unchanged;
use super::support::*;
pub(super) use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::ModelManager;

pub(super) async fn toggle_case_lock(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
) -> Result<()> {
	let (status, current) = request_json_with_reason(
		app,
		cookie,
		"GET",
		format!("/api/cases/{case_id}"),
		None,
		"read case lock state",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{current}");
	let expected_status = current["data"]["status"]
		.as_str()
		.ok_or("case response is missing status")?;
	let (status, body) = request_json_with_reason(
		app,
		cookie,
		"POST",
		format!("/api/cases/{case_id}/lock/toggle"),
		Some(json!({"data":{"expected_status":expected_status}})),
		"toggle case lock for subresource contract",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	Ok(())
}

#[derive(Clone)]
pub(super) struct RowLifecycleContract {
	pub name: &'static str,
	pub table: &'static str,
	pub id: Uuid,
	pub item: String,
	pub item_readable: bool,
	pub collection: String,
	pub wrong_item: String,
	pub update: Value,
	pub changed_field: &'static str,
	pub changed_value: Value,
	pub delete_status: StatusCode,
	pub repeat_delete_status: StatusCode,
	pub deleted_get_status: StatusCode,
	pub deleted_update_status: StatusCode,
	pub restore: Option<String>,
	pub restore_status: StatusCode,
}

pub(super) async fn create_lifecycle_row(
	app: &Router,
	cookie: &str,
	collection: &str,
	body: Value,
	name: &str,
) -> Result<(Uuid, Value)> {
	let (status, created) = request_json_with_reason(
		app,
		cookie,
		"POST",
		collection.to_string(),
		Some(body),
		&format!("create {name} lifecycle"),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{name}: {created}");
	let id = created["data"]["id"]
		.as_str()
		.ok_or("created row response is missing data.id")?;
	Ok((Uuid::parse_str(id)?, created))
}

fn response_has_id(value: &Value, id: Uuid) -> bool {
	let expected = Value::String(id.to_string());
	match &value["data"] {
		Value::Array(rows) => rows.iter().any(|row| row["id"] == expected),
		Value::Object(_) => value["data"]["id"] == expected,
		_ => false,
	}
}

fn response_id_count(value: &Value, id: Uuid) -> usize {
	let expected = Value::String(id.to_string());
	match &value["data"] {
		Value::Array(rows) => {
			rows.iter().filter(|row| row["id"] == expected).count()
		}
		Value::Object(_) => usize::from(value["data"]["id"] == expected),
		_ => 0,
	}
}

pub(super) async fn assert_row_visible(
	app: &Router,
	cookie: &str,
	contract: &RowLifecycleContract,
) -> Result<Value> {
	let item = if contract.item_readable {
		let (status, item) = request_json_with_reason(
			app,
			cookie,
			"GET",
			contract.item.clone(),
			None,
			"read active lifecycle row",
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}: {item}", contract.name);
		assert_eq!(
			item["data"]["id"],
			contract.id.to_string(),
			"{}",
			contract.name
		);
		Some(item)
	} else {
		None
	};
	let (status, list) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.collection.clone(),
		None,
		"list active lifecycle row",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {list}", contract.name);
	assert_eq!(
		response_id_count(&list, contract.id),
		1,
		"{}: {list}",
		contract.name
	);
	if let Some(item) = item {
		Ok(item)
	} else {
		let row = list["data"]
			.as_array()
			.and_then(|rows| {
				rows.iter().find(|row| row["id"] == contract.id.to_string())
			})
			.cloned()
			.ok_or("active lifecycle row is missing from collection")?;
		Ok(json!({"data":row}))
	}
}

pub(super) async fn assert_row_hidden(
	app: &Router,
	cookie: &str,
	contract: &RowLifecycleContract,
) -> Result<()> {
	if contract.item_readable {
		let (status, item) = request_json_with_reason(
			app,
			cookie,
			"GET",
			contract.item.clone(),
			None,
			"read deleted lifecycle row",
		)
		.await?;
		assert_eq!(
			status, contract.deleted_get_status,
			"{}: {item}",
			contract.name
		);
		if status == StatusCode::OK {
			assert_eq!(item["data"]["id"], contract.id.to_string());
			assert_eq!(item["data"]["deleted"], true);
		}
	}
	let (status, list) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.collection.clone(),
		None,
		"list deleted lifecycle row",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {list}", contract.name);
	assert!(
		!response_has_id(&list, contract.id),
		"{}: {list}",
		contract.name
	);
	Ok(())
}

pub(super) async fn assert_soft_row_lifecycle(
	app: &Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &RowLifecycleContract,
) -> Result<()> {
	let initial = assert_row_visible(app, cookie, contract).await?;
	let old_changed_value = initial["data"][contract.changed_field].clone();
	let update_reason = format!("update {} lifecycle", contract.name);
	let audit_before = audit_rows(mm, ctx, contract.table, contract.id).await?;
	let (status, updated) = request_json_with_reason(
		app,
		cookie,
		"PUT",
		contract.item.clone(),
		Some(contract.update.clone()),
		&update_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {updated}", contract.name);
	assert_eq!(
		updated["data"][contract.changed_field],
		contract.changed_value
	);
	let fresh = assert_row_visible(app, cookie, contract).await?;
	assert_eq!(
		fresh["data"][contract.changed_field],
		contract.changed_value
	);
	let mut audit = assert_audit_appended(
		mm,
		ctx,
		contract.table,
		contract.id,
		&audit_before,
		"UPDATE",
		&update_reason,
		None,
		Some((
			contract.changed_field,
			old_changed_value,
			contract.changed_value.clone(),
		)),
	)
	.await?;
	let no_op_before = audit.clone();
	let (status, repeated) = request_json_with_reason(
		app,
		cookie,
		"PUT",
		contract.item.clone(),
		Some(contract.update.clone()),
		&format!("repeat update {} lifecycle", contract.name),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {repeated}", contract.name);
	assert_eq!(repeated["data"]["id"], contract.id.to_string());
	assert_eq!(
		repeated["data"][contract.changed_field],
		contract.changed_value
	);
	assert_row_visible(app, cookie, contract).await?;
	assert_eq!(
		audit_rows(mm, ctx, contract.table, contract.id).await?,
		no_op_before,
		"{}: identical update wrote an audit row",
		contract.name
	);

	let delete_reason = format!("delete {} lifecycle", contract.name);
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		contract.item.clone(),
		None,
		&delete_reason,
	)
	.await?;
	assert_eq!(
		status, contract.delete_status,
		"{}: {response}",
		contract.name
	);
	assert_row_hidden(app, cookie, contract).await?;
	audit = assert_audit_appended(
		mm,
		ctx,
		contract.table,
		contract.id,
		&audit,
		"UPDATE",
		&delete_reason,
		Some((false, true)),
		None,
	)
	.await?;

	let repeated_before = audit.clone();
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		contract.item.clone(),
		None,
		"repeat deleted lifecycle row",
	)
	.await?;
	assert_eq!(
		status, contract.repeat_delete_status,
		"{}: {response}",
		contract.name
	);
	assert_row_hidden(app, cookie, contract).await?;
	assert_eq!(
		audit_rows(mm, ctx, contract.table, contract.id).await?,
		repeated_before
	);

	if contract.deleted_update_status == StatusCode::NOT_FOUND {
		let mut rejected_update = contract.update.clone();
		let rejected_value =
			Value::String(format!("rejected deleted {} update", contract.name));
		let update_field = rejected_update
			.get_mut("data")
			.and_then(Value::as_object_mut)
			.and_then(|data| data.get_mut(contract.changed_field))
			.ok_or("deleted-row regression payload is missing changed field")?;
		assert_ne!(*update_field, rejected_value);
		*update_field = rejected_value;
		assert_rejected_mutation_unchanged(
			app,
			cookie,
			cookie,
			mm,
			ctx,
			contract.table,
			contract.id,
			&[contract.item.clone(), contract.collection.clone()],
			"PUT",
			contract.item.clone(),
			Some(rejected_update),
			StatusCode::NOT_FOUND,
			&format!("{} deleted row resave", contract.name),
		)
		.await?;
	} else {
		let audit_before_resave = audit.clone();
		let (status, response) = request_json_with_reason(
			app,
			cookie,
			"PUT",
			contract.item.clone(),
			Some(contract.update.clone()),
			&format!("resave deleted {} lifecycle", contract.name),
		)
		.await?;
		assert_eq!(
			status, contract.deleted_update_status,
			"{}: {response}",
			contract.name
		);
		assert_eq!(response["data"]["id"], contract.id.to_string());
		assert_eq!(response["data"]["deleted"], true);
		assert_row_hidden(app, cookie, contract).await?;
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			audit_before_resave,
			"{}: identical deleted-row resave wrote a false audit change",
			contract.name
		);
	}

	if let Some(restore_uri) = &contract.restore {
		let restore_reason = format!("restore {} lifecycle", contract.name);
		let (status, restored) = request_json_with_reason(
			app,
			cookie,
			"POST",
			restore_uri.clone(),
			Some(json!({})),
			&restore_reason,
		)
		.await?;
		assert_eq!(
			status, contract.restore_status,
			"{}: {restored}",
			contract.name
		);
		if status == StatusCode::OK {
			assert_eq!(restored["data"]["id"], contract.id.to_string());
		}
		let visible = assert_row_visible(app, cookie, contract).await?;
		assert_eq!(
			visible["data"][contract.changed_field], contract.changed_value,
			"{}: deleted-row resave changed the restored value",
			contract.name
		);
		audit = assert_audit_appended(
			mm,
			ctx,
			contract.table,
			contract.id,
			&audit,
			"UPDATE",
			&restore_reason,
			Some((true, false)),
			None,
		)
		.await?;
		let (status, restored) = request_json_with_reason(
			app,
			cookie,
			"POST",
			restore_uri.clone(),
			Some(json!({})),
			"repeat restored lifecycle row",
		)
		.await?;
		assert_eq!(
			status, contract.restore_status,
			"{}: {restored}",
			contract.name
		);
		if status == StatusCode::OK {
			assert_eq!(restored["data"]["id"], contract.id.to_string());
		}
		assert_row_visible(app, cookie, contract).await?;
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			audit
		);
	}
	Ok(())
}

pub(super) async fn assert_hard_row_lifecycle(
	app: &Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &RowLifecycleContract,
	recreate_body: Value,
) -> Result<Uuid> {
	let initial = assert_row_visible(app, cookie, contract).await?;
	let old_changed_value = initial["data"][contract.changed_field].clone();
	let update_reason = format!("update {} lifecycle", contract.name);
	let before = audit_rows(mm, ctx, contract.table, contract.id).await?;
	let (status, updated) = request_json_with_reason(
		app,
		cookie,
		"PUT",
		contract.item.clone(),
		Some(contract.update.clone()),
		&update_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {updated}", contract.name);
	assert_eq!(
		updated["data"][contract.changed_field],
		contract.changed_value
	);
	let fresh = assert_row_visible(app, cookie, contract).await?;
	assert_eq!(
		fresh["data"][contract.changed_field],
		contract.changed_value
	);
	let audit = assert_audit_appended(
		mm,
		ctx,
		contract.table,
		contract.id,
		&before,
		"UPDATE",
		&update_reason,
		None,
		Some((
			contract.changed_field,
			old_changed_value,
			contract.changed_value.clone(),
		)),
	)
	.await?;
	let no_op_before = audit.clone();
	let (status, repeated) = request_json_with_reason(
		app,
		cookie,
		"PUT",
		contract.item.clone(),
		Some(contract.update.clone()),
		&format!("repeat update {} lifecycle", contract.name),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {repeated}", contract.name);
	assert_eq!(repeated["data"]["id"], contract.id.to_string());
	assert_eq!(
		repeated["data"][contract.changed_field],
		contract.changed_value
	);
	assert_row_visible(app, cookie, contract).await?;
	assert_eq!(
		audit_rows(mm, ctx, contract.table, contract.id).await?,
		no_op_before,
		"{}: identical update wrote an audit row",
		contract.name
	);

	let delete_reason = format!("delete {} lifecycle", contract.name);
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		contract.item.clone(),
		None,
		&delete_reason,
	)
	.await?;
	assert_eq!(
		status, contract.delete_status,
		"{}: {response}",
		contract.name
	);
	assert_row_hidden(app, cookie, contract).await?;
	let audit = assert_audit_appended(
		mm,
		ctx,
		contract.table,
		contract.id,
		&audit,
		"DELETE",
		&delete_reason,
		None,
		None,
	)
	.await?;
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		contract.item.clone(),
		None,
		"repeat hard deleted lifecycle row",
	)
	.await?;
	assert_eq!(
		status, contract.repeat_delete_status,
		"{}: {response}",
		contract.name
	);
	assert_row_hidden(app, cookie, contract).await?;
	assert_eq!(
		audit_rows(mm, ctx, contract.table, contract.id).await?,
		audit
	);

	let (status, recreated) = request_json_with_reason(
		app,
		cookie,
		"POST",
		contract.collection.clone(),
		Some(recreate_body),
		&format!("recreate {} lifecycle", contract.name),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}: {recreated}",
		contract.name
	);
	let recreated_id = recreated["data"]["id"]
		.as_str()
		.ok_or("recreated row response is missing data.id")?;
	let recreated_id = Uuid::parse_str(recreated_id)?;
	assert_ne!(
		recreated_id, contract.id,
		"{} reused a hard-deleted ID",
		contract.name
	);
	let (status, fresh) = request_json_with_reason(
		app,
		cookie,
		"GET",
		if contract.item_readable {
			format!("{}/{recreated_id}", contract.collection)
		} else {
			contract.collection.clone()
		},
		None,
		"read recreated lifecycle row",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {fresh}", contract.name);
	assert_eq!(
		response_id_count(&fresh, recreated_id),
		1,
		"{}: {fresh}",
		contract.name
	);
	Ok(recreated_id)
}
