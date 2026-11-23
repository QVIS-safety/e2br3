use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::message_header::{
	MessageHeader, MessageHeaderBmc, MessageHeaderForCreate,
};
use lib_core::model::presave::SenderPresaveGatewayBmc;
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SenderInformationBmc, SenderInformationFilter,
};
use lib_core::model::store::{
	set_compliance_context_dbx, set_full_context_dbx,
	set_full_context_dbx_or_rollback,
};
use lib_core::model::submission_receiver_option::SubmissionReceiverOptionBmc;
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::{Error, Result};
use modql::filter::{OpValValue, OpValsValue};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;
use xml::validation::{validate_e2b_xml, XmlValidatorConfig};
use xml::{export_case_xml_with_options, ExportXmlOptions, OutboundMessageHeader};

const SYSTEM_REASON_ACK_CALLBACK: &str =
	"system submission: gateway ack callback processing";
const SYSTEM_REASON_RECONCILE_SCAN: &str =
	"system submission: reconcile due submissions scan";
const SYSTEM_REASON_RECONCILE_RETRY: &str =
	"system submission: reconcile retry dispatch";

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
	apply_gateway_ack_by_remote, get_ack_download, get_submission_dispatch_state,
	list_submission_events,
};
pub use create::{
	create_submission_idempotent, dispatch_reserved_submission, get_submission,
	get_submission_artifact, list_by_case, PreparedSubmission,
};
pub use persistence::list_submission_history;
pub use reconcile::reconcile_due_submissions_with_runtime_status;
pub use reconcile_runtime::get_reconcile_runtime_status;
pub use types::{
	GatewayAckCallbackInput, SubmissionAck, SubmissionAckDownload,
	SubmissionAuthority, SubmissionDispatchStateRecord, SubmissionEventRecord,
	SubmissionHistoryRecord, SubmissionReconcileResult,
	SubmissionReconcileRuntimeStatus, SubmissionRecord, SubmissionStatus,
};

// Both initial submission and automatic retry must validate the current case.
async fn prepare_submission_xml(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
) -> Result<String> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	if matches!(
		case.status.trim().to_ascii_lowercase().as_str(),
		"locked" | "deleted" | "archived"
	) {
		return Err(Error::BadRequest {
			message: format!("Cannot submit a {} case.", case.status),
		});
	}
	let report =
		validator::validate_case_for_authority(ctx, mm, case_id, authority).await?;
	check_submission_validation(&report)?;
	let header =
		prepare_outbound_message_header(ctx, mm, case_id, authority, None).await?;
	let xml = export_case_xml_with_options(
		ctx,
		mm,
		case_id,
		ExportXmlOptions {
			apply_comments: true,
			include_source_documents: false,
			authority,
			outbound_message_header: export_message_header(&header)?,
		},
	)
	.await
	.map_err(Error::from)?;
	let schema_report = validate_e2b_xml(
		xml.as_bytes(),
		Some(XmlValidatorConfig {
			authority: Some(authority),
			..XmlValidatorConfig::default()
		}),
	)
	.map_err(Error::from)?;
	if !schema_report.ok {
		let messages = schema_report
			.errors
			.iter()
			.take(3)
			.map(|issue| issue.message.as_str())
			.collect::<Vec<_>>()
			.join("; ");
		return Err(Error::BadRequest {
			message: format!("Cannot submit: XML validation failed. {messages}"),
		});
	}
	Ok(xml)
}

fn check_submission_validation(
	report: &validator::CaseValidationReport,
) -> Result<()> {
	let issues = &report.issues;
	if issues.is_empty() {
		return Ok(());
	}
	let messages = issues
		.iter()
		.take(3)
		.map(|issue| format!("[{}] {}", issue.section, issue.message))
		.collect::<Vec<_>>()
		.join("; ");
	Err(Error::BadRequest {
		message: format!(
			"Cannot submit: {} validation issue(s). Correct the highlighted case fields. {}",
			issues.len(), messages
		),
	})
}

