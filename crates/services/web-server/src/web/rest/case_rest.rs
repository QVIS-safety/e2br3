use crate::web::rest::compliance::{
	capture_e_signature, ComplianceActionInput, ESignatureInput,
};
use crate::web::rest::{
	case_editor_dto::CaseFollowUpPagePatchRequest,
	case_editor_rest::apply_follow_up_page_patches,
};
use axum::extract::{Path, State};
use axum::Json;
use lib_core::authorization::EnforcedScopeFilter;
use lib_core::ctx::Ctx;
use lib_core::model::authorization::CaseMutationKind;
use lib_core::model::case::{
	is_allowed_case_status_transition, is_valid_case_status,
	update_touches_non_status_fields, Case, CaseBmc, CaseFilter,
	CaseForCreate as InternalCaseForCreate, CaseForUpdate as InternalCaseForUpdate,
	CaseLinkOption, CaseListViewRow,
};
use lib_core::model::case_numbering::generate_case_number;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::presave::{ReceiverPresave, ReceiverPresaveBmc};
use lib_core::model::reaction::{Reaction, ReactionBmc};
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SafetyReportIdentificationForCreate,
};
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_core::report_due::{
	classify_report, report_due_date, ReceiverTimeline, ReportCategory,
};
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_rest_core::{
	case_read_decisions_with_workflow, case_write_block_reason_for_case,
	load_workflow_runtime_settings, qc_state_for_case_status,
	workflow_actionability_for_case, WorkflowActionability, WorkflowBlockReason,
	WorkflowRuntimeSettings,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::{types::time::OffsetDateTime, FromRow};
use uuid::Uuid;

const SYSTEM_VALIDATION_REASON_VALIDATOR: &str =
	"system validation: validator mark-validated endpoint";
const FDA_REPORT_TYPE_VALUES: &[&str] = &["1", "2", "3", "4"];
const REVIEW_RECEIVER_MAX_LEN: usize = 128;
const REVIEW_RECEIVER_ROW_FIELDS: &[&str] = &[
	"receiver",
	"receiverName",
	"receiver_name",
	"reportDue",
	"report_due",
	"reportDueDate",
	"report_due_date",
	"reportedDate",
	"reported_date",
];

#[derive(Default)]
struct ReviewReceiverRequirements {
	needs_report_due_default: bool,
	needs_report_due_date: bool,
}

// -- Public helpers (used by sibling modules)

pub fn parse_authority_or_bad_request(value: &str) -> Result<RegulatoryAuthority> {
	RegulatoryAuthority::parse(value).ok_or_else(|| Error::BadRequest {
		message: format!("invalid authority '{value}' (expected: ich, fda or mfds)"),
	})
}

pub fn validate_case_create_payload(data: &InternalCaseForCreate) -> Result<()> {
	validate_fda_report_type(data.fda_report_type.as_deref())?;

	if let Some(status) = data.status.as_deref() {
		if !is_valid_case_status(status) {
			return Err(Error::BadRequest {
				message: format!("invalid case status '{status}'"),
			});
		}
		if !status.trim().eq_ignore_ascii_case("draft") {
			return Err(Error::BadRequest {
				message: "case creation only accepts draft status; use the dedicated lifecycle action after creation".to_string(),
			});
		}
	}

	Ok(())
}

// -- Private helpers

fn validate_case_update_payload(data: &InternalCaseForUpdate) -> Result<()> {
	validate_fda_report_type(data.fda_report_type.as_deref())?;

	if let Some(status) = data.status.as_deref() {
		if !is_valid_case_status(status) {
			return Err(Error::BadRequest {
				message: format!("invalid case status '{status}'"),
			});
		}
	}

	Ok(())
}

fn validate_fda_report_type(value: Option<&str>) -> Result<()> {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(());
	};
	if FDA_REPORT_TYPE_VALUES.contains(&value) {
		return Ok(());
	}
	Err(Error::BadRequest {
		message: "fda_report_type must be one of: 1, 2, 3, 4".to_string(),
	})
}

async fn normalize_review_receivers_for_update(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	data: &mut InternalCaseForUpdate,
) -> Result<()> {
	let Some(raw) = data.review_receivers_json.as_deref() else {
		return Ok(());
	};
	let mut value: Value =
		serde_json::from_str(raw).map_err(|err| Error::BadRequest {
			message: format!("review_receivers_json must be valid JSON: {err}"),
		})?;

	let requirements = prepare_review_receiver_rows(&mut value)?;
	if requirements.needs_report_due_default || requirements.needs_report_due_date {
		let report = SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
			.await
			.map_err(Error::Model)?;
		let c15 = if requirements.needs_report_due_date {
			Some(report.date_of_most_recent_information.as_deref().and_then(lib_core::serde::flex_date::e2b_datetime_date).ok_or_else(|| {
				Error::BadRequest {
					message: "C.1.5 date_of_most_recent_information is required to calculate review receiver reportDueDate".to_string(),
				}
			})?)
		} else {
			None
		};
		let (category, receiver_presaves) = if requirements.needs_report_due_default
		{
			let reactions = ReactionBmc::list_by_case(ctx, mm, case_id)
				.await
				.map_err(Error::Model)?;
			let is_serious = reactions.iter().any(reaction_is_serious);
			let category =
				classify_report(report.report_type.as_deref(), is_serious);
			let receiver_presaves = ReceiverPresaveBmc::list(ctx, mm, None)
				.await
				.map_err(Error::Model)?;
			(category, receiver_presaves)
		} else {
			(ReportCategory::NonSaeSpontaneous, Vec::new())
		};
		apply_review_receiver_defaults(
			&mut value,
			c15,
			category,
			&receiver_presaves,
		)?;
	}

	data.review_receivers_json = Some(serde_json::to_string(&value).map_err(
		|err| Error::BadRequest {
			message: format!("failed to serialize review_receivers_json: {err}"),
		},
	)?);
	Ok(())
}

