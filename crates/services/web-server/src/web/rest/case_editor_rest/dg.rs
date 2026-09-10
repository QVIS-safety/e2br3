use super::common::{
	bool_field, child_row_has_content, ci_date, editor_page_row_response,
	explicit_null_model_fields, i32_field, insert_alias, json,
	mark_editor_validation_summary_stale, next_child_sequence, parse_row_model,
	query_authorities_csv, repeatable_page_projection_response, required_row_object,
	row_model_value, string_field, uuid_eq, validate_request_projection_context,
	validate_row_payload, CaseEditorDgListRowDto, CaseEditorPagePatchRequest,
	CaseEditorPageProjectionQuery, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CtxW, DosageInformationBmc,
	DosageInformationFilter, DrugActiveSubstanceBmc, DrugActiveSubstanceFilter,
	DrugIndicationBmc, DrugIndicationFilter, DrugInformationBmc,
	DrugInformationForCreate, DrugInformationForUpdate, DrugReactionAssessmentBmc,
	Error, Json, ListOptions, Map, ModelManager, OpValValue, OpValsValue, Path,
	Query, ReactionBmc, Result, State, Uuid, Value,
};
use super::handler_macros::{
	repeatable_list_handler, repeatable_page_row_delete_restore_handlers,
	repeatable_page_row_read_handler,
};
use lib_core::model::drug::{
	DosageInformationForCreate, DosageInformationForUpdate,
	DrugActiveSubstanceForCreate, DrugActiveSubstanceForUpdate,
	DrugDeviceCharacteristicBmc, DrugDeviceCharacteristicFilter,
	DrugIndicationForCreate, DrugIndicationForUpdate, FdaDeviceCodeBmc,
	FdaDeviceCodeFilter, FdaDeviceInformationBmc, FdaDeviceInformationFilter,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentForCreate, DrugReactionAssessmentForUpdate,
	RelatednessAssessmentBmc, RelatednessAssessmentFilter,
	RelatednessAssessmentForCreate, RelatednessAssessmentForUpdate,
};
use std::collections::BTreeMap;

#[path = "dg_devices.rs"]
mod devices;

const DRUG_ROW_ALIASES: &[(&str, &[&str])] = &[
	("source_product_presave_id", &["sourceProductPresaveId"]),
	("medicinal_product", &["medicinalProduct"]),
	("drug_characterization", &["drugCharacterization"]),
	("batch_lot_number", &["drugBatchNumber"]),
	("action_taken", &["drugActionTaken"]),
	("mpid_version", &["mpidVersion"]),
	("mpid", &["mpid"]),
	("phpid_version", &["phpidVersion"]),
	("phpid", &["phpid"]),
	("mfds_mpid_version", &["mfdsMpidVersion"]),
	("mfds_mpid", &["mfdsMpid"]),
	("obtain_drug_country", &["obtainDrugCountry"]),
	(
		"investigational_product_blinded",
		&["investigationalProductBlinded"],
	),
	("drug_authorization_number", &["drugAuthorizationNumber"]),
	("manufacturer_country", &["drugAuthorizationCountry"]),
	("manufacturer_name", &["drugAuthorizationHolder"]),
	(
		"cumulative_dose_first_reaction_value",
		&["cumulativeDoseValue"],
	),
	(
		"cumulative_dose_first_reaction_unit",
		&["cumulativeDoseUnit"],
	),
	(
		"gestation_period_exposure_value",
		&["gestationPeriodExposureValue"],
	),
	(
		"gestation_period_exposure_unit",
		&["gestationPeriodExposureUnit"],
	),
	("fda_additional_info_coded", &["fdaAdditionalInfoCoded"]),
	(
		"fda_additional_info_coded_null_flavor",
		&["fdaAdditionalInfoCodedNullFlavor"],
	),
	(
		"drug_additional_info_codes_json",
		&["drugAdditionalInformationCodes"],
	),
	(
		"drug_additional_information",
		&["drugAdditionalInformation"],
	),
	(
		"fda_specialized_product_category",
		&["fdaSpecializedProductCategory"],
	),
	("fda_other_characterization", &["fdaOtherCharacterization"]),
	("sequence_number", &["sequenceNumber"]),
];

