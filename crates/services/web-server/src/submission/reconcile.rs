use super::*;
use lib_core::model::Error as ModelError;

pub async fn reconcile_due_submissions(
	mm: &ModelManager,
	limit: i64,
) -> Result<SubmissionReconcileResult> {
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
				result.attempted += 1;
				result.processed_submission_ids.push(submission_id);
				match reconcile_one_submission(mm, submission_id).await {
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

async fn record_retry_preparation_failure(
	mm: &ModelManager,
	submission_id: Uuid,
	error: &Error,
) -> Result<()> {
	let ctx = Ctx::root_ctx();
	let now = OffsetDateTime::now_utc();
	// Invalid case data/lifecycle needs user correction, not another dispatch attempt.
	let next_retry_at = if matches!(
		error,
		Error::BadRequest { .. }
			| Error::Model(ModelError::EntityUuidNotFound { .. })
	) {
		None
	} else {
		Some(now + time::Duration::minutes(5))
	};
	mm.dbx().begin_txn().await.map_err(ModelError::from)?;
	let result = async {
		set_full_context_dbx(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let attempts = get_dispatch_attempt_count(mm, submission_id).await?;
		upsert_dispatch_state_submit_failure(
			mm,
			submission_id,
			now,
			attempts,
			&error.to_string(),
			next_retry_at,
		)
		.await?;
		append_submission_event(
			mm,
			submission_id,
			"submission_retry_failed",
			Some(json!({
				"attempts": 0,
				"error": error.to_string(),
				"next_retry_at": next_retry_at,
			})),
		)
		.await
	}
	.await;
	match result {
		Ok(()) => mm
			.dbx()
			.commit_txn()
			.await
			.map_err(|err| Error::from(ModelError::from(err))),
		Err(err) => {
			let _ = mm.dbx().rollback_txn().await;
			Err(err)
		}
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
					"SELECT submission.id, submission.case_id, submission.gateway,
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
			if !row.status.eq_ignore_ascii_case("rejected") {
				return Ok(ReconcileOutcome::Skipped);
			}
			let authority = if row.gateway.to_ascii_lowercase().contains("mfds") {
				SubmissionAuthority::Mfds
			} else {
				SubmissionAuthority::Fda
			};
			let export_authority = match authority {
				SubmissionAuthority::Fda => RegulatoryAuthority::Fda,
				SubmissionAuthority::Mfds => RegulatoryAuthority::Mfds,
			};
			let export_ctx = system_ctx.with_compliance(
				Some(SYSTEM_REASON_RECONCILE_EXPORT.to_string()),
				None,
			);
			let xml = match prepare_submission_xml(
				&export_ctx,
				mm,
				row.case_id,
				export_authority,
			)
			.await
			{
				Ok(xml) => xml,
				Err(error) => {
					let _ = mm.dbx().rollback_txn().await;
					record_retry_preparation_failure(mm, submission_id, &error)
						.await?;
					return Ok(ReconcileOutcome::Failed);
				}
			};

			let now = OffsetDateTime::now_utc();
			let prior_attempts =
				get_dispatch_attempt_count(mm, submission_id).await?;

			match submit_to_gateway_with_retry(row.case_id, &xml, authority).await {
				Ok((outcome, attempts)) => {
					mm.dbx()
						.begin_txn()
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
					set_full_context_dbx_or_rollback(
						mm.dbx(),
						system_ctx.user_id(),
						system_ctx.organization_id(),
						system_ctx.role(),
					)
					.await?;
					set_compliance_context_dbx(
						mm.dbx(),
						system_ctx.change_reason(),
						system_ctx.change_category(),
						system_ctx.e_signature_id(),
					)
					.await
					.map_err(Error::from)?;

					mm.dbx()
						.execute(
							sqlx::query(
								"UPDATE cases
								 SET status = 'submitted',
								     submitted_by = $2,
								     submitted_at = $3,
								     raw_xml = $4,
								     dirty_c = false,
								     dirty_d = false,
								     dirty_e = false,
								     dirty_f = false,
								     dirty_g = false,
								     dirty_h = false,
								     updated_at = now()
								 WHERE id = $1",
							)
							.bind(row.case_id)
							.bind(system_ctx.user_id())
							.bind(now)
							.bind(xml.as_bytes()),
						)
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

					mm.dbx()
						.execute(
							sqlx::query(
								"UPDATE case_submissions
								 SET gateway = $2,
								     remote_submission_id = $3,
								     status = $4,
								     updated_at = now()
								 WHERE id = $1",
							)
							.bind(submission_id)
							.bind(outcome.gateway)
							.bind(outcome.remote_submission_id)
							.bind(status_to_db(&SubmissionStatus::Ack1Received)),
						)
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

					mm.dbx()
						.execute(
							sqlx::query(
								"INSERT INTO submission_acks (
									submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
								)
								VALUES ($1, $2, $3, $4, $5, $6, $7)",
							)
							.bind(submission_id)
							.bind(outcome.ack1.level as i16)
							.bind(outcome.ack1.success)
							.bind(outcome.ack1.code.as_deref())
							.bind(outcome.ack1.message.as_deref())
							.bind(outcome.ack1.received_at)
							.bind(json!({
								"source": "reconcile_retry",
								"level": outcome.ack1.level,
								"success": outcome.ack1.success,
								"code": outcome.ack1.code,
								"message": outcome.ack1.message,
							})),
						)
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

					append_submission_event(
						mm,
						submission_id,
						"submission_retried",
						Some(json!({
							"status": "ack1_received",
							"attempts": attempts,
						})),
					)
					.await?;
					upsert_dispatch_state_submit_success(
						mm,
						submission_id,
						now,
						prior_attempts + attempts as i32,
					)
					.await?;

					mm.dbx()
						.commit_txn()
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
					Ok(ReconcileOutcome::Succeeded)
				}
				Err(failure) => {
					mm.dbx()
						.begin_txn()
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
					set_full_context_dbx_or_rollback(
						mm.dbx(),
						system_ctx.user_id(),
						system_ctx.organization_id(),
						system_ctx.role(),
					)
					.await?;
					set_compliance_context_dbx(
						mm.dbx(),
						system_ctx.change_reason(),
						system_ctx.change_category(),
						system_ctx.e_signature_id(),
					)
					.await
					.map_err(Error::from)?;
					upsert_dispatch_state_submit_failure(
						mm,
						submission_id,
						now,
						prior_attempts + failure.attempts as i32,
						&failure.message,
						failure.next_retry_at,
					)
					.await?;
					append_submission_event(
						mm,
						submission_id,
						"submission_retry_failed",
						Some(json!({
							"attempts": failure.attempts,
							"error": failure.message,
							"next_retry_at": failure.next_retry_at,
						})),
					)
					.await?;
					mm.dbx()
						.commit_txn()
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
					Ok(ReconcileOutcome::Failed)
				}
			}
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
