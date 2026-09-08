use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::patient::{
	PatientInformation, PatientInformationBmc, PatientInformationForCreate,
	PatientInformationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{get_or_create_singleton, Result};
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

pub async fn create_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PatientInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"patient",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForCreate { data } = params;
				let mut data = data;
				data.case_id = case_id;
				get_or_create_singleton(
					|| PatientInformationBmc::get_by_case(ctx, mm, case_id),
					PatientInformationBmc::create(ctx, mm, data),
				)
				.await
			})
		},
	)
	.await
}

pub async fn get_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"patient",
		move |ctx, mm| {
			Box::pin(async move {
				let entity =
					PatientInformationBmc::get_by_case(ctx, mm, case_id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn update_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<PatientInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"patient",
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				PatientInformationBmc::update_by_case(ctx, mm, case_id, data)
					.await?;
				let entity =
					PatientInformationBmc::get_by_case(ctx, mm, case_id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
			})
		},
	)
	.await
}

pub async fn delete_patient(
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
		"patient",
		move |ctx, mm| {
			Box::pin(async move {
				PatientInformationBmc::delete_by_case(ctx, mm, case_id).await?;
				Ok(StatusCode::NO_CONTENT)
			})
		},
	)
	.await
}
