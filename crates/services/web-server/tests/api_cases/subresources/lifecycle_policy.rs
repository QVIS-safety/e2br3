use super::lifecycle_audit::*;
use super::lifecycle_support::RowLifecycleContract;
use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;

fn row_state_paths(contract: &RowLifecycleContract) -> Vec<String> {
	let mut paths = Vec::with_capacity(2);
	if contract.item_readable {
		paths.push(contract.item.clone());
	}
	paths.push(contract.collection.clone());
	paths
}

pub(super) async fn read_json_state(
	app: &Router,
	cookie: &str,
	paths: &[String],
) -> Result<Vec<(StatusCode, Value)>> {
	let mut state = Vec::with_capacity(paths.len());
	for path in paths {
		state.push(
			request_json_with_reason(
				app,
				cookie,
				"GET",
				path.clone(),
				None,
				"read subresource state",
			)
			.await?,
		);
	}
	Ok(state)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn assert_rejected_mutation_unchanged(
	app: &Router,
	admin_cookie: &str,
	actor_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	table: &str,
	record_id: Uuid,
	state_paths: &[String],
	method: &str,
	uri: String,
	body: Option<Value>,
	expected_status: StatusCode,
	label: &str,
) -> Result<()> {
	let state_before = read_json_state(app, admin_cookie, state_paths).await?;
	let audit_before = audit_rows(mm, ctx, table, record_id).await?;
	let (status, response) = request_json_with_reason(
		app,
		actor_cookie,
		method,
		uri,
		body,
		"rejected subresource mutation",
	)
	.await?;
	assert_eq!(status, expected_status, "{label}: {response}");
	assert_eq!(
		read_json_state(app, admin_cookie, state_paths).await?,
		state_before,
		"{label}: rejected request changed API state"
	);
	assert_eq!(
		audit_rows(mm, ctx, table, record_id).await?,
		audit_before,
		"{label}: rejected request wrote audit rows"
	);
	Ok(())
}

pub(super) async fn assert_row_mutation_policy(
	app: &Router,
	admin_cookie: &str,
	viewer_cookie: &str,
	other_org_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &RowLifecycleContract,
) -> Result<()> {
	let state = row_state_paths(contract);
	for (actor, uri, expected, label) in [
		(
			viewer_cookie,
			contract.item.clone(),
			StatusCode::FORBIDDEN,
			"viewer",
		),
		(
			other_org_cookie,
			contract.item.clone(),
			StatusCode::NOT_FOUND,
			"other organization",
		),
		(
			admin_cookie,
			contract.wrong_item.clone(),
			StatusCode::NOT_FOUND,
			"wrong parent",
		),
	] {
		for (method, operation_uri, body, operation) in [
			("PUT", uri.clone(), Some(contract.update.clone()), "update"),
			("DELETE", uri.clone(), None, "delete"),
		] {
			assert_rejected_mutation_unchanged(
				app,
				admin_cookie,
				actor,
				mm,
				ctx,
				contract.table,
				contract.id,
				&state,
				method,
				operation_uri,
				body,
				expected,
				&format!("{} {label} {operation}", contract.name),
			)
			.await?;
		}
		if let Some(restore) = &contract.restore {
			let restore_uri = if label == "wrong parent" {
				restore.replacen(&contract.item, &contract.wrong_item, 1)
			} else {
				restore.clone()
			};
			assert_rejected_mutation_unchanged(
				app,
				admin_cookie,
				actor,
				mm,
				ctx,
				contract.table,
				contract.id,
				&state,
				"POST",
				restore_uri,
				Some(json!({})),
				expected,
				&format!("{} {label} restore", contract.name),
			)
			.await?;
		}
	}
	Ok(())
}

pub(super) async fn assert_locked_row_mutation(
	app: &Router,
	admin_cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	contract: &RowLifecycleContract,
) -> Result<()> {
	let state = row_state_paths(contract);
	for (method, uri, body, operation) in [
		(
			"PUT",
			contract.item.clone(),
			Some(contract.update.clone()),
			"update",
		),
		("DELETE", contract.item.clone(), None, "delete"),
	] {
		assert_rejected_mutation_unchanged(
			app,
			admin_cookie,
			admin_cookie,
			mm,
			ctx,
			contract.table,
			contract.id,
			&state,
			method,
			uri,
			body,
			StatusCode::FORBIDDEN,
			&format!("{} locked case {operation}", contract.name),
		)
		.await?;
	}
	if let Some(restore) = &contract.restore {
		assert_rejected_mutation_unchanged(
			app,
			admin_cookie,
			admin_cookie,
			mm,
			ctx,
			contract.table,
			contract.id,
			&state,
			"POST",
			restore.clone(),
			Some(json!({})),
			StatusCode::FORBIDDEN,
			&format!("{} locked case restore", contract.name),
		)
		.await?;
	}
	Ok(())
}
