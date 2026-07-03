use super::common::*;

async fn load_editor_dh_list_rows(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Vec<CaseEditorDhListRowDto>> {
	let patient = match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
		Ok(patient) => patient,
		Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "patient_information",
			..
		}) => return Ok(Vec::new()),
		Err(err) => return Err(err.into()),
	};
	let filter = PastDrugHistoryFilter {
		patient_id: Some(OpValsValue::from(vec![OpValValue::Eq(json!(patient
			.id
			.to_string()))])),
		..Default::default()
	};
	Ok(PastDrugHistoryBmc::list(
		ctx,
		mm,
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
	.collect())
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

	let rows = load_editor_dh_list_rows(&ctx, &mm, case_id).await?;

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorListResponse { case_id, rows }),
	))
}

pub async fn get_editor_dh_page_projection(
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
	require_permission(&ctx, PAST_DRUG_LIST)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let rows = load_editor_dh_list_rows(&ctx, &mm, case_id).await?;
	let projection = repeatable_page_projection_response(
		case_id,
		"DH",
		query_authorities_csv(&query)?,
		json!({ "rows": rows }),
	)?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub async fn get_editor_dh(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, past_drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let history = PastDrugHistoryBmc::get(&ctx, &mm, past_drug_id).await?;
	if history.patient_id != patient.id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "past_drug_history",
			id: past_drug_id,
		}
		.into());
	}

	Ok((
		axum::http::StatusCode::OK,
		Json(CaseEditorRowDetailResponse {
			case_id,
			row_id: past_drug_id,
			data: json!({
				"patientInformation": {
					"pastDrugHistory": [history]
				}
			}),
		}),
	))
}

pub async fn get_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, CASE_READ)?;
	require_permission(&ctx, PAST_DRUG_READ)?;
	lib_rest_core::require_case_read_allowed(&ctx, &mm, case_id).await?;

	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		query_authorities_csv(&query)?,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

async fn load_editor_dh_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<Value> {
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let history = PastDrugHistoryBmc::get(&ctx, &mm, row_id).await?;
	if history.patient_id != patient.id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "past_drug_history",
			id: row_id,
		}
		.into());
	}
	Ok(json!(history))
}

async fn build_editor_dh_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let history = load_editor_dh_row_detail(ctx, mm, case_id, row_id).await?;
	let mut response = Map::new();
	response.insert("caseId".to_string(), json!(case_id));
	response.insert("section".to_string(), json!("DH"));
	response.insert("rowId".to_string(), json!(row_id));
	insert_editor_json_context(&mut response, authorities)?;
	response.insert("data".to_string(), json!({ "pastDrugHistory": history }));
	Ok(Value::Object(response))
}

pub async fn create_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_CREATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	let row = required_row_object("DH", &request.rows, "pastDrugHistory")?;
	let value = row_model_value(
		row,
		&[
			("drug_name", &["drugName"][..]),
			("indication_meddra_code", &["indication"][..]),
			("sequence_number", &["sequenceNumber"][..]),
		],
		&[
			("patient_id", json!(patient.id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber", "sequence_number"])
					.unwrap_or(1)),
			),
		],
	);
	let create =
		parse_row_model::<PastDrugHistoryForCreate>("DH", "pastDrugHistory", value)?;
	let row_id = PastDrugHistoryBmc::create(&ctx, &mm, create).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::CREATED, Json(response)))
}

pub async fn patch_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;

	load_editor_dh_row_detail(&ctx, &mm, case_id, row_id).await?;
	let synthesized_rows;
	let rows = if !request.changes.is_empty() {
		synthesized_rows = row_payload_from_changes(
			"DH",
			"pastDrugHistory",
			&request.changes,
			&[("drugName", "drugName"), ("indication", "indication")],
		)?;
		&synthesized_rows
	} else {
		&request.rows
	};
	let row = required_row_object("DH", rows, "pastDrugHistory")?;
	let value = row_model_value(
		row,
		&[
			("drug_name", &["drugName"][..]),
			("indication_meddra_code", &["indication"][..]),
		],
		&[],
	);
	let update =
		parse_row_model::<PastDrugHistoryForUpdate>("DH", "pastDrugHistory", value)?;
	PastDrugHistoryBmc::update(&ctx, &mm, row_id, update).await?;
	mark_editor_validation_cache_stale(
		&ctx,
		&mm,
		case_id,
		requested_authorities.clone(),
	)
	.await?;
	let response = build_editor_dh_page_row_response(
		&ctx,
		&mm,
		case_id,
		row_id,
		requested_authorities,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(response)))
}

pub async fn delete_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode> {
	let ctx = ctx_w.0;
	require_permission(&ctx, PAST_DRUG_DELETE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;

	load_editor_dh_row_detail(&ctx, &mm, case_id, row_id).await?;
	PastDrugHistoryBmc::delete(&ctx, &mm, row_id).await?;
	mark_editor_validation_cache_stale(&ctx, &mm, case_id, None).await?;
	Ok(axum::http::StatusCode::NO_CONTENT)
}
