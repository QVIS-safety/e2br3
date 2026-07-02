use crate::web::rest::compliance::{
	capture_e_signature, ComplianceActionInput, ESignatureInput,
};
use axum::extract::{Path, State};
use axum::Json;
use lib_core::ctx::Ctx;
use lib_core::model::acs::{
	CASE_CREATE, CASE_DELETE, CASE_LIST, CASE_READ, CASE_UPDATE,
};
use lib_core::model::case::{
	is_allowed_case_status_transition, is_valid_case_status,
	update_touches_non_status_fields, Case, CaseBmc, CaseFilter,
	CaseForCreate as InternalCaseForCreate, CaseForUpdate as InternalCaseForUpdate,
	CaseLinkOption, CaseListViewRow,
};
use lib_core::model::case_numbering::generate_case_number;
use lib_core::model::case_validation_summary::CaseValidationSummaryBmc;
use lib_core::model::safety_report::{
	SafetyReportIdentificationBmc, SafetyReportIdentificationForCreate,
};
use lib_core::model::ModelManager;
use lib_core::regulatory::RegulatoryAuthority;
use lib_core::validation::validate_case_for_authority;
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_rest_core::{
	case_write_block_reason_for_case, qc_state_for_case_status,
	workflow_actionability_for_case,
};
use lib_web::middleware::mw_auth::CtxW;
use serde::{Deserialize, Serialize};
use sqlx::{types::time::OffsetDateTime, FromRow};
use uuid::Uuid;

const SYSTEM_VALIDATION_REASON_VALIDATOR: &str =
	"system validation: validator mark-validated endpoint";
const FDA_REPORT_TYPE_VALUES: &[&str] = &["1", "2", "3", "4"];

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
		if status.eq_ignore_ascii_case("validated") {
			return Err(Error::BadRequest {
				message: "cannot set case to validated manually: status is managed by validator".to_string(),
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

#[derive(Debug, Deserialize)]
pub struct PublicCaseDeleteRequest {
	pub reason_for_change: Option<String>,
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
	pub raw_xml: Option<Vec<u8>>,
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
			raw_xml: case.raw_xml,
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
}

// -- Shared helper (used by case_workflow_rest)

pub async fn case_to_read_result(
	ctx: &Ctx,
	mm: &ModelManager,
	case: Case,
) -> Result<CaseReadResult> {
	let actionability = workflow_actionability_for_case(ctx, mm, &case).await?;
	let status = case.status.clone();
	Ok(CaseReadResult {
		qc_state: qc_state_for_case_status(&status),
		is_locked: status.eq_ignore_ascii_case("locked"),
		case: case.into(),
		can_act_on_workflow: actionability.can_act_on_workflow,
		workflow_block_reason: actionability.workflow_block_reason,
	})
}

// -- Handlers

/// POST /api/cases
pub async fn create_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Json(params): Json<ParamsForCreate<PublicCaseForCreate>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;
	let ParamsForCreate { data } = params;
	let provided_safety_report_id = data
		.safety_report_identification
		.as_ref()
		.and_then(|value| value.safety_report_id.as_deref())
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(ToOwned::to_owned);
	let generated_case_number = if provided_safety_report_id.is_none() {
		Some(
			generate_case_number(&ctx, &mm)
				.await
				.map_err(Error::Model)?,
		)
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
	let next_version = next_case_version(&ctx, &mm, &safety_report_id).await?;
	let worldwide_unique_id =
		generated_case_number.map(|value| value.worldwide_unique_id);
	let data = to_internal_case_for_create(&ctx, data);
	validate_case_create_payload(&data)?;

	let id = CaseBmc::create(&ctx, &mm, data).await?;
	let creation_timestamp =
		crate::web::rest::case_export_rest::format_message_timestamp_utc_pub(
			OffsetDateTime::now_utc(),
		);
	SafetyReportIdentificationBmc::create(
		&ctx,
		&mm,
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
			first_sender_type_null_flavor: None,
			additional_documents_available: None,
			other_case_identifiers_exist: None,
			other_case_identifiers_exist_null_flavor: None,
			worldwide_unique_id,
			worldwide_unique_id_null_flavor: None,
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: None,
		},
	)
	.await
	.map_err(Error::Model)?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity = case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::CREATED,
		Json(DataRestResult { data: entity }),
	))
}

/// GET /api/cases/{id}
pub async fn get_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity = case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// GET /api/cases
pub async fn list_cases(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<Vec<CaseReadResult>>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_LIST)?;
	let params = ParamsList::<CaseFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	let entities =
		CaseBmc::list(&ctx, &mm, params.filters, params.list_options).await?;
	let mut scoped = Vec::with_capacity(entities.len());
	for entity in entities {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, entity.id).await? {
			scoped.push(case_to_read_result(&ctx, &mm, entity).await?);
		}
	}
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: scoped }),
	))
}

