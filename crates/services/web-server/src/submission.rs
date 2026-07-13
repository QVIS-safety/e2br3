use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::store::{
	set_compliance_context_dbx, set_full_context_dbx,
	set_full_context_dbx_or_rollback,
};
use lib_core::model::Error as ModelError;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_core::xml::export_case_xml;
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
use validator::xml::{
	should_skip_xml_validation, validate_e2b_xml, validate_e2b_xml_business,
};

const SYSTEM_REASON_ACK_CALLBACK: &str =
	"system submission: gateway ack callback processing";
const SYSTEM_REASON_RECONCILE_SCAN: &str =
	"system submission: reconcile due submissions scan";
const SYSTEM_REASON_RECONCILE_RETRY: &str =
	"system submission: reconcile retry dispatch";
const SYSTEM_REASON_RECONCILE_EXPORT: &str =
	"system submission: reconcile retry export";

mod ack;
mod create;
mod gateway;
mod persistence;
mod reconcile;
mod reconcile_runtime;
mod rows;
mod types;

#[cfg(test)]
mod tests;

pub use ack::{
	apply_gateway_ack_by_remote, apply_mock_ack, get_ack_download,
	get_submission_dispatch_state, list_submission_events,
};
#[allow(unused_imports)]
pub use create::{
	assert_case_ready_for_fda_submission, assert_case_ready_for_submission,
	create_fda_submission, create_submission, create_submission_idempotent,
	get_submission, list_by_case,
};
pub use persistence::list_submission_history;
#[allow(unused_imports)]
pub use reconcile::{
	reconcile_due_submissions, reconcile_due_submissions_with_runtime_status,
};
pub use reconcile_runtime::get_reconcile_runtime_status;
pub use types::{
	GatewayAckCallbackInput, MockAckInput, SubmissionAck, SubmissionAckDownload,
	SubmissionAuthority, SubmissionDispatchStateRecord, SubmissionEventRecord,
	SubmissionHistoryRecord, SubmissionReconcileResult,
	SubmissionReconcileRuntimeStatus, SubmissionRecord, SubmissionStatus,
};

use gateway::{select_gateway_name, submit_to_gateway_with_retry};
use persistence::{
	ack_event_exists, append_submission_event, compose_submission_record,
	find_submission_idempotency, get_dispatch_attempt_count, get_submission_row,
	get_submission_row_for_ctx, insert_submission_idempotency, list_ack_rows,
	list_submission_rows_by_case, mark_dispatch_terminal,
	upsert_dispatch_state_submit_failure, upsert_dispatch_state_submit_success,
};
use reconcile_runtime::{record_reconcile_error, record_reconcile_result};
use rows::*;
use rows::{status_from_db, status_to_db};
