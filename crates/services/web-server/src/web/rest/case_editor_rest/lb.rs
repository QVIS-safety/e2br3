use super::common::{
	ci_date, editor_page_row_response, i32_field, json, query_authorities_csv,
	repeatable_page_projection_response, CaseEditorLbListRowDto,
	CaseEditorPageProjectionQuery, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CtxW, Json, ModelManager, Path, Query, Result,
	State, TestResultBmc, TestResultForCreate, TestResultForUpdate, Uuid, Value,
};
use super::handler_macros::{
	repeatable_list_handler, repeatable_page_row_create_handler,
	repeatable_page_row_delete_restore_handlers, repeatable_page_row_patch_handler,
	repeatable_page_row_read_handler,
};

const TEST_RESULT_ROW_ALIASES: &[(&str, &[&str])] = &[
	("test_date", &["testDate"]),
	("test_date_null_flavor", &["testDateNullFlavor"]),
	("test_name", &["testName"]),
	("test_meddra_version", &["testMeddraVersion"]),
	("test_meddra_code", &["testMeddraCode"]),
	("test_result_code", &["testResultCode"]),
	("test_result_value", &["testResult"]),
	("test_result_qualifier", &["testResultQualifier"]),
	("test_result_unit", &["testUnit"]),
	("result_unstructured", &["testResultUnstructured"]),
	("normal_low_value", &["lowRange"]),
	("normal_high_value", &["highRange"]),
	("comments", &["comments"]),
	("more_info_available", &["moreInformationAvailable"]),
	("sequence_number", &["sequenceNumber"]),
];

async fn load_editor_lb_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	include_deleted: bool,
) -> Result<Vec<CaseEditorLbListRowDto>> {
	Ok(
		TestResultBmc::list_by_case_with_deleted(ctx, mm, case_id, include_deleted)
			.await?
			.into_iter()
			.map(|test| CaseEditorLbListRowDto {
				id: test.id,
				sequence_number: test.sequence_number,
				deleted: test.deleted,
				test_name: test.test_name,
				test_date: test.test_date.map(|date| date.to_string()),
				test_result_code: test.test_result_code,
				test_result: test.test_result_value,
				test_result_unstructured: test.result_unstructured,
				comments: test.comments,
			})
			.collect(),
	)
}

repeatable_list_handler!(
	list_editor_lb,
	CaseEditorLbListRowDto,
	load_editor_lb_list_rows,
	include_deleted,
);

pub async fn get_editor_lb_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/LB",
		move |ctx, mm| {
			Box::pin(async move {
				let rows = load_editor_lb_list_rows(
					ctx,
					mm,
					case_id,
					query.include_deleted.unwrap_or(false),
				)
				.await?;
				let projection = repeatable_page_projection_response(
					case_id,
					"LB",
					query_authorities_csv(&query)?,
					json!({ "rows": rows }),
				)?;
				Ok((axum::http::StatusCode::OK, Json(projection)))
			})
		},
	)
	.await
}

pub async fn get_editor_lb(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, test_result_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/LB/{test_result_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let test_result =
					TestResultBmc::get_in_case(ctx, mm, case_id, test_result_id)
						.await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(CaseEditorRowDetailResponse {
						case_id,
						row_id: test_result_id,
						data: json!({ "testResults": [test_result] }),
					}),
				))
			})
		},
	)
	.await
}

repeatable_page_row_read_handler!(
	get_editor_lb_page_row,
	build_editor_lb_page_row_response,
);

async fn build_editor_lb_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let persisted = TestResultBmc::get_in_case(ctx, mm, case_id, row_id).await?;
	let test_date = ci_date(persisted.test_date);
	let mut test_result = json!(persisted);
	if let Value::Object(ref mut map) = test_result {
		map.insert("test_date".to_string(), json!(test_date));
	}
	editor_page_row_response(
		case_id,
		"LB",
		row_id,
		authorities,
		json!({ "testResult": test_result }),
	)
}

repeatable_page_row_create_handler!(
	create_editor_lb_page_row,
	apply: apply_editor_lb_page_row_create,
	section: "LB",
	row_key: "testResult",
	bmc: TestResultBmc,
	model: TestResultForCreate,
	aliases: TEST_RESULT_ROW_ALIASES,
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		(
			"sequence_number",
			json!(i32_field(row, &["sequenceNumber"]).unwrap_or(1)),
		),
	],
	build_response: build_editor_lb_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_lb_page_row,
	section: "LB",
	row_key: "testResult",
	bmc: TestResultBmc,
	model: TestResultForUpdate,
	aliases: TEST_RESULT_ROW_ALIASES,
	build_response: build_editor_lb_page_row_response,
);

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_lb_page_row,
	restore: restore_editor_lb_page_row,
	bmc: TestResultBmc,
	build_response: build_editor_lb_page_row_response,
);
