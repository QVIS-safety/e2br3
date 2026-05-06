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
use lib_core::model::ModelManager;
use lib_core::validation::{validate_case_for_profiles, ValidationProfile};
use lib_rest_core::prelude::*;
use lib_rest_core::rest_params::ParamsForCreate;
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::Error;
use lib_rest_core::{
	case_write_block_reason_for_case, qc_state_for_case_status,
	workflow_actionability_for_case,
};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValString, OpValsString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const SYSTEM_VALIDATION_REASON_VALIDATOR: &str =
	"system validation: validator mark-validated endpoint";

// -- Public helpers (used by sibling modules)

pub fn parse_appendix_profile_or_bad_request(
	value: &str,
) -> Result<ValidationProfile> {
	ValidationProfile::parse(value).ok_or_else(|| Error::BadRequest {
		message: format!(
			"invalid appendix profile '{value}' (expected: ich, fda or mfds)"
		),
	})
}

pub fn normalize_appendices_json(value: &str) -> Result<String> {
	let parsed: Vec<String> =
		serde_json::from_str(value).map_err(|_| Error::BadRequest {
			message: "appendices_json must be a JSON array".to_string(),
		})?;
	let mut normalized = Vec::new();
	for item in parsed {
		let profile = parse_appendix_profile_or_bad_request(&item)?;
		let as_str = profile.as_str().to_string();
		if !normalized.contains(&as_str) {
			normalized.push(as_str);
		}
	}
	if normalized.is_empty() {
		return Err(Error::BadRequest {
			message: "appendices_json cannot be empty".to_string(),
		});
	}
	Ok(json!(normalized).to_string())
}

pub fn first_appendix_profile_from_json(value: &str) -> Result<ValidationProfile> {
	let normalized = normalize_appendices_json(value)?;
	let parsed: Vec<String> =
		serde_json::from_str(&normalized).map_err(|_| Error::BadRequest {
			message: "appendices_json must be a JSON array".to_string(),
		})?;
	parsed
		.first()
		.and_then(|item| ValidationProfile::parse(item))
		.ok_or_else(|| Error::BadRequest {
			message: "appendices_json cannot be empty".to_string(),
		})
}

/// Resolves the ordered list of validation profiles for a case.
/// Prefers `appendices_json` (multi-profile), then defaults to FDA.
fn resolve_appendix_profiles(
	case: &lib_core::model::case::Case,
) -> Vec<ValidationProfile> {
	if let Some(json) = case.appendices_json.as_deref() {
		if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
			let profiles: Vec<ValidationProfile> = items
				.iter()
				.filter_map(|v| v.as_str())
				.filter_map(ValidationProfile::parse)
				.fold(Vec::new(), |mut acc, p| {
					if !acc.contains(&p) {
						acc.push(p);
					}
					acc
				});
			if !profiles.is_empty() {
				return profiles;
			}
		}
	}
	vec![ValidationProfile::Fda]
}

pub fn validate_case_create_payload(data: &InternalCaseForCreate) -> Result<()> {
	if data.safety_report_id.trim().is_empty() {
		return Err(Error::BadRequest {
			message: "safety_report_id is required".to_string(),
		});
	}

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

	if let Some(appendices_json) = data.appendices_json.as_deref() {
		let _ = normalize_appendices_json(appendices_json)?;
	}

	Ok(())
}

// -- Private helpers

fn validate_case_update_payload(data: &InternalCaseForUpdate) -> Result<()> {
	if let Some(safety_report_id) = data.safety_report_id.as_deref() {
		if safety_report_id.trim().is_empty() {
			return Err(Error::BadRequest {
				message: "safety_report_id cannot be empty".to_string(),
			});
		}
	}

	if let Some(status) = data.status.as_deref() {
		if !is_valid_case_status(status) {
			return Err(Error::BadRequest {
				message: format!("invalid case status '{status}'"),
			});
		}
	}

	if let Some(appendices_json) = data.appendices_json.as_deref() {
		let _ = normalize_appendices_json(appendices_json)?;
	}

	Ok(())
}

fn to_internal_case_for_create(
	ctx: &lib_core::ctx::Ctx,
	data: PublicCaseForCreate,
	version: i32,
) -> InternalCaseForCreate {
	InternalCaseForCreate {
		organization_id: ctx.organization_id(),
		safety_report_id: data.safety_report_id,
		dg_prd_key: data.dg_prd_key,
		status: data.status,
		appendices_json: data.appendices_json,
		review_receivers_json: data.review_receivers_json,
		workflow_routes_json: data.workflow_routes_json,
		mfds_report_type: data.mfds_report_type,
		report_year: data.report_year,
		source_document_name: data.source_document_name,
		source_document_base64: data.source_document_base64,
		source_document_media_type: data.source_document_media_type,
		version: Some(version),
	}
}