const ACTIVE_SUBSTANCE_ALIASES: &[(&str, &[&str])] = &[
	("sequence_number", &["sequenceNumber"]),
	("substance_name", &["substanceName"]),
	("substance_termid_version", &["substanceTermIdVersion"]),
	("substance_termid", &["substanceTermId"]),
	(
		"substance_termid_code_system",
		&["substanceTermIdCodeSystem"],
	),
	("mfds_version", &["mfdsVersion"]),
	("mfds_id", &["mfdsId"]),
	("strength_value", &["substanceStrengthValue"]),
	("strength_unit", &["substanceStrengthUnit"]),
];

const DOSAGE_ALIASES: &[(&str, &[&str])] = &[
	("sequence_number", &["sequenceNumber"]),
	("dose_value", &["doseValue"]),
	("dose_unit", &["doseUnit"]),
	("number_of_units", &["numberOfUnits"]),
	("frequency_unit", &["frequencyUnit"]),
	("first_administration_date", &["firstAdministrationDate"]),
	(
		"first_administration_date_raw",
		&["firstAdministrationDate", "first_administration_date"],
	),
	("last_administration_date", &["lastAdministrationDate"]),
	(
		"last_administration_date_raw",
		&["lastAdministrationDate", "last_administration_date"],
	),
	("duration_value", &["durationValue"]),
	("duration_unit", &["durationUnit"]),
	("continuing", &["continuing"]),
	("batch_lot_number", &["batchNumber"]),
	("dosage_text", &["dosageText"]),
	("dose_form", &["doseForm"]),
	("dose_form_null_flavor", &["doseFormNullFlavor"]),
	("dose_form_termid_version", &["doseFormTermIdVersion"]),
	("dose_form_termid", &["doseFormTermId"]),
	("route_of_administration", &["routeOfAdministration"]),
	(
		"route_of_administration_null_flavor",
		&["routeOfAdministrationNullFlavor"],
	),
	("route_termid_version", &["routeTermIdVersion"]),
	("route_termid", &["routeTermId"]),
	("route_termid_code_system", &["routeTermIdCodeSystem"]),
	("parent_route", &["parentRouteOfAdministration"]),
	(
		"parent_route_null_flavor",
		&["parentRouteOfAdministrationNullFlavor"],
	),
	("parent_route_termid_version", &["parentRouteTermIdVersion"]),
	("parent_route_termid", &["parentRouteTermId"]),
	(
		"parent_route_termid_code_system",
		&["parentRouteTermIdCodeSystem"],
	),
	(
		"first_administration_date_null_flavor",
		&["firstAdministrationDateNullFlavor"],
	),
	(
		"last_administration_date_null_flavor",
		&["lastAdministrationDateNullFlavor"],
	),
];

const INDICATION_ALIASES: &[(&str, &[&str])] = &[
	("sequence_number", &["sequenceNumber"]),
	("indication_text", &["indicationText"]),
	("indication_text_null_flavor", &["indicationTextNullFlavor"]),
	("indication_meddra_version", &["indicationMeddraVersion"]),
	("indication_meddra_code", &["indicationMeddraCode"]),
];

const ASSESSMENT_ALIASES: &[(&str, &[&str])] = &[
	(
		"administration_start_interval_value",
		&["administrationStartIntervalValue"],
	),
	(
		"administration_start_interval_unit",
		&["administrationStartIntervalUnit"],
	),
	("last_dose_interval_value", &["lastDoseIntervalValue"]),
	("last_dose_interval_unit", &["lastDoseIntervalUnit"]),
	("recurrence_action", &["recurrenceAction"]),
	("reaction_recurred", &["reactionRecurred"]),
	("dechallenge_result", &["dechallengeResult"]),
	("expectedness", &["expectedness"]),
];

const RELATEDNESS_ALIASES: &[(&str, &[&str])] = &[
	("source_of_assessment", &["sourceOfAssessment"]),
	("method_of_assessment", &["methodOfAssessment"]),
	("method_of_assessment_kr1", &["methodOfAssessmentKr1"]),
	("result_of_assessment", &["resultOfAssessment"]),
	("result_of_assessment_kr1", &["resultOfAssessmentKr1"]),
	(
		"result_of_assessment_kr1_null_flavor",
		&["resultOfAssessmentKr1NullFlavor"],
	),
	("result_of_assessment_kr2", &["resultOfAssessmentKr2"]),
];

