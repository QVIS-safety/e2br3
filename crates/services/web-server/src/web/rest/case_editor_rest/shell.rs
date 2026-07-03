use super::common::*;

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
