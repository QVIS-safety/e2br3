use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::store::{
	set_compliance_context_dbx, set_full_context_dbx_or_rollback,
};
use lib_core::model::ModelManager;
use lib_core::xml::{export_case_xml, should_skip_xml_validation, validate_e2b_xml};
use lib_rest_core::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::runtime::Handle;
use tokio::task;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
	Ack1Received,
	Ack2Received,
	Ack3Received,
	Ack4Received,
	Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionAck {
	pub level: u8,
	pub success: bool,
	pub code: Option<String>,
	pub message: Option<String>,
	pub received_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionRecord {
	pub id: Uuid,
	pub case_id: Uuid,
	pub gateway: String,
	pub remote_submission_id: String,
	pub status: SubmissionStatus,
	pub xml_bytes: usize,
	pub submitted_by: Uuid,
	pub submitted_at: OffsetDateTime,
	pub ack1: Option<SubmissionAck>,
	pub ack2: Option<SubmissionAck>,
	pub ack3: Option<SubmissionAck>,
	pub ack4: Option<SubmissionAck>,
}

#[derive(Debug, Deserialize)]
pub struct MockAckInput {
	pub level: u8,
	#[serde(default = "default_true")]
	pub success: bool,
	pub code: Option<String>,
	pub message: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct CaseSubmissionRow {
	id: Uuid,
	case_id: Uuid,
	gateway: String,
	remote_submission_id: String,
	status: String,
	xml_bytes: i32,
	submitted_by: Uuid,
	submitted_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
struct SubmissionAckRow {
	ack_level: i16,
	success: bool,
	ack_code: Option<String>,
	ack_message: Option<String>,
	received_at: OffsetDateTime,
}

#[derive(Debug)]
struct GatewaySubmissionOutcome {
	gateway: String,
	remote_submission_id: String,
	ack1: SubmissionAck,
}

#[derive(Debug, Deserialize)]
struct EsgSubmitResponse {
	remote_submission_id: Option<String>,
	submission_id: Option<String>,
	id: Option<String>,
	ack: Option<EsgAckResponse>,
}

#[derive(Debug, Deserialize)]
struct EsgAckResponse {
	level: Option<u8>,
	success: Option<bool>,
	code: Option<String>,
	message: Option<String>,
	received_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct As2SubmitResponse {
	remote_submission_id: Option<String>,
	submission_id: Option<String>,
	status: Option<String>,
	authority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GatewayAckCallbackInput {
	pub remote_submission_id: String,
	pub ack_level: u8,
	#[serde(default = "default_true")]
	pub success: bool,
	pub ack_code: Option<String>,
	pub ack_message: Option<String>,
}

fn default_true() -> bool {
	true
}

fn env_truthy(name: &str) -> bool {
	matches!(
		std::env::var(name),
		Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
	)
}

fn is_esg_enabled() -> bool {
	env_truthy("FDA_ESG_ENABLED")
}

fn allow_mock_submission() -> bool {
	env_truthy("E2BR3_ALLOW_MOCK_SUBMISSION")
}

fn as2_submitter_url() -> Option<String> {
	std::env::var("AS2_SUBMITTER_URL").ok().and_then(|v| {
		let trimmed = v.trim();
		if trimmed.is_empty() {
			None
		} else {
			Some(trimmed.to_string())
		}
	})
}

fn parse_timeout_secs(name: &str, default_secs: u64) -> u64 {
	std::env::var(name)
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(default_secs)
}

fn is_fda_profile(case_profile: Option<&str>) -> bool {
	case_profile
		.map(|v| v.eq_ignore_ascii_case("fda"))
		.unwrap_or(true)
}

fn status_to_db(status: &SubmissionStatus) -> &'static str {
	match status {
		SubmissionStatus::Ack1Received => "ack1_received",
		SubmissionStatus::Ack2Received => "ack2_received",
		SubmissionStatus::Ack3Received => "ack3_received",
		SubmissionStatus::Ack4Received => "ack4_received",
		SubmissionStatus::Rejected => "rejected",
	}
}

fn status_from_db(status: &str) -> Result<SubmissionStatus> {
	match status.trim().to_ascii_lowercase().as_str() {
		"ack1_received" => Ok(SubmissionStatus::Ack1Received),
		"ack2_received" => Ok(SubmissionStatus::Ack2Received),
		"ack3_received" => Ok(SubmissionStatus::Ack3Received),
		"ack4_received" => Ok(SubmissionStatus::Ack4Received),
		"rejected" => Ok(SubmissionStatus::Rejected),
		other => Err(Error::BadRequest {
			message: format!("invalid submission status in database: '{other}'"),
		}),
	}
}

fn status_from_ack(level: u8, success: bool) -> Result<SubmissionStatus> {
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

fn submission_status_rank(status: &SubmissionStatus) -> u8 {
	match status {
		SubmissionStatus::Ack1Received => 1,
		SubmissionStatus::Ack2Received => 2,
		SubmissionStatus::Ack3Received => 3,
		SubmissionStatus::Ack4Received => 4,
		SubmissionStatus::Rejected => 5,
	}
}

fn is_submission_terminal(status: &SubmissionStatus) -> bool {
	matches!(
		status,
		SubmissionStatus::Ack4Received | SubmissionStatus::Rejected
	)
}

fn merge_submission_status(
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

async fn submit_to_gateway(
	case_id: Uuid,
	xml: &str,
) -> Result<GatewaySubmissionOutcome> {
	let now = OffsetDateTime::now_utc();
	if let Some(base_url) = as2_submitter_url() {
		let submit_url = format!("{}/submit", base_url.trim_end_matches('/'));
		let timeout_secs = parse_timeout_secs("AS2_SUBMITTER_TIMEOUT_SECS", 30);
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(timeout_secs))
			.build()
			.map_err(|err| Error::BadRequest {
				message: format!("failed to initialize AS2 submitter client: {err}"),
			})?;
		let callback_url = std::env::var("AS2_ACK_CALLBACK_URL").ok();
		let resp = client
			.post(&submit_url)
			.json(&json!({
				"caseId": case_id.to_string(),
				"authority": "fda",
				"xmlPayload": xml,
				"callbackUrl": callback_url,
			}))
			.send()
			.await
			.map_err(|err| Error::BadRequest {
				message: format!("AS2 submitter request failed: {err}"),
			})?;
		let status = resp.status();
		let body_text = resp.text().await.map_err(|err| Error::BadRequest {
			message: format!("AS2 submitter response read failed: {err}"),
		})?;
		if !status.is_success() {
			let body_snippet = body_text.chars().take(200).collect::<String>();
			return Err(Error::BadRequest {
				message: format!(
					"AS2 submitter rejected request ({status}): {body_snippet}"
				),
			});
		}
		let parsed: As2SubmitResponse =
			serde_json::from_str(&body_text).map_err(|err| Error::BadRequest {
				message: format!("AS2 submitter response is not valid JSON: {err}"),
			})?;
		let remote_submission_id = parsed
			.remote_submission_id
			.or(parsed.submission_id)
			.ok_or(Error::BadRequest {
				message:
					"AS2 submitter response missing remote submission identifier"
						.to_string(),
			})?;
		let ack_message = match (parsed.status, parsed.authority) {
			(Some(status), Some(authority)) => {
				Some(format!("AS2 accepted: {status} ({authority})"))
			}
			(Some(status), None) => Some(format!("AS2 accepted: {status}")),
			(None, Some(authority)) => Some(format!("AS2 accepted ({authority})")),
			(None, None) => None,
		};
		return Ok(GatewaySubmissionOutcome {
			gateway: "as2-submitter-http".to_string(),
			remote_submission_id,
			ack1: SubmissionAck {
				level: 1,
				success: true,
				code: Some("ACK1_ACCEPTED".to_string()),
				message: ack_message,
				received_at: now,
			},
		});
	}

	if allow_mock_submission() {
		let submission_id = Uuid::new_v4();
		return Ok(GatewaySubmissionOutcome {
			gateway: "fda-esg-nextgen-mock".to_string(),
			remote_submission_id: format!(
				"FDA-MOCK-{}",
				submission_id.simple().to_string().to_uppercase()
			),
			ack1: SubmissionAck {
				level: 1,
				success: true,
				code: Some("ACK1_ACCEPTED".to_string()),
				message: Some("Upload accepted by mock FDA gateway".to_string()),
				received_at: now,
			},
		});
	}
	if !is_esg_enabled() {
		return Err(Error::BadRequest {
			message: "no submission transport configured: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		});
	}

	let base_url =
		std::env::var("FDA_ESG_BASE_URL").map_err(|_| Error::BadRequest {
			message: "FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string(),
		})?;
	let submit_path = std::env::var("FDA_ESG_SUBMIT_PATH")
		.unwrap_or_else(|_| "/submissions".to_string());
	let submit_url = format!(
		"{}/{}",
		base_url.trim_end_matches('/'),
		submit_path.trim_start_matches('/')
	);
	let timeout_secs = parse_timeout_secs("FDA_ESG_TIMEOUT_SECS", 30);
	let client = reqwest::Client::builder()
		.timeout(Duration::from_secs(timeout_secs))
		.build()
		.map_err(|err| Error::BadRequest {
			message: format!("failed to initialize FDA ESG client: {err}"),
		})?;

	let mut headers = HeaderMap::new();
	if let Ok(token) = std::env::var("FDA_ESG_BEARER_TOKEN") {
		let value = format!("Bearer {}", token.trim());
		let hv = HeaderValue::from_str(&value).map_err(|_| Error::BadRequest {
			message: "invalid FDA_ESG_BEARER_TOKEN".to_string(),
		})?;
		headers.insert(AUTHORIZATION, hv);
	}
	if let Ok(api_key) = std::env::var("FDA_ESG_API_KEY") {
		let hv = HeaderValue::from_str(api_key.trim()).map_err(|_| {
			Error::BadRequest {
				message: "invalid FDA_ESG_API_KEY".to_string(),
			}
		})?;
		headers.insert("x-api-key", hv);
	}

	let resp = client
		.post(&submit_url)
		.headers(headers)
		.json(&json!({ "xml": xml }))
		.send()
		.await
		.map_err(|err| Error::BadRequest {
			message: format!("FDA ESG submit request failed: {err}"),
		})?;
	let status = resp.status();
	let body_text = resp.text().await.map_err(|err| Error::BadRequest {
		message: format!("FDA ESG submit response read failed: {err}"),
	})?;
	if !status.is_success() {
		let body_snippet = body_text.chars().take(200).collect::<String>();
		return Err(Error::BadRequest {
			message: format!("FDA ESG submit failed ({status}): {body_snippet}"),
		});
	}

	let parsed: EsgSubmitResponse =
		serde_json::from_str(&body_text).map_err(|err| Error::BadRequest {
			message: format!("FDA ESG submit response is not valid JSON: {err}"),
		})?;
	let remote_submission_id = parsed
		.remote_submission_id
		.or(parsed.submission_id)
		.or(parsed.id)
		.ok_or(Error::BadRequest {
			message: "FDA ESG submit response missing remote submission identifier"
				.to_string(),
		})?;
	let ack = parsed.ack.unwrap_or(EsgAckResponse {
		level: Some(1),
		success: Some(true),
		code: Some("ACK1_ACCEPTED".to_string()),
		message: Some(
			"Submitted to FDA ESG; awaiting downstream ACK updates".to_string(),
		),
		received_at: None,
	});
	let ack1 = SubmissionAck {
		level: ack.level.unwrap_or(1),
		success: ack.success.unwrap_or(true),
		code: ack.code,
		message: ack.message,
		received_at: now,
	};
	Ok(GatewaySubmissionOutcome {
		gateway: "fda-esg-nextgen-api".to_string(),
		remote_submission_id,
		ack1,
	})
}

fn compose_submission_record(
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

async fn get_submission_row(
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

async fn list_submission_rows_by_case(
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

async fn list_ack_rows(
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

async fn ack_event_exists(
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

pub async fn create_fda_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<SubmissionRecord> {
	assert_case_ready_for_fda_submission(ctx, mm, case_id).await?;

	let ctx_clone = ctx.clone();
	let mm_clone = mm.clone();
	let xml = task::spawn_blocking(move || {
		Handle::current().block_on(export_case_xml(&ctx_clone, &mm_clone, case_id))
	})
	.await
	.map_err(|err| Error::BadRequest {
		message: format!("submission export task failed: {err}"),
	})?
	.map_err(Error::from)?;
	if !should_skip_xml_validation() {
		let report = validate_e2b_xml(xml.as_bytes(), None).map_err(Error::from)?;
		if !report.ok {
			let preview = report
				.errors
				.iter()
				.take(3)
				.map(|err| err.message.as_str())
				.collect::<Vec<_>>()
				.join("; ");
			return Err(Error::BadRequest {
				message: format!(
					"cannot submit case: XML validation failed ({} issue(s)): {}",
					report.errors.len(),
					preview
				),
			});
		}
	}

	let now = OffsetDateTime::now_utc();
	let submission_id = Uuid::new_v4();
	let gateway_outcome = submit_to_gateway(case_id, &xml).await?;
	let remote_submission_id = gateway_outcome.remote_submission_id;
	let ack1 = gateway_outcome.ack1;
	let gateway = gateway_outcome.gateway;

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
	set_compliance_context_dbx(mm.dbx(), ctx.change_reason(), ctx.e_signature_id())
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

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
				 WHERE id = $1",
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
			message: format!("case not found: {case_id}"),
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
			.bind(gateway)
			.bind(&remote_submission_id)
			.bind(status_to_db(&SubmissionStatus::Ack1Received))
			.bind(xml.len() as i32)
			.bind(ctx.user_id())
			.bind(now),
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
	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let row =
		get_submission_row(mm, submission_id)
			.await?
			.ok_or(Error::BadRequest {
				message: format!(
					"submission not found after insert: {submission_id}"
				),
			})?;
	let acks = list_ack_rows(mm, submission_id).await?;
	compose_submission_record(row, acks)
}

pub async fn assert_case_ready_for_fda_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	if !is_fda_profile(case.validation_profile.as_deref()) {
		return Err(Error::BadRequest {
			message: "case validation_profile must be fda for FDA submission"
				.to_string(),
		});
	}
	if !case.status.eq_ignore_ascii_case("validated") {
		return Err(Error::BadRequest {
			message: "case must be in 'validated' status before FDA submission"
				.to_string(),
		});
	}
	Ok(())
}

pub async fn list_by_case(
	_ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<SubmissionRecord>> {
	let rows = list_submission_rows_by_case(mm, case_id).await?;
	let mut out = Vec::with_capacity(rows.len());
	for row in rows {
		let acks = list_ack_rows(mm, row.id).await?;
		out.push(compose_submission_record(row, acks)?);
	}
	Ok(out)
}

pub async fn get_submission(
	_ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<Option<SubmissionRecord>> {
	let Some(row) = get_submission_row(mm, id).await? else {
		return Ok(None);
	};
	let acks = list_ack_rows(mm, id).await?;
	Ok(Some(compose_submission_record(row, acks)?))
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
	set_compliance_context_dbx(mm.dbx(), ctx.change_reason(), ctx.e_signature_id())
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

	mm.dbx()
		.commit_txn()
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	let row =
		get_submission_row(mm, submission_id)
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
	let system_ctx = Ctx::root_ctx();

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

#[cfg(test)]
mod tests {
	use super::{merge_submission_status, status_from_ack, SubmissionStatus};

	#[test]
	fn ack_status_mapping_success() {
		assert_eq!(
			status_from_ack(1, true).unwrap(),
			SubmissionStatus::Ack1Received
		);
		assert_eq!(
			status_from_ack(2, true).unwrap(),
			SubmissionStatus::Ack2Received
		);
		assert_eq!(
			status_from_ack(3, true).unwrap(),
			SubmissionStatus::Ack3Received
		);
		assert_eq!(
			status_from_ack(4, true).unwrap(),
			SubmissionStatus::Ack4Received
		);
	}

	#[test]
	fn ack_status_mapping_rejected() {
		assert_eq!(
			status_from_ack(2, false).unwrap(),
			SubmissionStatus::Rejected
		);
	}

	#[test]
	fn ack_status_merge_never_regresses() {
		assert_eq!(
			merge_submission_status(
				&SubmissionStatus::Ack3Received,
				&SubmissionStatus::Ack2Received
			),
			SubmissionStatus::Ack3Received
		);
	}

	#[test]
	fn ack_status_merge_respects_terminal() {
		assert_eq!(
			merge_submission_status(
				&SubmissionStatus::Ack4Received,
				&SubmissionStatus::Ack2Received
			),
			SubmissionStatus::Ack4Received
		);
		assert_eq!(
			merge_submission_status(
				&SubmissionStatus::Rejected,
				&SubmissionStatus::Ack4Received
			),
			SubmissionStatus::Rejected
		);
	}
}
