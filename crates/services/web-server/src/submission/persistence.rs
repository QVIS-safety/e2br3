use super::*;

pub(super) async fn append_submission_event(
	mm: &ModelManager,
	submission_id: Uuid,
	event_type: &str,
	event_data: Option<Value>,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_events (
					submission_id, event_type, event_data, created_at
				)
				VALUES ($1, $2, $3, now())",
			)
			.bind(submission_id)
			.bind(event_type)
			.bind(event_data),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
}

pub(super) async fn upsert_dispatch_state_submit_success(
	mm: &ModelManager,
	submission_id: Uuid,
	attempted_at: OffsetDateTime,
	attempt_count: i32,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, $3, $2, NULL, NULL, NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = NULL,
					next_retry_at = NULL,
					updated_at = now()",
			)
			.bind(submission_id)
			.bind(attempted_at)
			.bind(attempt_count),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
}

pub(super) async fn upsert_dispatch_state_submit_failure(
	mm: &ModelManager,
	submission_id: Uuid,
	attempted_at: OffsetDateTime,
	attempt_count: i32,
	last_error: &str,
	next_retry_at: Option<OffsetDateTime>,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_dispatch_state (
					submission_id, attempt_count, last_attempt_at, last_error, next_retry_at, terminal_at, created_at, updated_at
				)
				VALUES ($1, $3, $2, $4, $5, NULL, now(), now())
				ON CONFLICT (submission_id)
				DO UPDATE SET
					attempt_count = EXCLUDED.attempt_count,
					last_attempt_at = EXCLUDED.last_attempt_at,
					last_error = EXCLUDED.last_error,
					next_retry_at = EXCLUDED.next_retry_at,
					updated_at = now()",
			)
			.bind(submission_id)
			.bind(attempted_at)
			.bind(attempt_count)
			.bind(last_error)
			.bind(next_retry_at),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
}

pub(super) async fn get_dispatch_attempt_count(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<i32> {
	let row = mm
		.dbx()
		.fetch_optional(sqlx::query_as::<_, (i32,)>(
			"SELECT attempt_count FROM submission_dispatch_state WHERE submission_id = $1",
		).bind(submission_id))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(row.map(|r| r.0).unwrap_or(0))
}

pub(super) async fn find_submission_idempotency(
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	key: &str,
) -> Result<Option<Uuid>> {
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT submission_id
				 FROM submission_idempotency
				 WHERE case_id = $1
				   AND authority = $2
				   AND idempotency_key = $3",
			)
			.bind(case_id)
			.bind(authority.as_str())
			.bind(key),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(row.map(|r| r.0))
}

pub(super) async fn insert_submission_idempotency(
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	key: &str,
	submission_id: Uuid,
	created_by: Uuid,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO submission_idempotency (
					case_id, authority, idempotency_key, submission_id, created_by, created_at
				)
				VALUES ($1, $2, $3, $4, $5, now())
				ON CONFLICT (case_id, authority, idempotency_key) DO NOTHING",
			)
			.bind(case_id)
			.bind(authority.as_str())
			.bind(key)
			.bind(submission_id)
			.bind(created_by),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
}

pub(super) async fn mark_dispatch_terminal(
	mm: &ModelManager,
	submission_id: Uuid,
	terminal_at: OffsetDateTime,
) -> Result<()> {
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE submission_dispatch_state
				 SET terminal_at = COALESCE(terminal_at, $2),
				     next_retry_at = NULL,
				     updated_at = now()
				 WHERE submission_id = $1",
			)
			.bind(submission_id)
			.bind(terminal_at),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(())
}

pub(super) fn compose_submission_record(
	row: CaseSubmissionRow,
	acks: Vec<SubmissionAckRow>,
) -> Result<SubmissionRecord> {
	let mut ack1 = None;
	let mut ack2 = None;
	let mut ack3 = None;
	let mut ack4 = None;
	for ack in acks {
		let item = SubmissionAck {
			level: ack.ack_level as u8,
			success: ack.success,
			code: ack.ack_code,
			message: ack.ack_message,
			received_at: ack.received_at,
		};
		match item.level {
			1 if ack1.is_none() => ack1 = Some(item),
			2 if ack2.is_none() => ack2 = Some(item),
			3 if ack3.is_none() => ack3 = Some(item),
			4 if ack4.is_none() => ack4 = Some(item),
			_ => {}
		}
	}

	Ok(SubmissionRecord {
		id: row.id,
		case_id: row.case_id,
		gateway: row.gateway,
		remote_submission_id: row.remote_submission_id,
		status: status_from_db(&row.status)?,
		xml_bytes: row.xml_bytes as usize,
		submitted_by: row.submitted_by,
		submitted_at: row.submitted_at,
		ack1,
		ack2,
		ack3,
		ack4,
	})
}

