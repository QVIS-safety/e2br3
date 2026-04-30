use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::store::{
	set_compliance_context_dbx, set_full_context_dbx,
	set_full_context_dbx_or_rollback,
};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use lib_core::validation::xml::should_skip_xml_validation;
use lib_core::validation::RegulatoryAuthority;
use lib_core::xml::{export_case_xml, validate_e2b_xml, validate_e2b_xml_business};
use lib_rest_core::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::runtime::Handle;
use tokio::task;
use tokio::time::sleep;
use uuid::Uuid;

const SYSTEM_REASON_ACK_CALLBACK: &str =
	"system submission: gateway ack callback processing";
const SYSTEM_REASON_RECONCILE_SCAN: &str =
	"system submission: reconcile due submissions scan";
const SYSTEM_REASON_RECONCILE_RETRY: &str =
	"system submission: reconcile retry dispatch";
const SYSTEM_REASON_RECONCILE_EXPORT: &str =
	"system submission: reconcile retry export";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionAuthority {
	Fda,
	Mfds,
}

impl SubmissionAuthority {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Fda => "fda",
			Self::Mfds => "mfds",
		}
	}

	pub fn parse(raw: &str) -> Option<Self> {
		match raw.trim().to_ascii_lowercase().as_str() {
			"fda" => Some(Self::Fda),
			"mfds" => Some(Self::Mfds),
			_ => None,
		}
	}
}

impl TryFrom<RegulatoryAuthority> for SubmissionAuthority {
	type Error = Error;

