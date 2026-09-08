use super::*;

pub async fn create_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
) -> Result<SubmissionRecord> {
	assert_case_not_submitted(ctx, mm, case_id).await?;

	let export_authority = match authority {
		SubmissionAuthority::Fda => RegulatoryAuthority::Fda,
		SubmissionAuthority::Mfds => RegulatoryAuthority::Mfds,
	};
	let xml = prepare_submission_xml(ctx, mm, case_id, export_authority).await?;
	let now = OffsetDateTime::now_utc();
	let submission_id = Uuid::new_v4();
	let gateway = select_gateway_name(authority)?;
	let dispatch = submit_to_gateway_with_retry(case_id, &xml, authority).await;

	let (gateway_outcome, attempt_count) = match dispatch {
		Ok((outcome, attempts)) => (outcome, attempts),
		Err(failure) => {
			let failed_remote = format!(
				"FAILED-{}",
				submission_id.simple().to_string().to_uppercase()
			);
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
			.map_err(Error::from)?;

			mm.dbx()
				.execute(
					sqlx::query(
						"INSERT INTO case_submissions (
							id, case_id, gateway, remote_submission_id, status, xml_bytes,
							submitted_by, submitted_at, created_at, updated_at
						)
						VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now())",
					)
					.bind(submission_id)
					.bind(case_id)
					.bind(&gateway)
					.bind(&failed_remote)
					.bind(status_to_db(&SubmissionStatus::Rejected))
					.bind(xml.len() as i32)
					.bind(ctx.user_id())
					.bind(now),
				)
				.await
				.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

			append_submission_event(
				mm,
				submission_id,
				"submission_dispatch_failed",
				Some(json!({
					"case_id": case_id,
					"gateway": gateway,
					"error": failure.message,
					"attempts": failure.attempts,
					"next_retry_at": failure.next_retry_at,
				})),
			)
			.await?;
			upsert_dispatch_state_submit_failure(
				mm,
				submission_id,
				now,
				failure.attempts as i32,
				&failure.message,
				failure.next_retry_at,
			)
			.await?;

			mm.dbx()
				.commit_txn()
				.await
				.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

			return Err(Error::BadRequest {
				message: format!(
					"submission dispatch failed after {} attempt(s); submission_id={submission_id}: {}",
					failure.attempts, failure.message
				),
			});
		}
	};

	let remote_submission_id = gateway_outcome.remote_submission_id;
	let ack1 = gateway_outcome.ack1;
	let actual_gateway = gateway_outcome.gateway;

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
	.map_err(Error::from)?;

	let updated = mm
		.dbx()
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
					 WHERE id = $1
					   AND status <> 'submitted'",
			)
			.bind(case_id)
			.bind(ctx.user_id())
			.bind(now)
			.bind(xml.as_bytes()),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if updated == 0 {
		let _ = mm.dbx().rollback_txn().await;
		return Err(Error::BadRequest {
			message: "case has already been submitted".to_string(),
		});
	}

	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO case_submissions (
					id, case_id, gateway, remote_submission_id, status, xml_bytes,
					submitted_by, submitted_at, created_at, updated_at
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now())",
			)
			.bind(submission_id)
			.bind(case_id)
			.bind(&actual_gateway)
			.bind(&remote_submission_id)
			.bind(status_to_db(&SubmissionStatus::Ack1Received))
			.bind(xml.len() as i32)
			.bind(ctx.user_id())
			.bind(now),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	append_submission_event(
		mm,
		submission_id,
		"submission_created",
		Some(json!({
			"case_id": case_id,
			"gateway": actual_gateway,
			"remote_submission_id": remote_submission_id,
			"status": "ack1_received",
		})),
	)
	.await?;
	upsert_dispatch_state_submit_success(
		mm,
		submission_id,
		now,
		attempt_count as i32,
	)
	.await?;

	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_acks (
					submission_id, ack_level, success, ack_code, ack_message, received_at, raw_payload
				)
				VALUES ($1, $2, $3, $4, $5, $6, $7)",
			)
			.bind(submission_id)
			.bind(ack1.level as i16)
			.bind(ack1.success)
			.bind(ack1.code.as_deref())
			.bind(ack1.message.as_deref())
			.bind(ack1.received_at)
			.bind(json!({
				"level": ack1.level,
				"success": ack1.success,
				"code": ack1.code,
				"message": ack1.message,
			})),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	append_submission_event(
		mm,
		submission_id,
		"ack_recorded",
		Some(json!({
			"source": "gateway_submit_response",
			"ack_level": ack1.level,
			"success": ack1.success,
			"ack_code": ack1.code,
			"ack_message": ack1.message,
		})),
	)
	.await?;
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let row = get_submission_row_for_ctx(ctx, mm, submission_id)
		.await?
		.ok_or(Error::BadRequest {
			message: format!("submission not found after insert: {submission_id}"),
		})?;
	let acks = list_ack_rows(mm, submission_id).await?;
	compose_submission_record(row, acks)
}

