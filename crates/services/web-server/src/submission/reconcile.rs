use super::*;

pub async fn reconcile_due_submissions(
	mm: &ModelManager,
	limit: i64,
) -> Result<SubmissionReconcileResult> {
	let scan_mm = mm.clone();
	let mm = &scan_mm;
	let safe_limit = limit.clamp(1, 100);
	let system_ctx = Ctx::root_ctx()
		.with_compliance(Some(SYSTEM_REASON_RECONCILE_SCAN.to_string()), None);
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let due_rows = async {
		set_full_context_dbx(
			mm.dbx(),
			system_ctx.user_id(),
			system_ctx.organization_id(),
			system_ctx.role(),
		)
		.await
		.map_err(Error::from)?;
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT submission_id
					   FROM submission_dispatch_state
					  WHERE next_retry_at IS NOT NULL
					    AND next_retry_at <= now()
					    AND terminal_at IS NULL
					  ORDER BY next_retry_at ASC
					  LIMIT $1",
				)
				.bind(safe_limit),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))
	}
	.await;
	match due_rows {
		Ok(rows) => {
			mm.dbx()
				.commit_txn()
				.await
				.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
			let due_rows = rows;

			let mut result = SubmissionReconcileResult {
				attempted: 0,
				succeeded: 0,
				failed: 0,
				skipped: 0,
				processed_submission_ids: Vec::new(),
			};

			for row in due_rows {
				let submission_id = row.0;
				let submission_mm = mm.clone();
				result.attempted += 1;
				result.processed_submission_ids.push(submission_id);
				match reconcile_one_submission(&submission_mm, submission_id).await {
					Ok(ReconcileOutcome::Succeeded) => result.succeeded += 1,
					Ok(ReconcileOutcome::Failed) => result.failed += 1,
					Ok(ReconcileOutcome::Skipped) => result.skipped += 1,
					Err(error) => {
						result.failed += 1;
						tracing::error!(%submission_id, %error, "Submission retry failed");
					}
				}
			}

			record_reconcile_result(&result);
			Ok(result)
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub(super) enum ReconcileOutcome {
	Succeeded,
	Failed,
	Skipped,
}

pub(super) async fn reconcile_one_submission(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<ReconcileOutcome> {
	let Some(lock_mm) = try_acquire_reconcile_lock(mm, submission_id).await? else {
		return Ok(ReconcileOutcome::Skipped);
	};

	let result = reconcile_one_submission_locked(mm, submission_id).await;
	let unlock_result = lock_mm
		.dbx()
		.rollback_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)));
	match (result, unlock_result) {
		(Err(err), _) => Err(err),
		(Ok(_), Err(err)) => Err(err),
		(Ok(outcome), Ok(())) => Ok(outcome),
	}
}

async fn try_acquire_reconcile_lock(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Option<ModelManager>> {
	let lock_mm = mm.clone();
	lock_mm
		.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let acquired = match lock_mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (bool,)>(
				"SELECT pg_try_advisory_xact_lock(
					hashtextextended('e2br3.submission.reconcile:' || $1::text, 0)
				)",
			)
			.bind(submission_id),
		)
		.await
	{
		Ok(row) => row.0,
		Err(err) => {
			let _ = lock_mm.dbx().rollback_txn().await;
			return Err(Error::from(lib_core::model::Error::from(err)));
		}
	};
	if !acquired {
		lock_mm
			.dbx()
			.rollback_txn()
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		return Ok(None);
	}
	Ok(Some(lock_mm))
}

