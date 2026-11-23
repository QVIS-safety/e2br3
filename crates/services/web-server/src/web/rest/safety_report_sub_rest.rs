// Safety Report sub-resources REST endpoints (C.2.r, C.3.x, C.4.r, C.5)

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use base64::{engine::general_purpose::STANDARD, Engine};
use lib_core::model::case::{
	CaseBmc, CaseForUpdate, SourceDocument, SourceDocumentBmc, SourceDocumentFilter,
	SourceDocumentForCreate, SourceDocumentForUpdate,
};
use lib_core::model::safety_report::{
	DocumentsHeldBySender, DocumentsHeldBySenderBmc, DocumentsHeldBySenderFilter,
	DocumentsHeldBySenderForCreate, DocumentsHeldBySenderForUpdate,
	LiteratureReference, LiteratureReferenceBmc, LiteratureReferenceFilter,
	LiteratureReferenceForCreate, LiteratureReferenceForUpdate, PrimarySource,
	PrimarySourceBmc, PrimarySourceFilter, PrimarySourceForCreate,
	PrimarySourceForUpdate, SenderInformation, SenderInformationBmc,
	SenderInformationFilter, SenderInformationForCreate, SenderInformationForUpdate,
	StudyFdaCrossReportedInd, StudyFdaCrossReportedIndBmc,
	StudyFdaCrossReportedIndFilter, StudyFdaCrossReportedIndForCreate,
	StudyFdaCrossReportedIndForUpdate, StudyInformation, StudyInformationBmc,
	StudyInformationFilter, StudyInformationForCreate, StudyInformationForUpdate,
	StudyRegistrationNumber, StudyRegistrationNumberBmc,
	StudyRegistrationNumberFilter, StudyRegistrationNumberForCreate,
	StudyRegistrationNumberForUpdate,
};
use lib_core::model::{self, ModelManager};
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{Error, Result};
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
use uuid::Uuid;

fn ensure_case_scope(
	case_id: Uuid,
	entity_case_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if case_id != entity_case_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

async fn case_id_for_case(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	case_id: Uuid,
) -> Result<Uuid> {
	Ok(case_id)
}

async fn ensure_case_child_scope(
	_ctx: &lib_core::ctx::Ctx,
	_mm: &ModelManager,
	case_id: Uuid,
	entity_case_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	ensure_case_scope(case_id, entity_case_id, entity_id, entity)
}

async fn ensure_study_case(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	study_id: Uuid,
) -> Result<()> {
	let study = StudyInformationBmc::get(ctx, mm, study_id).await?;
	ensure_case_scope(case_id, study.case_id, study_id, "study_information")
}

async fn mark_case_dirty_c(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<()> {
	CaseBmc::update(
		ctx,
		mm,
		case_id,
		CaseForUpdate {
			dg_prd_key: None,
			status: None,
			review_receivers_json: None,
			workflow_routes_json: None,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: Some(true),
			dirty_d: None,
			dirty_e: None,
			dirty_f: None,
			dirty_g: None,
			dirty_h: None,
		},
	)
	.await
	.map_err(Into::into)
}

fn normalize_primary_source_regulatory_value(
	value: Option<String>,
) -> Result<Option<String>> {
	let normalized = value.and_then(|raw| {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			None
		} else {
			Some(trimmed.to_string())
		}
	});
	match normalized.as_deref() {
		None => Ok(None),
		Some("1") => Ok(Some("1".to_string())),
		Some("2") | Some("3") => Ok(Some("2".to_string())),
		Some(other) => Err(Error::BadRequest {
			message: format!("invalid C.2.r.5 value '{other}' (expected: 1 or 2)"),
		}),
	}
}

fn primary_source_flag_update(value: &str) -> PrimarySourceForUpdate {
	PrimarySourceForUpdate {
		primary_source_regulatory: Some(value.to_string()),
		..Default::default()
	}
}

async fn normalize_primary_source_flags(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	preferred_primary_id: Option<Uuid>,
) -> Result<()> {
	let filter = PrimarySourceFilter {
		case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
			case_id.to_string()
		))])),
		..Default::default()
	};
	let mut entities = PrimarySourceBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?;
	if entities.is_empty() {
		return Ok(());
	}
	entities.sort_by_key(|entity| entity.sequence_number);

	let mut primary_ids = Vec::new();
	for entity in &entities {
		if normalize_primary_source_regulatory_value(
			entity.primary_source_regulatory.clone(),
		)?
		.as_deref()
			== Some("1")
		{
			primary_ids.push(entity.id);
		}
	}
	let chosen_primary_id = match preferred_primary_id {
		Some(id) if entities.iter().any(|entity| entity.id == id) => id,
		Some(_) => {
			return Err(Error::BadRequest {
				message: "preferred primary source does not belong to the case"
					.into(),
			})
		}
		None if primary_ids.len() == 1 => primary_ids[0],
		None => {
			return Err(Error::BadRequest {
				message: "exactly one primary source must be selected".into(),
			})
		}
	};

	for entity in entities {
		let desired = if entity.id == chosen_primary_id {
			"1"
		} else {
			"2"
		};
		let current = normalize_primary_source_regulatory_value(
			entity.primary_source_regulatory.clone(),
		)?;
		if current.as_deref() != Some(desired) {
			PrimarySourceBmc::update(
				ctx,
				mm,
				entity.id,
				primary_source_flag_update(desired),
			)
			.await?;
		}
	}

	Ok(())
}

