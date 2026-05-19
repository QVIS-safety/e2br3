use crate::web::rest::case_editor_dto::CaseEditorShellDto;
use crate::web::rest::case_rest::case_to_read_result;
use axum::extract::{Path, State};
use axum::Json;
use lib_core::model::acs::CASE_READ;
use lib_core::model::case::CaseBmc;
use lib_core::model::ModelManager;
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;
use uuid::Uuid;

pub async fn get_editor_shell(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorShellDto>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;
	let case = CaseBmc::get(&ctx, &mm, case_id).await?;
	let case = case_to_read_result(&ctx, &mm, case).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorShellDto::from(case)),
	))
}
