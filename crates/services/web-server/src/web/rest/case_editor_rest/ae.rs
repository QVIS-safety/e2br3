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

repeatable_list_handler!(
	list_editor_ae,
	CaseEditorAeListRowDto,
	REACTION_LIST,
	load_editor_ae_list_rows,
	include_deleted,
);

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

repeatable_page_row_read_handler!(
	get_editor_ae_page_row,
	[CASE_READ, REACTION_READ],
	build_editor_ae_page_row_response,
);

async fn build_editor_ae_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let reaction = ReactionBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	editor_page_row_response(
		case_id,
		"AE",
		row_id,
		authorities,
		json!({ "reaction": reaction }),
	)
}

repeatable_page_row_create_handler!(
	create_editor_ae_page_row,
	section: "AE",
	row_key: "reaction",
	permission: REACTION_CREATE,
	bmc: ReactionBmc,
	model: ReactionForCreate,
	aliases: &[
		("primary_source_reaction", &["reactionPrimarySourceNative"][..]),
		(
			"primary_source_reaction_translation",
			&["reactionPrimarySourceTranslation"][..],
		),
		("reaction_meddra_version", &["meddraVersion"][..]),
		("reaction_meddra_code", &["meddraCode"][..]),
		("sequence_number", &["sequenceNumber"][..]),
	],
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		(
			"sequence_number",
			json!(i32_field(row, &["sequenceNumber", "sequence_number"]).unwrap_or(1)),
		),
	],
	build_response: build_editor_ae_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_ae_page_row,
	section: "AE",
	row_key: "reaction",
	permission: REACTION_UPDATE,
	bmc: ReactionBmc,
	model: ReactionForUpdate,
	changes: &[
		("reactionPrimarySourceNative", "reactionPrimarySourceNative"),
		(
			"reactionPrimarySourceTranslation",
			"reactionPrimarySourceTranslation",
		),
		("meddraVersion", "meddraVersion"),
		("meddraCode", "meddraCode"),
		("outcome", "outcome"),
	],
	aliases: &[
		("primary_source_reaction", &["reactionPrimarySourceNative"][..]),
		(
			"primary_source_reaction_translation",
			&["reactionPrimarySourceTranslation"][..],
		),
		("reaction_meddra_version", &["meddraVersion"][..]),
		("reaction_meddra_code", &["meddraCode"][..]),
	],
	build_response: build_editor_ae_page_row_response,
);

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_ae_page_row,
	restore: restore_editor_ae_page_row,
	bmc: ReactionBmc,
	delete_permission: REACTION_DELETE,
	update_permission: REACTION_UPDATE,
	build_response: build_editor_ae_page_row_response,
);
