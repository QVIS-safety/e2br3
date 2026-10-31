use super::lifecycle_audit::{assert_audit_appended, audit_rows};
use super::lifecycle_singleton::SingletonLifecycleContract;
use super::lifecycle_support::RowLifecycleContract;
use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;

async fn read_row(
	app: &Router,
	cookie: &str,
	contract: &RowLifecycleContract,
) -> Result<Value> {
	let uri = if contract.item_readable {
		contract.item.clone()
	} else {
		contract.collection.clone()
	};
	let (status, value) = request_json_with_reason(
		app,
		cookie,
		"GET",
		uri,
		None,
		"read child across case delete restore",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{}: {value}", contract.name);
	Ok(value)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn assert_case_soft_parent_preserves_rows(
	app: &Router,
	cookie: &str,
	password: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	case_id: Uuid,
	rows: &[&RowLifecycleContract],
	singletons: &[&SingletonLifecycleContract],
) -> Result<()> {
	let case_audit = audit_rows(mm, ctx, "cases", case_id).await?;
	let mut row_state = Vec::new();
	for contract in rows {
		row_state.push((
			*contract,
			read_row(app, cookie, contract).await?,
			audit_rows(mm, ctx, contract.table, contract.id).await?,
		));
	}
	let mut singleton_state = Vec::new();
	for contract in singletons {
		let (status, value) = request_json_with_reason(
			app,
			cookie,
			"GET",
			contract.path.clone(),
			None,
			"read singleton across case delete restore",
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}: {value}", contract.name);
		singleton_state.push((
			*contract,
			value,
			audit_rows(mm, ctx, contract.table, contract.id).await?,
		));
	}

	let (status, deleted) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		format!("/api/cases/{case_id}"),
		Some(json!({
			"reason_for_change":"verify parent preserves children",
			"e_signature":{"meaning":"delete case","password":password}
		})),
		"verify case parent delete",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{deleted}");
	assert_eq!(deleted["data"]["status"], "deleted");
	let deleted_audit = assert_audit_appended(
		mm,
		ctx,
		"cases",
		case_id,
		&case_audit,
		"UPDATE",
		"verify parent preserves children",
		None,
		Some(("status", json!("draft"), json!("deleted"))),
	)
	.await?;
	assert_eq!(deleted_audit.len(), case_audit.len() + 1);
	let mut audit_tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *audit_tx)
		.await?;
	let signature_ids = sqlx::query_scalar::<_, Option<Uuid>>(
		"SELECT e_signature_id FROM audit_logs WHERE table_name = 'cases' AND record_id = $1 AND action = 'UPDATE' AND changed_fields->'status'->>'old' = 'draft' AND changed_fields->'status'->>'new' = 'deleted' AND reason_for_change = $2",
	)
	.bind(case_id)
	.bind("verify parent preserves children")
	.fetch_all(&mut *audit_tx)
	.await?;
	audit_tx.rollback().await?;
	assert_eq!(signature_ids.len(), 1);
	assert!(signature_ids[0].is_some());

	for (contract, before, audit) in &row_state {
		assert_eq!(
			read_row(app, cookie, contract).await?,
			*before,
			"{}",
			contract.name
		);
		let (status, response) = request_json_with_reason(
			app,
			cookie,
			"PUT",
			contract.item.clone(),
			Some(contract.update.clone()),
			"reject child mutation under deleted case",
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::FORBIDDEN,
			"{}: {response}",
			contract.name
		);
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			*audit
		);
	}
	for (contract, before, audit) in &singleton_state {
		let (status, value) = request_json_with_reason(
			app,
			cookie,
			"GET",
			contract.path.clone(),
			None,
			"read singleton under deleted case",
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}: {value}", contract.name);
		assert_eq!(value, *before, "{}", contract.name);
		let (status, response) = request_json_with_reason(
			app,
			cookie,
			"PUT",
			contract.path.clone(),
			Some(contract.update.clone()),
			"reject singleton mutation under deleted case",
		)
		.await?;
		assert_eq!(
			status,
			StatusCode::FORBIDDEN,
			"{}: {response}",
			contract.name
		);
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			*audit
		);
	}

	let (status, restored) = request_json_with_reason(
		app,
		cookie,
		"POST",
		format!("/api/cases/{case_id}/restore"),
		Some(
			json!({"reason_for_change":"restore parent after child preservation check"}),
		),
		"verify case parent restore",
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{restored}");
	assert_eq!(restored["data"]["status"], "draft");
	let restored_audit = assert_audit_appended(
		mm,
		ctx,
		"cases",
		case_id,
		&deleted_audit,
		"UPDATE",
		"restore parent after child preservation check",
		None,
		Some(("status", json!("deleted"), json!("draft"))),
	)
	.await?;
	assert_eq!(restored_audit.len(), deleted_audit.len() + 1);
	for (contract, before, audit) in row_state {
		assert_eq!(
			read_row(app, cookie, contract).await?,
			before,
			"{}",
			contract.name
		);
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			audit
		);
	}
	for (contract, before, audit) in singleton_state {
		let (status, value) = request_json_with_reason(
			app,
			cookie,
			"GET",
			contract.path.clone(),
			None,
			"read singleton after case restore",
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}: {value}", contract.name);
		assert_eq!(value, before, "{}", contract.name);
		assert_eq!(
			audit_rows(mm, ctx, contract.table, contract.id).await?,
			audit
		);
	}
	Ok(())
}

