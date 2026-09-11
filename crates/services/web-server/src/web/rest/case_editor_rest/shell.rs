use super::common::{
	case_to_read_result, CaseBmc, CaseEditorShellDto, CtxW, Error, Json,
	ModelManager, Path, Result, SafetyReportIdentificationBmc, State, Uuid,
};

pub async fn get_editor_shell(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorShellDto>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/shell",
		move |ctx, mm| {
			Box::pin(async move {
				let case = CaseBmc::get(ctx, mm, case_id).await?;
				let case = case_to_read_result(ctx, mm, case).await?;
				let safety_report_id =
					SafetyReportIdentificationBmc::get_by_case(ctx, mm, case_id)
						.await?
						.safety_report_id
						.filter(|value| !value.trim().is_empty())
						.ok_or_else(|| Error::BadRequest {
							message: format!(
								"case {case_id} has no safety report ID"
							),
						})?;
				Ok((
					axum::http::StatusCode::OK,
					Json(CaseEditorShellDto::from_case_read_result(
						case,
						safety_report_id,
					)?),
				))
			})
		},
	)
	.await
}
