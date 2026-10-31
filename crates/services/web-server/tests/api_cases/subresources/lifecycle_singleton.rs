use super::lifecycle_audit::*;
use super::lifecycle_policy::*;
use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;

#[derive(Clone)]
pub(super) struct SingletonLifecycleContract {
	pub name: &'static str,
	pub table: &'static str,
	pub id: Uuid,
	pub path: String,
	pub update: Value,
	pub changed_field: &'static str,
	pub changed_value: Value,
	pub create: Value,
	pub delete_status: StatusCode,
	pub deleted_get_status: StatusCode,
}

pub(super) async fn assert_singleton_policy(
	app: &Router,
	admin_cookie: &str,
	viewer_cookie: &str,
	other_org_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &SingletonLifecycleContract,
) -> Result<()> {
	for (actor, expected, label) in [
		(viewer_cookie, StatusCode::FORBIDDEN, "viewer"),
		(
			other_org_cookie,
			StatusCode::NOT_FOUND,
			"other organization",
		),
	] {
		let mutations = [
			("PUT", Some(contract.update.clone()), "update"),
			("DELETE", None, "delete"),
			("POST", Some(contract.create.clone()), "create"),
		];
		for (method, body, operation) in mutations {
			assert_rejected_mutation_unchanged(
				app,
				admin_cookie,
				actor,
				mm,
				ctx,
				contract.table,
				contract.id,
				std::slice::from_ref(&contract.path),
				method,
				contract.path.clone(),
				body,
				expected,
				&format!("{} {label} {operation}", contract.name),
			)
			.await?;
		}
	}
	Ok(())
}

pub(super) async fn assert_locked_singleton(
	app: &Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &SingletonLifecycleContract,
) -> Result<()> {
	for (method, body, operation) in [
		("PUT", Some(contract.update.clone()), "update"),
		("DELETE", None, "delete"),
		("POST", Some(contract.create.clone()), "create"),
	] {
		assert_rejected_mutation_unchanged(
			app,
			cookie,
			cookie,
			mm,
			ctx,
			contract.table,
			contract.id,
			std::slice::from_ref(&contract.path),
			method,
			contract.path.clone(),
			body,
			StatusCode::FORBIDDEN,
			&format!("{} locked case {operation}", contract.name),
		)
		.await?;
	}
	Ok(())
}

pub(super) async fn assert_hard_singleton_lifecycle(
	app: &Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &SingletonLifecycleContract,
) -> Result<Uuid> {
	let (status, initial) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.path.clone(),
		None,
		"read singleton lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {initial}", contract.name);
	assert_eq!(initial["data"]["id"], contract.id.to_string());
	let old_changed_value = initial["data"][contract.changed_field].clone();
	let update_reason = format!("update {} lifecycle", contract.name);
	let before = audit_rows(mm, ctx, contract.table, contract.id).await?;
	let (status, updated) = request_json_with_reason(
		app,
		cookie,
		"PUT",
		contract.path.clone(),
		Some(contract.update.clone()),
		&update_reason,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {updated}", contract.name);
	assert_eq!(
		updated["data"][contract.changed_field],
		contract.changed_value
	);
	let (status, fresh) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.path.clone(),
		None,
		"read updated singleton lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {fresh}", contract.name);
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
		contract.path.clone(),
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
		contract.path.clone(),
		None,
		&delete_reason,
	)
	.await?;
	assert_eq!(
		status, contract.delete_status,
		"{}: {response}",
		contract.name
	);
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.path.clone(),
		None,
		"read deleted singleton lifecycle",
	)
	.await?;
	assert_eq!(
		status, contract.deleted_get_status,
		"{}: {response}",
		contract.name
	);
	if status == StatusCode::OK {
		assert!(response["data"].is_null(), "{}: {response}", contract.name);
	}
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
		contract.path.clone(),
		None,
		"repeat hard deleted singleton",
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::NOT_FOUND,
		"{}: {response}",
		contract.name
	);
	assert_eq!(
		audit_rows(mm, ctx, contract.table, contract.id).await?,
		audit
	);

	let (status, recreated) = request_json_with_reason(
		app,
		cookie,
		"POST",
		contract.path.clone(),
		Some(contract.create.clone()),
		&format!("recreate {} lifecycle", contract.name),
	)
	.await?;
	assert_eq!(
		status,
		StatusCode::CREATED,
		"{}: {recreated}",
		contract.name
	);
	let recreated_id = Uuid::parse_str(
		recreated["data"]["id"]
			.as_str()
			.ok_or("recreated singleton is missing data.id")?,
	)?;
	assert_ne!(recreated_id, contract.id);
	let (status, fresh) = request_json_with_reason(
		app,
		cookie,
		"GET",
		contract.path.clone(),
		None,
		"read recreated singleton lifecycle",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {fresh}", contract.name);
	assert_eq!(fresh["data"]["id"], recreated_id.to_string());
	Ok(recreated_id)
}
