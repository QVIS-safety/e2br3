use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::model::acs::{
	PATIENT_CREATE, PATIENT_DELETE, PATIENT_READ, PATIENT_UPDATE,
};
use lib_core::model::patient::{
	PatientInformation, PatientInformationBmc, PatientInformationForCreate,
	PatientInformationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{require_case_write_allowed, require_permission, Result};
use lib_web::middleware::mw_auth::CtxW;
use std::borrow::Cow;
use uuid::Uuid;

fn is_unique_violation(err: &lib_core::model::Error) -> bool {
	matches!(err, lib_core::model::Error::UniqueViolation { .. })
		|| matches!(
			err.as_database_error().and_then(|db| db.code()),
			Some(Cow::Borrowed("23505"))
		) || {
		let text = format!("{err:?}").to_ascii_lowercase();
		text.contains("duplicate") || text.contains("unique")
	}
}

pub async fn create_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForCreate<PatientInformationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_CREATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForCreate { data } = params;
	let mut data = data;
	data.case_id = case_id;

	match PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await {
		Ok(entity) => {
			return Ok((StatusCode::OK, Json(DataRestResult { data: entity })));
		}
		Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {}
		Err(err) => return Err(err.into()),
	}

	match PatientInformationBmc::create(&ctx, &mm, data).await {
		Ok(_) => {
			let entity =
				PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
			Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
		}
		Err(err) if is_unique_violation(&err) => {
			match PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await {
				Ok(entity) => {
					Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
				}
				Err(_) => Err(err.into()),
			}
		}
		Err(err) => Err(err.into()),
	}
}

pub async fn get_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_READ)?;
	let entity = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn update_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<PatientInformationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<PatientInformation>>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	let ParamsForUpdate { data } = params;
	PatientInformationBmc::update_by_case(&ctx, &mm, case_id, data).await?;
	let entity = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_patient(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PATIENT_DELETE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	PatientInformationBmc::delete_by_case(&ctx, &mm, case_id).await?;
	Ok(StatusCode::NO_CONTENT)
}