async fn persist_active_substances(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	drug_id: Uuid,
	drug_row: &Map<String, Value>,
) -> Result<()> {
	let Some(value) = drug_row.get("activeSubstances") else {
		return Ok(());
	};
	let rows = value.as_array().ok_or_else(|| Error::BadRequest {
		message: "invalid DG.drug.activeSubstances payload: expected an array"
			.to_string(),
	})?;

	for (index, value) in rows.iter().enumerate() {
		let row = value.as_object().ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid DG.drug.activeSubstances[{index}] payload: expected an object"
			),
		})?;
		let id = string_field(row, &["id"])
			.map(|value| {
				Uuid::parse_str(&value).map_err(|_| Error::BadRequest {
					message: format!("invalid DG.drug.activeSubstances[{index}].id"),
				})
			})
			.transpose()?;
		let deleted = bool_field(row, &["deleted"]).unwrap_or(false);

		if let Some(id) = id {
			let persisted = DrugActiveSubstanceBmc::get(ctx, mm, id).await?;
			if persisted.drug_id != drug_id {
				return Err(Error::BadRequest {
					message: format!(
						"DG.drug.activeSubstances[{index}].id does not belong to the current drug"
					),
				});
			}
			if deleted {
				DrugActiveSubstanceBmc::delete(ctx, mm, id).await?;
			} else {
				let clear_fields =
					explicit_null_model_fields(row, ACTIVE_SUBSTANCE_ALIASES);
				let model = row_model_value(
					"DG",
					"activeSubstances[].",
					row,
					ACTIVE_SUBSTANCE_ALIASES,
					&[],
				);
				let update = parse_row_model::<DrugActiveSubstanceForUpdate>(
					"DG",
					"activeSubstances",
					model,
				)?;
				lib_core::model::update_uuid_patch::<
					DrugActiveSubstanceBmc,
					DrugActiveSubstanceForUpdate,
				>(ctx, mm, id, update, &clear_fields)
				.await?;
			}
		} else if !deleted {
			let model = row_model_value(
				"DG",
				"activeSubstances[].",
				row,
				ACTIVE_SUBSTANCE_ALIASES,
				&[
					("drug_id", json!(drug_id)),
					(
						"sequence_number",
						json!(i32_field(row, &["sequenceNumber"]).unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"drug_active_substances",
								"drug_id",
								drug_id,
								true,
							)
							.await?,
						)),
					),
				],
			);
			let create = parse_row_model::<DrugActiveSubstanceForCreate>(
				"DG",
				"activeSubstances",
				model,
			)?;
			DrugActiveSubstanceBmc::create(ctx, mm, create).await?;
		}
	}

	Ok(())
}

