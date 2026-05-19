use crate::web::rest::case_editor_dto::{
	CaseEditorAeListRowDto, CaseEditorDgListRowDto, CaseEditorDhListRowDto,
	CaseEditorLbListRowDto, CaseEditorListResponse, CaseEditorShellDto,
};
use crate::web::rest::case_rest::case_to_read_result;
use axum::extract::{Path, State};
use axum::Json;
use lib_core::model::acs::{
	CASE_READ, DRUG_LIST, PAST_DRUG_LIST, REACTION_LIST, TEST_RESULT_LIST,
};
use lib_core::model::case::CaseBmc;
use lib_core::model::drug::DrugInformationBmc;
use lib_core::model::patient::{
	PastDrugHistoryBmc, PastDrugHistoryFilter, PatientInformationBmc,
};
use lib_core::model::reaction::ReactionBmc;
use lib_core::model::test_result::TestResultBmc;
use lib_core::model::ModelManager;
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;
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

pub async fn list_editor_ae(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorAeListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = ReactionBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|reaction| CaseEditorAeListRowDto {
			id: reaction.id,
			sequence_number: reaction.sequence_number,
			reaction_primary_source_native: reaction.primary_source_reaction,
			reaction_primary_source_translation: reaction
				.primary_source_reaction_translation,
			meddra_version: reaction.reaction_meddra_version,
			meddra_code: reaction.reaction_meddra_code,
			seriousness: reaction.serious,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn list_editor_lb(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorLbListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = TestResultBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|test| CaseEditorLbListRowDto {
			id: test.id,
			sequence_number: test.sequence_number,
			test_name: test.test_name,
			test_date: test.test_date.map(|date| date.to_string()),
			result_value: test.test_result_value,
			result_unit: test.test_result_unit,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn list_editor_dg(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorDgListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = DrugInformationBmc::list_by_case(&ctx, &mm, case_id)
		.await?
		.into_iter()
		.map(|drug| CaseEditorDgListRowDto {
			id: drug.id,
			sequence_number: drug.sequence_number,
			drug_role: drug.drug_characterization,
			dg_prd_key: None,
			medicinal_product: drug.medicinal_product,
			action_taken: drug.action_taken,
			warning_count: 0,
		})
		.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn list_editor_dh(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorListResponse<CaseEditorDhListRowDto>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(patient
			.id
			.to_string()))])),
		..Default::default()
	};
	let rows = PastDrugHistoryBmc::list(
		&ctx,
		&mm,
		Some(vec![filter]),
		Some(ListOptions::default()),
	)
	.await?
	.into_iter()
	.map(|history| CaseEditorDhListRowDto {
		id: history.id,
		sequence_number: history.sequence_number,
		drug_name: history.drug_name,
		indication: history.indication_meddra_code,
		start_date: history.start_date.map(|date| date.to_string()),
		end_date: history.end_date.map(|date| date.to_string()),
	})
	.collect();

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}
