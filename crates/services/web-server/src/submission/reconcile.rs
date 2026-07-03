use super::*;

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
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
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
				match reconcile_one_submission(mm, submission_id).await? {
					ReconcileOutcome::Succeeded => result.succeeded += 1,
					ReconcileOutcome::Failed => result.failed += 1,
					ReconcileOutcome::Skipped => result.skipped += 1,
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
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		mm.dbx()
			.fetch_optional(
				sqlx::query_as::<_, CaseSubmissionRow>(
					"SELECT id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at
					 FROM case_submissions
					 WHERE id = $1",
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
			let _case = match CaseBmc::get(&system_ctx, mm, row.case_id).await {
				Ok(case) => case,
				Err(ModelError::EntityUuidNotFound { .. }) => {
					return Ok(ReconcileOutcome::Skipped);
				}
				Err(e) => return Err(Error::from(e)),
			};
			let authority = if row.gateway.to_ascii_lowercase().contains("mfds") {
				SubmissionAuthority::Mfds
			} else {
				SubmissionAuthority::Fda
			};

			let ctx_clone = system_ctx.with_compliance(
				Some(SYSTEM_REASON_RECONCILE_EXPORT.to_string()),
				None,
			);
			let mm_clone = mm.clone();
			let case_id = row.case_id;
			let xml = task::spawn_blocking(move || {
				Handle::current()
					.block_on(export_case_xml(&ctx_clone, &mm_clone, case_id))
			})
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("reconcile export task failed: {err}"),
			})?
			.map_err(Error::from)?;

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
					.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

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
					.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
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