pub(super) async fn assert_nested_parent_contract(
	app: &Router,
	cookie: &str,
	mm: &ModelManager,
	ctx: &Ctx,
	parent: &RowLifecycleContract,
	children: &[&RowLifecycleContract],
) -> Result<()> {
	let parent_audit = audit_rows(mm, ctx, parent.table, parent.id).await?;
	let mut child_audits = Vec::new();
	let mut child_state = Vec::new();
	for child in children {
		child_audits.push(audit_rows(mm, ctx, child.table, child.id).await?);
		let (status, response) = request_json_with_reason(
			app,
			cookie,
			"GET",
			child.item.clone(),
			None,
			"read child before parent delete",
		)
		.await?;
		assert_eq!(status, StatusCode::OK, "{}: {response}", child.name);
		child_state.push(response);
	}
	let (status, response) = request_json_with_reason(
		app,
		cookie,
		"DELETE",
		parent.item.clone(),
		None,
		"delete nested parent lifecycle",
	)
	.await?;
	assert_eq!(status, parent.delete_status, "{}: {response}", parent.name);
	let deleted_audit = assert_audit_appended(
		mm,
		ctx,
		parent.table,
		parent.id,
		&parent_audit,
		"UPDATE",
		"delete nested parent lifecycle",
		Some((false, true)),
		None,
	)
	.await?;
	assert_eq!(deleted_audit.len(), parent_audit.len() + 1);
	for (child, audit) in children.iter().zip(&child_audits) {
		let (status, _) = request_json_with_reason(
			app,
			cookie,
			"GET",
			child.item.clone(),
			None,
			"child inaccessible under deleted parent",
		)
		.await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{}", child.name);
		assert_eq!(audit_rows(mm, ctx, child.table, child.id).await?, *audit);
	}
	if let Some(restore) = &parent.restore {
		let (status, response) = request_json_with_reason(
			app,
			cookie,
			"POST",
			restore.clone(),
			Some(json!({})),
			"restore nested parent lifecycle",
		)
		.await?;
		assert_eq!(status, parent.restore_status, "{}: {response}", parent.name);
		let restored_audit = assert_audit_appended(
			mm,
			ctx,
			parent.table,
			parent.id,
			&deleted_audit,
			"UPDATE",
			"restore nested parent lifecycle",
			Some((true, false)),
			None,
		)
		.await?;
		assert_eq!(restored_audit.len(), deleted_audit.len() + 1);
		for ((child, before), audit) in
			children.iter().zip(child_state).zip(child_audits)
		{
			let (status, response) = request_json_with_reason(
				app,
				cookie,
				"GET",
				child.item.clone(),
				None,
				"read child after parent restore",
			)
			.await?;
			assert_eq!(status, StatusCode::OK, "{}: {response}", child.name);
			assert_eq!(response, before, "{}", child.name);
			assert_eq!(audit_rows(mm, ctx, child.table, child.id).await?, audit);
		}
	}
	Ok(())
}