fn prepare_review_receiver_rows(
	value: &mut Value,
) -> Result<ReviewReceiverRequirements> {
	match value {
		Value::Array(rows) => validate_review_receiver_rows(rows),
		Value::Object(object) => {
			let rows = object
				.get_mut("reviewReceivers")
				.and_then(Value::as_array_mut)
				.ok_or_else(|| Error::BadRequest {
					message: "review_receivers_json must be an array or contain reviewReceivers array".to_string(),
				})?;
			validate_review_receiver_rows(rows)
		}
		_ => Err(Error::BadRequest {
			message: "review_receivers_json must be an array or object".to_string(),
		}),
	}
}

fn apply_review_receiver_defaults(
	value: &mut Value,
	c15: Option<time::Date>,
	category: ReportCategory,
	receiver_presaves: &[ReceiverPresave],
) -> Result<()> {
	match value {
		Value::Array(rows) => {
			normalize_review_receiver_rows(rows, c15, category, receiver_presaves)
		}
		Value::Object(object) => {
			let rows = object
				.get_mut("reviewReceivers")
				.and_then(Value::as_array_mut)
				.ok_or_else(|| Error::BadRequest {
					message: "review_receivers_json must be an array or contain reviewReceivers array".to_string(),
				})?;
			normalize_review_receiver_rows(rows, c15, category, receiver_presaves)
		}
		_ => Err(Error::BadRequest {
			message: "review_receivers_json must be an array or object".to_string(),
		}),
	}
}

fn validate_review_receiver_rows(
	rows: &mut Vec<Value>,
) -> Result<ReviewReceiverRequirements> {
	let mut requirements = ReviewReceiverRequirements::default();
	let mut filtered = Vec::with_capacity(rows.len());
	for (idx, mut row) in rows.drain(..).enumerate() {
		let Some(object) = row.as_object_mut() else {
			return Err(Error::BadRequest {
				message: format!("review receiver row {idx} must be an object"),
			});
		};
		if is_blank_review_receiver_row(object) {
			continue;
		}
		validate_review_receiver_row(idx, object, &mut requirements)?;
		filtered.push(row);
	}
	*rows = filtered;
	Ok(requirements)
}

fn validate_review_receiver_row(
	idx: usize,
	object: &Map<String, Value>,
	requirements: &mut ReviewReceiverRequirements,
) -> Result<()> {
	let receiver =
		text_field(object, &["receiver", "receiverName", "receiver_name"])
			.ok_or_else(|| Error::BadRequest {
				message: format!("review receiver row {idx} receiver is required"),
			})?;
	if receiver.chars().count() > REVIEW_RECEIVER_MAX_LEN {
		return Err(Error::BadRequest {
			message: format!(
				"review receiver row {idx} receiver must be {REVIEW_RECEIVER_MAX_LEN} characters or fewer"
			),
		});
	}

	let report_due = review_receiver_integer_field(
		idx,
		object,
		&["reportDue", "report_due"],
		"reportDue",
	)?;
	if let Some(report_due) = report_due {
		if report_due < 0 {
			return Err(Error::BadRequest {
				message: format!(
					"review receiver row {idx} reportDue must be non-negative"
				),
			});
		}
	}

	let has_report_due_date = if let Some(value) = review_receiver_date_field(
		idx,
		object,
		&["reportDueDate", "report_due_date"],
		"reportDueDate",
	)? {
		validate_review_receiver_date(idx, "reportDueDate", value)?;
		true
	} else {
		requirements.needs_report_due_date = true;
		false
	};
	if report_due.is_none() && !has_report_due_date {
		requirements.needs_report_due_default = true;
	}
	if let Some(value) = review_receiver_date_field(
		idx,
		object,
		&["reportedDate", "reported_date"],
		"reportedDate",
	)? {
		validate_review_receiver_date(idx, "reportedDate", value)?;
	}
	Ok(())
}

fn normalize_review_receiver_rows(
	rows: &mut [Value],
	c15: Option<time::Date>,
	category: ReportCategory,
	receiver_presaves: &[ReceiverPresave],
) -> Result<()> {
	for (idx, row) in rows.iter_mut().enumerate() {
		let Some(object) = row.as_object_mut() else {
			return Err(Error::BadRequest {
				message: format!("review receiver row {idx} must be an object"),
			});
		};
		normalize_review_receiver_row(
			idx,
			object,
			c15,
			category,
			receiver_presaves,
		)?;
	}
	Ok(())
}

fn normalize_review_receiver_row(
	idx: usize,
	object: &mut Map<String, Value>,
	c15: Option<time::Date>,
	category: ReportCategory,
	receiver_presaves: &[ReceiverPresave],
) -> Result<()> {
	let receiver =
		text_field(object, &["receiver", "receiverName", "receiver_name"])
			.ok_or_else(|| Error::BadRequest {
				message: format!("review receiver row {idx} receiver is required"),
			})?;
	if text_field(object, &["reportDueDate", "report_due_date"]).is_some() {
		return Ok(());
	}
	let report_due = match review_receiver_integer_field(
		idx,
		object,
		&["reportDue", "report_due"],
		"reportDue",
	)? {
		Some(value) => value,
		None => {
			let value = default_report_due_from_receiver(
				idx,
				receiver,
				category,
				receiver_presaves,
			)?;
			object.insert("reportDue".to_string(), Value::Number(value.into()));
			value
		}
	};
	if report_due < 0 {
		return Err(Error::BadRequest {
			message: format!(
				"review receiver row {idx} reportDue must be non-negative"
			),
		});
	}
	let c15 = c15.ok_or_else(|| Error::BadRequest {
		message: "C.1.5 date_of_most_recent_information is required to calculate review receiver reportDueDate".to_string(),
	})?;
	let report_due_i32 =
		i32::try_from(report_due).map_err(|_| Error::BadRequest {
			message: format!(
			"review receiver row {idx} reportDue is too large to calculate reportDueDate"
		),
		})?;
	let due_date = report_due_date(
		c15,
		&ReceiverTimeline {
			nsae_spontaneous: Some(report_due_i32),
			sae_spontaneous: Some(report_due_i32),
			nsae_solicited: Some(report_due_i32),
			sae_solicited: Some(report_due_i32),
		},
		category,
	)
	.ok_or_else(|| Error::BadRequest {
		message: format!(
			"review receiver row {idx} reportDueDate calculation overflowed"
		),
	})?;
	object.insert(
		"reportDueDate".to_string(),
		Value::String(format_date(due_date)),
	);
	Ok(())
}

