use super::common::{
	ci_ts, editor_page_row_response, i32_field, json, query_authorities_csv,
	repeatable_page_projection_response, CaseEditorAeListRowDto,
	CaseEditorPageProjectionQuery, CaseEditorPageProjectionResponse,
	CaseEditorRowDetailResponse, CtxW, Error, Json, ModelManager, Path, Query,
	ReactionBmc, ReactionForCreate, ReactionForUpdate, Result, State, Uuid, Value,
};
use super::handler_macros::{
	repeatable_list_handler, repeatable_page_row_create_handler,
	repeatable_page_row_delete_restore_handlers, repeatable_page_row_patch_handler,
	repeatable_page_row_read_handler,
};

const REACTION_ROW_ALIASES: &[(&str, &[&str])] = &[
	("primary_source_reaction", &["primarySourceReaction"]),
	(
		"primary_source_reaction_translation",
		&["primarySourceReactionTranslation"],
	),
	("reaction_language", &["reactionLanguage"]),
	("reaction_meddra_version", &["reactionMeddraVersionLLT"]),
	("reaction_meddra_code", &["reactionMeddraCodeLLT"]),
	("term_highlighted", &["termHighlighted"]),
	("serious", &["seriousness.serious"]),
	("criteria_death", &["seriousness.criteriaResultsInDeath"]),
	(
		"criteria_death_null_flavor",
		&["seriousness.criteriaResultsInDeathNullFlavor"],
	),
	(
		"criteria_life_threatening",
		&["seriousness.criteriaLifeThreatening"],
	),
	(
		"criteria_life_threatening_null_flavor",
		&["seriousness.criteriaLifeThreateningNullFlavor"],
	),
	(
		"criteria_hospitalization",
		&["seriousness.criteriaHospitalization"],
	),
	(
		"criteria_hospitalization_null_flavor",
		&["seriousness.criteriaHospitalizationNullFlavor"],
	),
	("criteria_disabling", &["seriousness.criteriaDisabling"]),
	(
		"criteria_disabling_null_flavor",
		&["seriousness.criteriaDisablingNullFlavor"],
	),
	(
		"criteria_congenital_anomaly",
		&["seriousness.criteriaCongenitalAnomaly"],
	),
	(
		"criteria_congenital_anomaly_null_flavor",
		&["seriousness.criteriaCongenitalAnomalyNullFlavor"],
	),
	(
		"criteria_other_medically_important",
		&["seriousness.criteriaOtherMedicallyImportant"],
	),
	(
		"criteria_other_medically_important_null_flavor",
		&["seriousness.criteriaOtherMedicallyImportantNullFlavor"],
	),
	("required_intervention", &["requiredIntervention"]),
	(
		"required_intervention_null_flavor",
		&["requiredInterventionNullFlavor"],
	),
	("expectedness", &["expectedness"]),
	("severity", &["severity"]),
	(
		"mfds_device_ae_classification",
		&["mfdsDeviceAe.aeClassification"],
	),
	("mfds_device_ae_outcome", &["mfdsDeviceAe.aeOutcome"]),
	(
		"mfds_device_cause_medical_device",
		&["mfdsDeviceAe.causeMedicalDevice"],
	),
	(
		"mfds_device_cause_procedure_issue",
		&["mfdsDeviceAe.causeProcedureIssue"],
	),
	(
		"mfds_device_cause_patient_condition",
		&["mfdsDeviceAe.causePatientCondition"],
	),
	(
		"mfds_device_cause_unable_to_assess",
		&["mfdsDeviceAe.causeUnableToAssess"],
	),
	("mfds_device_cause_other", &["mfdsDeviceAe.causeOther"]),
	("mfds_device_action_reason", &["mfdsDeviceAe.actionReason"]),
	("mfds_device_action_recall", &["mfdsDeviceAe.actionRecall"]),
	("mfds_device_action_repair", &["mfdsDeviceAe.actionRepair"]),
	(
		"mfds_device_action_inspection",
		&["mfdsDeviceAe.actionInspection"],
	),
	(
		"mfds_device_action_replacement",
		&["mfdsDeviceAe.actionReplacement"],
	),
	(
		"mfds_device_action_improvement",
		&["mfdsDeviceAe.actionImprovement"],
	),
	(
		"mfds_device_action_monitoring",
		&["mfdsDeviceAe.actionMonitoring"],
	),
	(
		"mfds_device_action_notification",
		&["mfdsDeviceAe.actionNotification"],
	),
	(
		"mfds_device_action_label_change",
		&["mfdsDeviceAe.actionLabelChange"],
	),
	("mfds_device_action_other", &["mfdsDeviceAe.actionOther"]),
	("start_date", &["reactionStartDate"]),
	("start_date_null_flavor", &["reactionStartDateNullFlavor"]),
	("end_date", &["reactionEndDate"]),
	("end_date_null_flavor", &["reactionEndDateNullFlavor"]),
	("duration_value", &["reactionDuration.value"]),
	("duration_unit", &["reactionDuration.unit"]),
	("outcome", &["reactionOutcome"]),
	("medical_confirmation", &["medicalConfirmation"]),
	("country_code", &["reactionCountry"]),
	("sequence_number", &["sequenceNumber"]),
];

