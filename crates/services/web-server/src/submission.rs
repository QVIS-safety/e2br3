use lib_core::ctx::Ctx;
use lib_core::model::case::CaseBmc;
use lib_core::model::presave::{
	ProductPresaveBmc, ReceiverPresaveBmc, ReceiverPresaveRouteBmc,
	SenderPresaveBmc, SenderPresaveGateway, SenderPresaveGatewayBmc,
};
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SenderInformationBmc, SenderInformationFilter,
};
use lib_core::model::store::{
	set_compliance_context_dbx, set_full_context_dbx,
	set_full_context_dbx_or_rollback,
};
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_rest_core::{Error, Result};
use modql::filter::{OpValValue, OpValsValue};
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
		prepare_outbound_message_header(ctx, mm, case_id, authority).await?;
	let xml = export_case_xml_with_options(
		ctx,
		mm,
		case_id,
		ExportXmlOptions {
			apply_comments: true,
			authority,
			outbound_message_header: header,
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

async fn submission_sender_gateway(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
) -> Result<SenderPresaveGateway> {
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
	gateways
		.into_iter()
		.next()
		.ok_or_else(|| Error::BadRequest {
			message: "case Sender gateway is missing".into(),
		})
}

pub async fn prepare_outbound_message_header(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
) -> Result<OutboundMessageHeader> {
	submission_sender_gateway(ctx, mm, case_id, authority).await?;
	let header = prepare_export_message_header(ctx, mm, case_id, authority).await?;
	for (field, value) in [
		("C.1.1 Safety Report ID", header.message_number.as_deref()),
		("C.1.2 Date of Creation", header.message_date.as_deref()),
		(
			"Sender template Sender ID",
			header.message_sender_identifier.as_deref(),
		),
		(
			"Receiver template batch receiver identifier",
			header.batch_receiver_identifier.as_deref(),
		),
		(
			"Receiver template message receiver identifier",
			header.message_receiver_identifier.as_deref(),
		),
	] {
		required_header_value(field, value)?;
	}
	Ok(header)
}

// Downloads preserve incomplete data; only submission requires a complete header.
pub(crate) async fn prepare_export_message_header(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	authority: RegulatoryAuthority,
) -> Result<OutboundMessageHeader> {
	let case = CaseBmc::get(ctx, mm, case_id).await?;
	let report =
		SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id).await?;
	let now = OffsetDateTime::now_utc();
	let mut header = OutboundMessageHeader {
		batch_number: Some(format!("BATCH-{case_id}")),
		batch_sender_identifier: None,
		batch_receiver_identifier: None,
		batch_transmission_date: Some(format!(
			"{}+0000",
			lib_utils::time::format_e2b_timestamp(
				now.to_offset(time::UtcOffset::UTC)
			)
		)),
		message_number: report.safety_report_id,
		message_date: report.transmission_date,
		message_sender_identifier: None,
		message_receiver_identifier: None,
	};
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
	let [sender] = senders.as_slice() else {
		if senders.len() > 1 {
			tracing::warn!(%case_id, "XML export omits routing identifiers for ambiguous Case Senders");
		}
		return Ok(header);
	};
	let Some(sender_presave_id) = sender.source_sender_presave_id else {
		return Ok(header);
	};
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
						.eq_ignore_ascii_case(authority.as_str())
			})
			.collect::<Vec<_>>();
	if let [gateway] = gateways.as_slice() {
		let sender_identifier = gateway
			.sender_identifier
			.as_deref()
			.map(str::trim)
			.map(str::to_owned);
		header.batch_sender_identifier = sender_identifier.clone();
		header.message_sender_identifier = sender_identifier;
	} else if gateways.len() > 1 {
		tracing::warn!(%case_id, "XML export omits ambiguous Sender gateway identifiers");
	}
	let Some(product_id) = case
		.dg_prd_key
		.as_deref()
		.filter(|value| !value.trim().is_empty())
	else {
		return Ok(header);
	};
	let products = ProductPresaveBmc::list_by_product_and_sender(
		ctx,
		mm,
		product_id,
		sender_presave_id,
	)
	.await?;
	let [product] = products.as_slice() else {
		if products.len() > 1 {
			tracing::warn!(%case_id, "XML export omits Receiver identifiers for ambiguous Product templates");
		}
		return Ok(header);
	};
	let Some(receiver_id) = product.receiver_presave_id else {
		return Ok(header);
	};
	let receiver = ReceiverPresaveBmc::get(ctx, mm, receiver_id).await?;
	if receiver.deleted {
		return Ok(header);
	}
	let (condition_field_code, report_type) = match authority {
		RegulatoryAuthority::Ich => ("C.1.3", report.report_type.as_deref()),
		RegulatoryAuthority::Fda => {
			("FDA_REPORT_TYPE", case.fda_report_type.as_deref())
		}
		RegulatoryAuthority::Mfds => {
			("MFDS_REPORT_TYPE", case.mfds_report_type.as_deref())
		}
	};
	let Some(report_type) = report_type.filter(|value| !value.trim().is_empty())
	else {
		return Ok(header);
	};
	let routes = ReceiverPresaveRouteBmc::list_by_parent(ctx, mm, receiver_id)
		.await?
		.into_iter()
		.filter(|route| {
			route.authority.eq_ignore_ascii_case(authority.as_str())
				&& route.condition_page.eq_ignore_ascii_case("CI")
				&& route.condition_field_code == condition_field_code
				&& route.condition_operator.eq_ignore_ascii_case("Equal")
				&& route.condition_value_code == report_type
		})
		.collect::<Vec<_>>();
	if let [route] = routes.as_slice() {
		header.batch_receiver_identifier = route.batch_receiver_identifier.clone();
		header.message_receiver_identifier =
			Some(route.message_receiver_identifier.clone());
	} else if routes.len() > 1 {
		tracing::warn!(%case_id, "XML export omits ambiguous Receiver route identifiers");
	}
	Ok(header)
}

fn required_header_value(field: &str, value: Option<&str>) -> Result<String> {
	value
		.filter(|value| !value.trim().is_empty())
		.map(str::to_string)
		.ok_or_else(|| Error::BadRequest {
			message: format!("{field} is required before submission"),
		})
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