macro_rules! persist_drug_children {
	(
		$fn_name:ident,
		key: $key:literal,
		aliases: $aliases:expr,
		table: $table:literal,
		bmc: $bmc:ident,
		create: $create:ty,
		update: $update:ty
	) => {
		async fn $fn_name(
			ctx: &lib_core::ctx::Ctx,
			mm: &ModelManager,
			drug_id: Uuid,
			drug_row: &Map<String, Value>,
		) -> Result<()> {
			let Some(value) = drug_row.get($key) else {
				return Ok(());
			};
			let rows = value.as_array().ok_or_else(|| Error::BadRequest {
				message: format!("invalid DG.drug.{} payload: expected an array", $key),
			})?;

			for (index, value) in rows.iter().enumerate() {
				let row = value.as_object().ok_or_else(|| Error::BadRequest {
					message: format!(
						"invalid DG.drug.{}[{index}] payload: expected an object",
						$key
					),
				})?;
				let id = string_field(row, &["id"])
					.map(|value| {
						Uuid::parse_str(&value).map_err(|_| Error::BadRequest {
							message: format!("invalid DG.drug.{}[{index}].id", $key),
						})
					})
					.transpose()?;
				let deleted = bool_field(row, &["deleted"]).unwrap_or(false);

				if let Some(id) = id {
					let persisted = $bmc::get(ctx, mm, id).await?;
					if persisted.drug_id != drug_id {
						return Err(Error::BadRequest {
							message: format!(
								"DG.drug.{}[{index}].id does not belong to the current drug",
								$key
							),
						});
					}
					if deleted {
						$bmc::delete(ctx, mm, id).await?;
					} else {
						let clear_fields =
							explicit_null_model_fields(row, $aliases);
						let model = row_model_value(
							"DG",
							concat!($key, "[]."),
							row,
							$aliases,
							&[],
						);
						let update =
							parse_row_model::<$update>("DG", $key, model)?;
						lib_core::model::update_uuid_patch::<$bmc, $update>(
							ctx,
							mm,
							id,
							update,
							&clear_fields,
						)
						.await?;
					}
				} else if !deleted {
					let model = row_model_value(
						"DG",
						concat!($key, "[]."),
						row,
						$aliases,
						&[
							("drug_id", json!(drug_id)),
							(
								"sequence_number",
								json!(i32_field(
									row,
									&["sequenceNumber"]
								)
								.unwrap_or(
									next_child_sequence(
										ctx,
										mm,
										$table,
										"drug_id",
										drug_id,
										true,
									)
									.await?,
								)),
							),
						],
					);
					let create =
						parse_row_model::<$create>("DG", $key, model)?;
					$bmc::create(ctx, mm, create).await?;
				}
			}

			Ok(())
		}
	};
}

persist_drug_children!(
	persist_dosage_information,
	key: "dosageInformation",
	aliases: DOSAGE_ALIASES,
	table: "dosage_information",
	bmc: DosageInformationBmc,
	create: DosageInformationForCreate,
	update: DosageInformationForUpdate
);

persist_drug_children!(
	persist_indications,
	key: "indications",
	aliases: INDICATION_ALIASES,
	table: "drug_indications",
	bmc: DrugIndicationBmc,
	create: DrugIndicationForCreate,
	update: DrugIndicationForUpdate
);