fn reaction_sequence_number(row: &serde_json::Map<String, Value>) -> Result<i32> {
	i32_field(row, &["sequenceNumber", "sequence_number"])
		.filter(|sequence_number| *sequence_number > 0)
		.ok_or_else(|| Error::BadRequest {
			message: "AE.reaction.sequenceNumber must be a positive integer"
				.to_string(),
		})
}

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
	load_editor_ae_list_rows,
	include_deleted,
);

pub async fn get_editor_ae_page_projection(
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
		"editor/AE",
		move |ctx, mm| {
			Box::pin(async move {
				let rows = load_editor_ae_list_rows(
					ctx,
					mm,
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
			})
		},
	)
	.await
}

pub async fn get_editor_ae(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, reaction_id)): Path<(Uuid, Uuid)>,
) -> Result<(axum::http::StatusCode, Json<CaseEditorRowDetailResponse>)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/AE/{reaction_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let reaction =
					ReactionBmc::get_in_case(ctx, mm, case_id, reaction_id).await?;
				Ok((
					axum::http::StatusCode::OK,
					Json(CaseEditorRowDetailResponse {
						case_id,
						row_id: reaction_id,
						data: json!({ "reactions": [reaction] }),
					}),
				))
			})
		},
	)
	.await
}

repeatable_page_row_read_handler!(
	get_editor_ae_page_row,
	build_editor_ae_page_row_response,
);

async fn build_editor_ae_page_row_response(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
	authorities: Option<String>,
) -> Result<Value> {
	let persisted =
		ReactionBmc::get_in_case_with_deleted(ctx, mm, case_id, row_id, true)
			.await?;
	let start_date = ci_ts(persisted.start_date.as_deref());
	let end_date = ci_ts(persisted.end_date.as_deref());
	let mut reaction = json!(persisted);
	if let Value::Object(ref mut map) = reaction {
		map.insert("start_date".to_string(), json!(start_date));
		map.insert("end_date".to_string(), json!(end_date));
	}
	editor_page_row_response(
		case_id,
		"AE",
		row_id,
		authorities,
		json!({ "reaction": reaction }),
	)
}

async fn verify_editor_ae_page_row(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	row_id: Uuid,
) -> Result<()> {
	ReactionBmc::get_in_case_with_deleted(ctx, mm, case_id, row_id, true).await?;
	Ok(())
}

repeatable_page_row_create_handler!(
	create_editor_ae_page_row,
	apply: apply_editor_ae_page_row_create,
	section: "AE",
	row_key: "reaction",
	bmc: ReactionBmc,
	model: ReactionForCreate,
	aliases: REACTION_ROW_ALIASES,
	extras: |case_id, row| [
		("case_id", json!(case_id)),
		("sequence_number", json!(reaction_sequence_number(row)?)),
	],
	build_response: build_editor_ae_page_row_response,
);

repeatable_page_row_patch_handler!(
	patch_editor_ae_page_row,
	section: "AE",
	row_key: "reaction",
	bmc: ReactionBmc,
	model: ReactionForUpdate,
	verify: verify_editor_ae_page_row,
	aliases: REACTION_ROW_ALIASES,
	restore_in_case_on_false: true,
	build_response: build_editor_ae_page_row_response,
);

repeatable_page_row_delete_restore_handlers!(
	delete: delete_editor_ae_page_row,
	restore: restore_editor_ae_page_row,
	bmc: ReactionBmc,
	build_response: build_editor_ae_page_row_response,
);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reaction_create_requires_an_explicit_positive_sequence_number() {
		let missing = json!({}).as_object().unwrap().clone();
		let zero = json!({ "sequenceNumber": 0 }).as_object().unwrap().clone();
		let valid = json!({ "sequenceNumber": 2 }).as_object().unwrap().clone();
		let persisted = json!({ "sequence_number": 3 }).as_object().unwrap().clone();

		assert!(reaction_sequence_number(&missing).is_err());
		assert!(reaction_sequence_number(&zero).is_err());
		assert_eq!(reaction_sequence_number(&valid).unwrap(), 2);
		assert_eq!(reaction_sequence_number(&persisted).unwrap(), 3);
	}
}
