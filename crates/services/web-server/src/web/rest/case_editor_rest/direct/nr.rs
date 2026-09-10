use super::super::common::{
	as_object, bool_field, direct_section_response, explicit_null_model_fields,
	i32_field, json, next_child_sequence, optional_row_object,
	reject_unknown_row_keys, string_field, uuid_eq, uuid_field, BTreeMap,
	CaseEditorDirectSectionResponse, CaseSummaryInformationBmc,
	CaseSummaryInformationFilter, CaseSummaryInformationForCreate,
	CaseSummaryInformationForUpdate, CtxW, Error, Json, ListOptions, ModelManager,
	NarrativeInformationBmc, NarrativeInformationForCreate,
	NarrativeInformationForUpdate, Path, Result, SenderDiagnosisBmc,
	SenderDiagnosisFilter, SenderDiagnosisForCreate, SenderDiagnosisForUpdate,
	State, Uuid, Value,
};
use super::super::handler_macros::direct_page_projection_handler;

const NR_NARRATIVE_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("source_narrative_presave_id", &["sourceNarrativePresaveId"]),
	("case_narrative", &["caseNarrative"]),
	("reporter_comments", &["reporterComments"]),
	("sender_comments", &["senderComments"]),
	("additional_information", &["additionalInformation"]),
];
const NR_SENDER_DIAGNOSIS_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("diagnosis_meddra_version", &["diagnosisMeddraVersion"]),
	("diagnosis_meddra_code", &["diagnosisMeddraCode"]),
];
const NR_CASE_SUMMARY_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("language_code", &["languageCode"]),
	("summary_text", &["summaryText"]),
];

