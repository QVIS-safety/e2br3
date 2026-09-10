use super::common::{
	ci_date, editor_page_row_response, i32_field, json, query_authorities_csv,
	repeatable_page_projection_response, CaseEditorDhListRowDto,
	CaseEditorPageProjectionQuery, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CtxW, Json, ListOptions, ModelManager, OpValValue,
	OpValsValue, PastDrugHistoryBmc, PastDrugHistoryFilter,
	PastDrugHistoryForCreate, PastDrugHistoryForUpdate, Path, PatientInformationBmc,
	Query, Result, State, Uuid, Value,
};
use super::handler_macros::{
	repeatable_list_handler, repeatable_page_row_create_handler,
	repeatable_page_row_delete_handler, repeatable_page_row_patch_handler,
};

const PAST_DRUG_ROW_ALIASES: &[(&str, &[&str])] = &[
	("drug_name", &["drugName"]),
	("drug_name_null_flavor", &["drugNameNullFlavor"]),
	(
		"mfds_medicinal_product_version",
		&["mfdsMedicinalProductVersion"],
	),
	("mfds_medicinal_product_id", &["mfdsMedicinalProductId"]),
	("mpid", &["mpid"]),
	("mpid_version", &["mpidVersion"]),
	("phpid", &["phpid"]),
	("phpid_version", &["phpidVersion"]),
	("start_date", &["startDate"]),
	("start_date_null_flavor", &["startDateNullFlavor"]),
	("end_date", &["endDate"]),
	("end_date_null_flavor", &["endDateNullFlavor"]),
	("indication_meddra_version", &["indicationMeddraVersion"]),
	(
		"indication_meddra_code",
		&["indicationMeddraCode", "indication"],
	),
	("reaction_meddra_version", &["reactionMeddraVersion"]),
	("reaction_meddra_code", &["reactionMeddraCode"]),
	("sequence_number", &["sequenceNumber"]),
];

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
		start_date: ci_date(history.start_date),
		end_date: ci_date(history.end_date),
	})
	.collect())
}

repeatable_list_handler!(
	list_editor_dh,
	CaseEditorDhListRowDto,
	load_editor_dh_list_rows,
);

pub async fn get_editor_dh_page_projection(
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
		"editor/DH",
		move |ctx, mm| {
			Box::pin(async move {
				let rows = load_editor_dh_list_rows(ctx, mm, case_id).await?;
				let projection = repeatable_page_projection_response(
					case_id,
					"DH",
					query_authorities_csv(&query)?,
					json!({ "rows": rows }),
				)?;
				Ok((axum::http::StatusCode::OK, Json(projection)))
			})
		},
	)
	.await
}

pub async fn get_editor_dh(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, past_drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/DH/{past_drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let patient =
					PatientInformationBmc::get_by_case(ctx, mm, case_id).await?;
				let history = PastDrugHistoryBmc::get(ctx, mm, past_drug_id).await?;
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
			})
		},
	)
	.await
}

pub async fn get_editor_dh_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Query(query): Query<CaseEditorPageProjectionQuery>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/DH/{row_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let response = build_editor_dh_page_row_response(
					ctx,
					mm,
					case_id,
					row_id,
					query_authorities_csv(&query)?,
				)
				.await?;
				Ok((axum::http::StatusCode::OK, Json(response)))
			})
		},
	)
	.await
}

async fn load_editor_dh_row_detail(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<Value> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id).await?;
	let history = PastDrugHistoryBmc::get(ctx, mm, row_id).await?;
	if history.patient_id != patient.id {
		return Err(lib_core::model::Error::EntityUuidNotFound {
			entity: "past_drug_history",
			id: row_id,
		}
		.into());
	}
	let mut value = json!(history);
	if let Value::Object(ref mut map) = value {
		map.insert("start_date".to_string(), json!(ci_date(history.start_date)));
		map.insert("end_date".to_string(), json!(ci_date(history.end_date)));
	}
	Ok(value)
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
	let patient_id =
		PatientInformationBmc::get_or_create_by_case(ctx, mm, case_id).await?;
	Ok(vec![
		("patient_id", json!(patient_id)),
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
	load_editor_dh_row_detail(ctx, mm, case_id, row_id).await?;
	Ok(())
}

repeatable_page_row_create_handler!(
	create_editor_dh_page_row,
	apply: apply_editor_dh_page_row_create,
	section: "DH",
	row_key: "pastDrugHistory",
	bmc: PastDrugHistoryBmc,
	model: PastDrugHistoryForCreate,
	aliases: PAST_DRUG_ROW_ALIASES,
	extras_fn: editor_dh_create_extras,
	build_response: build_editor_dh_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_dh_page_row,
	section: "DH",
	row_key: "pastDrugHistory",
	bmc: PastDrugHistoryBmc,
	model: PastDrugHistoryForUpdate,
	verify: verify_editor_dh_page_row,
	aliases: PAST_DRUG_ROW_ALIASES,
	build_response: build_editor_dh_page_row_response,
);

repeatable_page_row_delete_handler!(
	delete_editor_dh_page_row,
	bmc: PastDrugHistoryBmc,
	verify: verify_editor_dh_page_row,
);