async fn reconcile_one_submission_locked(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<ReconcileOutcome> {
	let system_ctx = Ctx::root_ctx()
		.with_compliance(Some(SYSTEM_REASON_RECONCILE_RETRY.to_string()), None);
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let row = async {
		set_full_context_dbx(
			mm.dbx(),
			system_ctx.user_id(),
			system_ctx.organization_id(),
			system_ctx.role(),
		)
		.await
		.map_err(Error::from)?;
		mm.dbx()
			.fetch_optional(
				sqlx::query_as::<_, CaseSubmissionRow>(
					"SELECT submission.id, submission.case_id, submission.gateway, submission.authority,
					        submission.remote_submission_id, submission.status,
					        submission.xml_bytes, submission.submitted_by,
					        submission.submitted_at
					   FROM case_submissions submission
					   JOIN submission_dispatch_state state
					     ON state.submission_id = submission.id
					  WHERE submission.id = $1
					    AND state.next_retry_at IS NOT NULL
					    AND state.next_retry_at <= now()
					    AND state.terminal_at IS NULL",
				)
				.bind(submission_id),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))
	}
	.await;
	match row {
		Ok(row) => {
			mm.dbx()
				.commit_txn()
				.await
				.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
			let Some(row) = row else {
				return Ok(ReconcileOutcome::Skipped);
			};
			let (authority, _) = match get_submission_artifact(
				&system_ctx,
				mm,
				submission_id,
			)
			.await
			{
				Ok(artifact) => artifact,
				Err(error @ Error::BadRequest { .. }) => {
					record_retry_preparation_failure(mm, submission_id, &error)
						.await?;
					return Ok(ReconcileOutcome::Failed);
				}
				Err(error) => return Err(error),
			};
			if !matches!(
				row.gateway.as_str(),
				"as2-submitter-http-fda" | "as2-submitter-http-mfds"
			) {
				let error = Error::BadRequest { message: "automatic resend blocked; transport requires outcome reconciliation".into() };
				record_retry_preparation_failure(mm, submission_id, &error).await?;
				return Ok(ReconcileOutcome::Failed);
			}
			let outcome = request_as2_state(row.case_id, authority, None).await;
			let observed = outcome.is_ok();
			persist_submission_observation(&system_ctx, mm, submission_id, outcome)
				.await?;
			Ok(if observed {
				ReconcileOutcome::Succeeded
			} else {
				ReconcileOutcome::Failed
			})
		}
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
	}
}

pub async fn reconcile_due_submissions_with_runtime_status(
	mm: &ModelManager,
	limit: i64,
) -> Result<SubmissionReconcileResult> {
	match reconcile_due_submissions(mm, limit).await {
		Ok(result) => Ok(result),
		Err(err) => {
			record_reconcile_error(&err.to_string());
			Err(err)
		}
	}
}

#[cfg(test)]
mod lock_tests {
	use super::*;

	#[serial_test::serial]
	#[tokio::test]
	async fn submission_lock_has_one_owner_and_releases_with_transaction() {
		std::env::set_var("SERVICE_DB_MAX_CONNECTIONS", "3");
		let mm = ModelManager::new().await.unwrap();
		let submission_id = Uuid::new_v4();

		let first = try_acquire_reconcile_lock(&mm, submission_id)
			.await
			.unwrap()
			.expect("first worker should own the submission lock");
		assert!(try_acquire_reconcile_lock(&mm, submission_id)
			.await
			.unwrap()
			.is_none());

		first.dbx().rollback_txn().await.unwrap();
		let next = try_acquire_reconcile_lock(&mm, submission_id)
			.await
			.unwrap()
			.expect("lock should be available after transaction end");
		next.dbx().rollback_txn().await.unwrap();
	}
}

async fn record_retry_preparation_failure(
	mm: &ModelManager,
	id: Uuid,
	error: &Error,
) -> Result<()> {
	let ctx = Ctx::root_ctx();
	mm.dbx()
		.begin_txn()
		.await
		.map_err(lib_core::model::Error::from)?;
	let result = async {
        set_full_context_dbx(mm.dbx(), ctx.user_id(), ctx.organization_id(), ctx.role()).await?;
        set_compliance_context_dbx(mm.dbx(), Some("system submission: stop unsafe retransmission"), None, None).await?;
        mm.dbx().execute(sqlx::query("UPDATE submission_dispatch_state SET next_retry_at=NULL,last_error=$2 WHERE submission_id=$1")
            .bind(id).bind(error.to_string())).await.map_err(lib_core::model::Error::from)?;
        append_submission_event(mm,id,"submission_reconciliation_required",Some(json!({"error":error.to_string(),"action":"manual_reconciliation_no_resend"}))).await
    }.await;
	match result {
		Ok(()) => mm
			.dbx()
			.commit_txn()
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e))),
		Err(error) => {
			mm.dbx()
				.rollback_txn()
				.await
				.map_err(lib_core::model::Error::from)?;
			Err(error)
		}
	}
}
