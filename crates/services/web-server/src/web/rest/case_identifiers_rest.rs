// Case Identifiers REST endpoints (C.1.9.r and C.1.10.r)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	CASE_IDENTIFIER_CREATE, CASE_IDENTIFIER_DELETE, CASE_IDENTIFIER_LIST,
	CASE_IDENTIFIER_READ, CASE_IDENTIFIER_UPDATE,
};
use lib_core::model::case_identifiers::{
	LinkedReportNumber, LinkedReportNumberBmc, LinkedReportNumberFilter,
	LinkedReportNumberForCreate, LinkedReportNumberForUpdate, OtherCaseIdentifier,
	OtherCaseIdentifierBmc, OtherCaseIdentifierFilter, OtherCaseIdentifierForCreate,
	OtherCaseIdentifierForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

// -- Other Case Identifiers (C.1.9.r)

/// POST /api/cases/{case_id}/other-identifiers
pub async fn create_other_case_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<OtherCaseIdentifierForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<OtherCaseIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest create_other_case_identifier case_id={}",
		"HANDLER",
		case_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = OtherCaseIdentifierBmc::create(&ctx, &mm, data).await?;
	let entity = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/other-identifiers
pub async fn list_other_case_identifiers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<OtherCaseIdentifier>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest list_other_case_identifiers case_id={}",
		"HANDLER",
		case_id
	);

	let filter = OtherCaseIdentifierFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let entities = OtherCaseIdentifierBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/other-identifiers/{id}
pub async fn get_other_case_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<OtherCaseIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest get_other_case_identifier id={}",
		"HANDLER",
		id
	);

	let entity = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/other-identifiers/{id}
pub async fn update_other_case_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<OtherCaseIdentifierForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<OtherCaseIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest update_other_case_identifier id={}",
		"HANDLER",
		id
	);

	let ParamsForUpdate { data } = params;
	OtherCaseIdentifierBmc::update(&ctx, &mm, id, data).await?;
	let entity = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/other-identifiers/{id}
pub async fn delete_other_case_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest delete_other_case_identifier id={}",
		"HANDLER",
		id
	);

	OtherCaseIdentifierBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/other-identifiers/{id}/restore
pub async fn restore_other_case_identifier(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<OtherCaseIdentifier>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;

	let entity = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;
	if entity.case_id != case_id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "other_case_identifiers",
			id,
		}
		.into());
	}
	OtherCaseIdentifierBmc::restore(&ctx, &mm, id).await?;
	let entity = OtherCaseIdentifierBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

// -- Linked Report Numbers (C.1.10.r)

/// POST /api/cases/{case_id}/linked-reports
pub async fn create_linked_report_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<LinkedReportNumberForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<LinkedReportNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest create_linked_report_number case_id={}",
		"HANDLER",
		case_id
	);

	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	let id = LinkedReportNumberBmc::create(&ctx, &mm, data).await?;
	let entity = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

/// GET /api/cases/{case_id}/linked-reports
pub async fn list_linked_report_numbers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<LinkedReportNumber>>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest list_linked_report_numbers case_id={}",
		"HANDLER",
		case_id
	);

	let filter = LinkedReportNumberFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let entities = LinkedReportNumberBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

/// GET /api/cases/{case_id}/linked-reports/{id}
pub async fn get_linked_report_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<LinkedReportNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest get_linked_report_number id={}",
		"HANDLER",
		id
	);

	let entity = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// PUT /api/cases/{case_id}/linked-reports/{id}
pub async fn update_linked_report_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<LinkedReportNumberForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<LinkedReportNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest update_linked_report_number id={}",
		"HANDLER",
		id
	);

	let ParamsForUpdate { data } = params;
	LinkedReportNumberBmc::update(&ctx, &mm, id, data).await?;
	let entity = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

/// DELETE /api/cases/{case_id}/linked-reports/{id}
pub async fn delete_linked_report_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	tracing::debug!(
		"{:<12} - rest delete_linked_report_number id={}",
		"HANDLER",
		id
	);

	LinkedReportNumberBmc::delete(&ctx, &mm, id).await?;

	Ok(StatusCode::NO_CONTENT)
}

/// POST /api/cases/{case_id}/linked-reports/{id}/restore
pub async fn restore_linked_report_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<LinkedReportNumber>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_IDENTIFIER_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;

	let entity = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;
	if entity.case_id != case_id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "linked_report_numbers",
			id,
		}
		.into());
	}
	LinkedReportNumberBmc::restore(&ctx, &mm, id).await?;
	let entity = LinkedReportNumberBmc::get(&ctx, &mm, id).await?;

	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}
