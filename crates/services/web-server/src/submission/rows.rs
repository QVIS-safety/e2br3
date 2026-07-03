use super::*;

#[derive(Debug, Clone, FromRow)]
pub(super) struct CaseSubmissionRow {
	pub(super) id: Uuid,
	pub(super) case_id: Uuid,
	pub(super) gateway: String,
	pub(super) remote_submission_id: String,
	pub(super) status: String,
	pub(super) xml_bytes: i32,
	pub(super) submitted_by: Uuid,
	pub(super) submitted_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SubmissionAckRow {
	pub(super) ack_level: i16,
	pub(super) success: bool,
	pub(super) ack_code: Option<String>,
	pub(super) ack_message: Option<String>,
	pub(super) received_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct LatestSubmissionAckRow {
	pub(super) submission_id: Uuid,
	pub(super) ack_level: i16,
	pub(super) success: bool,
	pub(super) ack_code: Option<String>,
	pub(super) ack_message: Option<String>,
	pub(super) received_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SubmissionAckDownloadRow {
	pub(super) submission_id: Uuid,
	pub(super) case_id: Uuid,
	pub(super) ack_level: i16,
	pub(super) success: bool,
	pub(super) ack_code: Option<String>,
	pub(super) ack_message: Option<String>,
	pub(super) received_at: OffsetDateTime,
	pub(super) raw_payload: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SubmissionEventRow {
	pub(super) id: Uuid,
	pub(super) submission_id: Uuid,
	pub(super) event_type: String,
	pub(super) event_data: Option<Value>,
	pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SubmissionDispatchStateRow {
	pub(super) submission_id: Uuid,
	pub(super) attempt_count: i32,
	pub(super) last_attempt_at: Option<OffsetDateTime>,
	pub(super) last_error: Option<String>,
	pub(super) next_retry_at: Option<OffsetDateTime>,
	pub(super) terminal_at: Option<OffsetDateTime>,
	pub(super) created_at: OffsetDateTime,
	pub(super) updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SubmissionHistoryRow {
	pub(super) submission_id: Uuid,
	pub(super) case_id: Uuid,
	pub(super) case_number: String,
	pub(super) gateway: String,
	pub(super) remote_submission_id: String,
	pub(super) status: String,
	pub(super) xml_bytes: i32,
	pub(super) submitted_by: Uuid,
	pub(super) submitted_by_email: Option<String>,
	pub(super) submitted_at: OffsetDateTime,
}

#[derive(Debug)]
pub(super) struct GatewaySubmissionOutcome {
	pub(super) gateway: String,
	pub(super) remote_submission_id: String,
	pub(super) ack1: SubmissionAck,
}

#[derive(Debug, Deserialize)]
pub(super) struct EsgSubmitResponse {
	pub(super) remote_submission_id: Option<String>,
	pub(super) submission_id: Option<String>,
	pub(super) id: Option<String>,
	pub(super) ack: Option<EsgAckResponse>,
}

#[derive(Debug, Deserialize)]
pub(super) struct EsgAckResponse {
	pub(super) level: Option<u8>,
	pub(super) success: Option<bool>,
	pub(super) code: Option<String>,
	pub(super) message: Option<String>,
	pub(super) received_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct As2SubmitResponse {
	pub(super) remote_submission_id: Option<String>,
	pub(super) submission_id: Option<String>,
	pub(super) status: Option<String>,
	pub(super) authority: Option<String>,
}

pub(super) fn env_truthy(name: &str) -> bool {
	matches!(
		std::env::var(name),
		Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
	)
}

pub(super) fn is_esg_enabled() -> bool {
	env_truthy("FDA_ESG_ENABLED")
}

pub(super) fn allow_mock_submission() -> bool {
	env_truthy("E2BR3_ALLOW_MOCK_SUBMISSION")
}

pub(super) fn as2_submitter_url() -> Option<String> {
	std::env::var("AS2_SUBMITTER_URL").ok().and_then(|v| {
		let trimmed = v.trim();
		if trimmed.is_empty() {
			None
		} else {
			Some(trimmed.to_string())
		}
	})
}

pub(super) fn parse_timeout_secs(name: &str, default_secs: u64) -> u64 {
	std::env::var(name)
		.ok()
		.and_then(|v| v.trim().parse::<u64>().ok())
		.filter(|v| *v > 0)
		.unwrap_or(default_secs)
}

pub(super) fn submission_history_export_authority(gateway: &str) -> &'static str {
	let gateway = gateway.to_ascii_lowercase();
	if gateway.contains("mfds") {
		return SubmissionAuthority::Mfds.as_str();
	}
	if gateway.contains("fda") || gateway.contains("esg") {
		return SubmissionAuthority::Fda.as_str();
	}
	SubmissionAuthority::Fda.as_str()
}

pub(super) fn status_to_db(status: &SubmissionStatus) -> &'static str {
	match status {
		SubmissionStatus::Ack1Received => "ack1_received",
		SubmissionStatus::Ack2Received => "ack2_received",
		SubmissionStatus::Ack3Received => "ack3_received",
		SubmissionStatus::Ack4Received => "ack4_received",
		SubmissionStatus::Rejected => "rejected",
	}
}

pub(super) fn status_from_db(status: &str) -> Result<SubmissionStatus> {
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