// -- Sender Information (C.3.x)

/// POST /api/cases/{case_id}/safety-report/senders
pub async fn create_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SenderInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"sender-information:new",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				let id = SenderInformationBmc::create(ctx, mm, data).await?;
				let entity = SenderInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/senders
pub async fn list_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SenderInformation>>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"sender-information:list",
		move |ctx, mm| {
			Box::pin(async move {
				let filter = SenderInformationFilter {
					case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
						case_id.to_string()
					))])),
				};
				let entities = SenderInformationBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/senders/{id}
pub async fn get_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("sender-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = SenderInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(
					case_id,
					entity.case_id,
					id,
					"sender_information",
				)?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/safety-report/senders/{id}
pub async fn update_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SenderInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SenderInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("sender-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = SenderInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(
					case_id,
					entity.case_id,
					id,
					"sender_information",
				)?;
				let ParamsForUpdate { data } = params;
				SenderInformationBmc::update(ctx, mm, id, data).await?;
				let entity = SenderInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/safety-report/senders/{id}
pub async fn delete_sender_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("sender-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = SenderInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(
					case_id,
					entity.case_id,
					id,
					"sender_information",
				)?;
				SenderInformationBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

// -- Primary Sources (C.2.r)

/// POST /api/cases/{case_id}/safety-report/primary-sources
pub async fn create_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PrimarySourceForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"primary-source:new",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				data.primary_source_regulatory =
					normalize_primary_source_regulatory_value(
						data.primary_source_regulatory,
					)?;
				let id = PrimarySourceBmc::create(ctx, mm, data).await?;
				let preferred_primary_id = PrimarySourceBmc::get(ctx, mm, id)
					.await?
					.primary_source_regulatory
					.as_deref()
					.filter(|value| *value == "1")
					.map(|_| id);
				normalize_primary_source_flags(
					ctx,
					mm,
					case_id,
					preferred_primary_id,
				)
				.await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/primary-sources
