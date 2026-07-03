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

repeatable_list_handler!(
	list_editor_dh,
	CaseEditorDhListRowDto,
	PAST_DRUG_LIST,
	load_editor_dh_list_rows,
);

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
	editor_page_row_response(
		case_id,
		"DH",
		row_id,
		authorities,
		json!({ "pastDrugHistory": history }),
	)
}

async fn editor_dh_create_extras(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row: &serde_json::Map<String, Value>,
) -> Result<Vec<(&'static str, Value)>> {
	let patient = PatientInformationBmc::get_by_case(&ctx, &mm, case_id).await?;
	Ok(vec![
		("patient_id", json!(patient.id)),
		(
			"sequence_number",
			json!(
				i32_field(row, &["sequenceNumber", "sequence_number"]).unwrap_or(1)
			),
		),
	])
}

async fn verify_editor_dh_page_row(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<()> {
	load_editor_dh_row_detail(&ctx, &mm, case_id, row_id).await?;
	Ok(())
}

repeatable_page_row_create_handler!(
	create_editor_dh_page_row,
	section: "DH",
	row_key: "pastDrugHistory",
	permission: PAST_DRUG_CREATE,
	bmc: PastDrugHistoryBmc,
	model: PastDrugHistoryForCreate,
	aliases: &[
		("drug_name", &["drugName"][..]),
		("indication_meddra_code", &["indication"][..]),
		("sequence_number", &["sequenceNumber"][..]),
	],
	extras_fn: editor_dh_create_extras,
	build_response: build_editor_dh_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_dh_page_row,
	section: "DH",
	row_key: "pastDrugHistory",
	permission: PAST_DRUG_UPDATE,
	bmc: PastDrugHistoryBmc,
	model: PastDrugHistoryForUpdate,
	verify: verify_editor_dh_page_row,
	changes: &[("drugName", "drugName"), ("indication", "indication")],
	aliases: &[
		("drug_name", &["drugName"][..]),
		("indication_meddra_code", &["indication"][..]),
	],
	build_response: build_editor_dh_page_row_response,
);

repeatable_page_row_delete_handler!(
	delete_editor_dh_page_row,
	permission: PAST_DRUG_DELETE,
	bmc: PastDrugHistoryBmc,
	verify: verify_editor_dh_page_row,
);