pub async fn prepare_outbound_message_header(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
	receiver_label: Option<&str>,
) -> Result<MessageHeader> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let report =
		SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await?;
	let message_number = required_header_value(
		"C.1.1 Safety Report ID",
		report.safety_report_id.as_deref(),
	)?;
	let message_date = required_header_value(
		"C.1.2 Date of Creation",
		report.transmission_date.as_deref(),
	)?;

	let senders = SenderInformationBmc::list(
		ctx,
		mm,
		Some(vec![SenderInformationFilter {
			case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				case_id.to_string()
			))])),
		}]),
		None,
	)
	.await?;
	if senders.len() != 1 {
		return Err(Error::BadRequest {
			message: "case must contain exactly one Sender information row"
				.to_string(),
		});
	}
	let sender_presave_id =
		senders[0]
			.source_sender_presave_id
			.ok_or_else(|| Error::BadRequest {
				message: "case Sender must reference a Sender template".to_string(),
			})?;
	let authority_name = authority.as_str();
	let gateways =
		SenderPresaveGatewayBmc::list_by_parent(ctx, mm, sender_presave_id)
			.await?
			.into_iter()
			.filter(|gateway| {
				!gateway.deleted
					&& gateway.is_default_for_authority
					&& gateway
						.gateway_authority
						.trim()
						.eq_ignore_ascii_case(authority_name)
					&& gateway
						.sender_identifier
						.as_deref()
						.is_some_and(|value| !value.trim().is_empty())
			})
			.collect::<Vec<_>>();
	if gateways.len() != 1 {
		return Err(Error::BadRequest {
			message: format!(
				"case Sender must have exactly one default {authority_name} gateway Sender ID"
			),
		});
	}
	let sender_identifier = gateways[0]
		.sender_identifier
		.as_deref()
		.map(str::trim)
		.ok_or_else(|| Error::BadRequest {
			message: "gateway sender identifier is missing".into(),
		})?
		.to_string();

	let (batch_receiver_identifier, message_receiver_identifier) = match authority {
		RegulatoryAuthority::Ich => {
			let receiver =
				required_env_identifier("E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH")?;
			(receiver.clone(), receiver)
		}
		RegulatoryAuthority::Fda | RegulatoryAuthority::Mfds => {
			let report_type = match authority {
				RegulatoryAuthority::Fda => case.fda_report_type.as_deref(),
				RegulatoryAuthority::Mfds => case.mfds_report_type.as_deref(),
				RegulatoryAuthority::Ich => unreachable!(),
			};
			let report_type = required_header_value(
				&format!("{authority_name} report type"),
				report_type,
			)?;
			let options = SubmissionReceiverOptionBmc::list_by_authority(
				ctx,
				mm,
				authority_name,
			)
			.await?;
			let matches = options
				.into_iter()
				.filter(|option| {
					option.condition_value_code == report_type
						&& receiver_label.is_none_or(|label| {
							option.receiver_label == label.trim()
						})
				})
				.collect::<Vec<_>>();
			if matches.len() != 1 {
				return Err(Error::BadRequest {
					message: format!(
						"expected exactly one configured {authority_name} receiver for report type {report_type}; found {}",
						matches.len()
					),
				});
			}
			let receiver = &matches[0];
			(
				required_header_value(
					"configured batch receiver identifier",
					Some(&receiver.batch_receiver_identifier),
				)?,
				required_header_value(
					"configured message receiver identifier",
					Some(&receiver.message_receiver_identifier),
				)?,
			)
		}
	};

	let now = OffsetDateTime::now_utc();
	MessageHeaderBmc::upsert_outbound(
		ctx,
		mm,
		MessageHeaderForCreate {
			case_id,
			batch_sender_identifier: Some(sender_identifier.clone()),
			batch_receiver_identifier: Some(batch_receiver_identifier),
			batch_transmission_date: Some(now),
			message_number,
			message_sender_identifier: sender_identifier,
			message_receiver_identifier,
			message_date,
		},
	)
	.await
	.map_err(Error::from)
}

pub fn export_message_header(
	header: &MessageHeader,
) -> Result<OutboundMessageHeader> {
	Ok(OutboundMessageHeader {
		batch_number: header.batch_number.clone(),
		batch_sender_identifier: header.batch_sender_identifier.clone(),
		batch_receiver_identifier: header.batch_receiver_identifier.clone(),
		batch_transmission_date: match (&header.batch_transmission_date_raw, header.batch_transmission_date) {
			(Some(raw), None) => Some(raw.clone()),
			(None, Some(date)) => {
				let date = date.to_offset(time::UtcOffset::UTC);
				let mut timestamp = lib_utils::time::format_e2b_timestamp(date);
				if date.nanosecond() != 0 {
					timestamp.push('.');
					timestamp.push_str(format!("{:09}", date.nanosecond()).trim_end_matches('0'));
				}
				timestamp.push_str("+0000");
				Some(timestamp)
			},
			(None, None) => None,
			(Some(_), Some(_)) => return Err(Error::BadRequest { message: "N.1.5 Batch Transmission Date has conflicting stored representations".to_string() }),
		},
		message_number: header.message_number.clone(),
		message_date: header.message_date.clone(),
		message_sender_identifier: header.message_sender_identifier.clone(),
		message_receiver_identifier: header.message_receiver_identifier.clone(),
	})
}

fn required_header_value(field: &str, value: Option<&str>) -> Result<String> {
	value
		.filter(|value| !value.trim().is_empty())
		.map(str::to_string)
		.ok_or_else(|| Error::BadRequest {
			message: format!("{field} is required before submission"),
		})
}

fn required_env_identifier(name: &'static str) -> Result<String> {
	let value = std::env::var(name).map_err(|_| Error::Configuration {
		key: name,
		message: format!("{name} must be configured before XML export"),
	})?;
	let value = value.trim();
	if value.is_empty() {
		return Err(Error::Configuration {
			key: name,
			message: format!("{name} must not be empty"),
		});
	}
	Ok(value.to_string())
}

