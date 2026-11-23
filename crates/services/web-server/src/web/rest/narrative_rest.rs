use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::narrative::{
	NarrativeInformation, NarrativeInformationBmc, NarrativeInformationForCreate,
	NarrativeInformationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{get_or_create_singleton, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

pub async fn create_narrative_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<NarrativeInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativeInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"narrative",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				get_or_create_singleton(
					|| NarrativeInformationBmc::get_by_case(ctx, mm, case_id),
					NarrativeInformationBmc::create(ctx, mm, data),
				)
				.await
			})
		},
	)
	.await
}

pub async fn get_narrative_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<Option<NarrativeInformation>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"narrative",
		move |ctx, mm| {
			Box::pin(async move {
				let entity =
					NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id)
						.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn update_narrative_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<serde_json::Value>>,
) -> Result<(StatusCode, Json<DataRestResult<NarrativeInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"narrative",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let update: NarrativeInformationForUpdate =
					serde_json::from_value(data.clone()).map_err(|_| {
						lib_rest_core::Error::BadRequest {
							message: "Invalid narrative fields.".into(),
						}
					})?;
				let clear_fields: Vec<&str> = [
					"source_narrative_presave_id",
					"case_narrative",
					"reporter_comments",
					"sender_comments",
					"additional_information",
				]
				.into_iter()
				.filter(|field| {
					data.get(*field).is_some_and(serde_json::Value::is_null)
				})
				.collect();
				NarrativeInformationBmc::update_by_case_patch(
					ctx,
					mm,
					case_id,
					update,
					&clear_fields,
				)
				.await?;
				let entity =
					NarrativeInformationBmc::get_by_case(ctx, mm, case_id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn delete_narrative_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"narrative",
		move |ctx, mm| {
			Box::pin(async move {
				NarrativeInformationBmc::delete_by_case(ctx, mm, case_id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}
