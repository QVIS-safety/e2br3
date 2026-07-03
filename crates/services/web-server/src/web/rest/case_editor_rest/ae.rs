use super::common::*;

async fn load_editor_ae_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	include_deleted: bool,
) -> Result<Vec<CaseEditorAeListRowDto>> {
	Ok(
		ReactionBmc::list_by_case_with_deleted(ctx, mm, case_id, include_deleted)
			.await?
			.into_iter()
			.map(|reaction| CaseEditorAeListRowDto {
				id: reaction.id,
				sequence_number: reaction.sequence_number,
				deleted: reaction.deleted,
				reaction_primary_source_native: reaction.primary_source_reaction,
				reaction_primary_source_translation: reaction
					.primary_source_reaction_translation,
				meddra_version: reaction.reaction_meddra_version,
				meddra_code: reaction.reaction_meddra_code,
				seriousness: reaction.serious,
			})
			.collect(),
	)
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

	let rows = load_editor_ae_list_rows(&ctx, &mm, case_id, false).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_ae_page_projection(
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
	require_permission(&ctx, REACTION_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_ae_list_rows(
		&ctx,
		&mm,
		case_id,
		query.include_deleted.unwrap_or(false),
	)
	.await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"AE",
		query_authorities_csv(&query)?,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub async fn get_editor_ae(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, reaction_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let reaction = ReactionBmc::get_in_case(&ctx, &mm, case_id, reaction_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: reaction_id,
			data: json!({ "reactions": [reaction] }),
		}),
	))
}

pub async fn get_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, REACTION_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query_authorities_csv(&query)?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_ae_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let reaction = ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("AE"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, authorities)?;
	response.insert("data".to_string(), json!({ "reaction": reaction }));
	Ok(Value::Object(response))
}

pub async fn create_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	let row = required_row_object("AE", &request.rows, "reaction")?;
	let value = row_model_value(
		row,
		&[
			(
				"primary_source_reaction",
				&["reactionPrimarySourceNative"][..],
			),
			(
				"primary_source_reaction_translation",
				&["reactionPrimarySourceTranslation"][..],
			),
			("reaction_meddra_version", &["meddraVersion"][..]),
			("reaction_meddra_code", &["meddraCode"][..]),
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
	let create = parse_row_model::<ReactionForCreate>("AE", "reaction", value)?;
	let row_id = ReactionBmc::create(&ctx, &mm, create).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let synthesized_rows;
	let rows = if !request.changes.is_empty() {
		synthesized_rows = row_payload_from_changes(
			"AE",
			"reaction",
			&request.changes,
			&[
				("reactionPrimarySourceNative", "reactionPrimarySourceNative"),
				(
					"reactionPrimarySourceTranslation",
					"reactionPrimarySourceTranslation",
				),
				("meddraVersion", "meddraVersion"),
				("meddraCode", "meddraCode"),
				("outcome", "outcome"),
			],
		)?;
		&synthesized_rows
	} else {
		&request.rows
	};
	let row = required_row_object("AE", rows, "reaction")?;
	let value = row_model_value(
		row,
		&[
			(
				"primary_source_reaction",
				&["reactionPrimarySourceNative"][..],
			),
			(
				"primary_source_reaction_translation",
				&["reactionPrimarySourceTranslation"][..],
			),
			("reaction_meddra_version", &["meddraVersion"][..]),
			("reaction_meddra_code", &["meddraCode"][..]),
		],
		&[],
	);
	let update = parse_row_model::<ReactionForUpdate>("AE", "reaction", value)?;
	ReactionBmc::update(&ctx, &mm, row_id, update).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_ae_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	ReactionBmc::delete(&ctx, &mm, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn restore_editor_ae_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	ReactionBmc::get_in_case_with_deleted(&ctx, &mm, case_id, row_id, true).await?;
	ReactionBmc::restore_in_case(&ctx, &mm, case_id, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	let response =
		build_editor_ae_page_row_response(&ctx, &mm, case_id, row_id, None).await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}
