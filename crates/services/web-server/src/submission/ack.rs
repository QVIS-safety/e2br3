use super::*;

pub(super) fn status_from_ack(level: u8, success: bool) -> Result<SubmissionStatus> {
	if !matches!(level, 1 | 2 | 3 | 4) {
		return Err(Error::BadRequest {
			message: "ack level must be one of: 1, 2, 3, 4".to_string(),
		});
	}
	if !success {
		return Ok(SubmissionStatus::Rejected);
	}
	let status = match level {
		1 => SubmissionStatus::Ack1Received,
		2 => SubmissionStatus::Ack2Received,
		3 => SubmissionStatus::Ack3Received,
		4 => SubmissionStatus::Ack4Received,
		_ => unreachable!(),
	};
	Ok(status)
}

pub(super) fn submission_status_rank(status: &SubmissionStatus) -> u8 {
	match status {
		SubmissionStatus::Ack1Received => 1,
		SubmissionStatus::Ack2Received => 2,
		SubmissionStatus::Ack3Received => 3,
		SubmissionStatus::Ack4Received => 4,
		SubmissionStatus::Rejected => 5,
	}
}

pub(super) fn is_submission_terminal(status: &SubmissionStatus) -> bool {
	matches!(
		status,
		SubmissionStatus::Ack4Received | SubmissionStatus::Rejected
	)
}

pub(super) fn merge_submission_status(
	current: &SubmissionStatus,
	incoming: &SubmissionStatus,
) -> SubmissionStatus {
	if is_submission_terminal(current) {
		return current.clone();
	}
	if matches!(incoming, SubmissionStatus::Rejected) {
		return SubmissionStatus::Rejected;
	}
	if submission_status_rank(incoming) >= submission_status_rank(current) {
		incoming.clone()
	} else {
		current.clone()
	}
}

pub async fn apply_mock_ack(
	ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
	input: MockAckInput,
) -> Result<SubmissionRecord> {
	if !allow_mock_submission() {
		return Err(Error::BadRequest {
			message:
				"mock ACK endpoint is disabled unless E2BR3_ALLOW_MOCK_SUBMISSION=1"
					.to_string(),
		});
	}
	let incoming_status = status_from_ack(input.level, input.success)?;
	let now = OffsetDateTime::now_utc();

	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await?;
	set_compliance_context_dbx(
		mm.dbx(),
		ctx.change_reason(),
		ctx.change_category(),
		ctx.e_signature_id(),
	)
	.await
	.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, CaseSubmissionRow>(
				"SELECT id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at
				 FROM case_submissions
				 WHERE id = $1
				 FOR UPDATE",
			)
			.bind(submission_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?
		.ok_or(Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		})?;
	let current_status = status_from_db(&row.status)?;
	let merged_status = merge_submission_status(&current_status, &incoming_status);
	let is_duplicate = ack_event_exists(
		mm,
		submission_id,
		input.level as i16,
		input.success,
		input.code.as_deref(),
		input.message.as_deref(),
	)
	.await?;

	if !is_duplicate {
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO submission_acks (
						submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7)",
				)
				.bind(submission_id)
				.bind(input.level as i16)
				.bind(input.success)
				.bind(input.code.as_deref())
				.bind(input.message.as_deref())
				.bind(now)
				.bind(json!({
					"level": input.level,
					"success": input.success,
					"code": input.code,
					"message": input.message,
				})),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		append_submission_event(
			mm,
			submission_id,
			"ack_recorded",
			Some(json!({
				"source": "mock_ack",
				"ack_level": input.level,
				"success": input.success,
				"ack_code": input.code,
				"ack_message": input.message,
			})),
		)
		.await?;
	} else {
		append_submission_event(
			mm,
			submission_id,
			"ack_duplicate_ignored",
			Some(json!({
				"source": "mock_ack",
				"ack_level": input.level,
				"success": input.success,
				"ack_code": input.code,
				"ack_message": input.message,
			})),
		)
		.await?;
	}

	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE case_submissions
				 SET status = $2,
				     updated_at = now()
				 WHERE id = $1",
			)
			.bind(submission_id)
			.bind(status_to_db(&merged_status)),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if merged_status != current_status {
		append_submission_event(
			mm,
			submission_id,
			"status_changed",
			Some(json!({
				"from": status_to_db(&current_status),
				"to": status_to_db(&merged_status),
			})),
		)
		.await?;
	}
	if is_submission_terminal(&merged_status) {
		mark_dispatch_terminal(mm, submission_id, now).await?;
	}

	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let row = get_submission_row_for_ctx(ctx, mm, submission_id)
		.await?
		.ok_or(Error::BadRequest {
			message: format!("submission not found: {submission_id}"),
		})?;
	let acks = list_ack_rows(mm, submission_id).await?;
	compose_submission_record(row, acks)
}