fn to_internal_case_for_update(data: PublicCaseForUpdate) -> InternalCaseForUpdate {
	InternalCaseForUpdate {
		safety_report_id: data.safety_report_id,
		dg_prd_key: data.dg_prd_key,
		status: data.status,
		appendices_json: data.appendices_json,
		review_receivers_json: data.review_receivers_json,
		workflow_routes_json: data.workflow_routes_json,
		mfds_report_type: data.mfds_report_type,
		report_year: data.report_year,
		source_document_name: data.source_document_name,
		source_document_base64: data.source_document_base64,
		source_document_media_type: data.source_document_media_type,
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
	action: &str,
) -> Result<String> {
	reason_for_change
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
	optional_text_changed(&data.safety_report_id, Some(&current.safety_report_id))
		|| optional_text_changed(&data.dg_prd_key, current.dg_prd_key.as_deref())
		|| optional_text_changed(
			&data.appendices_json,
			current.appendices_json.as_deref(),
		) || optional_text_changed(
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
	let max = CaseBmc::list(
		ctx,
		mm,
		Some(vec![CaseFilter {
			organization_id: None,
			safety_report_id: Some(OpValsString::from(vec![OpValString::Eq(
				safety_report_id.to_string(),
			)])),
			status: None,
		}]),
		Some(ListOptions {
			limit: Some(100),
			offset: None,
			order_bys: Some("version".into()),
		}),
	)
	.await
	.map_err(lib_rest_core::Error::from)?
	.into_iter()
	.map(|case: Case| case.version)
	.max()
	.unwrap_or(0);
	Ok(max + 1)
}

// -- Types

#[derive(Debug, Deserialize)]
pub struct PublicCaseForCreate {
	pub safety_report_id: String,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub appendices_json: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PublicCaseForUpdate {
	pub safety_report_id: Option<String>,
	pub dg_prd_key: Option<String>,
	pub status: Option<String>,
	pub appendices_json: Option<String>,
	pub review_receivers_json: Option<String>,
	pub workflow_routes_json: Option<String>,
	pub mfds_report_type: Option<String>,
	pub report_year: Option<String>,
	pub source_document_name: Option<String>,
	pub source_document_base64: Option<String>,
	pub source_document_media_type: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct CaseReadResult {
	#[serde(flatten)]
	pub case: Case,
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
	let mut case = case;
	let profiles = resolve_appendix_profiles(&case);
	case.appendices_json = Some(
		json!(profiles
			.iter()
			.map(|profile| profile.as_str())
			.collect::<Vec<_>>())
		.to_string(),
	);
	let actionability = workflow_actionability_for_case(ctx, mm, &case).await?;
	Ok(CaseReadResult {
		qc_state: qc_state_for_case_status(&case.status),
		is_locked: case.status.eq_ignore_ascii_case("locked"),
		case,
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
) -> Result<(axum::http::StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_CREATE)?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	if let Some(appendices_json) = data.appendices_json.as_deref() {
		let normalized = normalize_appendices_json(appendices_json)?;
		data.appendices_json = Some(normalized);
	}
	let next_version = next_case_version(&ctx, &mm, &data.safety_report_id).await?;
	let data = to_internal_case_for_create(&ctx, data, next_version);
	validate_case_create_payload(&data)?;

	let id = CaseBmc::create(&ctx, &mm, data).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
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
) -> Result<(axum::http::StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_UPDATE)?;
	let PublicCaseUpdateRequest {
		mut data,
		reason_for_change,
		e_signature,
	} = params;
	if let Some(appendices_json) = data.appendices_json.as_deref() {
		let normalized = normalize_appendices_json(appendices_json)?;
		data.appendices_json = Some(normalized);
	}
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
			"case identity/scope updates",
		)?;
		ctx.with_compliance(Some(reason), None)
	} else {
		ctx.clone()
	};

	CaseBmc::update(&ctx_for_update, &mm, id, data).await?;
	let entity = CaseBmc::get(&ctx, &mm, id).await?;

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
) -> Result<(axum::http::StatusCode, Json<DataRestResult<Case>>)> {
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
	let entity = CaseBmc::get(&ctx, &mm, id).await?;
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
) -> Result<(axum::http::StatusCode, Json<DataRestResult<Case>>)> {
	let ctx = ctx_w.0;
	if !ctx.is_system_admin() {
		return Err(Error::BadRequest {
			message:
				"only validator service/system administrator can mark case validated"
					.to_string(),
		});
	}

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

	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let profiles = resolve_appendix_profiles(&case);
	let reports = validate_case_for_profiles(&ctx, &mm, id, &profiles).await?;
	let total_blocking: usize = reports.iter().map(|r| r.blocking_count).sum();
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
	let case = CaseBmc::get(&ctx, &mm, id).await?;
	let rows = CaseBmc::list(
		&ctx,
		&mm,
		Some(vec![CaseFilter {
			organization_id: None,
			safety_report_id: Some(OpValsString::from(vec![OpValString::Eq(
				case.safety_report_id.clone(),
			)])),
			status: None,
		}]),
		None,
	)
	.await?;
	let mut versions = Vec::new();
	for row in rows {
		if row.safety_report_id == case.safety_report_id
			&& lib_rest_core::case_matches_user_scope(&ctx, &mm, row.id).await?
		{
			versions.push(row);
		}
	}
	versions.sort_by(|a, b| a.version.cmp(&b.version));
	let items = versions
		.into_iter()
		.map(|row| CaseLifecycleItem {
			case_id: row.id,
			version: row.version,
			status: row.status,
			created_at: row.created_at.to_string(),
			updated_at: row.updated_at.to_string(),
			is_current: row.id == id,
		})
		.collect();
	Ok((
		axum::http::StatusCode::OK,
		Json(DataRestResult {
			data: CaseLifecycleResult {
				safety_report_id: case.safety_report_id,
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
