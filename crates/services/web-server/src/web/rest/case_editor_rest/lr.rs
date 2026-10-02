use super::common::{
	editor_page_row_response, i32_field, json, query_authorities_csv,
	repeatable_page_projection_response, uuid_eq, CaseEditorPageProjectionQuery,
	CaseEditorPageProjectionResponse, CtxW, Error, Json, ListOptions,
	LiteratureReferenceBmc, LiteratureReferenceFilter, LiteratureReferenceForCreate,
	LiteratureReferenceForUpdate, ModelManager, Path, Query, Result, State, Uuid,
	Value,
};
use super::handler_macros::{
	repeatable_page_row_create_handler, repeatable_page_row_delete_handler,
	repeatable_page_row_patch_handler, repeatable_page_row_read_handler,
};

const LITERATURE_REFERENCE_ROW_ALIASES: &[(&str, &[&str])] = &[
	("reference_text", &["referenceText"]),
	("reference_text_null_flavor", &["referenceTextNullFlavor"]),
	("sequence_number", &["sequenceNumber"]),
	("document_base64", &["documentBase64"]),
	("file_name", &["fileName"]),
	("media_type", &["mediaType"]),
	("representation", &["representation"]),
	("compression", &["compression"]),
];

async fn load_editor_lr_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<Value>> {
	Ok(LiteratureReferenceBmc::list(
		ctx,
		mm,
		Some(vec![LiteratureReferenceFilter {
			case_id: Some(uuid_eq(case_id)),
			..Default::default()
		}]),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?
	.into_iter()
	.map(|row| {
		json!({
			"id": row.id,
			"sequenceNumber": row.sequence_number,
			"referenceText": row.reference_text,
			"referenceTextNullFlavor": row.reference_text_null_flavor,
			"documentBase64": row.document_base64,
			"fileName": row.file_name,
			"mediaType": row.media_type,
			"representation": row.representation,
			"compression": row.compression,
			"deleted": row.deleted,
		})
	})
	.collect())
}

pub async fn get_editor_lr_page_projection(
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
		"editor/LR",
		move |ctx, mm| {
			Box::pin(async move {
				let rows = load_editor_lr_list_rows(ctx, mm, case_id).await?;
				let projection = repeatable_page_projection_response(
					case_id,
					"LR",
					query_authorities_csv(&query)?,
					json!({ "rows": rows }),
				)?;
				Ok((axum::http::StatusCode::OK, Json(projection)))
			})
		},
	)
	.await
}

async fn load_editor_lr_row(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<lib_core::model::safety_report::LiteratureReference> {
	let row = LiteratureReferenceBmc::get(ctx, mm, row_id).await?;
	if row.case_id != case_id {
		return Err(Error::BadRequest {
			message: "LR literature reference does not belong to the current case"
				.to_string(),
		});
	}
	Ok(row)
}

async fn build_editor_lr_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let row = load_editor_lr_row(ctx, mm, case_id, row_id).await?;
	editor_page_row_response(
		case_id,
		"LR",
		row_id,
		authorities,
		json!({ "literatureReference": row }),
	)
}

async fn verify_editor_lr_page_row(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<()> {
	load_editor_lr_row(ctx, mm, case_id, row_id).await?;
	Ok(())
}

repeatable_page_row_read_handler!(
	get_editor_lr_page_row,
	build_editor_lr_page_row_response,
);

repeatable_page_row_create_handler!(
	create_editor_lr_page_row,
	apply: apply_editor_lr_page_row_create,
	section: "LR",
	row_key: "literatureReference",
	bmc: LiteratureReferenceBmc,
	model: LiteratureReferenceForCreate,
	aliases: LITERATURE_REFERENCE_ROW_ALIASES,
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		(
			"sequence_number",
			json!(i32_field(row, &["sequenceNumber"]).unwrap_or(1)),
		),
	],
	build_response: build_editor_lr_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_lr_page_row,
	section: "LR",
	row_key: "literatureReference",
	bmc: LiteratureReferenceBmc,
	model: LiteratureReferenceForUpdate,
	verify: verify_editor_lr_page_row,
	aliases: LITERATURE_REFERENCE_ROW_ALIASES,
	base_patch: true,
	restore_on_false: true,
	build_response: build_editor_lr_page_row_response,
);

repeatable_page_row_delete_handler!(
	delete_editor_lr_page_row,
	bmc: LiteratureReferenceBmc,
	verify: verify_editor_lr_page_row,
);