/// GET /api/cases/list-view
pub async fn list_case_view_rows(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseListViewResult>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_LIST)?;
	let params = ParamsList::<CaseFilter>::from_raw_query(raw_query.as_deref())
		.map_err(|message| Error::BadRequest { message })?;
	let list_options = params.list_options;
	let offset = list_options
		.as_ref()
		.and_then(|options| options.offset)
		.unwrap_or(0)
		.max(0) as usize;
	let limit = list_options
		.as_ref()
		.and_then(|options| options.limit)
		.unwrap_or(500)
		.clamp(0, 500) as usize;
	if limit == 0 {
		return Ok((
			axum::http::StatusCode::OK,
			Json(DataRestResult {
				data: CaseListViewResult { items: Vec::new() },
			}),
		));
	}

	let items = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		let list_options = list_options.clone();
		Box::pin(async move {
			CaseBmc::list_view_rows(dbx, list_options.as_ref())
				.await
				.map_err(Error::from)
		})
	})
	.await?;

	let mut scoped = Vec::with_capacity(limit.min(items.len()));
	let mut scoped_offset = 0usize;
	for item in items {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, item.case_id).await? {
			if scoped_offset < offset {
				scoped_offset += 1;
				continue;
			}
			scoped.push(item);
			if scoped.len() >= limit {
				break;
			}
		}
	}
	let case_ids = scoped.iter().map(|item| item.case_id).collect::<Vec<_>>();
	let cached_totals =
		CaseValidationSummaryBmc::cached_totals_by_case(&ctx, &mm, &case_ids)
			.await?;
	for item in &mut scoped {
		item.warn = cached_totals
			.get(&item.case_id)
			.copied()
			.unwrap_or(0)
			.to_string();
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseListViewResult { items: scoped },
		}),
	))
}

/// PUT /api/cases/{id}
pub async fn update_case_guarded(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	Json(params): Json<PublicCaseUpdateRequest>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let PublicCaseUpdateRequest {
		data,
		reason_for_change,
		e_signature,
	} = params;
	let data = to_internal_case_for_update(data);
	validate_case_update_payload(&data)?;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	let requested_status = data.status.clone();
	if update_touches_non_status_fields(&data) {
		if let Some(reason) =
			case_write_block_reason_for_case(&ctx, &mm, &current).await?
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
			&ctx,
			&mm,
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

	CaseBmc::update(&ctx_for_update, &mm, id, data).await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, id).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity = case_to_read_result(&ctx, &mm, entity).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// DELETE /api/cases/{id}
pub async fn delete_case(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
	payload: Option<Json<PublicCaseDeleteRequest>>,
) -> Result<(axum::http::StatusCode, Json<DataRestResult<CaseReadResult>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_DELETE)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let current = CaseBmc::get(&ctx, &mm, id).await?;
	if !is_allowed_case_status_transition(&current.status, "deleted") {
		return Err(Error::BadRequest {
			message: format!(
				"illegal case status transition: '{}' -> 'deleted'",
				current.status
			),
		});
	}
	let reason = required_reason_for_change(
		payload.and_then(|Json(params)| params.reason_for_change),
		ctx.change_reason(),
		"delete",
	)?;
	let ctx_for_update = ctx.with_compliance(Some(reason), None);
	CaseBmc::update(
		&ctx_for_update,
		&mm,
		id,
		case_status_update("deleted".to_string()),
	)
	.await?;
	CaseValidationSummaryBmc::mark_stale_for_case(&ctx, &mm, id).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity = case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// POST /api/cases/{id}/validator/mark-validated
pub async fn mark_case_validated_by_validator(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
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
	require_permission(&ctx, CASE_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, id).await?;

	let report =
		validate_case_for_authority(&ctx, &mm, id, RegulatoryAuthority::Fda).await?;
	CaseValidationSummaryBmc::upsert_for_reports(&ctx, &mm, id, &[report.clone()])
		.await?;
	let total_blocking = report.blocking_count;
	if total_blocking > 0 {
		return Err(Error::BadRequest {
			message: format!(
				"validator cannot mark case validated: {} blocking issue(s) remain",
				total_blocking
			),
		});
	}

	let validator_ctx = ctx
		.with_compliance(Some(SYSTEM_VALIDATION_REASON_VALIDATOR.to_string()), None);
	CaseBmc::update(
		&validator_ctx,
		&mm,
		id,
		case_status_update("validated".to_string()),
	)
	.await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
	let entity = case_to_read_result(&ctx, &mm, entity).await?;
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult { data: entity }),
	))
}

/// GET /api/cases/{id}/lifecycle
pub async fn get_case_lifecycle(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLifecycleResult>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, id).await?;
	let safety_report =
		SafetyReportIdentificationBmc::get_by_case(&ctx, &mm, id).await?;
	let safety_report_id = safety_report.safety_report_id.unwrap_or_default();
	let versions = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		let safety_report_id = safety_report_id.clone();
		Box::pin(async move {
			dbx.fetch_all(
				sqlx::query_as::<_, CaseLifecycleRow>(
					r#"
					SELECT c.id AS case_id,
					       s.version,
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
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, row.case_id).await? {
			items.push(row);
		}
	}
	let items = items
		.into_iter()
		.map(|row| CaseLifecycleItem {
			case_id: row.case_id,
			version: row.version,
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
) -> Result<(
	axum::http::StatusCode,
	Json<DataRestResult<CaseLinkOptionList>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_LIST)?;

	let items = lib_rest_core::with_rls_read(&mm, &ctx, |dbx| {
		Box::pin(async move {
			CaseBmc::list_link_options(dbx).await.map_err(Error::from)
		})
	})
	.await?;

	let mut scoped = Vec::with_capacity(items.len());
	for item in items {
		if lib_rest_core::case_matches_user_scope(&ctx, &mm, item.case_id).await? {
			scoped.push(item);
		}
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseLinkOptionList { items: scoped },
		}),
	))
}