fn default_report_due_from_receiver(
	idx: usize,
	receiver: &str,
	category: ReportCategory,
	receiver_presaves: &[ReceiverPresave],
) -> Result<i64> {
	let receiver = receiver.trim();
	let presave = receiver_presaves
		.iter()
		.find(|row| {
			!row.deleted
				&& row
					.organization_name
					.as_deref()
					.map(str::trim)
					.is_some_and(|name| name == receiver)
		})
		.ok_or_else(|| Error::BadRequest {
			message: format!(
				"review receiver row {idx} receiver '{receiver}' was not found in INFO receivers"
			),
		})?;
	let timeline = receiver_timeline(presave);
	let day_count = timeline.day_count(category).ok_or_else(|| Error::BadRequest {
		message: format!(
			"review receiver row {idx} receiver '{receiver}' has no reportDue timeline for the case category"
		),
	})?;
	if day_count < 0 {
		return Err(Error::BadRequest {
			message: format!(
				"review receiver row {idx} receiver '{receiver}' reportDue timeline must be non-negative"
			),
		});
	}
	Ok(day_count as i64)
}

fn receiver_timeline(receiver: &ReceiverPresave) -> ReceiverTimeline {
	ReceiverTimeline {
		nsae_spontaneous: timeline_day_count(
			receiver.nsae_non_solicited_day_count,
			receiver.nsae_non_solicited_not_applicable,
		),
		sae_spontaneous: timeline_day_count(
			receiver.sae_non_solicited_day_count,
			receiver.sae_non_solicited_not_applicable,
		),
		nsae_solicited: timeline_day_count(
			receiver.nsae_solicited_day_count,
			receiver.nsae_solicited_not_applicable,
		),
		sae_solicited: timeline_day_count(
			receiver.sae_solicited_day_count,
			receiver.sae_solicited_not_applicable,
		),
	}
}

fn timeline_day_count(
	day_count: Option<i32>,
	not_applicable: Option<bool>,
) -> Option<i32> {
	if not_applicable == Some(true) {
		None
	} else {
		day_count
	}
}

fn reaction_is_serious(reaction: &Reaction) -> bool {
	reaction.serious.unwrap_or(false)
		|| reaction.criteria_death.unwrap_or(false)
		|| reaction.criteria_life_threatening.unwrap_or(false)
		|| reaction.criteria_hospitalization.unwrap_or(false)
		|| reaction.criteria_disabling.unwrap_or(false)
		|| reaction.criteria_congenital_anomaly.unwrap_or(false)
		|| reaction.criteria_other_medically_important.unwrap_or(false)
}

fn is_blank_review_receiver_row(object: &Map<String, Value>) -> bool {
	REVIEW_RECEIVER_ROW_FIELDS.iter().all(|key| {
		object.get(*key).is_none_or(|value| match value {
			Value::Null => true,
			Value::String(value) => value.trim().is_empty(),
			_ => false,
		})
	})
}

fn text_field<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
	keys.iter()
		.find_map(|key| object.get(*key).and_then(Value::as_str))
		.map(str::trim)
		.filter(|value| !value.is_empty())
}

fn review_receiver_integer_field(
	idx: usize,
	object: &Map<String, Value>,
	keys: &[&str],
	label: &str,
) -> Result<Option<i64>> {
	for key in keys {
		let Some(value) = object.get(*key) else {
			continue;
		};
		return match value {
			Value::Null => Ok(None),
			Value::Number(value) => {
				value.as_i64().map(Some).ok_or_else(|| Error::BadRequest {
					message: format!(
						"review receiver row {idx} {label} must be an integer"
					),
				})
			}
			Value::String(value) => {
				let value = value.trim();
				if value.is_empty() {
					Ok(None)
				} else {
					value
						.parse::<i64>()
						.map(Some)
						.map_err(|_| Error::BadRequest {
							message: format!(
								"review receiver row {idx} {label} must be an integer"
							),
						})
				}
			}
			_ => Err(Error::BadRequest {
				message: format!(
					"review receiver row {idx} {label} must be an integer"
				),
			}),
		};
	}
	Ok(None)
}

fn review_receiver_date_field<'a>(
	idx: usize,
	object: &'a Map<String, Value>,
	keys: &[&str],
	label: &str,
) -> Result<Option<&'a str>> {
	for key in keys {
		let Some(value) = object.get(*key) else {
			continue;
		};
		return match value {
			Value::Null => Ok(None),
			Value::String(value) => {
				let value = value.trim();
				if value.is_empty() {
					Ok(None)
				} else {
					Ok(Some(value))
				}
			}
			_ => Err(Error::BadRequest {
				message: format!(
					"review receiver row {idx} {label} must be YYYY-MM-DD"
				),
			}),
		};
	}
	Ok(None)
}

fn validate_review_receiver_date(
	idx: usize,
	label: &str,
	value: &str,
) -> Result<()> {
	parse_review_receiver_date(value).ok_or_else(|| Error::BadRequest {
		message: format!("review receiver row {idx} {label} must be YYYY-MM-DD"),
	})?;
	Ok(())
}

fn parse_review_receiver_date(value: &str) -> Option<time::Date> {
	if value.len() != 10 {
		return None;
	}
	let bytes = value.as_bytes();
	if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
		return None;
	}
	let year = value.get(0..4)?.parse::<i32>().ok()?;
	let month = value.get(5..7)?.parse::<u8>().ok()?;
	let day = value.get(8..10)?.parse::<u8>().ok()?;
	time::Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day)
		.ok()
}