pub(super) async fn apply_nr_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&["narrative", "senderDiagnoses", "caseSummaryInformation"],
	)?;
	let rows = &super::super::common::without_empty_row_lists(rows);
	let empty_narrative = serde_json::Map::new();
	let has_nested_rows = rows.contains_key("senderDiagnoses")
		|| rows.contains_key("caseSummaryInformation");
	let narrative_patch = optional_row_object(page_id, rows, "narrative")?
		.or_else(|| has_nested_rows.then_some(&empty_narrative));
	if let Some(narrative) = narrative_patch {
		let case_narrative = narrative
			.get("caseNarrative")
			.and_then(Value::as_str)
			.map(str::to_owned);
		let update = NarrativeInformationForUpdate {
			source_narrative_presave_id: uuid_field(
				narrative,
				&["sourceNarrativePresaveId"],
			),
			case_narrative: case_narrative.clone(),
			reporter_comments: string_field(narrative, &["reporterComments"]),
			sender_comments: string_field(narrative, &["senderComments"]),
			additional_information: string_field(
				narrative,
				&["additionalInformation"],
			),
		};
		match NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?
		{
			Some(_) => {
				let clear_fields =
					explicit_null_model_fields(narrative, NR_NARRATIVE_PATCH_FIELDS);
				NarrativeInformationBmc::update_by_case_patch(
					ctx,
					mm,
					case_id,
					update,
					&clear_fields,
				)
				.await?
			}
			None => {
				NarrativeInformationBmc::create(
					ctx,
					mm,
					NarrativeInformationForCreate {
						case_id,
						source_narrative_presave_id: update
							.source_narrative_presave_id,
						case_narrative: case_narrative.unwrap_or_default(),
						reporter_comments: update.reporter_comments,
						sender_comments: update.sender_comments,
						additional_information: update.additional_information,
					},
				)
				.await?;
			}
		}
	}

	let Some(narrative) =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?
	else {
		return Ok(());
	};

	if let Some(value) = rows.get("senderDiagnoses") {
		let diagnoses = value.as_array().ok_or_else(|| Error::BadRequest {
			message: format!("{page_id}.senderDiagnoses must be an array"),
		})?;
		for (index, value) in diagnoses.iter().enumerate() {
			let diagnosis = as_object(page_id, "senderDiagnoses", value)?;
			let id = uuid_field(diagnosis, &["id"]);
			let deleted = bool_field(diagnosis, &["deleted"]).unwrap_or(false);
			if let Some(id) = id {
				let persisted = SenderDiagnosisBmc::get(ctx, mm, id).await?;
				if persisted.narrative_id != narrative.id {
					return Err(Error::BadRequest {
						message: format!(
							"{page_id}.senderDiagnoses[{index}].id does not belong to the current narrative"
						),
					});
				}
				if deleted {
					SenderDiagnosisBmc::delete(ctx, mm, id).await?;
				} else {
					let clear_fields = explicit_null_model_fields(
						diagnosis,
						NR_SENDER_DIAGNOSIS_PATCH_FIELDS,
					);
					lib_core::model::update_uuid_patch::<SenderDiagnosisBmc, _>(
						ctx,
						mm,
						id,
						SenderDiagnosisForUpdate {
							diagnosis_meddra_version: string_field(
								diagnosis,
								&["diagnosisMeddraVersion"],
							),
							diagnosis_meddra_code: string_field(
								diagnosis,
								&["diagnosisMeddraCode"],
							),
						},
						&clear_fields,
					)
					.await?;
				}
			} else if !deleted {
				SenderDiagnosisBmc::create(
					ctx,
					mm,
					SenderDiagnosisForCreate {
						narrative_id: narrative.id,
						sequence_number: i32_field(diagnosis, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"sender_diagnoses",
									"narrative_id",
									narrative.id,
									true,
								)
								.await?,
							),
						diagnosis_meddra_version: string_field(
							diagnosis,
							&["diagnosisMeddraVersion"],
						),
						diagnosis_meddra_code: string_field(
							diagnosis,
							&["diagnosisMeddraCode"],
						),
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("caseSummaryInformation") {
		let summaries = value.as_array().ok_or_else(|| Error::BadRequest {
			message: format!("{page_id}.caseSummaryInformation must be an array"),
		})?;
		for (index, value) in summaries.iter().enumerate() {
			let summary = as_object(page_id, "caseSummaryInformation", value)?;
			let id = uuid_field(summary, &["id"]);
			let deleted = bool_field(summary, &["deleted"]).unwrap_or(false);
			if let Some(id) = id {
				let persisted = CaseSummaryInformationBmc::get(ctx, mm, id).await?;
				if persisted.narrative_id != narrative.id {
					return Err(Error::BadRequest {
						message: format!(
							"{page_id}.caseSummaryInformation[{index}].id does not belong to the current narrative"
						),
					});
				}
				if deleted {
					CaseSummaryInformationBmc::delete(ctx, mm, id).await?;
				} else {
					let clear_fields = explicit_null_model_fields(
						summary,
						NR_CASE_SUMMARY_PATCH_FIELDS,
					);
					lib_core::model::update_uuid_patch::<CaseSummaryInformationBmc, _>(
						ctx,
						mm,
						id,
						CaseSummaryInformationForUpdate {
							language_code: string_field(summary, &["languageCode"]),
							summary_text: string_field(summary, &["summaryText"]),
						},
						&clear_fields,
					)
					.await?;
				}
			} else if !deleted {
				CaseSummaryInformationBmc::create(
					ctx,
					mm,
					CaseSummaryInformationForCreate {
						narrative_id: narrative.id,
						sequence_number: i32_field(summary, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"case_summary_information",
									"narrative_id",
									narrative.id,
									true,
								)
								.await?,
							),
						language_code: string_field(summary, &["languageCode"]),
						summary_text: string_field(summary, &["summaryText"]),
					},
				)
				.await?;
			}
		}
	}
	Ok(())
}

pub(super) async fn load_editor_nr_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let narrative =
		NarrativeInformationBmc::get_by_case_optional(ctx, mm, case_id).await?;
	let (sender_diagnoses, case_summary_information) =
		if let Some(ref narrative) = narrative {
			let sender_diagnoses = SenderDiagnosisBmc::list(
				ctx,
				mm,
				Some(vec![SenderDiagnosisFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			let case_summary_information = CaseSummaryInformationBmc::list(
				ctx,
				mm,
				Some(vec![CaseSummaryInformationFilter {
					narrative_id: Some(uuid_eq(narrative.id)),
					..Default::default()
				}]),
				Some(ListOptions::default()),
			)
			.await?;
			(sender_diagnoses, case_summary_information)
		} else {
			(Vec::new(), Vec::new())
		};

	Ok(json!({
		"narrative": narrative,
		"senderDiagnoses": sender_diagnoses,
		"caseSummaryInformation": case_summary_information,
	}))
}

pub async fn get_editor_nr(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorDirectSectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		"editor/NR",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_nr_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

direct_page_projection_handler!(
	get_editor_nr_page_projection,
	"NR",
	load_editor_nr_data,
);