pub async fn list_primary_sources(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<PrimarySource>>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"primary-source:list",
		move |ctx, mm| {
			Box::pin(async move {
				let filter = PrimarySourceFilter {
					case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
						case_id.to_string()
					))])),
					..Default::default()
				};
				let entities = PrimarySourceBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn get_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("primary-source:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn update_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<PrimarySourceForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("primary-source:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let mut data = data;
				data.primary_source_regulatory =
					normalize_primary_source_regulatory_value(
						data.primary_source_regulatory,
					)?;
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
				PrimarySourceBmc::update(ctx, mm, id, data).await?;
				let preferred_primary_id = PrimarySourceBmc::get(ctx, mm, id)
					.await?
					.primary_source_regulatory
					.as_deref()
					.filter(|value| *value == "1")
					.map(|_| id);
				normalize_primary_source_flags(
					ctx,
					mm,
					case_id,
					preferred_primary_id,
				)
				.await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/safety-report/primary-sources/{id}
pub async fn delete_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("primary-source:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
				PrimarySourceBmc::delete(ctx, mm, id).await?;
				normalize_primary_source_flags(ctx, mm, case_id, None).await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

/// POST /api/cases/{case_id}/safety-report/primary-sources/{id}/restore
pub async fn restore_primary_source(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<PrimarySource>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("primary-source:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "primary_sources")?;
				PrimarySourceBmc::restore(ctx, mm, id).await?;
				normalize_primary_source_flags(ctx, mm, case_id, Some(id)).await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				let entity = PrimarySourceBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

// -- Literature References (C.4.r)

pub(super) fn validate_source_document_base64(
	value: &str,
	path: &str,
) -> Result<()> {
	STANDARD.decode(value).map_err(|_| {
		Error::ConstraintViolation(lib_rest_core::ConstraintViolation {
			rule_code: "LOCAL.SOURCE_DOCUMENT.BASE64".into(),
			path: path.into(),
			message: "Attachment content must be valid Base64.".into(),
		})
	})?;
	Ok(())
}

/// POST /api/cases/{case_id}/source-documents
pub async fn create_source_document(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<SourceDocumentForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<SourceDocument>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"source-document:new",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				if let Some(value) = data.source_document_base64.as_deref() {
					validate_source_document_base64(value, "sourceDocumentBase64")?;
				}
				let id = SourceDocumentBmc::create(ctx, mm, data).await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				let entity = SourceDocumentBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/source-documents
pub async fn list_source_documents(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<SourceDocument>>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"source-document:list",
		move |ctx, mm| {
			Box::pin(async move {
				let filter = SourceDocumentFilter {
					case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
						case_id.to_string()
					))])),
				};
				let entities = SourceDocumentBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/source-documents/{id}
pub async fn update_source_document(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<SourceDocumentForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<SourceDocument>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("source-document:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = SourceDocumentBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "source_documents")?;
				if let Some(value) = params.data.source_document_base64.as_deref() {
					validate_source_document_base64(value, "sourceDocumentBase64")?;
				}
				SourceDocumentBmc::update(ctx, mm, id, params.data).await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				let entity = SourceDocumentBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/source-documents/{id}
pub async fn delete_source_document(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("source-document:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = SourceDocumentBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "source_documents")?;
				SourceDocumentBmc::delete(ctx, mm, id).await?;
				mark_case_dirty_c(ctx, mm, case_id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

fn validate_attachment_input(
	value: Option<&str>,
	path: &str,
	check: fn(input_contracts::FieldInput<'_>) -> Vec<input_contracts::InputIssue>,
) -> Result<()> {
	let input = match value {
		Some(value) => input_contracts::InputValue::String(value),
		None => input_contracts::InputValue::Missing,
	};
	if let Some(issue) = check(input_contracts::FieldInput::new(input, None))
		.into_iter()
		.next()
	{
		return Err(Error::ConstraintViolation(
			lib_rest_core::ConstraintViolation {
				rule_code: issue.code.to_string(),
				path: path.to_string(),
				message: issue.message,
			},
		));
	}
	Ok(())
}

fn validate_included_document(value: Option<&str>) -> Result<()> {
	validate_attachment_input(
		value,
		"includedDocument",
		input_contracts::generated::c::c_1_6_1_r_2,
	)
}

fn validate_literature_document(value: Option<&str>) -> Result<()> {
	validate_attachment_input(
		value,
		"documentBase64",
		input_contracts::generated::c::c_4_r_2,
	)
}

#[cfg(test)]
mod attachment_input_tests {
	use super::*;

	#[test]
	fn legacy_attachments_share_scalar_contracts_without_submission_requirements() {
		for (check, code, path) in [
			(
				validate_included_document as fn(Option<&str>) -> Result<()>,
				"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
				"includedDocument",
			),
			(
				validate_literature_document,
				"ICH.C.4.r.2.ALLOWED.VALUE",
				"documentBase64",
			),
		] {
			for value in ["%invalid-base64%", "A", "AA=", "TR=="] {
				match check(Some(value)).expect_err("invalid encoding") {
					Error::ConstraintViolation(detail) => {
						assert_eq!(detail.rule_code, code);
						assert_eq!(detail.path, path);
					}
					other => panic!("unexpected error: {other:?}"),
				}
			}
			for value in [None, Some(""), Some("TQ=="), Some("TWE="), Some("TWFu")] {
				check(value).expect("absent or valid content");
			}
		}
	}
}

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: DocumentsHeldBySenderBmc,
	Entity: DocumentsHeldBySender,
	ForCreate: DocumentsHeldBySenderForCreate,
	ForUpdate: DocumentsHeldBySenderForUpdate,
	Filter: DocumentsHeldBySenderFilter,
	CreateFn: create_documents_held_by_sender,
	ListFn: list_documents_held_by_sender,
	GetFn: get_documents_held_by_sender,
	UpdateFn: update_documents_held_by_sender,
	DeleteFn: delete_documents_held_by_sender,
	RestoreFn: restore_documents_held_by_sender,
	ParentField: case_id,
	ResolveParentFn: case_id_for_case,
	ScopeFn: ensure_case_child_scope,
	EntityName: "documents_held_by_sender",
	DeleteResult: StatusCode,
	DeleteResponse: no_content,
	ValidateField: document_base64 => validate_included_document
}

lib_rest_core::generate_patient_child_rest_fns! {
	Bmc: LiteratureReferenceBmc,
	Entity: LiteratureReference,
	ForCreate: LiteratureReferenceForCreate,
	ForUpdate: LiteratureReferenceForUpdate,
	Filter: LiteratureReferenceFilter,
	CreateFn: create_literature_reference,
	ListFn: list_literature_references,
	GetFn: get_literature_reference,
	UpdateFn: update_literature_reference,
	DeleteFn: delete_literature_reference,
	RestoreFn: restore_literature_reference,
	ParentField: case_id,
	ResolveParentFn: case_id_for_case,
	ScopeFn: ensure_case_child_scope,
	EntityName: "literature_references",
	DeleteResult: StatusCode,
	DeleteResponse: no_content,
	ValidateField: document_base64 => validate_literature_document
}

// -- Study Information (C.5)

/// POST /api/cases/{case_id}/safety-report/studies
pub async fn create_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<StudyInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"study-information:new",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				let id = StudyInformationBmc::create(ctx, mm, data).await?;
				let entity = StudyInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies
pub async fn list_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Vec<StudyInformation>>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"study-information:list",
		move |ctx, mm| {
			Box::pin(async move {
				let filter = StudyInformationFilter {
					case_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
						case_id.to_string()
					))])),
				};
				let entities = StudyInformationBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies/{id}
pub async fn get_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = StudyInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/safety-report/studies/{id}
pub async fn update_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = StudyInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
				let ParamsForUpdate { data } = params;
				StudyInformationBmc::update(ctx, mm, id, data).await?;
				let entity = StudyInformationBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{id}
pub async fn delete_study_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-information:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let entity = StudyInformationBmc::get(ctx, mm, id).await?;
				ensure_case_scope(case_id, entity.case_id, id, "study_information")?;
				StudyInformationBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

// -- Study Registration Numbers (C.5.1.r)

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/registrations
pub async fn create_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<StudyRegistrationNumberForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:new:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.study_information_id = study_id;
				let id = StudyRegistrationNumberBmc::create(ctx, mm, data).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/registrations
pub async fn list_study_registration_numbers(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyRegistrationNumber>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:list:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let filter = StudyRegistrationNumberFilter {
					study_information_id: Some(OpValsValue::from(vec![
						OpValValue::Eq(json!(study_id.to_string())),
					])),
					..Default::default()
				};
				let entities = StudyRegistrationNumberBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn get_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_registration_numbers",
						id,
					}
					.into());
				}
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn update_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyRegistrationNumberForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_registration_numbers",
						id,
					}
					.into());
				}
				let ParamsForUpdate { data } = params;
				StudyRegistrationNumberBmc::update(ctx, mm, id, data).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}
pub async fn delete_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_registration_numbers",
						id,
					}
					.into());
				}
				StudyRegistrationNumberBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}/restore