fn format_date(date: time::Date) -> String {
	format!(
		"{:04}-{:02}-{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn to_internal_case_for_create(
	ctx: &lib_core::ctx::Ctx,
	data: PublicCaseForCreate,
) -> InternalCaseForCreate {
	InternalCaseForCreate {
		organization_id: ctx.organization_id(),
		dg_prd_key: data.dg_prd_key,
		status: data.status,
		review_receivers_json: data.review_receivers_json,
		workflow_routes_json: data.workflow_routes_json,
		mfds_report_type: data.mfds_report_type,
		fda_report_type: data.fda_report_type,
		report_year: data.report_year,
	}
}

fn to_internal_case_for_update(data: PublicCaseForUpdate) -> InternalCaseForUpdate {
	InternalCaseForUpdate {
		dg_prd_key: data.dg_prd_key,
		status: data.status,
		review_receivers_json: data.review_receivers_json,
		workflow_routes_json: data.workflow_routes_json,
		mfds_report_type: data.mfds_report_type,
		fda_report_type: data.fda_report_type,
		report_year: data.report_year,
		..Default::default()
	}
}

fn case_status_update(status: String) -> InternalCaseForUpdate {
	InternalCaseForUpdate {
		status: Some(status),
		..Default::default()
	}
}

fn required_reason_for_change(
	reason_for_change: Option<String>,
	fallback_reason_for_change: Option<&str>,
	action: &str,
) -> Result<String> {
	reason_for_change
		.or_else(|| fallback_reason_for_change.map(ToString::to_string))
		.and_then(|v| {
			let trimmed = v.trim().to_string();
			if trimmed.is_empty() {
				None
			} else {
				Some(trimmed)
			}
		})
		.ok_or_else(|| Error::BadRequest {
			message: format!("reason_for_change is required for {action}"),
		})
}

fn optional_text_changed(next: &Option<String>, current: Option<&str>) -> bool {
	let Some(next) = next.as_deref() else {
		return false;
	};
	next.trim() != current.unwrap_or_default().trim()
}

fn case_identity_or_scope_update_requires_reason(
	current: &Case,
	data: &InternalCaseForUpdate,
) -> bool {
	optional_text_changed(&data.dg_prd_key, current.dg_prd_key.as_deref())
		|| optional_text_changed(
			&data.review_receivers_json,
			current.review_receivers_json.as_deref(),
		) || optional_text_changed(
		&data.workflow_routes_json,
		current.workflow_routes_json.as_deref(),
	)
}

async fn next_case_version(
	ctx: &Ctx,
	mm: &ModelManager,
	safety_report_id: &str,
) -> Result<i32> {
	Ok(
		SafetyReportIdentificationBmc::max_version_by_safety_report_id(
			ctx,
			mm,
			safety_report_id,
		)
		.await
		.map_err(Error::Model)?
			+ 1,
	)
}

// -- Types

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicCaseForCreate {
	pub safety_report_identification:
		Option<PublicSafetyReportIdentificationForCaseCreate>,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCaseForUpdate {
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSafetyReportIdentificationForCaseCreate {
	pub safety_report_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PublicCaseUpdateRequest {
	pub data: PublicCaseForUpdate,
	pub reason_for_change: Option<String>,
	pub e_signature: Option<ESignatureInput>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PublicCaseDeleteRequest {
	pub reason_for_change: Option<String>,
	pub e_signature: Option<ESignatureInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseLinkOptionList {
	pub items: Vec<CaseLinkOption>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseListViewResult {
	pub items: Vec<CaseListViewRow>,
}

#[derive(Debug, Serialize)]
pub struct CaseLifecycleItem {
	pub case_id: Uuid,
	pub version: i32,
	pub transmission_date: Option<String>,
	pub status: String,
	pub created_at: String,
	pub updated_at: String,
	pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct CaseLifecycleResult {
	pub safety_report_id: String,
	pub current_case_id: Uuid,
	pub items: Vec<CaseLifecycleItem>,
}

#[derive(Debug, FromRow)]
struct CaseLifecycleRow {
	case_id: Uuid,
	version: i32,
	transmission_date: Option<String>,
	status: String,
	created_at: sqlx::types::time::OffsetDateTime,
	updated_at: sqlx::types::time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct PublicCaseView {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub dg_prd_key: Option<String>,
	pub status: String,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub workflow_status: String,
	pub workflow_assigned_role: Option<String>,
	pub workflow_assigned_user_id: Option<Uuid>,
	pub workflow_due_at: Option<sqlx::types::time::OffsetDateTime>,
	pub workflow_description: Option<String>,
	pub workflow_updated_at: sqlx::types::time::OffsetDateTime,
	pub mfds_report_type: Option<String>,
	pub fda_report_type: Option<String>,
	pub report_year: Option<String>,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
	pub submitted_by: Option<Uuid>,
	pub submitted_at: Option<sqlx::types::time::OffsetDateTime>,
	pub dirty_c: bool,
	pub dirty_d: bool,
	pub dirty_e: bool,
	pub dirty_f: bool,
	pub dirty_g: bool,
	pub dirty_h: bool,
	pub created_at: sqlx::types::time::OffsetDateTime,
	pub updated_at: sqlx::types::time::OffsetDateTime,
}

impl From<Case> for PublicCaseView {
	fn from(case: Case) -> Self {
		Self {
			id: case.id,
			organization_id: case.organization_id,
			dg_prd_key: case.dg_prd_key,
			status: case.status,
			review_receivers_json: case.review_receivers_json,
			workflow_routes_json: case.workflow_routes_json,
			workflow_status: case.workflow_status,
			workflow_assigned_role: case.workflow_assigned_role,
			workflow_assigned_user_id: case.workflow_assigned_user_id,
			workflow_due_at: case.workflow_due_at,
			workflow_description: case.workflow_description,
			workflow_updated_at: case.workflow_updated_at,
			mfds_report_type: case.mfds_report_type,
			fda_report_type: case.fda_report_type,
			report_year: case.report_year,
			created_by: case.created_by,
			updated_by: case.updated_by,
			submitted_by: case.submitted_by,
			submitted_at: case.submitted_at,
			dirty_c: case.dirty_c,
			dirty_d: case.dirty_d,
			dirty_e: case.dirty_e,
			dirty_f: case.dirty_f,
			dirty_g: case.dirty_g,
			dirty_h: case.dirty_h,
			created_at: case.created_at,
			updated_at: case.updated_at,
		}
	}
}

#[derive(Debug, Serialize)]
pub struct CaseReadResult {
	#[serde(flatten)]
	pub case: PublicCaseView,
	pub qc_state: &'static str,
	pub is_locked: bool,
	pub can_act_on_workflow: bool,
	pub workflow_block_reason: Option<&'static str>,
	pub case_write_block_reason: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct CaseStatusToggleInput {
	pub expected_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFollowUpCreateResult {
	pub case_id: Uuid,
	pub safety_report_version: i32,
}

// -- Shared helper (used by case_workflow_rest)

pub async fn case_to_read_result(
	ctx: &Ctx,
	mm: &ModelManager,
	case: Case,
) -> Result<CaseReadResult> {
	let actionability = workflow_actionability_for_case(ctx, mm, &case).await?;
	let write_block_reason =
		case_write_block_reason_for_case(ctx, mm, &case).await?;
	Ok(build_case_read_result(
		case,
		actionability,
		write_block_reason,
	))
}

fn case_to_read_result_with_workflow(
	ctx: &Ctx,
	case: Case,
	workflow: &WorkflowRuntimeSettings,
) -> CaseReadResult {
	let (actionability, write_block_reason) =
		case_read_decisions_with_workflow(ctx, &case, workflow);
	build_case_read_result(case, actionability, write_block_reason)
}

fn build_case_read_result(
	case: Case,
	actionability: WorkflowActionability,
	write_block_reason: Option<WorkflowBlockReason>,
) -> CaseReadResult {
	CaseReadResult {
		qc_state: qc_state_for_case_status(
			&case.status,
			case.status_before_lock.as_deref(),
		),
		is_locked: case.status.eq_ignore_ascii_case("locked"),
		case: case.into(),
		can_act_on_workflow: actionability.can_act_on_workflow,
		workflow_block_reason: actionability.workflow_block_reason,
		case_write_block_reason: write_block_reason.map(|reason| reason.code),
	}
}

fn case_scope_ids(
	ctx: &Ctx,
	scope: &EnforcedScopeFilter,
) -> (Vec<String>, Vec<String>, Vec<String>) {
	if ctx.is_system_admin() || ctx.is_sponsor_admin() {
		(Vec::new(), Vec::new(), Vec::new())
	} else {
		(
			scope.sender_ids().to_vec(),
			scope.product_ids().to_vec(),
			scope.study_ids().to_vec(),
		)
	}
}

// -- Handlers

/// POST /api/cases
pub async fn create_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<PublicCaseForCreate>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	let ParamsForCreate { data } = params;
	let product_key = data.dg_prd_key.clone();
	lib_rest_core::with_authorized_case_create(
		&ctx,
		&snapshot,
		&mm,
		product_key.as_deref(),
		move |ctx, mm| Box::pin(create_case_authorized(ctx, mm, data)),
	)
	.await
}

/// POST /api/cases/{source_case_id}/follow-up
pub async fn create_follow_up_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(source_case_id): Path<Uuid>,
	Json(pages): Json<CaseFollowUpPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFollowUpCreateResult>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_create(
		&ctx,
		&snapshot,
		&mm,
		None,
		move |ctx, mm| {
			Box::pin(create_follow_up_case_authorized(
				ctx,
				mm,
				source_case_id,
				pages,
			))
		},
	)
	.await
}

async fn create_follow_up_case_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	source_case_id: Uuid,
	pages: CaseFollowUpPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFollowUpCreateResult>>,
)> {
	create_follow_up_case_in_txn(ctx, mm, source_case_id, pages).await
}

async fn create_follow_up_case_in_txn(
	ctx: &Ctx,
	mm: &ModelManager,
	source_case_id: Uuid,
	mut pages: CaseFollowUpPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseFollowUpCreateResult>>,
)> {
	let source_case = CaseBmc::get(ctx, mm, source_case_id).await?;
	let source_report =
		SafetyReportIdentificationBmc::get_by_case(ctx, mm, source_case_id)
			.await
			.map_err(Error::Model)?;
	let safety_report_id = source_report
		.safety_report_id
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.ok_or_else(|| Error::BadRequest {
			message: "source case C.1.1 safety report ID is required".to_string(),
		})?
		.to_string();

	// Serialize version allocation for this report within the transaction.
	mm.dbx()
		.execute(
			sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
				.bind(&safety_report_id),
		)
		.await
		.map_err(lib_core::model::Error::from)
		.map_err(Error::Model)?;
	let next_version = next_case_version(ctx, mm, &safety_report_id).await?;

	let case_create = InternalCaseForCreate {
		organization_id: source_case.organization_id,
		dg_prd_key: source_case.dg_prd_key,
		status: Some("draft".to_string()),
		review_receivers_json: None,
		workflow_routes_json: None,
		mfds_report_type: source_case.mfds_report_type,
		fda_report_type: source_case.fda_report_type,
		report_year: source_case.report_year,
	};
	validate_case_create_payload(&case_create)?;
	let case_id = CaseBmc::create(ctx, mm, case_create).await?;
	let transmission_date =
		lib_utils::time::format_e2b_timestamp(OffsetDateTime::now_utc());
	SafetyReportIdentificationBmc::create(
		ctx,
		mm,
		SafetyReportIdentificationForCreate {
			case_id,
			safety_report_id: Some(safety_report_id.clone()),
			version: Some(next_version),
			transmission_date: Some(transmission_date),
			report_type: source_report.report_type,
			date_first_received_from_source: source_report
				.date_first_received_from_source,
			date_of_most_recent_information: source_report
				.date_of_most_recent_information,
			fulfil_expedited_criteria: source_report.fulfil_expedited_criteria,
			fulfil_expedited_criteria_null_flavor: source_report
				.fulfil_expedited_criteria_null_flavor,
			local_criteria_report_type: source_report.local_criteria_report_type,
			combination_product_report_indicator: source_report
				.combination_product_report_indicator,
			combination_product_report_indicator_null_flavor: source_report
				.combination_product_report_indicator_null_flavor,
			first_sender_type: source_report.first_sender_type,
			additional_documents_available: source_report
				.additional_documents_available,
			other_case_identifiers_exist: source_report.other_case_identifiers_exist,
			other_case_identifiers_exist_null_flavor: source_report
				.other_case_identifiers_exist_null_flavor,
			worldwide_unique_id: source_report.worldwide_unique_id,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: source_report.receiver_organization,
		},
	)
	.await
	.map_err(Error::Model)?;
	if let Some(report) = pages
		.ci
		.rows
		.get_mut("safetyReportIdentification")
		.and_then(Value::as_object_mut)
	{
		report.insert("safetyReportId".to_string(), json!(safety_report_id));
		report.insert("safetyReportVersion".to_string(), json!(next_version));
	}
	apply_follow_up_page_patches(ctx, mm, case_id, pages).await?;

	Ok((
		axum::http::StatusCode::CREATED,
		Json(DataRestResult {
			data: CaseFollowUpCreateResult {
				case_id,
				safety_report_version: next_version,
			},
		}),
	))
}

async fn create_case_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	data: PublicCaseForCreate,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let provided_safety_report_id = data
		.safety_report_identification
		.as_ref()
		.and_then(|value| value.safety_report_id.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(ToOwned::to_owned);
	let generated_case_number = if provided_safety_report_id.is_none() {
		Some(generate_case_number(ctx, mm).await.map_err(Error::Model)?)
	} else {
		None
	};
	let safety_report_id = provided_safety_report_id
		.or_else(|| {
			generated_case_number
				.as_ref()
				.map(|value| value.safety_report_id.clone())
		})
		.ok_or_else(|| Error::BadRequest {
			message: "safetyReportIdentification.safetyReportId is required"
				.to_string(),
		})?;
	let next_version = next_case_version(ctx, mm, &safety_report_id).await?;
	let worldwide_unique_id =
		generated_case_number.map(|value| value.worldwide_unique_id);
	let data = to_internal_case_for_create(ctx, data);
	validate_case_create_payload(&data)?;

	let id = CaseBmc::create(ctx, mm, data).await?;
	let creation_timestamp =
		lib_utils::time::format_e2b_timestamp(OffsetDateTime::now_utc());
	SafetyReportIdentificationBmc::create(
		ctx,
		mm,
		SafetyReportIdentificationForCreate {
			case_id: id,
			safety_report_id: Some(safety_report_id),
			version: Some(next_version),
			transmission_date: Some(creation_timestamp),
			report_type: None,
			date_first_received_from_source: None,
			date_of_most_recent_information: None,
			fulfil_expedited_criteria: None,
			fulfil_expedited_criteria_null_flavor: None,
			local_criteria_report_type: None,
			combination_product_report_indicator: None,
			first_sender_type: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			combination_product_report_indicator_null_flavor: None,
			worldwide_unique_id,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await
	.map_err(Error::Model)?;
	let entity = CaseBmc::get(ctx, mm, id).await?;
	let entity = case_to_read_result(ctx, mm, entity).await?;
	Ok((
		axum::http::StatusCode::CREATED,
		Json(DataRestResult { data: entity }),
	))
}

/// GET /api/cases/{id}
pub async fn get_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_read(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.read",
		move |ctx, mm| {
			Box::pin(async move {
				let entity = CaseBmc::get(ctx, mm, id).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// GET /api/cases
pub async fn list_cases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<Vec<CaseReadResult>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm, scope| {
			Box::pin(async move {
				let params =
					ParamsList::<CaseFilter>::from_raw_query(raw_query.as_deref())
						.map_err(|message| Error::BadRequest { message })?;
				let (sender_ids, product_ids, study_ids) =
					case_scope_ids(ctx, scope);
				let entities = CaseBmc::list(
					ctx,
					mm,
					params.filters,
					params.list_options,
					&sender_ids,
					&product_ids,
					&study_ids,
				)
				.await?;
				if entities.is_empty() {
					return Ok((
						axum::http::StatusCode::OK,
						Json(DataRestResult { data: Vec::new() }),
					));
				}
				let workflow = load_workflow_runtime_settings(ctx, mm).await?;
				let data = entities
					.into_iter()
					.map(|entity| {
						case_to_read_result_with_workflow(ctx, entity, &workflow)
					})
					.collect();
				Ok((axum::http::StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}

/// GET /api/cases/list-view
pub async fn list_case_view_rows(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
	axum::extract::Query(validation_query): axum::extract::Query<
		super::case_validation_rest::ValidationAuthoritiesQuery,
	>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseListViewResult>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm, scope| {
			Box::pin(list_case_view_rows_authorized(
				ctx,
				mm,
				scope,
				raw_query,
				validation_query,
			))
		},
	)
	.await
}

async fn list_case_view_rows_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	scope: &EnforcedScopeFilter,
	raw_query: Option<String>,
	validation_query: super::case_validation_rest::ValidationAuthoritiesQuery,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseListViewResult>>,
)> {
	let params = ParamsList::<CaseFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	let authorities = validation_query.resolve()?;
	let list_options = params.list_options;
	let (sender_ids, product_ids, study_ids) = case_scope_ids(ctx, scope);

	let mut items = lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		let list_options = list_options.clone();
		let sender_ids = sender_ids.clone();
		let product_ids = product_ids.clone();
		let study_ids = study_ids.clone();
		Box::pin(async move {
			CaseBmc::list_view_rows(
				dbx,
				list_options.as_ref(),
				&sender_ids,
				&product_ids,
				&study_ids,
			)
			.await
			.map_err(Error::from)
		})
	})
	.await?;

	let case_ids = items.iter().map(|item| item.case_id).collect::<Vec<_>>();
	let cached_totals = CaseValidationSummaryBmc::cached_totals_by_case(
		ctx,
		mm,
		&case_ids,
		&authorities,
	)
	.await?;
	for item in &mut items {
		item.warn = cached_totals
			.get(&item.case_id)
			.map(|count| count.to_string())
			.unwrap_or_else(|| "Not checked".to_string());
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseListViewResult { items },
		}),
	))
}

/// PUT /api/cases/{id}
pub async fn update_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<PublicCaseUpdateRequest>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.update",
		CaseMutationKind::Edit,
		move |ctx, mm| Box::pin(update_case_authorized(ctx, mm, id, params)),
	)
	.await
}

async fn update_case_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
	params: PublicCaseUpdateRequest,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let PublicCaseUpdateRequest {
		data,
		reason_for_change,
		e_signature,
	} = params;
	let mut data = to_internal_case_for_update(data);
	validate_case_update_payload(&data)?;
	let touches_non_status = update_touches_non_status_fields(&data);
	let current = CaseBmc::get(ctx, mm, id).await?;
	normalize_review_receivers_for_update(ctx, mm, id, &mut data).await?;
	let requested_status = data.status.clone();
	if touches_non_status {
		if let Some(reason) =
			case_write_block_reason_for_case(ctx, mm, &current).await?
		{
			return Err(Error::BadRequest {
				message: format!(
					"{}; only status transitions are allowed",
					reason.message
				),
			});
		}
	}
	if let Some(next_status) = data.status.as_deref() {
		if !is_allowed_case_status_transition(&current.status, next_status) {
			return Err(Error::BadRequest {
				message: format!(
					"illegal case status transition: '{}' -> '{}'",
					current.status, next_status
				),
			});
		}
		// Reference privilege rows CASE|Review|Edit and CASE|Lock|Edit:
		// entering reviewed/validated is a review-grade action; entering or
		// leaving locked is a lock-grade action. Everything else stays a
		// regular case edit.
		let prev = current.status.trim().to_ascii_lowercase();
		let next = next_status.trim().to_ascii_lowercase();
		if prev != next {
			if prev == "locked"
				|| matches!(next.as_str(), "reviewed" | "validated" | "locked")
				|| (next == "draft"
					&& matches!(prev.as_str(), "reviewed" | "validated"))
			{
				return Err(Error::BadRequest {
					message: "use the dedicated case review/lock toggle endpoint for QC or lock state changes".to_string(),
				});
			}
		}
	}

	let requires_compliance = requested_status
		.as_deref()
		.map(|next_status| {
			let prev = current.status.trim().to_ascii_lowercase();
			let next = next_status.trim().to_ascii_lowercase();
			prev != next
				&& matches!(next.as_str(), "submitted" | "nullified" | "deleted")
		})
		.unwrap_or(false);
	let requires_reason_for_identity_or_scope =
		case_identity_or_scope_update_requires_reason(&current, &data);

	let ctx_for_update = if requires_compliance {
		let reason = required_reason_for_change(
			reason_for_change,
			ctx.change_reason(),
			"submitted/nullified/deleted status transitions",
		)?;
		let e_signature = e_signature.ok_or(Error::BadRequest {
			message:
				"e_signature is required for submitted/nullified/deleted status transitions"
					.to_string(),
		})?;
		let compliance = ComplianceActionInput {
			reason_for_change: reason.clone(),
			e_signature,
		};
		let signature_id = capture_e_signature(
			ctx,
			mm,
			Some(id),
			"CASE_STATUS_TRANSITION",
			&compliance,
		)
		.await?;
		ctx.with_compliance(Some(reason), Some(signature_id))
	} else if requires_reason_for_identity_or_scope {
		let reason = required_reason_for_change(
			reason_for_change,
			ctx.change_reason(),
			"case identity/scope updates",
		)?;
		ctx.with_compliance(Some(reason), None)
	} else {
		ctx.clone()
	};

	CaseBmc::update(&ctx_for_update, mm, id, data).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, id).await?;
	let entity = CaseBmc::get(ctx, mm, id).await?;
	let entity = case_to_read_result(ctx, mm, entity).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// POST /api/cases/{id}/review/toggle
pub async fn toggle_case_review(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<CaseStatusToggleInput>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	let expected_status = params.data.expected_status;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.review.toggle",
		CaseMutationKind::ReviewToggle,
		move |ctx, mm| {
			Box::pin(async move {
				let entity =
					CaseBmc::toggle_review(ctx, mm, id, &expected_status).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// POST /api/cases/{id}/lock/toggle
pub async fn toggle_case_lock(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<CaseStatusToggleInput>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	let expected_status = params.data.expected_status;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.lock.toggle",
		CaseMutationKind::LockToggle,
		move |ctx, mm| {
			Box::pin(async move {
				let entity =
					CaseBmc::toggle_lock(ctx, mm, id, &expected_status).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{id}
pub async fn delete_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	payload: Option<Json<PublicCaseDeleteRequest>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.delete",
		CaseMutationKind::Delete,
		move |ctx, mm| {
			Box::pin(async move {
				let PublicCaseDeleteRequest {
					reason_for_change,
					e_signature,
				} = payload.map(|Json(params)| params).unwrap_or_default();
				let current = CaseBmc::get(ctx, mm, id).await?;
				if !is_allowed_case_status_transition(&current.status, "deleted") {
					return Err(Error::BadRequest {
						message: format!(
							"illegal case status transition: '{}' -> 'deleted'",
							current.status
						),
					});
				}
				let reason = required_reason_for_change(
					reason_for_change,
					ctx.change_reason(),
					"delete",
				)?;
				let e_signature = e_signature.ok_or(Error::BadRequest {
					message: "e_signature is required for delete".to_string(),
				})?;
				let compliance = ComplianceActionInput {
					reason_for_change: reason.clone(),
					e_signature,
				};
				let signature_id = capture_e_signature(
					ctx,
					mm,
					Some(id),
					"CASE_STATUS_TRANSITION",
					&compliance,
				)
				.await?;
				let ctx_for_update =
					ctx.with_compliance(Some(reason), Some(signature_id));
				CaseBmc::update(
					&ctx_for_update,
					mm,
					id,
					case_status_update("deleted".to_string()),
				)
				.await?;
				CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, id).await?;
				let entity = CaseBmc::get(ctx, mm, id).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// POST /api/cases/{id}/restore
pub async fn restore_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(payload): Json<PublicCaseDeleteRequest>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.delete",
		CaseMutationKind::Restore,
		move |ctx, mm| {
			Box::pin(async move {
				let current = CaseBmc::get(ctx, mm, id).await?;
				if !is_allowed_case_status_transition(&current.status, "draft") {
					return Err(Error::BadRequest {
						message: format!(
							"illegal case status transition: '{}' -> 'draft'",
							current.status
						),
					});
				}
				let reason = required_reason_for_change(
					payload.reason_for_change,
					ctx.change_reason(),
					"restore",
				)?;
				let ctx_for_update = ctx.with_compliance(Some(reason), None);
				CaseBmc::update(
					&ctx_for_update,
					mm,
					id,
					case_status_update("draft".to_string()),
				)
				.await?;
				CaseValidationSummaryBmc::mark_stale_for_case(ctx, mm, id).await?;
				let entity = CaseBmc::get(ctx, mm, id).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// POST /api/cases/{id}/validator/mark-validated
pub async fn mark_case_validated_by_validator(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	headers: axum::http::HeaderMap,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	let required_token =
		std::env::var("E2BR3_VALIDATOR_TOKEN").map_err(|_| Error::BadRequest {
			message: "validator token is not configured".to_string(),
		})?;
	let provided_token = headers
		.get("x-validator-token")
		.and_then(|value| value.to_str().ok())
		.unwrap_or_default();
	if provided_token != required_token {
		return Err(Error::BadRequest {
			message: "invalid validator token".to_string(),
		});
	}
	lib_rest_core::with_authorized_case_mutation(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.validate",
		CaseMutationKind::Validate,
		move |ctx, mm| {
			Box::pin(async move {
				let authority =
					crate::web::rest::case_validation_rest::resolve_authority(
						ctx, mm, id, None,
					)
					.await?;
				let report =
					super::case_validation_rest::refresh_case_validation_cache(
						ctx,
						mm,
						id,
						&[authority],
					)
					.await?
					.remove(0);
				if report.issue_count > 0 {
					return Err(Error::BadRequest {
						message: format!(
							"validator cannot mark case validated: {} validation issue(s) remain",
							report.issue_count
						),
					});
				}

				let validator_ctx = ctx.with_compliance(
					Some(SYSTEM_VALIDATION_REASON_VALIDATOR.to_string()),
					None,
				);
				CaseBmc::update(
					&validator_ctx,
					mm,
					id,
					case_status_update("validated".to_string()),
				)
				.await?;
				let entity = CaseBmc::get(ctx, mm, id).await?;
				let entity = case_to_read_result(ctx, mm, entity).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(DataRestResult { data: entity }),
				))
			})
		},
	)
	.await
}

/// GET /api/cases/{id}/lifecycle
pub async fn get_case_lifecycle(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLifecycleResult>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_read(
		&ctx,
		&snapshot,
		&mm,
		id,
		"case.read",
		move |ctx, mm| Box::pin(get_case_lifecycle_authorized(ctx, mm, id)),
	)
	.await
}

async fn get_case_lifecycle_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
	id: Uuid,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLifecycleResult>>,
)> {
	let safety_report =
		SafetyReportIdentificationBmc::get_by_case(ctx, mm, id).await?;
	let safety_report_id = safety_report
		.safety_report_id
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::BadRequest {
			message: format!("case {id} has no safety report ID"),
		})?;
	let versions = lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		let safety_report_id = safety_report_id.clone();
		Box::pin(async move {
			dbx.fetch_all(
				sqlx::query_as::<_, CaseLifecycleRow>(
					r#"
					SELECT c.id AS case_id,
					       s.version,
					       s.transmission_date,
					       c.status,
					       c.created_at,
					       c.updated_at
					  FROM cases c
					  JOIN safety_report_identification s ON s.case_id = c.id
					 WHERE s.safety_report_id = $1
					 ORDER BY s.version ASC, c.created_at ASC, c.id ASC
					"#,
				)
				.bind(safety_report_id),
			)
			.await
			.map_err(|err| Error::Model(err.into()))
		})
	})
	.await?;
	let mut items = Vec::new();
	for row in versions {
		if lib_rest_core::case_matches_user_scope(ctx, mm, row.case_id).await? {
			items.push(row);
		}
	}
	let items = items
		.into_iter()
		.map(|row| CaseLifecycleItem {
			case_id: row.case_id,
			version: row.version,
			transmission_date: row.transmission_date,
			status: row.status,
			created_at: row.created_at.to_string(),
			updated_at: row.updated_at.to_string(),
			is_current: row.case_id == id,
		})
		.collect();
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseLifecycleResult {
				safety_report_id,
				current_case_id: id,
				items,
			},
		}),
	))
}

/// GET /api/cases/link-options
pub async fn list_case_link_options(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLinkOptionList>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_collection(
		&ctx,
		&snapshot,
		&mm,
		move |ctx, mm, _scope| Box::pin(list_case_link_options_authorized(ctx, mm)),
	)
	.await
}

async fn list_case_link_options_authorized(
	ctx: &Ctx,
	mm: &ModelManager,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLinkOptionList>>,
)> {
	let items = lib_rest_core::with_rls_read(mm, ctx, |dbx| {
		Box::pin(async move {
			CaseBmc::list_link_options(dbx).await.map_err(Error::from)
		})
	})
	.await?;

	let item_ids = items.iter().map(|item| item.case_id).collect::<Vec<_>>();
	let visible_case_ids =
		lib_rest_core::case_ids_matching_user_scope(ctx, mm, &item_ids).await?;
	let scoped = items
		.into_iter()
		.filter(|item| visible_case_ids.contains(&item.case_id))
		.collect::<Vec<_>>();

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseLinkOptionList { items: scoped },
		}),
	))
}