pub async fn create_submission_idempotent(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	idempotency_key: Option<String>,
) -> Result<SubmissionRecord> {
	let normalized_key = idempotency_key
		.map(|v| v.trim().to_string())
		.filter(|v| !v.is_empty());

	if let Some(key) = normalized_key.as_deref() {
		if let Some(existing_id) =
			find_submission_idempotency(ctx, mm, case_id, authority, key).await?
		{
			return get_submission(ctx, mm, existing_id).await?.ok_or(
				Error::BadRequest {
					message: format!(
						"idempotent submission reference not found: {existing_id}"
					),
				},
			);
		}
	}

	let record = match create_submission(ctx, mm, case_id, authority).await {
		Ok(record) => record,
		Err(err) => {
			if normalized_key.is_some() && is_case_already_submitted_error(&err) {
				if let Some(existing_id) = wait_for_submission_idempotency(
					ctx,
					mm,
					case_id,
					authority,
					normalized_key.as_deref().unwrap_or_default(),
				)
				.await?
				{
					return get_submission(ctx, mm, existing_id).await?.ok_or(
						Error::BadRequest {
							message: format!(
								"idempotent submission reference not found: {existing_id}"
							),
						},
					);
				}
			}
			return Err(err);
		}
	};

	if let Some(key) = normalized_key.as_deref() {
		insert_submission_idempotency(
			ctx,
			mm,
			case_id,
			authority,
			key,
			record.id,
			ctx.user_id(),
		)
		.await?;
	}
	Ok(record)
}

pub(super) fn is_case_already_submitted_error(err: &Error) -> bool {
	match err {
		Error::BadRequest { message } => message.contains("already been submitted"),
		_ => false,
	}
}

pub(super) async fn wait_for_submission_idempotency(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	key: &str,
) -> Result<Option<Uuid>> {
	for _ in 0..10 {
		if let Some(existing_id) =
			find_submission_idempotency(ctx, mm, case_id, authority, key).await?
		{
			return Ok(Some(existing_id));
		}
		sleep(Duration::from_millis(50)).await;
	}
	Ok(None)
}

pub async fn assert_case_not_submitted(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	if case.status.eq_ignore_ascii_case("submitted") {
		return Err(Error::BadRequest {
			message: "case has already been submitted".to_string(),
		});
	}
	Ok(())
}

pub async fn list_by_case(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<SubmissionRecord>> {
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
	let rows = list_submission_rows_by_case(mm, case_id).await?;
	let mut out = Vec::with_capacity(rows.len());
	for row in rows {
		let acks = list_ack_rows(mm, row.id).await?;
		out.push(compose_submission_record(row, acks)?);
	}
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(out)
}

pub async fn get_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<Option<SubmissionRecord>> {
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
	let Some(row) = get_submission_row(mm, id).await? else {
		mm.dbx()
			.commit_txn()
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
		return Ok(None);
	};
	let acks = list_ack_rows(mm, id).await?;
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(Some(compose_submission_record(row, acks)?))
}
