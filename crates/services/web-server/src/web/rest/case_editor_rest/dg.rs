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

repeatable_list_handler!(
	list_editor_dg,
	CaseEditorDgListRowDto,
	DRUG_LIST,
	load_editor_dg_list_rows,
	include_deleted,
);

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
	let mut drug = json!(drug);
	if let Value::Object(ref mut map) = drug {
		map.insert("activeSubstances".to_string(), json!(active_substances));
		map.insert("dosageInformation".to_string(), json!(dosage_information));
		map.insert("indications".to_string(), json!(indications));
		map.insert(
			"drugReactionAssessments".to_string(),
			json!(drug_reaction_assessments),
		);
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

repeatable_page_row_read_handler!(
	get_editor_dg_page_row,
	[
		CASE_READ,
		DRUG_READ,
		DRUG_SUBSTANCE_LIST,
		DRUG_DOSAGE_LIST,
		DRUG_INDICATION_LIST,
		DRUG_REACTION_ASSESSMENT_LIST,
		DRUG_RECURRENCE_LIST,
	],
	build_editor_dg_page_row_response,
);

async fn build_editor_dg_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let drug = load_editor_dg_row_detail(&ctx, &mm, case_id, row_id).await?;
	editor_page_row_response(
		case_id,
		"DG",
		row_id,
		authorities,
		json!({ "drug": drug }),
	)
}

repeatable_page_row_create_handler!(
	create_editor_dg_page_row,
	section: "DG",
	row_key: "drug",
	permission: DRUG_CREATE,
	bmc: DrugInformationBmc,
	model: DrugInformationForCreate,
	aliases: &[
		("source_product_presave_id", &["sourceProductPresaveId"][..]),
		("medicinal_product", &["medicinalProduct"][..]),
		("drug_characterization", &["drugRole"][..]),
		("action_taken", &["actionTaken"][..]),
		("sequence_number", &["sequenceNumber"][..]),
	],
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		(
			"sequence_number",
			json!(i32_field(row, &["sequenceNumber", "sequence_number"]).unwrap_or(1)),
		),
		(
			"drug_characterization",
			json!(
				string_field(row, &["drugRole", "drug_characterization"])
					.unwrap_or_else(|| "1".to_string())
			),
		),
	],
	build_response: build_editor_dg_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_dg_page_row,
	section: "DG",
	row_key: "drug",
	permission: DRUG_UPDATE,
	bmc: DrugInformationBmc,
	model: DrugInformationForUpdate,
	changes: &[
		("medicinalProduct", "medicinalProduct"),
		("drugCharacterization", "drugRole"),
		("drugRole", "drugRole"),
		("actionTaken", "actionTaken"),
	],
	aliases: &[
		("source_product_presave_id", &["sourceProductPresaveId"][..]),
		("medicinal_product", &["medicinalProduct"][..]),
		("drug_characterization", &["drugRole"][..]),
		("action_taken", &["actionTaken"][..]),
	],
	build_response: build_editor_dg_page_row_response,
);

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_dg_page_row,
	restore: restore_editor_dg_page_row,
	bmc: DrugInformationBmc,
	delete_permission: DRUG_DELETE,
	update_permission: DRUG_UPDATE,
	build_response: build_editor_dg_page_row_response,
);
