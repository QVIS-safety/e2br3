use super::*;

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
				message: "submission authority must be fda or mfds".to_string(),
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
#[serde(rename_all = "camelCase")]
pub struct SubmissionHistoryRecord {
	pub submission_id: Uuid,
	pub case_id: Uuid,
	pub case_number: String,
	pub gateway: String,
	pub remote_submission_id: String,
	pub status: SubmissionStatus,
	pub batch_result: String,
	pub message_result: Option<String>,
	pub xml_bytes: usize,
	pub submitted_by: Uuid,
	pub submitted_by_email: Option<String>,
	pub submitted_at: String,
	pub latest_ack_received_at: Option<String>,
	pub acknowledged_date: Option<String>,
	pub latest_event_type: Option<String>,
	pub icsr_count: i32,
	pub data_file_name: String,
	pub data_file_download_url: String,
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

#[derive(Debug, Deserialize)]
pub struct MockAckInput {
	pub level: u8,
	#[serde(default = "default_true")]
	pub success: bool,
	pub code: Option<String>,
	pub message: Option<String>,
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

pub(super) fn default_true() -> bool {
	true
}
