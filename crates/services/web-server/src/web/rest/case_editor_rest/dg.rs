use super::common::*;

async fn load_editor_dg_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	include_deleted: bool,
) -> Result<Vec<CaseEditorDgListRowDto>> {
	Ok(DrugInformationBmc::list_by_case_with_deleted(
		ctx,
		mm,
		case_id,
		include_deleted,
	)
	.await?
	.into_iter()
	.map(|drug| CaseEditorDgListRowDto {
		id: drug.id,
		sequence_number: drug.sequence_number,
		deleted: drug.deleted,
		drug_role: drug.drug_characterization,
		dg_prd_key: drug.source_product_presave_id.map(|id| id.to_string()),
		medicinal_product: drug.medicinal_product,
		action_taken: drug.action_taken,
		warning_count: 0,
	})
	.collect())
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

	let rows = load_editor_dg_list_rows(&ctx, &mm, case_id, false).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_dg_page_projection(
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
	require_permission(&ctx, DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_dg_list_rows(
		&ctx,
		&mm,
		case_id,
		query.include_deleted.unwrap_or(false),
	)
	.await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"DG",
		query_authorities_csv(&query)?,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

fn drug_id_filter<T>(drug_id: Uuid) -> Option<Vec<T>>
where
	T: Default,
	T: FromDrugIdFilter,
{
	Some(vec![T::from_drug_id(drug_id)])
}

trait FromDrugIdFilter {
	fn from_drug_id(drug_id: Uuid) -> Self;
}

impl FromDrugIdFilter for DrugActiveSubstanceFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

impl FromDrugIdFilter for DosageInformationFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

impl FromDrugIdFilter for DrugIndicationFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(
				drug_id.to_string()
			))])),
			..Default::default()
		}
	}
}

async fn load_editor_dg_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Value> {
	let drug = DrugInformationBmc::get_in_case(ctx, mm, case_id, drug_id).await?;
	let active_substances = DrugActiveSubstanceBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugActiveSubstanceFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let dosage_information = DosageInformationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DosageInformationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let indications = DrugIndicationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugIndicationFilter>(drug_id),
		Some(ListOptions::default()),
	)
	.await?;
	let drug_reaction_assessments =
		DrugReactionAssessmentBmc::list_by_drug(ctx, mm, drug_id).await?;
	let drug_recurrences =
		DrugRecurrenceInformationBmc::list_by_drug(ctx, mm, drug_id).await?;

	let mut drug = json!(drug);
	if let Value::Object(ref mut map) = drug {
		map.insert("activeSubstances".to_string(), json!(active_substances));
		map.insert("dosageInformation".to_string(), json!(dosage_information));
		map.insert("indications".to_string(), json!(indications));
		map.insert(
			"drugReactionAssessments".to_string(),
			json!(drug_reaction_assessments),
		);
		map.insert("drugRecurrences".to_string(), json!(drug_recurrences));
	}
	Ok(drug)
}

pub async fn get_editor_dg(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_READ)?;
	require_permission(&ctx, DRUG_SUBSTANCE_LIST)?;
	require_permission(&ctx, DRUG_DOSAGE_LIST)?;
	require_permission(&ctx, DRUG_INDICATION_LIST)?;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_LIST)?;
	require_permission(&ctx, DRUG_RECURRENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let drug = load_editor_dg_row_detail(&ctx, &mm, case_id, drug_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: drug_id,
			data: json!({ "drugs": [drug] }),
		}),
	))
}

pub async fn get_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, DRUG_READ)?;
	require_permission(&ctx, DRUG_SUBSTANCE_LIST)?;
	require_permission(&ctx, DRUG_DOSAGE_LIST)?;
	require_permission(&ctx, DRUG_INDICATION_LIST)?;
	require_permission(&ctx, DRUG_REACTION_ASSESSMENT_LIST)?;
	require_permission(&ctx, DRUG_RECURRENCE_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query_authorities_csv(&query)?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn build_editor_dg_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let drug = load_editor_dg_row_detail(&ctx, &mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("DG"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, authorities)?;
	response.insert("data".to_string(), json!({ "drug": drug }));
	Ok(Value::Object(response))
}

pub async fn create_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	let row = required_row_object("DG", &request.rows, "drug")?;
	let value = row_model_value(
		row,
		&[
			("source_product_presave_id", &["sourceProductPresaveId"][..]),
			("medicinal_product", &["medicinalProduct"][..]),
			("drug_characterization", &["drugRole"][..]),
			("action_taken", &["actionTaken"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
			(
				"drug_characterization",
				json!(string_field(row, &["drugRole", "drug_characterization"])
					.unwrap_or_else(|| "1".to_string())),
			),
		],
	);
	let create = parse_row_model::<DrugInformationForCreate>("DG", "drug", value)?;
	let row_id = DrugInformationBmc::create(&ctx, &mm, create).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	DrugInformationBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	let synthesized_rows;
	let rows = if !request.changes.is_empty() {
		synthesized_rows = row_payload_from_changes(
			"DG",
			"drug",
			&request.changes,
			&[
				("medicinalProduct", "medicinalProduct"),
				("drugCharacterization", "drugRole"),
				("drugRole", "drugRole"),
				("actionTaken", "actionTaken"),
			],
		)?;
		&synthesized_rows
	} else {
		&request.rows
	};
	let row = required_row_object("DG", rows, "drug")?;
	let value = row_model_value(
		row,
		&[
			("source_product_presave_id", &["sourceProductPresaveId"][..]),
			("medicinal_product", &["medicinalProduct"][..]),
			("drug_characterization", &["drugRole"][..]),
			("action_taken", &["actionTaken"][..]),
		],
		&[],
	);
	let update = parse_row_model::<DrugInformationForUpdate>("DG", "drug", value)?;
	DrugInformationBmc::update(&ctx, &mm, row_id, update).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_dg_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	DrugInformationBmc::get_in_case(&ctx, &mm, case_id, row_id).await?;
	DrugInformationBmc::delete(&ctx, &mm, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn restore_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	DrugInformationBmc::get_in_case_with_deleted(&ctx, &mm, case_id, row_id, true)
		.await?;
	DrugInformationBmc::restore_in_case(&ctx, &mm, case_id, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	let response =
		build_editor_dg_page_row_response(&ctx, &mm, case_id, row_id, None).await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}