async fn persist_drug_reaction_assessments(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	drug_id: Uuid,
	drug_row: &Map<String, Value>,
) -> Result<()> {
	let Some(value) = drug_row.get("drugReactionAssessments") else {
		return Ok(());
	};
	let rows = value.as_array().ok_or_else(|| Error::BadRequest {
		message:
			"invalid DG.drug.drugReactionAssessments payload: expected an array"
				.to_string(),
	})?;

	for (index, value) in rows.iter().enumerate() {
		let row = value.as_object().ok_or_else(|| Error::BadRequest {
			message: format!(
				"invalid DG.drug.drugReactionAssessments[{index}] payload: expected an object"
			),
		})?;
		let delete_assessment =
			bool_field(row, &["_deleteAssessment"]).unwrap_or(false);
		if !delete_assessment && !child_row_has_content(row) {
			continue;
		}
		let assessment_id = string_field(row, &["drugReactionAssessmentId"])
			.map(|value| {
				Uuid::parse_str(&value).map_err(|_| Error::BadRequest {
					message: format!(
					"invalid DG.drug.drugReactionAssessments[{index}].drugReactionAssessmentId"
				),
				})
			})
			.transpose()?;

		let persisted_assessment = if let Some(assessment_id) = assessment_id {
			let assessment =
				DrugReactionAssessmentBmc::get(ctx, mm, assessment_id).await?;
			if assessment.drug_id != drug_id {
				return Err(Error::BadRequest {
					message: format!(
						"DG.drug.drugReactionAssessments[{index}] does not belong to the current drug"
					),
				});
			}
			Some(assessment)
		} else {
			None
		};
		let reaction_id = string_field(row, &["reactionId"])
			.map(|value| {
				Uuid::parse_str(&value).map_err(|_| Error::BadRequest {
					message: format!(
						"invalid DG.drug.drugReactionAssessments[{index}].reactionId"
					),
				})
			})
			.transpose()?
			.or_else(|| {
				persisted_assessment
					.as_ref()
					.map(|assessment| assessment.reaction_id)
			})
			.ok_or_else(|| Error::BadRequest {
				message: format!(
					"missing DG.drug.drugReactionAssessments[{index}].reactionId"
				),
			})?;
		ReactionBmc::get_in_case(ctx, mm, case_id, reaction_id).await?;

		if delete_assessment {
			let assessment = persisted_assessment.ok_or_else(|| Error::BadRequest {
				message: format!(
					"missing DG.drug.drugReactionAssessments[{index}].drugReactionAssessmentId for deletion"
				),
			})?;
			DrugReactionAssessmentBmc::delete(ctx, mm, assessment.id).await?;
			continue;
		}

		let assessment_id = if let Some(assessment) = persisted_assessment {
			let clear_fields = explicit_null_model_fields(row, ASSESSMENT_ALIASES);
			let model = row_model_value(
				"DG",
				"drugReactionAssessments[].",
				row,
				ASSESSMENT_ALIASES,
				&[],
			);
			let update = parse_row_model::<DrugReactionAssessmentForUpdate>(
				"DG",
				"drugReactionAssessments",
				model,
			)?;
			DrugReactionAssessmentBmc::update_patch(
				ctx,
				mm,
				assessment.id,
				update,
				&clear_fields,
			)
			.await?;
			assessment.id
		} else if let Some(assessment) =
			DrugReactionAssessmentBmc::get_by_drug_and_reaction(
				ctx,
				mm,
				drug_id,
				reaction_id,
			)
			.await?
		{
			let clear_fields = explicit_null_model_fields(row, ASSESSMENT_ALIASES);
			let model = row_model_value(
				"DG",
				"drugReactionAssessments[].",
				row,
				ASSESSMENT_ALIASES,
				&[],
			);
			let update = parse_row_model::<DrugReactionAssessmentForUpdate>(
				"DG",
				"drugReactionAssessments",
				model,
			)?;
			DrugReactionAssessmentBmc::update_patch(
				ctx,
				mm,
				assessment.id,
				update,
				&clear_fields,
			)
			.await?;
			assessment.id
		} else {
			let model = row_model_value(
				"DG",
				"drugReactionAssessments[].",
				row,
				ASSESSMENT_ALIASES,
				&[
					("drug_id", json!(drug_id)),
					("reaction_id", json!(reaction_id)),
				],
			);
			let create = parse_row_model::<DrugReactionAssessmentForCreate>(
				"DG",
				"drugReactionAssessments",
				model,
			)?;
			DrugReactionAssessmentBmc::create(ctx, mm, create).await?
		};

		let relatedness_id = string_field(row, &["id"])
			.map(|value| {
				Uuid::parse_str(&value).map_err(|_| Error::BadRequest {
					message: format!(
						"invalid DG.drug.drugReactionAssessments[{index}].id"
					),
				})
			})
			.transpose()?;
		let deleted = bool_field(row, &["deleted"]).unwrap_or(false);
		if let Some(relatedness_id) = relatedness_id {
			let relatedness =
				RelatednessAssessmentBmc::get(ctx, mm, relatedness_id).await?;
			if relatedness.drug_reaction_assessment_id != assessment_id {
				return Err(Error::BadRequest {
					message: format!(
						"DG.drug.drugReactionAssessments[{index}].id does not belong to the current assessment"
					),
				});
			}
			if deleted {
				RelatednessAssessmentBmc::delete(ctx, mm, relatedness_id).await?;
			} else {
				let clear_fields =
					explicit_null_model_fields(row, RELATEDNESS_ALIASES);
				let model = row_model_value(
					"DG",
					"drugReactionAssessments[].",
					row,
					RELATEDNESS_ALIASES,
					&[],
				);
				let update = parse_row_model::<RelatednessAssessmentForUpdate>(
					"DG",
					"drugReactionAssessments",
					model,
				)?;
				lib_core::model::update_uuid_patch::<
					RelatednessAssessmentBmc,
					RelatednessAssessmentForUpdate,
				>(ctx, mm, relatedness_id, update, &clear_fields)
				.await?;
			}
		} else if !deleted
			&& RELATEDNESS_ALIASES.iter().any(|(_, aliases)| {
				aliases.iter().any(|alias| {
					row.get(*alias).is_some_and(|value| !value.is_null())
				})
			}) {
			let model = row_model_value(
				"DG",
				"drugReactionAssessments[].",
				row,
				RELATEDNESS_ALIASES,
				&[
					("drug_reaction_assessment_id", json!(assessment_id)),
					(
						"sequence_number",
						json!(i32_field(row, &["sequenceNumber"]).unwrap_or(
							next_child_sequence(
								ctx,
								mm,
								"relatedness_assessments",
								"drug_reaction_assessment_id",
								assessment_id,
								true,
							)
							.await?,
						)),
					),
				],
			);
			let create = parse_row_model::<RelatednessAssessmentForCreate>(
				"DG",
				"drugReactionAssessments",
				model,
			)?;
			RelatednessAssessmentBmc::create(ctx, mm, create).await?;
		}
	}

	Ok(())
}

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
	load_editor_dg_list_rows,
	include_deleted,
);