pub async fn apply_gateway_ack_by_remote(
	mm: &ModelManager,
	input: GatewayAckCallbackInput,
) -> Result<SubmissionRecord> {
	let incoming_status = status_from_ack(input.ack_level, input.success)?;
	let now = OffsetDateTime::now_utc();
	let system_ctx = Ctx::root_ctx()
		.with_compliance(Some(SYSTEM_REASON_ACK_CALLBACK.to_string()), None);

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

	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, CaseSubmissionRow>(
				"SELECT id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at
				 FROM case_submissions
				 WHERE remote_submission_id = $1
				 FOR UPDATE",
			)
			.bind(input.remote_submission_id.trim()),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?
		.ok_or(Error::BadRequest {
			message: format!(
				"submission not found for remote_submission_id: {}",
				input.remote_submission_id
			),
		})?;
	let current_status = status_from_db(&row.status)?;
	let merged_status = merge_submission_status(&current_status, &incoming_status);
	let is_duplicate = ack_event_exists(
		mm,
		row.id,
		input.ack_level as i16,
		input.success,
		input.ack_code.as_deref(),
		input.ack_message.as_deref(),
	)
	.await?;

	if !is_duplicate {
		mm.dbx()
			.execute(
				sqlx::query(
					"INSERT INTO submission_acks (
						submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7)",
				)
				.bind(row.id)
				.bind(input.ack_level as i16)
				.bind(input.success)
				.bind(input.ack_code.as_deref())
				.bind(input.ack_message.as_deref())
				.bind(now)
				.bind(json!({
					"source": "gateway_callback",
					"ack_level": input.ack_level,
					"success": input.success,
					"ack_code": input.ack_code,
					"ack_message": input.ack_message,
				})),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		append_submission_event(
			mm,
			row.id,
			"ack_recorded",
			Some(json!({
				"source": "gateway_callback",
				"ack_level": input.ack_level,
				"success": input.success,
				"ack_code": input.ack_code,
				"ack_message": input.ack_message,
			})),
		)
		.await?;
	} else {
		append_submission_event(
			mm,
			row.id,
			"ack_duplicate_ignored",
			Some(json!({
				"source": "gateway_callback",
				"ack_level": input.ack_level,
				"success": input.success,
				"ack_code": input.ack_code,
				"ack_message": input.ack_message,
			})),
		)
		.await?;
	}

	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE case_submissions
				 SET status = $2,
				     updated_at = now()
				 WHERE id = $1",
			)
			.bind(row.id)
			.bind(status_to_db(&merged_status)),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if merged_status != current_status {
		append_submission_event(
			mm,
			row.id,
			"status_changed",
			Some(json!({
				"from": status_to_db(&current_status),
				"to": status_to_db(&merged_status),
			})),
		)
		.await?;
	}
	if is_submission_terminal(&merged_status) {
		mark_dispatch_terminal(mm, row.id, now).await?;
	}

	let mut row_for_response = row.clone();
	row_for_response.status = status_to_db(&merged_status).to_string();
	let acks = list_ack_rows(mm, row.id).await?;
	let response = compose_submission_record(row_for_response, acks)?;

	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	Ok(response)
}

pub async fn list_submission_events(
	ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Vec<SubmissionEventRecord>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if let Err(err) = set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, SubmissionEventRow>(
				"SELECT id, submission_id, event_type, event_data, created_at
				 FROM submission_events
				 WHERE submission_id = $1
				 ORDER BY created_at ASC",
			)
			.bind(submission_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let events = rows
		.into_iter()
		.map(|row| SubmissionEventRecord {
			id: row.id,
			submission_id: row.submission_id,
			event_type: row.event_type,
			event_data: row.event_data,
			created_at: row.created_at,
		})
		.collect();
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(events)
}

pub async fn get_ack_download(
	ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
	level: u8,
) -> Result<Option<SubmissionAckDownload>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if let Err(err) = set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, SubmissionAckDownloadRow>(
				"SELECT a.submission_id,
				        cs.case_id,
				        a.ack_level,
				        a.success,
				        a.ack_code,
				        a.ack_message,
				        a.received_at,
				        a.raw_payload
				   FROM submission_acks a
				   JOIN case_submissions cs ON cs.id = a.submission_id
				  WHERE a.submission_id = $1
				    AND a.ack_level = $2
				  ORDER BY a.received_at DESC, a.id DESC
				  LIMIT 1",
			)
			.bind(submission_id)
			.bind(level as i16),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let ack = match row {
		Some(row) => Some(SubmissionAckDownload {
			submission_id: row.submission_id,
			case_id: row.case_id,
			level: u8::try_from(row.ack_level).map_err(|_| Error::BadRequest {
				message: format!("invalid ACK level stored: {}", row.ack_level),
			})?,
			success: row.success,
			code: row.ack_code,
			message: row.ack_message,
			received_at: row.received_at,
			raw_payload: row.raw_payload,
		}),
		None => None,
	};
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(ack)
}

pub async fn get_submission_dispatch_state(
	ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Option<SubmissionDispatchStateRecord>> {
	mm.dbx()
		.begin_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if let Err(err) = set_full_context_dbx_or_rollback(
		mm.dbx(),
		ctx.user_id(),
		ctx.organization_id(),
		ctx.role(),
	)
	.await
	{
		let _ = mm.dbx().rollback_txn().await;
		return Err(err.into());
	}
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, SubmissionDispatchStateRow>(
				"SELECT submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				 FROM submission_dispatch_state
				 WHERE submission_id = $1",
			)
			.bind(submission_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let state = row.map(|r| SubmissionDispatchStateRecord {
		submission_id: r.submission_id,
		attempt_count: r.attempt_count,
		last_attempt_at: r.last_attempt_at,
		last_error: r.last_error,
		next_retry_at: r.next_retry_at,
		terminal_at: r.terminal_at,
		created_at: r.created_at,
		updated_at: r.updated_at,
	});
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(state)
}
