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

	let rows = load_editor_lb_list_rows(&ctx, &mm, case_id, false).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

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

pub async fn get_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, TEST_RESULT_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query_authorities_csv(&query)?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_lb_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let test_result = TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("LB"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, authorities)?;
	response.insert("data".to_string(), json!({ "testResult": test_result }));
	Ok(Value::Object(response))
}

pub async fn create_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	let row = required_row_object("LB", &request.rows, "testResult")?;
	let value = row_model_value(
		row,
		&[
			("test_name", &["testName"][..]),
			("test_result_value", &["resultValue"][..]),
			("test_result_unit", &["resultUnit"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
		],
	);
	let create = parse_row_model::<TestResultForCreate>("LB", "testResult", value)?;
	let row_id = TestResultBmc::create(&ctx, &mm, create).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let synthesized_rows;
	let rows = if !request.changes.is_empty() {
		synthesized_rows = row_payload_from_changes(
			"LB",
			"testResult",
			&request.changes,
			&[
				("testName", "testName"),
				("resultValue", "resultValue"),
				("resultUnit", "resultUnit"),
			],
		)?;
		&synthesized_rows
	} else {
		&request.rows
	};
	let row = required_row_object("LB", rows, "testResult")?;
	let value = row_model_value(
		row,
		&[
			("test_name", &["testName"][..]),
			("test_result_value", &["resultValue"][..]),
			("test_result_unit", &["resultUnit"][..]),
		],
		&[],
	);
	let update = parse_row_model::<TestResultForUpdate>("LB", "testResult", value)?;
	TestResultBmc::update(&ctx, &mm, row_id, update).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_lb_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	TestResultBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	TestResultBmc::delete(&ctx, &mm, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn restore_editor_lb_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	TestResultBmc::get_in_case_with_deleted(&ctx, &mm, case_id, row_id, true)
		.await?;
	TestResultBmc::restore_in_case(&ctx, &mm, case_id, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	let response =
		build_editor_lb_page_row_response(&ctx, &mm, case_id, row_id, None).await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}