pub async fn get_editor_dg_page_projection(
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
		"editor/DG",
		move |ctx, mm| {
			Box::pin(async move {
				let rows = load_editor_dg_list_rows(
					ctx,
					mm,
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
			})
		},
	)
	.await
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

impl FromDrugIdFilter for DrugDeviceCharacteristicFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(uuid_eq(drug_id)),
			..Default::default()
		}
	}
}

impl FromDrugIdFilter for FdaDeviceInformationFilter {
	fn from_drug_id(drug_id: Uuid) -> Self {
		Self {
			drug_id: Some(uuid_eq(drug_id)),
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
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let dosage_information = DosageInformationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DosageInformationFilter>(drug_id),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let dosage_information = dosage_information
		.into_iter()
		.map(|dosage| {
			let first_administration_date = dosage
				.first_administration_date_raw
				.clone()
				.filter(|value| !value.trim().is_empty())
				.or_else(|| ci_date(dosage.first_administration_date));
			let last_administration_date = dosage
				.last_administration_date_raw
				.clone()
				.filter(|value| !value.trim().is_empty())
				.or_else(|| ci_date(dosage.last_administration_date));
			let mut value = json!(dosage);
			if let Value::Object(ref mut map) = value {
				map.remove("first_administration_date_raw");
				map.remove("last_administration_date_raw");
				map.insert(
					"first_administration_date".to_string(),
					json!(first_administration_date),
				);
				map.insert(
					"last_administration_date".to_string(),
					json!(last_administration_date),
				);
			}
			value
		})
		.collect::<Vec<_>>();
	let indications = DrugIndicationBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugIndicationFilter>(drug_id),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let device_characteristics = DrugDeviceCharacteristicBmc::list(
		ctx,
		mm,
		drug_id_filter::<DrugDeviceCharacteristicFilter>(drug_id),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let fda_devices = FdaDeviceInformationBmc::list(
		ctx,
		mm,
		drug_id_filter::<FdaDeviceInformationFilter>(drug_id),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let device_ids = fda_devices
		.iter()
		.map(|device| device.id)
		.collect::<Vec<_>>();
	let device_codes = if device_ids.is_empty() {
		Vec::new()
	} else {
		FdaDeviceCodeBmc::list(
			ctx,
			mm,
			Some(
				device_ids
					.iter()
					.copied()
					.map(|device_id| FdaDeviceCodeFilter {
						device_id: Some(uuid_eq(device_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let mut codes_by_device = BTreeMap::new();
	for code in device_codes {
		codes_by_device
			.entry(code.device_id)
			.or_insert_with(Vec::new)
			.push(code);
	}
	let mut fda_devices_with_codes = Vec::with_capacity(fda_devices.len());
	for device in fda_devices {
		let codes = codes_by_device.remove(&device.id).unwrap_or_default();
		let mut value = json!(device);
		if let Value::Object(ref mut map) = value {
			for (element, key) in [
				("follow_up_type", "followUpTypes"),
				("device_problem", "deviceProblemCodes"),
				("remedial_action", "remedialActions"),
			] {
				map.insert(
					key.to_string(),
					json!(codes
						.iter()
						.filter(|code| code.element == element)
						.collect::<Vec<_>>()),
				);
			}
		}
		fda_devices_with_codes.push(value);
	}
	let assessments =
		DrugReactionAssessmentBmc::list_by_drug(ctx, mm, drug_id).await?;
	let assessment_ids = assessments
		.iter()
		.map(|assessment| assessment.id)
		.collect::<Vec<_>>();
	let relatedness_rows = if assessment_ids.is_empty() {
		Vec::new()
	} else {
		RelatednessAssessmentBmc::list(
			ctx,
			mm,
			Some(
				assessment_ids
					.iter()
					.copied()
					.map(|assessment_id| RelatednessAssessmentFilter {
						drug_reaction_assessment_id: Some(uuid_eq(assessment_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let mut relatedness_by_assessment = BTreeMap::new();
	for relatedness in relatedness_rows {
		relatedness_by_assessment
			.entry(relatedness.drug_reaction_assessment_id)
			.or_insert_with(Vec::new)
			.push(relatedness);
	}
	let mut drug_reaction_assessments = Vec::new();
	for assessment in assessments {
		let relatedness = relatedness_by_assessment
			.remove(&assessment.id)
			.unwrap_or_default();
		let base = json!({
			"drugReactionAssessmentId": assessment.id,
			"reactionId": assessment.reaction_id,
			"administrationStartIntervalValue":
				assessment.administration_start_interval_value,
			"administrationStartIntervalUnit":
				assessment.administration_start_interval_unit,
			"lastDoseIntervalValue": assessment.last_dose_interval_value,
			"lastDoseIntervalUnit": assessment.last_dose_interval_unit,
			"recurrenceAction": assessment.recurrence_action,
			"reactionRecurred": assessment.reaction_recurred,
			"dechallengeResult": assessment.dechallenge_result,
			"expectedness": assessment.expectedness,
		});
		if relatedness.is_empty() {
			drug_reaction_assessments.push(base);
		} else {
			for relatedness in relatedness {
				let mut row = base.clone();
				if let Value::Object(ref mut map) = row {
					map.insert("id".to_string(), json!(relatedness.id));
					map.insert(
						"sequenceNumber".to_string(),
						json!(relatedness.sequence_number),
					);
					map.insert(
						"sourceOfAssessment".to_string(),
						json!(relatedness.source_of_assessment),
					);
					map.insert(
						"methodOfAssessment".to_string(),
						json!(relatedness.method_of_assessment),
					);
					map.insert(
						"methodOfAssessmentKr1".to_string(),
						json!(relatedness.method_of_assessment_kr1),
					);
					map.insert(
						"resultOfAssessment".to_string(),
						json!(relatedness.result_of_assessment),
					);
					map.insert(
						"resultOfAssessmentKr1".to_string(),
						json!(relatedness.result_of_assessment_kr1),
					);
					map.insert(
						"resultOfAssessmentKr1NullFlavor".to_string(),
						json!(relatedness.result_of_assessment_kr1_null_flavor),
					);
					map.insert(
						"resultOfAssessmentKr2".to_string(),
						json!(relatedness.result_of_assessment_kr2),
					);
				}
				drug_reaction_assessments.push(row);
			}
		}
	}
	let mut drug = json!(drug);
	if let Value::Object(ref mut map) = drug {
		insert_alias(map, "drugAuthorizationCountry", &["manufacturer_country"]);
		map.insert("activeSubstances".to_string(), json!(active_substances));
		map.insert("dosageInformation".to_string(), json!(dosage_information));
		map.insert("indications".to_string(), json!(indications));
		map.insert(
			"deviceCharacteristics".to_string(),
			json!(device_characteristics),
		);
		map.insert("fdaDevices".to_string(), json!(fda_devices_with_codes));
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
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, drug_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/DG/{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let drug =
					load_editor_dg_row_detail(ctx, mm, case_id, drug_id).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(CaseEditorRowDetailResponse {
						case_id,
						row_id: drug_id,
						data: json!({ "drugs": [drug] }),
					}),
				))
			})
		},
	)
	.await
}

repeatable_page_row_read_handler!(
	get_editor_dg_page_row,
	build_editor_dg_page_row_response,
);

async fn build_editor_dg_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let drug = load_editor_dg_row_detail(ctx, mm, case_id, row_id).await?;
	editor_page_row_response(
		case_id,
		"DG",
		row_id,
		authorities,
		json!({ "drug": drug }),
	)
}

pub(crate) async fn apply_editor_dg_page_row_create(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	request: &CaseEditorPagePatchRequest,
) -> Result<(Uuid, Option<String>)> {
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;
	let row = required_row_object("DG", &request.rows, "drug")?;
	validate_row_payload("DG", "drug", row, None)?;

	let model = row_model_value(
		"DG",
		"",
		row,
		DRUG_ROW_ALIASES,
		&[
			("case_id", json!(case_id)),
			(
				"sequence_number",
				json!(i32_field(row, &["sequenceNumber"]).unwrap_or(1)),
			),
			(
				"drug_characterization",
				json!(string_field(
					row,
					&["drugCharacterization", "drugRole", "drug_characterization",],
				)
				.unwrap_or_default()),
			),
		],
	);
	let create = parse_row_model::<DrugInformationForCreate>("DG", "drug", model)?;
	let row_id = DrugInformationBmc::create(ctx, mm, create).await?;
	persist_active_substances(ctx, mm, row_id, row).await?;
	persist_dosage_information(ctx, mm, row_id, row).await?;
	persist_indications(ctx, mm, row_id, row).await?;
	persist_drug_reaction_assessments(ctx, mm, case_id, row_id, row).await?;
	devices::persist_devices(ctx, mm, row_id, row).await?;
	Ok((row_id, requested_authorities))
}

pub async fn create_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/DG/drug",
		move |ctx, mm| {
			Box::pin(async move {
				let (row_id, requested_authorities) =
					apply_editor_dg_page_row_create(ctx, mm, case_id, &request)
						.await?;
				mark_editor_validation_summary_stale(
					ctx,
					mm,
					case_id,
					requested_authorities.clone(),
				)
				.await?;
				let response = build_editor_dg_page_row_response(
					ctx,
					mm,
					case_id,
					row_id,
					requested_authorities,
				)
				.await?;
				Ok((axum::http::StatusCode::CREATED, Json(response)))
			})
		},
	)
	.await
}

pub async fn patch_editor_dg_page_row(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, row_id)): Path<(Uuid, Uuid)>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/DG/drug",
		move |ctx, mm| {
			Box::pin(async move {
				let requested_authorities = validate_request_projection_context(
					request.authorities.as_deref(),
				)?;
				DrugInformationBmc::get_in_case(ctx, mm, case_id, row_id).await?;

				let row = required_row_object("DG", &request.rows, "drug")?;
				validate_row_payload("DG", "drug", row, None)?;

				let model = row_model_value("DG", "", row, DRUG_ROW_ALIASES, &[]);
				let update = parse_row_model::<DrugInformationForUpdate>(
					"DG", "drug", model,
				)?;
				let clear_fields = explicit_null_model_fields(row, DRUG_ROW_ALIASES);
				DrugInformationBmc::update_in_case_patch(
					ctx,
					mm,
					case_id,
					row_id,
					update,
					&clear_fields,
				)
				.await?;
				persist_active_substances(ctx, mm, row_id, row).await?;
				persist_dosage_information(ctx, mm, row_id, row).await?;
				persist_indications(ctx, mm, row_id, row).await?;
				persist_drug_reaction_assessments(ctx, mm, case_id, row_id, row)
					.await?;
				devices::persist_devices(ctx, mm, row_id, row).await?;
				mark_editor_validation_summary_stale(
					ctx,
					mm,
					case_id,
					requested_authorities.clone(),
				)
				.await?;
				let response = build_editor_dg_page_row_response(
					ctx,
					mm,
					case_id,
					row_id,
					requested_authorities,
				)
				.await?;
				Ok((axum::http::StatusCode::OK, Json(response)))
			})
		},
	)
	.await
}

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_dg_page_row,
	restore: restore_editor_dg_page_row,
	bmc: DrugInformationBmc,
	build_response: build_editor_dg_page_row_response,
);