	fn try_from(value: RegulatoryAuthority) -> Result<Self> {
		match value {
			RegulatoryAuthority::Fda => Ok(Self::Fda),
			RegulatoryAuthority::Mfds => Ok(Self::Mfds),
			RegulatoryAuthority::Ich => Err(Error::BadRequest {
				message:
					"case validation_profile must be fda or mfds for submission"
						.to_string(),
			}),
		}
	}
}

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
pub struct SubmissionAckDownload {
	pub submission_id: Uuid,
	pub case_id: Uuid,
	pub level: u8,
	pub success: bool,
	pub code: Option<String>,
	pub message: Option<String>,
	pub received_at: OffsetDateTime,
	pub raw_payload: Option<Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionHistoryRecord {
	pub submission_id: Uuid,
	pub case_id: Uuid,
	pub case_number: String,
	pub gateway: String,
	pub remote_submission_id: String,
	pub status: SubmissionStatus,
	pub xml_bytes: usize,
	pub submitted_by: Uuid,
	pub submitted_by_email: Option<String>,
	pub submitted_at: String,
	pub latest_ack_received_at: Option<String>,
	pub latest_event_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionEventRecord {
	pub id: Uuid,
	pub submission_id: Uuid,
	pub event_type: String,
	pub event_data: Option<Value>,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionDispatchStateRecord {
	pub submission_id: Uuid,
	pub attempt_count: i32,
	pub last_attempt_at: Option<OffsetDateTime>,
	pub last_error: Option<String>,
	pub next_retry_at: Option<OffsetDateTime>,
	pub terminal_at: Option<OffsetDateTime>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionReconcileResult {
	pub attempted: usize,
	pub succeeded: usize,
	pub failed: usize,
	pub skipped: usize,
	pub processed_submission_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionReconcileRuntimeStatus {
	pub last_run_at: Option<OffsetDateTime>,
	pub last_success_at: Option<OffsetDateTime>,
	pub last_error: Option<String>,
	pub total_runs: u64,
	pub total_errors: u64,
	pub total_attempted: u64,
	pub total_succeeded: u64,
	pub total_failed: u64,
	pub total_skipped: u64,
}

#[derive(Debug, Default)]
struct ReconcileRuntimeStore {
	last_run_at: Option<OffsetDateTime>,
	last_success_at: Option<OffsetDateTime>,
	last_error: Option<String>,
	total_runs: u64,
	total_errors: u64,
	total_attempted: u64,
	total_succeeded: u64,
	total_failed: u64,
	total_skipped: u64,
}

fn reconcile_runtime_store() -> &'static Mutex<ReconcileRuntimeStore> {
	static STORE: OnceLock<Mutex<ReconcileRuntimeStore>> = OnceLock::new();
	STORE.get_or_init(|| Mutex::new(ReconcileRuntimeStore::default()))
}

fn record_reconcile_result(result: &SubmissionReconcileResult) {
	let now = OffsetDateTime::now_utc();
	let mut store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	store.last_run_at = Some(now);
	store.last_success_at = Some(now);
	store.last_error = None;
	store.total_runs = store.total_runs.saturating_add(1);
	store.total_attempted = store
		.total_attempted
		.saturating_add(result.attempted as u64);
	store.total_succeeded = store
		.total_succeeded
		.saturating_add(result.succeeded as u64);
	store.total_failed = store.total_failed.saturating_add(result.failed as u64);
	store.total_skipped = store.total_skipped.saturating_add(result.skipped as u64);
}

fn record_reconcile_error(err: &str) {
	let now = OffsetDateTime::now_utc();
	let mut store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	store.last_run_at = Some(now);
	store.last_error = Some(err.to_string());
	store.total_runs = store.total_runs.saturating_add(1);
	store.total_errors = store.total_errors.saturating_add(1);
}

pub fn get_reconcile_runtime_status() -> SubmissionReconcileRuntimeStatus {
	let store = reconcile_runtime_store()
		.lock()
		.expect("reconcile runtime stats lock");
	SubmissionReconcileRuntimeStatus {
		last_run_at: store.last_run_at,
		last_success_at: store.last_success_at,
		last_error: store.last_error.clone(),
		total_runs: store.total_runs,
		total_errors: store.total_errors,
		total_attempted: store.total_attempted,
		total_succeeded: store.total_succeeded,
		total_failed: store.total_failed,
		total_skipped: store.total_skipped,
	}
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

#[derive(Debug, Clone, FromRow)]
struct SubmissionAckDownloadRow {
	submission_id: Uuid,
	case_id: Uuid,
	ack_level: i16,
	success: bool,
	ack_code: Option<String>,
	ack_message: Option<String>,
	received_at: OffsetDateTime,
	raw_payload: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
struct SubmissionEventRow {
	id: Uuid,
	submission_id: Uuid,
	event_type: String,
	event_data: Option<Value>,
	created_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
struct SubmissionDispatchStateRow {
	submission_id: Uuid,
	attempt_count: i32,
	last_attempt_at: Option<OffsetDateTime>,
	last_error: Option<String>,
	next_retry_at: Option<OffsetDateTime>,
	terminal_at: Option<OffsetDateTime>,
	created_at: OffsetDateTime,
	updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
struct SubmissionHistoryRow {
	submission_id: Uuid,
	case_id: Uuid,
	case_number: String,
	gateway: String,
	remote_submission_id: String,
	status: String,
	xml_bytes: i32,
	submitted_by: Uuid,
	submitted_by_email: Option<String>,
	submitted_at: OffsetDateTime,
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

fn authority_from_case_profile(
	case_profile: Option<&str>,
) -> Result<SubmissionAuthority> {
	let authority = RegulatoryAuthority::from_case_profile(case_profile)
		.unwrap_or(RegulatoryAuthority::Fda);
	SubmissionAuthority::try_from(authority)
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
			message: format!("unknown submission status: {other}"),
		}),
	}
}

async fn submission_events_table_exists(mm: &ModelManager) -> Result<bool> {
	let row = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Option<String>,)>(
			"SELECT to_regclass('public.submission_events')::text",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(row.0.is_some())
}

async fn submission_dispatch_state_table_exists(mm: &ModelManager) -> Result<bool> {
	let row = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Option<String>,)>(
			"SELECT to_regclass('public.submission_dispatch_state')::text",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	if row.0.is_none() {
		return Ok(false);
	}
	let audit_trigger_fn = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Option<String>,)>(
			"SELECT p.proname::text
			 FROM pg_trigger t
			 JOIN pg_class c ON c.oid = t.tgrelid
			 JOIN pg_namespace n ON n.oid = c.relnamespace
			 JOIN pg_proc p ON p.oid = t.tgfoid
			 WHERE n.nspname = 'public'
			   AND c.relname = 'submission_dispatch_state'
			   AND t.tgname = 'audit_submission_dispatch_state'
			   AND NOT t.tgisinternal
			 LIMIT 1",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let audit_trigger_fn = audit_trigger_fn.0;
	if audit_trigger_fn.is_none() {
		return Ok(true);
	}
	let has_id_column = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (bool,)>(
			"SELECT EXISTS (
				SELECT 1
				FROM information_schema.columns
				WHERE table_schema = 'public'
				  AND table_name = 'submission_dispatch_state'
				  AND column_name = 'id'
			)",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	let has_incompatible_trigger =
		matches!(audit_trigger_fn.as_deref(), Some("audit_trigger_function"));
	if has_incompatible_trigger && !has_id_column.0 {
		eprintln!(
			"submission_dispatch_state disabled: incompatible audit trigger (audit_trigger_function) requires id column"
		);
		return Ok(false);
	}
	Ok(true)
}

async fn submission_idempotency_table_exists(mm: &ModelManager) -> Result<bool> {
	let row = mm
		.dbx()
		.fetch_one(sqlx::query_as::<_, (Option<String>,)>(
			"SELECT to_regclass('public.submission_idempotency')::text",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(row.0.is_some())
}

async fn append_submission_event(
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

async fn upsert_dispatch_state_submit_success(
	mm: &ModelManager,
	submission_id: Uuid,
	attempted_at: OffsetDateTime,
	attempt_count: i32,
) -> Result<()> {
	let res = mm
		.dbx()
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
		.await;
	if let Err(err) = res {
		eprintln!("dispatch state write skipped (submit_success): {err}");
	}
	Ok(())
}

async fn upsert_dispatch_state_submit_failure(
	mm: &ModelManager,
	submission_id: Uuid,
	attempted_at: OffsetDateTime,
	attempt_count: i32,
	last_error: &str,
	next_retry_at: Option<OffsetDateTime>,
) -> Result<()> {
	let res = mm
		.dbx()
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
		.await;
	if let Err(err) = res {
		eprintln!("dispatch state write skipped (submit_failure): {err}");
	}
	Ok(())
}

async fn get_dispatch_attempt_count(
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

async fn find_submission_idempotency(
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

async fn insert_submission_idempotency(
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

async fn mark_dispatch_terminal(
	mm: &ModelManager,
	submission_id: Uuid,
	terminal_at: OffsetDateTime,
) -> Result<()> {
	let res = mm
		.dbx()
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
		.await;
	if let Err(err) = res {
		eprintln!("dispatch state write skipped (terminal): {err}");
	}
	Ok(())
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
	authority: SubmissionAuthority,
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
		let mut req = client.post(&submit_url);
		if let Ok(token) = std::env::var("AS2_SUBMITTER_TOKEN")
			.or_else(|_| std::env::var("AS2_CALLBACK_TOKEN"))
		{
			let token = token.trim();
			if !token.is_empty() {
				req = req
					.header("x-api-token", token)
					.header("x-callback-token", token)
					.header(AUTHORIZATION, format!("Bearer {token}"));
			}
		}
		let resp = req
			.json(&json!({
				"caseId": case_id.to_string(),
				"authority": authority.as_str(),
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
				"{}-MOCK-{}",
				authority.as_str().to_ascii_uppercase(),
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
	if authority != SubmissionAuthority::Fda {
		return Err(Error::BadRequest {
			message:
				"FDA ESG transport only supports authority=fda; configure AS2 for MFDS submissions"
					.to_string(),
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

fn select_gateway_name(authority: SubmissionAuthority) -> Result<String> {
	if as2_submitter_url().is_some() {
		return Ok("as2-submitter-http".to_string());
	}
	if allow_mock_submission() {
		return Ok("fda-esg-nextgen-mock".to_string());
	}
	if !is_esg_enabled() {
		return Err(Error::BadRequest {
			message: "no submission transport configured: set AS2_SUBMITTER_URL or FDA_ESG_ENABLED=1".to_string(),
		});
	}
	if authority != SubmissionAuthority::Fda {
		return Err(Error::BadRequest {
			message:
				"FDA ESG transport only supports authority=fda; configure AS2 for MFDS submissions"
					.to_string(),
		});
	}
	let _ = std::env::var("FDA_ESG_BASE_URL").map_err(|_| Error::BadRequest {
		message: "FDA_ESG_ENABLED=1 requires FDA_ESG_BASE_URL".to_string(),
	})?;
	Ok("fda-esg-nextgen-api".to_string())
}

fn submission_max_attempts() -> u32 {
	std::env::var("SUBMISSION_MAX_ATTEMPTS")
		.ok()
		.and_then(|v| v.trim().parse::<u32>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(1)
}

fn submission_retry_base_ms() -> u64 {
	std::env::var("SUBMISSION_RETRY_BASE_MS")
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(500)
}

fn submission_retry_max_ms() -> u64 {
	std::env::var("SUBMISSION_RETRY_MAX_MS")
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(10_000)
}

fn backoff_ms_for_attempt(attempt_number: u32) -> u64 {
	let base = submission_retry_base_ms();
	let max = submission_retry_max_ms();
	let shift = attempt_number.saturating_sub(1).min(16);
	let pow = 1u64 << shift;
	base.saturating_mul(pow).min(max)
}

fn is_retryable_submit_error(msg: &str) -> bool {
	let lower = msg.to_ascii_lowercase();
	!(lower.contains("missing remote submission identifier")
		|| lower.contains("response is not valid json")
		|| lower.contains("rejected request (")
		|| lower.contains("submit failed ("))
}

struct GatewayDispatchFailure {
	message: String,
	attempts: u32,
	next_retry_at: Option<OffsetDateTime>,
}

async fn submit_to_gateway_with_retry(
	case_id: Uuid,
	xml: &str,
	authority: SubmissionAuthority,
) -> core::result::Result<(GatewaySubmissionOutcome, u32), GatewayDispatchFailure> {
	let max_attempts = submission_max_attempts();
	let mut last_error = "submission failed".to_string();

	for attempt in 1..=max_attempts {
		match submit_to_gateway(case_id, xml, authority).await {
			Ok(outcome) => return Ok((outcome, attempt)),
			Err(err) => {
				last_error = err.to_string();
				let retryable = is_retryable_submit_error(&last_error);
				if attempt >= max_attempts || !retryable {
					let next_retry_at = if retryable {
						Some(
							OffsetDateTime::now_utc()
								+ time::Duration::milliseconds(
									backoff_ms_for_attempt(attempt) as i64,
								),
						)
					} else {
						None
					};
					return Err(GatewayDispatchFailure {
						message: last_error,
						attempts: attempt,
						next_retry_at,
					});
				}
				sleep(Duration::from_millis(backoff_ms_for_attempt(attempt))).await;
			}
		}
	}

	Err(GatewayDispatchFailure {
		message: last_error,
		attempts: max_attempts,
		next_retry_at: None,
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

pub async fn list_submission_history(
	_ctx: &Ctx,
	mm: &ModelManager,
) -> Result<Vec<SubmissionHistoryRecord>> {
	let latest_ack_received_at = list_latest_ack_received_at(mm).await?;
	let latest_event_type = if submission_events_table_exists(mm).await? {
		list_latest_submission_event_types(mm).await?
	} else {
		HashMap::new()
	};
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, SubmissionHistoryRow>(
			"SELECT cs.id AS submission_id,
				        cs.case_id,
				        c.safety_report_id AS case_number,
				        cs.gateway,
				        cs.remote_submission_id,
				        cs.status,
				        cs.xml_bytes,
				        cs.submitted_by,
				        u.email AS submitted_by_email,
				        cs.submitted_at
				   FROM case_submissions cs
				   JOIN cases c ON c.id = cs.case_id
				   LEFT JOIN users u ON u.id = cs.submitted_by
				  ORDER BY cs.submitted_at DESC
				  LIMIT 200",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

	rows.into_iter()
		.map(|row| {
			Ok(SubmissionHistoryRecord {
				submission_id: row.submission_id,
				case_id: row.case_id,
				case_number: row.case_number,
				gateway: row.gateway,
				remote_submission_id: row.remote_submission_id,
				status: status_from_db(&row.status)?,
				xml_bytes: row.xml_bytes as usize,
				submitted_by: row.submitted_by,
				submitted_by_email: row.submitted_by_email,
				submitted_at: format_history_timestamp(row.submitted_at)?,
				latest_ack_received_at: latest_ack_received_at
					.get(&row.submission_id)
					.copied()
					.map(format_history_timestamp)
					.transpose()?,
				latest_event_type: latest_event_type
					.get(&row.submission_id)
					.cloned(),
			})
		})
		.collect()
}

fn format_history_timestamp(value: OffsetDateTime) -> Result<String> {
	value.format(&Rfc3339).map_err(|err| Error::BadRequest {
		message: format!("failed to format submission history timestamp: {err}"),
	})
}

async fn list_latest_ack_received_at(
	mm: &ModelManager,
) -> Result<HashMap<Uuid, OffsetDateTime>> {
	let rows = mm
		.dbx()
		.fetch_all(sqlx::query_as::<_, (Uuid, OffsetDateTime)>(
			"SELECT submission_id, MAX(received_at) AS received_at
				 FROM submission_acks
				 GROUP BY submission_id",
		))
		.await
		.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
	Ok(rows.into_iter().collect())
}

async fn list_latest_submission_event_types(
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
	create_submission(ctx, mm, case_id, SubmissionAuthority::Fda).await
}

pub async fn create_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
) -> Result<SubmissionRecord> {
	assert_case_ready_for_submission(ctx, mm, case_id, authority).await?;

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
		let schema_report =
			validate_e2b_xml(xml.as_bytes(), None).map_err(Error::from)?;
		if !schema_report.ok {
			let preview = schema_report
				.errors
				.iter()
				.take(3)
				.map(|err| err.message.as_str())
				.collect::<Vec<_>>()
				.join("; ");
			return Err(Error::BadRequest {
				message: format!(
					"cannot submit case: XML schema/basic validation failed ({} issue(s)): {}",
					schema_report.errors.len(),
					preview
				),
			});
		}
		let business_report =
			validate_e2b_xml_business(xml.as_bytes(), None).map_err(Error::from)?;
		if !business_report.ok {
			let preview = business_report
				.errors
				.iter()
				.take(3)
				.map(|err| err.message.as_str())
				.collect::<Vec<_>>()
				.join("; ");
			return Err(Error::BadRequest {
				message: format!(
					"cannot submit case: XML business validation failed ({} issue(s)): {}",
					business_report.errors.len(),
					preview
				),
			});
		}
	}

	let now = OffsetDateTime::now_utc();
	let submission_id = Uuid::new_v4();
	let events_enabled = submission_events_table_exists(mm).await?;
	let dispatch_enabled = submission_dispatch_state_table_exists(mm).await?;
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
				ctx.e_signature_id(),
			)
			.await
			.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;

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

			if events_enabled {
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
			}
			if dispatch_enabled {
				upsert_dispatch_state_submit_failure(
					mm,
					submission_id,
					now,
					failure.attempts as i32,
					&failure.message,
					failure.next_retry_at,
				)
				.await?;
			}

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
					 WHERE id = $1
					   AND status = 'validated'",
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
			message: format!(
				"case must be in 'validated' status before {} submission",
				authority.as_str().to_ascii_uppercase()
			),
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
	if events_enabled {
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
	}
	if dispatch_enabled {
		upsert_dispatch_state_submit_success(
			mm,
			submission_id,
			now,
			attempt_count as i32,
		)
		.await?;
	}

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
	if events_enabled {
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
	}
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
	let idempotency_enabled = submission_idempotency_table_exists(mm).await?;
	if normalized_key.is_some() && !idempotency_enabled {
		return Err(Error::BadRequest {
			message:
				"submission idempotency is not available: apply submission_idempotency schema first"
					.to_string(),
		});
	}

	if idempotency_enabled {
		if let Some(key) = normalized_key.as_deref() {
			if let Some(existing_id) =
				find_submission_idempotency(mm, case_id, authority, key).await?
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
	}

	let record = match create_submission(ctx, mm, case_id, authority).await {
		Ok(record) => record,
		Err(err) => {
			if idempotency_enabled
				&& normalized_key.is_some()
				&& is_case_not_validated_for_submission_error(&err)
			{
				if let Some(existing_id) = wait_for_submission_idempotency(
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

	if idempotency_enabled {
		if let Some(key) = normalized_key.as_deref() {
			insert_submission_idempotency(
				mm,
				case_id,
				authority,
				key,
				record.id,
				ctx.user_id(),
			)
			.await?;
		}
	}
	Ok(record)
}

fn is_case_not_validated_for_submission_error(err: &Error) -> bool {
	match err {
		Error::BadRequest { message } => {
			message.contains("case must be in 'validated' status before")
		}
		_ => false,
	}
}

async fn wait_for_submission_idempotency(
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
	key: &str,
) -> Result<Option<Uuid>> {
	for _ in 0..10 {
		if let Some(existing_id) =
			find_submission_idempotency(mm, case_id, authority, key).await?
		{
			return Ok(Some(existing_id));
		}
		sleep(Duration::from_millis(50)).await;
	}
	Ok(None)
}

pub async fn assert_case_ready_for_fda_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	assert_case_ready_for_submission(ctx, mm, case_id, SubmissionAuthority::Fda)
		.await
}

pub async fn assert_case_ready_for_submission(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: SubmissionAuthority,
) -> Result<()> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let case_authority =
		authority_from_case_profile(case.validation_profile.as_deref())?;
	if case_authority != authority {
		let expected = match authority {
			SubmissionAuthority::Fda => "fda",
			SubmissionAuthority::Mfds => "mfds",
		};
		return Err(Error::BadRequest {
			message: format!(
				"case validation_profile must be {expected} for {} submission",
				authority.as_str().to_ascii_uppercase()
			),
		});
	}
	if !case.status.eq_ignore_ascii_case("validated") {
		return Err(Error::BadRequest {
			message: format!(
				"case must be in 'validated' status before {} submission",
				authority.as_str().to_ascii_uppercase()
			),
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
	let events_enabled = submission_events_table_exists(mm).await?;
	let dispatch_enabled = submission_dispatch_state_table_exists(mm).await?;
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
		if events_enabled {
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
		}
	} else if events_enabled {
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
	if events_enabled && merged_status != current_status {
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
	if dispatch_enabled && is_submission_terminal(&merged_status) {
		mark_dispatch_terminal(mm, submission_id, now).await?;
	}

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
	let events_enabled = submission_events_table_exists(mm).await?;
	let dispatch_enabled = submission_dispatch_state_table_exists(mm).await?;
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
		if events_enabled {
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
		}
	} else if events_enabled {
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
	if events_enabled && merged_status != current_status {
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
	if dispatch_enabled && is_submission_terminal(&merged_status) {
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
	_ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Vec<SubmissionEventRecord>> {
	if !submission_events_table_exists(mm).await? {
		return Ok(Vec::new());
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
	Ok(rows
		.into_iter()
		.map(|row| SubmissionEventRecord {
			id: row.id,
			submission_id: row.submission_id,
			event_type: row.event_type,
			event_data: row.event_data,
			created_at: row.created_at,
		})
		.collect())
}

pub async fn get_ack_download(
	_ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
	level: u8,
) -> Result<Option<SubmissionAckDownload>> {
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

	row.map(|row| {
		Ok(SubmissionAckDownload {
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
		})
	})
	.transpose()
}

pub async fn get_submission_dispatch_state(
	_ctx: &Ctx,
	mm: &ModelManager,
	submission_id: Uuid,
) -> Result<Option<SubmissionDispatchStateRecord>> {
	if !submission_dispatch_state_table_exists(mm).await? {
		return Ok(None);
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
	Ok(row.map(|r| SubmissionDispatchStateRecord {
		submission_id: r.submission_id,
		attempt_count: r.attempt_count,
		last_attempt_at: r.last_attempt_at,
		last_error: r.last_error,
		next_retry_at: r.next_retry_at,
		terminal_at: r.terminal_at,
		created_at: r.created_at,
		updated_at: r.updated_at,
	}))
}

pub async fn reconcile_due_submissions(
	mm: &ModelManager,
	limit: i64,
) -> Result<SubmissionReconcileResult> {
	let safe_limit = limit.clamp(1, 100);
	if !submission_dispatch_state_table_exists(mm).await? {
		let result = SubmissionReconcileResult {
			attempted: 0,
			succeeded: 0,
			failed: 0,
			skipped: 0,
			processed_submission_ids: Vec::new(),
		};
		record_reconcile_result(&result);
		return Ok(result);
	}
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

enum ReconcileOutcome {
	Succeeded,
	Failed,
	Skipped,
}

async fn reconcile_one_submission(
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
			let case = match CaseBmc::get(&system_ctx, mm, row.case_id).await {
				Ok(case) => case,
				Err(ModelError::EntityUuidNotFound { .. }) => {
					return Ok(ReconcileOutcome::Skipped);
				}
				Err(e) => return Err(Error::from(e)),
			};
			let authority =
				authority_from_case_profile(case.validation_profile.as_deref())?;

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

			let events_enabled = submission_events_table_exists(mm).await?;
			let dispatch_enabled =
				submission_dispatch_state_table_exists(mm).await?;
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

					if events_enabled {
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
					}
					if dispatch_enabled {
						upsert_dispatch_state_submit_success(
							mm,
							submission_id,
							now,
							prior_attempts + attempts as i32,
						)
						.await?;
					}

					mm.dbx()
						.commit_txn()
						.await
						.map_err(|e| Error::from(lib_core::model::Error::from(e)))?;
					Ok(ReconcileOutcome::Succeeded)
				}
				Err(failure) => {
					if dispatch_enabled {
						mm.dbx().begin_txn().await.map_err(|e| {
							Error::from(lib_core::model::Error::from(e))
						})?;
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
						upsert_dispatch_state_submit_failure(
							mm,
							submission_id,
							now,
							prior_attempts + failure.attempts as i32,
							&failure.message,
							failure.next_retry_at,
						)
						.await?;
						if events_enabled {
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
						}
						mm.dbx().commit_txn().await.map_err(|e| {
							Error::from(lib_core::model::Error::from(e))
						})?;
					}
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