#[cfg(test)]
mod header_tests {
	#[test]
	fn imported_timestamp_is_not_a_silently_ignored_public_write() {
		use lib_core::model::message_header::MessageHeaderForUpdate;
		assert!(serde_json::from_value::<MessageHeaderForUpdate>(
			serde_json::json!({"batch_transmission_date_raw": "2024"})
		)
		.is_err());
		assert!(serde_json::from_value::<MessageHeaderForUpdate>(
			serde_json::json!({"batch_number": "BATCH"})
		)
		.is_ok());
	}
	use super::*;

	fn valid_header() -> MessageHeader {
		let now = OffsetDateTime::now_utc();
		MessageHeader {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			batch_number: Some("BATCH".to_string()),
			batch_sender_identifier: Some("SENDER".to_string()),
			batch_receiver_identifier: Some("RECEIVER".to_string()),
			batch_transmission_date: Some(now),
			message_type: "ichicsr".to_string(),
			batch_transmission_date_raw: None,
			message_format_version: "2.1".to_string(),
			message_format_release: "2.0".to_string(),
			message_number: "CASE".to_string(),
			message_sender_identifier: "SENDER".to_string(),
			message_receiver_identifier: "RECEIVER".to_string(),
			message_date_format: "204".to_string(),
			message_date: "20260814000000".to_string(),
			created_at: now,
			updated_at: now,
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn export_preserves_imported_batch_timestamp_precision_and_offset() {
		for raw in [
			"2024",
			"202402",
			"20240229123059.1234+0900",
			"20241121010203-0400",
		] {
			let mut header = valid_header();
			header.batch_transmission_date = None;
			header.batch_transmission_date_raw = Some(raw.to_string());
			assert_eq!(
				export_message_header(&header)
					.expect("stored raw timestamp")
					.batch_transmission_date
					.as_deref(),
				Some(raw)
			);
		}
		let mut header = valid_header();
		header.batch_transmission_date_raw = Some("2024".to_string());
		assert!(
			export_message_header(&header).is_err(),
			"ambiguous representations must fail"
		);
		header.batch_transmission_date_raw = None;
		header.batch_transmission_date = None;
		assert_eq!(
			export_message_header(&header)
				.expect("incomplete download")
				.batch_transmission_date,
			None
		);
	}

	#[test]
	fn export_preserves_typed_batch_timestamp_fraction() {
		for (input, expected) in [
			(
				"2024-02-29T12:30:59.123456789+09:00",
				"20240229033059.123456789+0000",
			),
			("2024-02-29T12:30:59.000001Z", "20240229123059.000001+0000"),
			("2024-02-29T12:30:59Z", "20240229123059+0000"),
		] {
			let mut header = valid_header();
			header.batch_transmission_date = Some(
				OffsetDateTime::parse(
					input,
					&time::format_description::well_known::Rfc3339,
				)
				.unwrap(),
			);
			assert_eq!(
				export_message_header(&header)
					.unwrap()
					.batch_transmission_date
					.as_deref(),
				Some(expected)
			);
		}
	}

	#[test]
	fn export_reads_message_identity_and_date_without_report_substitution() {
		let mut header = valid_header();
		header.message_number = " MESSAGE-ID ".to_string();
		header.message_date = "20240229123059.1234+0900".to_string();
		let exported = export_message_header(&header).unwrap();
		assert_eq!(exported.message_number, header.message_number);
		assert_eq!(exported.message_date, header.message_date);
	}

	#[test]
	fn incomplete_generated_header_never_reaches_xml_export() {
		for field in [
			"batch_number",
			"batch_sender_identifier",
			"batch_receiver_identifier",
			"message_sender_identifier",
			"message_receiver_identifier",
			"message_number",
			"message_date",
		] {
			let mut header = valid_header();
			match field {
				"batch_number" => header.batch_number = None,
				"batch_sender_identifier" => header.batch_sender_identifier = None,
				"batch_receiver_identifier" => {
					header.batch_receiver_identifier = None
				}
				"message_sender_identifier" => {
					header.message_sender_identifier.clear()
				}
				"message_receiver_identifier" => {
					header.message_receiver_identifier.clear()
				}
				"message_number" => header.message_number.clear(),
				"message_date" => header.message_date.clear(),
				_ => unreachable!(),
			}
			assert!(
				export_message_header(&header).is_err(),
				"{field} unexpectedly reached XML export"
			);
		}
	}
}

use gateway::select_gateway_name;
use persistence::{
	ack_event_exists, append_submission_event, compose_submission_record,
	find_submission_idempotency, get_submission_row, insert_submission_idempotency,
	list_ack_rows, list_submission_rows_by_case, mark_dispatch_terminal,
	upsert_dispatch_state_submit_failure,
};
use reconcile_runtime::{record_reconcile_error, record_reconcile_result};
use rows::*;
use rows::{status_from_db, status_to_db};

use create::persist_submission_observation;
use gateway::{request_as2_state, As2GatewaySubmitResponse};

use ack::{
	is_submission_terminal, merge_submission_status, status_from_ack,
	submission_status_rank,
};