pub async fn restore_study_registration_number(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyRegistrationNumber>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-registration:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_registration_numbers",
						id,
					}
					.into());
				}
				StudyRegistrationNumberBmc::restore(ctx, mm, id).await?;
				let entity = StudyRegistrationNumberBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

// -- Study FDA Cross-Reported INDs (FDA.C.5.6.r)

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds
pub async fn create_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForCreate<StudyFdaCrossReportedIndForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:new:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.study_information_id = study_id;
				let id = StudyFdaCrossReportedIndBmc::create(ctx, mm, data).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds
pub async fn list_study_fda_cross_reported_inds(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Vec<StudyFdaCrossReportedInd>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:list:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let filter = StudyFdaCrossReportedIndFilter {
					study_information_id: Some(OpValsValue::from(vec![
						OpValValue::Eq(json!(study_id.to_string())),
					])),
					..Default::default()
				};
				let entities = StudyFdaCrossReportedIndBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(ListOptions::default()),
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
			})
		},
	)
	.await
}

/// GET /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn get_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_fda_cross_reported_inds",
						id,
					}
					.into());
				}
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// PUT /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn update_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<StudyFdaCrossReportedIndForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_fda_cross_reported_inds",
						id,
					}
					.into());
				}
				let ParamsForUpdate { data } = params;
				StudyFdaCrossReportedIndBmc::update(ctx, mm, id, data).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

/// DELETE /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}
pub async fn delete_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_fda_cross_reported_inds",
						id,
					}
					.into());
				}
				StudyFdaCrossReportedIndBmc::delete(ctx, mm, id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}

/// POST /api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}/restore
pub async fn restore_study_fda_cross_reported_ind(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, study_id, id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<DataRestResult<StudyFdaCrossReportedInd>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("study-fda-cross-reported-ind:{id}:study:{study_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_study_case(ctx, mm, case_id, study_id).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				if entity.study_information_id != study_id {
					return Err(model::Error::EntityUuidNotFound {
						entity: "study_fda_cross_reported_inds",
						id,
					}
					.into());
				}
				StudyFdaCrossReportedIndBmc::restore(ctx, mm, id).await?;
				let entity = StudyFdaCrossReportedIndBmc::get(ctx, mm, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}