pub(super) async fn get_submission_row(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Option<CaseSubmissionRow>> {
	let row = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, CaseSubmissionRow>(
				"SELECT id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at
				 FROM case_submissions
				 WHERE id = $1",
			)
			.bind(submission_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(row)
}

pub(super) async fn list_submission_rows_by_case(
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseSubmissionRow>> {
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, CaseSubmissionRow>(
				"SELECT id, case_id, gateway, remote_submission_id, status, xml_bytes, submitted_by, submitted_at
				 FROM case_submissions
				 WHERE case_id = $1
				 ORDER BY submitted_at DESC",
			)
			.bind(case_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(rows)
}

pub async fn list_submission_history(
	_ctx: &Ctx,
	mm: &ModelManager,
) -> Result<Vec<SubmissionHistoryRecord>> {
	let latest_ack_summaries = list_latest_ack_summaries(mm).await?;
	let latest_event_type = list_latest_submission_event_types(mm).await?;
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, SubmissionHistoryRow>(
			"SELECT cs.id AS submission_id,
				        cs.case_id,
				        sri.safety_report_id AS case_number,
				        cs.gateway,
				        cs.remote_submission_id,
				        cs.status,
				        cs.xml_bytes,
				        cs.submitted_by,
				        u.email AS submitted_by_email,
				        cs.submitted_at
				   FROM case_submissions cs
				   JOIN cases c ON c.id = cs.case_id
				   JOIN safety_report_identification sri ON sri.case_id = c.id
				   LEFT JOIN users u ON u.id = cs.submitted_by
				  ORDER BY cs.submitted_at DESC
				  LIMIT 200",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	rows.into_iter()
		.map(|row| {
			let latest_ack = latest_ack_summaries.get(&row.submission_id);
			let latest_ack_received_at = latest_ack
				.map(|ack| format_history_timestamp(ack.received_at))
				.transpose()?;
			let gateway = row.gateway;
			let export_authority = submission_history_export_authority(&gateway);
			let data_file_name = format!(
				"{}-{}-{}.xml",
				row.case_number, row.case_id, export_authority
			);
			let data_file_download_url = format!(
				"/api/cases/{}/export/xml?authority={}",
				row.case_id, export_authority
			);
			Ok(SubmissionHistoryRecord {
				submission_id: row.submission_id,
				case_id: row.case_id,
				case_number: row.case_number,
				gateway,
				remote_submission_id: row.remote_submission_id,
				status: status_from_db(&row.status)?,
				batch_result: row.status,
				message_result: latest_ack.and_then(format_latest_ack_result),
				xml_bytes: row.xml_bytes as usize,
				submitted_by: row.submitted_by,
				submitted_by_email: row.submitted_by_email,
				submitted_at: format_history_timestamp(row.submitted_at)?,
				latest_ack_received_at: latest_ack_received_at.clone(),
				acknowledged_date: latest_ack_received_at,
				latest_event_type: latest_event_type
					.get(&row.submission_id)
					.cloned(),
				icsr_count: 1,
				data_file_name,
				data_file_download_url,
			})
		})
		.collect()
}

pub(super) fn format_history_timestamp(value: OffsetDateTime) -> Result<String> {
	value.format(&Rfc3339).map_err(|err| Error::BadRequest {
		message: format!("failed to format submission history timestamp: {err}"),
	})
}

pub(super) async fn list_latest_ack_summaries(
	mm: &ModelManager,
) -> Result<HashMap<Uuid, LatestSubmissionAckRow>> {
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, LatestSubmissionAckRow>(
			"SELECT DISTINCT ON (submission_id)
				        submission_id,
				        ack_level,
				        success,
				        ack_code,
				        ack_message,
				        received_at
				 FROM submission_acks
				 ORDER BY submission_id, received_at DESC, ack_level DESC",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(rows
		.into_iter()
		.map(|row| (row.submission_id, row))
		.collect())
}

pub(super) fn format_latest_ack_result(
	ack: &LatestSubmissionAckRow,
) -> Option<String> {
	let code = ack
		.ack_code
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| format!("ACK{}", ack.ack_level));
	let message = ack
		.ack_message
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty());
	match (ack.success, message) {
		(_, Some(message)) => Some(format!("{code}: {message}")),
		(true, None) => Some(code),
		(false, None) => Some(format!("{code}: Rejected")),
	}
}

pub(super) async fn list_latest_submission_event_types(
	mm: &ModelManager,
) -> Result<HashMap<Uuid, String>> {
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, (Uuid, String)>(
			"SELECT DISTINCT ON (submission_id) submission_id, event_type
				 FROM submission_events
				 ORDER BY submission_id, created_at DESC, id DESC",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(rows.into_iter().collect())
}

pub(super) async fn list_ack_rows(
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Vec<SubmissionAckRow>> {
	let rows = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, SubmissionAckRow>(
				"SELECT ack_level, success, ack_code, ack_message, received_at
				 FROM submission_acks
				 WHERE submission_id = $1
				 ORDER BY received_at DESC",
			)
			.bind(submission_id),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(rows)
}

pub(super) async fn ack_event_exists(
	mm: &ModelManager,
	submission_id: Uuid,
	ack_level: i16,
	success: bool,
	ack_code: Option<&str>,
	ack_message: Option<&str>,
) -> Result<bool> {
	let count = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (i64,)>(
				"SELECT COUNT(*)::bigint
				 FROM submission_acks
				 WHERE submission_id = $1
				   AND ack_level = $2
				   AND success = $3
				   AND COALESCE(ack_code, '') = COALESCE($4, '')
				   AND COALESCE(ack_message, '') = COALESCE($5, '')",
			)
			.bind(submission_id)
			.bind(ack_level)
			.bind(success)
			.bind(ack_code)
			.bind(ack_message),
		)
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?
		.0;
	Ok(count > 0)
}
