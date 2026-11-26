use super::*;

#[derive(Debug, Clone, FromRow)]
pub(super) struct CaseSubmissionRow {
	pub(super) id: Uuid,
	pub(super) case_id: Uuid,
	pub(super) gateway: String,
	pub(super) authority: Option<String>,
	pub(super) remote_submission_id: Option<String>,
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
	pub(super) authority: Option<String>,
	pub(super) remote_submission_id: Option<String>,
	pub(super) status: String,
	pub(super) xml_bytes: i32,
	pub(super) submitted_by: Uuid,
	pub(super) submitted_by_email: Option<String>,
	pub(super) submitted_at: OffsetDateTime,
}

#[derive(Debug)]
pub(super) struct GatewaySubmissionOutcome {
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
}

pub(super) fn is_esg_configured() -> Result<bool> {
	crate::config::env_non_empty("FDA_ESG_BASE_URL").map_err(|message| {
		Error::Configuration {
			key: "FDA_ESG_BASE_URL",
			message,
		}
	})
}

pub(super) fn as2_submitter_url() -> Result<Option<String>> {
	match std::env::var("AS2_SUBMITTER_URL") {
		Ok(value) if !value.trim().is_empty() => Ok(Some(value.trim().to_string())),
		Err(std::env::VarError::NotPresent) => Ok(None),
		_ => Err(Error::Configuration {
			key: "AS2_SUBMITTER_URL",
			message: "AS2_SUBMITTER_URL must be nonempty UTF-8".into(),
		}),
	}
}

pub(super) fn parse_timeout_secs(name: &'static str) -> Result<u64> {
	let value = std::env::var(name).map_err(|error| Error::Configuration {
		key: name,
		message: error.to_string(),
	})?;
	let seconds =
		value
			.trim()
			.parse::<u64>()
			.map_err(|error| Error::Configuration {
				key: name,
				message: error.to_string(),
			})?;
	if seconds == 0 {
		return Err(Error::Configuration {
			key: name,
			message: "timeout must be positive".into(),
		});
	}
	Ok(seconds)
}

pub(super) fn status_to_db(status: &SubmissionStatus) -> &'static str {
	match status {
		SubmissionStatus::DispatchFailed => "dispatch_failed",
		SubmissionStatus::DispatchUnknown => "dispatch_unknown",
		SubmissionStatus::SubmittedAck1Pending => "submitted_ack1_pending",
		SubmissionStatus::Ack1Received => "ack1_received",
		SubmissionStatus::Ack2Received => "ack2_received",
		SubmissionStatus::Ack3Received => "ack3_received",
		SubmissionStatus::Ack4Received => "ack4_received",
		SubmissionStatus::Rejected => "rejected",
	}
}

pub(super) fn status_from_db(status: &str) -> Result<SubmissionStatus> {
	match status.trim().to_ascii_lowercase().as_str() {
		"dispatch_failed" => Ok(SubmissionStatus::DispatchFailed),
		"dispatch_unknown" => Ok(SubmissionStatus::DispatchUnknown),
		"submitted_ack1_pending" => Ok(SubmissionStatus::SubmittedAck1Pending),
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

#[cfg(test)]
mod strict_config_tests {
	use super::*;

	#[test]
	fn timeout_requires_an_explicit_positive_integer() {
		const KEY: &str = "E2BR3_TEST_STRICT_TIMEOUT";
		std::env::remove_var(KEY);
		assert!(parse_timeout_secs(KEY).is_err());
		for value in ["", "broken", "0", "-1"] {
			std::env::set_var(KEY, value);
			assert!(parse_timeout_secs(KEY).is_err(), "{value}");
		}
		std::env::set_var(KEY, "17");
		assert_eq!(parse_timeout_secs(KEY).unwrap(), 17);
		std::env::remove_var(KEY);
	}
}
