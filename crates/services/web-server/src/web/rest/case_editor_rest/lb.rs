use super::common::*;

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
				result_value: test.test_result_value,
				result_unit: test.test_result_unit,
			})
			.collect(),
	)
}

repeatable_list_handler!(
	list_editor_lb,
	CaseEditorLbListRowDto,
	TEST_RESULT_LIST,
	load_editor_lb_list_rows,
	include_deleted,
);

pub async fn get_editor_lb_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_lb_list_rows(
		&ctx,
		&mm,
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
}

pub async fn get_editor_lb(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, test_result_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let test_result =
		TestResultBmc::get_in_case(&ctx, &mm, case_id, test_result_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: test_result_id,
			data: json!({ "testResults": [test_result] }),
		}),
	))
}

repeatable_page_row_read_handler!(
	get_editor_lb_page_row,
	[CASE_READ, TEST_RESULT_READ],
	build_editor_lb_page_row_response,
);

async fn build_editor_lb_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let test_result = TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
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
	section: "LB",
	row_key: "testResult",
	permission: TEST_RESULT_CREATE,
	bmc: TestResultBmc,
	model: TestResultForCreate,
	aliases: &[
		("test_name", &["testName"][..]),
		("test_result_value", &["resultValue"][..]),
		("test_result_unit", &["resultUnit"][..]),
		("sequence_number", &["sequenceNumber"][..]),
	],
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		(
			"sequence_number",
			json!(i32_field(row, &["sequenceNumber", "sequence_number"]).unwrap_or(1)),
		),
	],
	build_response: build_editor_lb_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_lb_page_row,
	section: "LB",
	row_key: "testResult",
	permission: TEST_RESULT_UPDATE,
	bmc: TestResultBmc,
	model: TestResultForUpdate,
	changes: &[
		("testName", "testName"),
		("resultValue", "resultValue"),
		("resultUnit", "resultUnit"),
	],
	aliases: &[
		("test_name", &["testName"][..]),
		("test_result_value", &["resultValue"][..]),
		("test_result_unit", &["resultUnit"][..]),
	],
	build_response: build_editor_lb_page_row_response,
);

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_lb_page_row,
	restore: restore_editor_lb_page_row,
	bmc: TestResultBmc,
	delete_permission: TEST_RESULT_DELETE,
	update_permission: TEST_RESULT_UPDATE,
	build_response: build_editor_lb_page_row_response,
);
