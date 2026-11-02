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
	let appended = deleted_audit
		.iter()
		.filter(|row| !case_audit.iter().any(|before| before.id == row.id))
		.collect::<Vec<_>>();
	assert_eq!(
		appended.len(),
		2,
		"unexpected recursive Case delete audit: {appended:?}"
	);
	let case_status_audit = appended
		.iter()
		.find(|row| row.table_name == "cases")
		.ok_or("missing Case status audit")?;
	assert_eq!(case_status_audit.record_id, case_id);
	assert_eq!(case_status_audit.action, "UPDATE");
	let signature_id = case_status_audit
		.e_signature_id
		.ok_or("Case status audit is missing its e-signature")?;
	let signature_audit = appended
		.iter()
		.find(|row| row.table_name == "e_signatures")
		.ok_or("missing e-signature audit")?;
	assert_eq!(signature_audit.record_id, signature_id);
	assert_eq!(signature_audit.action, "CREATE");
	let appended_ids = deleted_audit
		.iter()
		.filter(|row| !case_audit.iter().any(|previous| previous.id == row.id))
		.map(|row| row.id)
		.collect::<Vec<_>>();
	let mut audit_tx = mm.dbx().db().begin().await?;
	sqlx::query("SET ROLE e2br3_auditor_role")
		.execute(&mut *audit_tx)
		.await?;
	let appended_kinds = sqlx::query_as::<_, (String, String)>(
		"SELECT table_name, action FROM audit_logs WHERE id = ANY($1) ORDER BY table_name",
	)
	.bind(&appended_ids)
	.fetch_all(&mut *audit_tx)
	.await?;
	assert_eq!(
		appended_kinds,
		vec![
			("cases".to_string(), "UPDATE".to_string()),
			("e_signatures".to_string(), "CREATE".to_string()),
		]
	);
	let case_signature_ids = sqlx::query_scalar::<_, Option<Uuid>>(
		"SELECT e_signature_id FROM audit_logs WHERE id = ANY($1) AND table_name = 'cases' AND record_id = $2 AND action = 'UPDATE' AND changed_fields->'status'->>'old' = 'draft' AND changed_fields->'status'->>'new' = 'deleted' AND reason_for_change = $3",
	)
	.bind(&appended_ids)
	.bind(case_id)
	.bind("verify parent preserves children")
	.fetch_all(&mut *audit_tx)
	.await?;
	assert_eq!(case_signature_ids, vec![Some(signature_id)]);
	let signature_audit_values = sqlx::query_as::<_, (Uuid, Option<String>, String, String, String)>(
		"SELECT record_id, reason_for_change, new_values->>'case_id', new_values->>'action', new_values->>'reason' FROM audit_logs WHERE id = ANY($1) AND table_name = 'e_signatures' AND action = 'CREATE'",
	)
	.bind(&appended_ids)
	.fetch_one(&mut *audit_tx)
	.await?;
	audit_tx.rollback().await?;
	assert_eq!(signature_audit_values.0, signature_id);
	assert_eq!(
		signature_audit_values.1.as_deref(),
		Some("verify case parent delete")
	);
	assert_eq!(signature_audit_values.2, case_id.to_string());
	assert_eq!(signature_audit_values.3, "CASE_STATUS_TRANSITION");
	assert_eq!(signature_audit_values.4, "verify parent preserves children");

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
