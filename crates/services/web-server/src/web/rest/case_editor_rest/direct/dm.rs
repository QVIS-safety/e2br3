use super::super::common::{
	as_object, bool_field, ci_date, direct_section_response,
	explicit_null_model_fields, i32_field, json, next_child_sequence,
	optional_row_object, reject_unknown_row_keys, string_field, uuid_eq, uuid_field,
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathFilter, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, BTreeMap, CaseEditorDirectSectionResponse, CtxW,
	Error, Json, ListOptions, Map, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeFilter, MedicalHistoryEpisodeForCreate,
	MedicalHistoryEpisodeForUpdate, ModelManager, ParentInformationBmc,
	ParentInformationFilter, ParentInformationForCreate, ParentInformationForUpdate,
	ParentMedicalHistoryBmc, ParentMedicalHistoryFilter,
	ParentMedicalHistoryForCreate, ParentMedicalHistoryForUpdate,
	ParentPastDrugHistoryBmc, ParentPastDrugHistoryFilter,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate, Path,
	PatientDeathInformationBmc, PatientDeathInformationFilter,
	PatientDeathInformationForCreate, PatientDeathInformationForUpdate,
	PatientIdentifierBmc, PatientIdentifierFilter, PatientIdentifierForCreate,
	PatientIdentifierForUpdate, PatientInformationBmc, PatientInformationForCreate,
	PatientInformationForUpdate, ReportedCauseOfDeathBmc,
	ReportedCauseOfDeathFilter, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate, Result, State, Uuid, Value,
};
use super::super::handler_macros::direct_page_projection_handler;
use super::ci::CiDatePatchValue;
use rust_decimal::Decimal;
use std::str::FromStr;

const DM_MEDICAL_HISTORY_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("meddra_version", &["meddraVersion"]),
	("meddra_code", &["meddraCode"]),
	("start_date", &["startDate"]),
	("start_date_null_flavor", &["startDateNullFlavor"]),
	("continuing", &["continuing"]),
	("continuing_null_flavor", &["continuingNullFlavor"]),
	("end_date", &["endDate"]),
	("end_date_null_flavor", &["endDateNullFlavor"]),
	("comments", &["comments"]),
	("family_history", &["familyHistory"]),
];
const DM_CAUSE_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("meddra_version", &["meddraVersion"]),
	("meddra_code", &["meddraCode"]),
	("comments", &["causeText"]),
];
const DM_PARENT_MEDICAL_HISTORY_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("meddra_version", &["meddraVersion"]),
	("meddra_code", &["meddraCode"]),
	("start_date", &["startDate"]),
	("start_date_null_flavor", &["startDateNullFlavor"]),
	("continuing", &["continuing"]),
	("continuing_null_flavor", &["continuingNullFlavor"]),
	("end_date", &["endDate"]),
	("end_date_null_flavor", &["endDateNullFlavor"]),
	("comments", &["comments"]),
];
const DM_PATIENT_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("patient_initials", &["patientInitials"]),
	(
		"patient_initials_null_flavor",
		&["patientInitialsNullFlavor"],
	),
	("birth_date", &["patientBirthDate"]),
	("birth_date_null_flavor", &["patientBirthDateNullFlavor"]),
	("age_at_time_of_onset", &["patientAge.value"]),
	("age_unit", &["patientAge.unit"]),
	("gestation_period", &["gestationPeriod.value"]),
	("gestation_period_unit", &["gestationPeriod.unit"]),
	("age_group", &["patientAgeGroup"]),
	("weight_kg", &["patientWeight.value"]),
	("height_cm", &["patientHeight.value"]),
	("sex", &["patientSex"]),
	("sex_null_flavor", &["patientSexNullFlavor"]),
	("race_codes", &["raceCodes", "raceCode"]),
	("race_code_null_flavor", &["raceCodeNullFlavor"]),
	("ethnicity_code", &["ethnicityCode"]),
	("ethnicity_code_null_flavor", &["ethnicityCodeNullFlavor"]),
	("last_menstrual_period_date", &["lastMenstrualPeriodDate"]),
	(
		"last_menstrual_period_date_null_flavor",
		&["lastMenstrualPeriodDateNullFlavor"],
	),
	("medical_history_text", &["medicalHistoryText"]),
	(
		"medical_history_text_null_flavor",
		&["medicalHistoryTextNullFlavor"],
	),
	("concomitant_therapy", &["concomitantTherapies"]),
];
const DM_DEATH_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("date_of_death", &["dateOfDeath"]),
	("date_of_death_null_flavor", &["dateOfDeathNullFlavor"]),
	("autopsy_performed", &["autopsyPerformed"]),
	(
		"autopsy_performed_null_flavor",
		&["autopsyPerformedNullFlavor"],
	),
];
const DM_PARENT_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("parent_identification", &["parentIdentification"]),
	(
		"parent_identification_null_flavor",
		&["parentIdentificationNullFlavor"],
	),
	("parent_birth_date", &["parentBirthDate"]),
	(
		"parent_birth_date_null_flavor",
		&["parentBirthDateNullFlavor"],
	),
	("parent_age", &["parentAge.value"]),
	("parent_age_unit", &["parentAge.unit"]),
	(
		"last_menstrual_period_date",
		&["parentLastMenstrualPeriodDate"],
	),
	(
		"last_menstrual_period_date_null_flavor",
		&["parentLastMenstrualPeriodDateNullFlavor"],
	),
	("weight_kg", &["parentWeight.value"]),
	("height_cm", &["parentHeight.value"]),
	("sex", &["parentSex"]),
	("sex_null_flavor", &["parentSexNullFlavor"]),
	("medical_history_text", &["medicalHistoryText"]),
];
const DM_PARENT_PAST_DRUG_PATCH_FIELDS: &[(&str, &[&str])] = &[
	("drug_name", &["drugName"]),
	("mpid", &["mpid"]),
	("mpid_version", &["mpidVersion"]),
	(
		"mfds_medicinal_product_version",
		&["mfdsMedicinalProductVersion"],
	),
	("mfds_medicinal_product_id", &["mfdsMedicinalProductId"]),
	("phpid", &["phpid"]),
	("phpid_version", &["phpidVersion"]),
	("start_date", &["startDate"]),
	("start_date_null_flavor", &["startDateNullFlavor"]),
	("end_date", &["endDate"]),
	("end_date_null_flavor", &["endDateNullFlavor"]),
	("indication_meddra_version", &["indicationMeddraVersion"]),
	("indication_meddra_code", &["indicationMeddraCode"]),
	("reaction_meddra_version", &["reactionMeddraVersion"]),
	("reaction_meddra_code", &["reactionMeddraCode"]),
];

pub(super) async fn apply_dm_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	reject_unknown_row_keys(
		page_id,
		rows,
		&[
			"patientInformation",
			"patientIdentifiers",
			"medicalHistoryEpisodes",
			"deathInfo",
			"reportedCauses",
			"autopsyCauses",
			"parentInfo",
			"parentMedicalHistory",
			"parentPastDrugs",
		],
	)?;
	let rows = &super::super::common::without_empty_row_lists(rows);
	if rows.is_empty() {
		return Ok(());
	}
	let patient = optional_row_object(page_id, rows, "patientInformation")?;
	fn value_at_path<'a>(
		row: &'a Map<String, Value>,
		paths: &[&str],
	) -> Option<&'a Value> {
		'paths: for path in paths {
			let mut segments = path.split('.');
			let Some(first) = segments.next() else {
				continue;
			};
			let Some(mut value) = row.get(first) else {
				continue;
			};
			for segment in segments {
				let Some(next) =
					value.as_object().and_then(|object| object.get(segment))
				else {
					continue 'paths;
				};
				value = next;
			}
			return Some(value);
		}
		None
	}
	fn decimal_field(
		page_id: &str,
		row: &Map<String, Value>,
		_request_path: &str,
		paths: &[&str],
	) -> Result<Option<Decimal>> {
		let Some(value) = value_at_path(row, paths) else {
			return Ok(None);
		};
		if value.is_null() {
			return Ok(None);
		}
		Decimal::from_str(&value.to_string())
			.map(Some)
			.map_err(|_| Error::BadRequest {
				message: format!(
					"{page_id}.{} must be a decimal number or null",
					paths[0]
				),
			})
	}
	fn patch_decimal_field(
		page_id: &str,
		row: &Map<String, Value>,
		request_path: &str,
		paths: &[&str],
	) -> Result<Option<Option<Decimal>>> {
		let Some(value) = value_at_path(row, paths) else {
			return Ok(None);
		};
		if value.is_null() {
			return Ok(Some(None));
		}
		Decimal::from_str(&value.to_string())
			.map(|value| Some(Some(value)))
			.map_err(|_| Error::BadRequest {
				message: format!(
					"{page_id}.{request_path} must be a decimal number or null"
				),
			})
	}
	fn date_field(
		page_id: &str,
		row: &Map<String, Value>,
		_request_path: &str,
		paths: &[&str],
	) -> Result<Option<sqlx::types::time::Date>> {
		let Some(value) = value_at_path(row, paths) else {
			return Ok(None);
		};
		if value.as_str().is_some_and(|value| value.trim().is_empty()) {
			return Ok(None);
		}
		serde_json::from_value::<CiDatePatchValue>(json!({"value": value}))
			.map(|parsed| parsed.value)
			.map_err(|err| Error::BadRequest {
				message: format!(
					"{page_id}.{} must be an E2B date or null: {err}",
					paths[0]
				),
			})
	}
	fn ts_field(
		_page_id: &str,
		row: &Map<String, Value>,
		_request_path: &str,
		paths: &[&str],
	) -> Result<Option<String>> {
		Ok(value_at_path(row, paths).and_then(|value| {
			value
				.as_str()
				.map(str::trim)
				.filter(|v| !v.is_empty())
				.map(str::to_owned)
		}))
	}
	fn boolean_field(row: &Map<String, Value>, paths: &[&str]) -> Option<bool> {
		value_at_path(row, paths).and_then(Value::as_bool)
	}
	fn nested_string_field(
		row: &Map<String, Value>,
		paths: &[&str],
	) -> Option<String> {
		value_at_path(row, paths)
			.filter(|value| !value.is_null())
			.and_then(|value| {
				value.as_str().map_or_else(
					|| Some(value.to_string()),
					|value| (!value.trim().is_empty()).then(|| value.to_owned()),
				)
			})
	}
	fn canonical_string_field(
		row: &Map<String, Value>,
		value_paths: &[&str],
		null_flavor_paths: &[&str],
	) -> (Option<String>, Option<String>) {
		(
			nested_string_field(row, value_paths),
			nested_string_field(row, null_flavor_paths),
		)
	}
	fn string_list_field(
		row: &Map<String, Value>,
		paths: &[&str],
	) -> Option<Vec<String>> {
		let value = value_at_path(row, paths)?;
		if let Some(value) = value.as_str() {
			return Some(
				(!value.trim().is_empty())
					.then(|| vec![value.to_string()])
					.unwrap_or_default(),
			);
		}
		value.as_array().map(|values| {
			values
				.iter()
				.filter_map(Value::as_str)
				.filter(|value| !value.trim().is_empty())
				.map(ToOwned::to_owned)
				.collect()
		})
	}
	fn null_flavor_field(
		row: &Map<String, Value>,
		paths: &[&str],
	) -> Option<String> {
		nested_string_field(row, paths)
	}
	let patient_id = if let Some(patient) = patient {
		let (patient_initials, patient_initials_null_flavor) =
			canonical_string_field(
				patient,
				&["patientInitials"],
				&["patientInitialsNullFlavor"],
			);
		let birth_date_paths = &["patientBirthDate"];
		let age_paths = &["patientAge.value"];
		let weight_paths = &["patientWeight.value"];
		let height_paths = &["patientHeight.value"];
		let (sex, sex_null_flavor) = canonical_string_field(
			patient,
			&["patientSex"],
			&["patientSexNullFlavor"],
		);
		let race_codes = string_list_field(patient, &["raceCodes", "raceCode"]);
		let race_code_null_flavor =
			null_flavor_field(patient, &["raceCodeNullFlavor"]);
		let (ethnicity_code, ethnicity_code_null_flavor) = canonical_string_field(
			patient,
			&["ethnicityCode"],
			&["ethnicityCodeNullFlavor"],
		);
		let lmp_paths = &["lastMenstrualPeriodDate"];
		let (medical_history_text, medical_history_text_null_flavor) =
			canonical_string_field(
				patient,
				&["medicalHistoryText"],
				&["medicalHistoryTextNullFlavor"],
			);
		let update = PatientInformationForUpdate {
			patient_initials,
			patient_initials_null_flavor,
			birth_date: date_field(
				page_id,
				patient,
				"patientBirthDate",
				birth_date_paths,
			)?,
			birth_date_null_flavor: null_flavor_field(
				patient,
				&["patientBirthDateNullFlavor"],
			),
			age_at_time_of_onset: patch_decimal_field(
				page_id,
				patient,
				"patientAge.value",
				age_paths,
			)?,
			age_unit: nested_string_field(patient, &["patientAge.unit"]),
			gestation_period: decimal_field(
				page_id,
				patient,
				"gestationPeriod.value",
				&["gestationPeriod.value"],
			)?,
			gestation_period_unit: nested_string_field(
				patient,
				&["gestationPeriod.unit"],
			),
			age_group: string_field(patient, &["patientAgeGroup"]),
			weight_kg: patch_decimal_field(
				page_id,
				patient,
				"patientWeight.value",
				weight_paths,
			)?,
			height_cm: patch_decimal_field(
				page_id,
				patient,
				"patientHeight.value",
				height_paths,
			)?,
			sex,
			sex_null_flavor,
			race_codes,
			race_code_null_flavor,
			ethnicity_code,
			ethnicity_code_null_flavor,
			last_menstrual_period_date: date_field(
				page_id,
				patient,
				"lastMenstrualPeriodDate",
				lmp_paths,
			)?,
			last_menstrual_period_date_null_flavor: null_flavor_field(
				patient,
				&["lastMenstrualPeriodDateNullFlavor"],
			),
			medical_history_text,
			medical_history_text_null_flavor,
			concomitant_therapy: boolean_field(patient, &["concomitantTherapies"]),
		};
		match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => {
				let clear_fields =
					explicit_null_model_fields(patient, DM_PATIENT_PATCH_FIELDS);
				PatientInformationBmc::update_by_case_patch(
					ctx,
					mm,
					case_id,
					update,
					&clear_fields,
				)
				.await?;
				entity.id
			}
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => {
				PatientInformationBmc::create(
					ctx,
					mm,
					PatientInformationForCreate {
						case_id,
						patient_initials: update.patient_initials,
						patient_initials_null_flavor: update
							.patient_initials_null_flavor,
						birth_date: update.birth_date,
						birth_date_null_flavor: update.birth_date_null_flavor,
						age_at_time_of_onset: update.age_at_time_of_onset.flatten(),
						age_unit: update.age_unit,
						gestation_period: update.gestation_period,
						gestation_period_unit: update.gestation_period_unit,
						age_group: update.age_group,
						weight_kg: update.weight_kg.flatten(),
						height_cm: update.height_cm.flatten(),
						sex: update.sex,
						sex_null_flavor: update.sex_null_flavor,
						race_codes: update.race_codes.unwrap_or_default(),
						race_code_null_flavor: update.race_code_null_flavor,
						ethnicity_code: update.ethnicity_code,
						ethnicity_code_null_flavor: update
							.ethnicity_code_null_flavor,
						last_menstrual_period_date: update
							.last_menstrual_period_date,
						last_menstrual_period_date_null_flavor: update
							.last_menstrual_period_date_null_flavor,
						medical_history_text: update.medical_history_text,
						medical_history_text_null_flavor: update
							.medical_history_text_null_flavor,
						concomitant_therapy: update.concomitant_therapy,
					},
				)
				.await?
			}
			Err(err) => return Err(err.into()),
		}
	} else {
		PatientInformationBmc::get_or_create_by_case(ctx, mm, case_id).await?
	};

	if let Some(value) = rows.get("medicalHistoryEpisodes") {
		let Some(episodes) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!(
					"{page_id}.medicalHistoryEpisodes must be an array"
				),
			});
		};
		for value in episodes {
			let episode = as_object(page_id, "medicalHistoryEpisodes", value)?;
			let id = uuid_field(episode, &["id"]);
			if bool_field(episode, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					MedicalHistoryEpisodeBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let meddra_version = string_field(episode, &["meddraVersion"]);
			let meddra_code = string_field(episode, &["meddraCode"]);
			let start_date = ts_field(
				page_id,
				episode,
				"medicalHistoryEpisodes[].startDate",
				&["startDate"],
			)?;
			let start_date_null_flavor =
				null_flavor_field(episode, &["startDateNullFlavor"]);
			let continuing = bool_field(episode, &["continuing"]);
			let continuing_null_flavor =
				null_flavor_field(episode, &["continuingNullFlavor"]);
			let end_date = ts_field(
				page_id,
				episode,
				"medicalHistoryEpisodes[].endDate",
				&["endDate"],
			)?;
			let end_date_null_flavor =
				null_flavor_field(episode, &["endDateNullFlavor"]);
			let comments = string_field(episode, &["comments"]);
			let family_history = bool_field(episode, &["familyHistory"]);
			let update = MedicalHistoryEpisodeForUpdate {
				meddra_version,
				meddra_code: meddra_code.clone(),
				start_date,
				start_date_null_flavor: start_date_null_flavor.clone(),
				continuing,
				continuing_null_flavor: continuing_null_flavor.clone(),
				end_date,
				end_date_null_flavor: end_date_null_flavor.clone(),
				comments,
				family_history,
			};
			let id = if let Some(id) = id {
				id
			} else {
				MedicalHistoryEpisodeBmc::create(
					ctx,
					mm,
					MedicalHistoryEpisodeForCreate {
						patient_id,
						sequence_number: i32_field(episode, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"medical_history_episodes",
									"patient_id",
									patient_id,
									true,
								)
								.await?,
							),
						meddra_code,
						start_date_null_flavor,
						continuing_null_flavor,
						end_date_null_flavor,
					},
				)
				.await?
			};
			let clear_fields =
				explicit_null_model_fields(episode, DM_MEDICAL_HISTORY_PATCH_FIELDS);
			lib_core::model::update_uuid_patch::<MedicalHistoryEpisodeBmc, _>(
				ctx,
				mm,
				id,
				update,
				&clear_fields,
			)
			.await?;
		}
	}

	if let Some(value) = rows.get("patientIdentifiers") {
		const PATIENT_IDENTIFIER_PATCH_FIELDS: &[(&str, &[&str])] = &[
			("identifier_value", &["identifierValue"]),
			(
				"identifier_value_null_flavor",
				&["identifierValueNullFlavor"],
			),
		];
		let Some(identifier_rows) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.patientIdentifiers must be an array"),
			});
		};
		for value in identifier_rows {
			let identifier = as_object(page_id, "patientIdentifiers", value)?;
			let id = uuid_field(identifier, &["id"]);
			if bool_field(identifier, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					PatientIdentifierBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let identifier_type_code =
				string_field(identifier, &["identifierTypeCode"]);
			let update = PatientIdentifierForUpdate {
				identifier_type_code: identifier_type_code.clone(),
				identifier_value: string_field(identifier, &["identifierValue"]),
				identifier_value_null_flavor: string_field(
					identifier,
					&["identifierValueNullFlavor"],
				),
			};
			if let Some(id) = id {
				let clear_fields = explicit_null_model_fields(
					identifier,
					PATIENT_IDENTIFIER_PATCH_FIELDS,
				);
				PatientIdentifierBmc::update_patch(
					ctx,
					mm,
					id,
					update,
					&clear_fields,
				)
				.await?;
			} else {
				let identifier_type_code =
					identifier_type_code.ok_or_else(|| Error::BadRequest {
						message: format!(
							"{page_id}.patientIdentifiers.identifierTypeCode is required"
						),
					})?;
				PatientIdentifierBmc::create(
					ctx,
					mm,
					PatientIdentifierForCreate {
						patient_id,
						sequence_number: i32_field(identifier, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"patient_identifiers",
									"patient_id",
									patient_id,
									true,
								)
								.await?,
							),
						identifier_type_code,
						identifier_value: update.identifier_value,
						identifier_value_null_flavor: update
							.identifier_value_null_flavor,
					},
				)
				.await?;
			}
		}
	}

	let has_death_children =
		rows.contains_key("reportedCauses") || rows.contains_key("autopsyCauses");
	let empty_death_info = Map::new();
	let death_info_row = optional_row_object(page_id, rows, "deathInfo")?
		.or_else(|| has_death_children.then_some(&empty_death_info));
	let existing_death_info = PatientDeathInformationBmc::list(
		ctx,
		mm,
		Some(vec![PatientDeathInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::from_limit(1)),
	)
	.await?
	.into_iter()
	.next();
	let death_info_id = if let Some(death_info) = death_info_row {
		let update = PatientDeathInformationForUpdate {
			date_of_death: date_field(
				page_id,
				death_info,
				"patientDeath.dateOfDeath",
				&["dateOfDeath"],
			)?,
			date_of_death_null_flavor: null_flavor_field(
				death_info,
				&["dateOfDeathNullFlavor"],
			),
			autopsy_performed: bool_field(death_info, &["autopsyPerformed"]),
			autopsy_performed_null_flavor: null_flavor_field(
				death_info,
				&["autopsyPerformedNullFlavor"],
			),
		};
		if let Some(existing) = existing_death_info {
			let clear_fields =
				explicit_null_model_fields(death_info, DM_DEATH_PATCH_FIELDS);
			PatientDeathInformationBmc::update_patch(
				ctx,
				mm,
				existing.id,
				update,
				&clear_fields,
			)
			.await?;
			Some(existing.id)
		} else {
			Some(
				PatientDeathInformationBmc::create(
					ctx,
					mm,
					PatientDeathInformationForCreate {
						patient_id,
						date_of_death: update.date_of_death,
						date_of_death_null_flavor: update.date_of_death_null_flavor,
						autopsy_performed: update.autopsy_performed,
						autopsy_performed_null_flavor: update
							.autopsy_performed_null_flavor,
					},
				)
				.await?,
			)
		}
	} else {
		existing_death_info.map(|row| row.id)
	};

	if has_death_children && death_info_id.is_none() {
		return Err(Error::BadRequest {
			message: format!(
				"{page_id}.deathInfo is required before death cause rows"
			),
		});
	}
	let death_info_id = death_info_id.unwrap_or(Uuid::nil());

	if let Some(value) = rows.get("reportedCauses") {
		let Some(causes) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.reportedCauses must be an array"),
			});
		};
		for value in causes {
			let cause = as_object(page_id, "reportedCauses", value)?;
			let id = uuid_field(cause, &["id"]);
			if bool_field(cause, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					ReportedCauseOfDeathBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let update = ReportedCauseOfDeathForUpdate {
				meddra_version: string_field(cause, &["meddraVersion"]),
				meddra_code: string_field(cause, &["meddraCode"]),
				comments: string_field(cause, &["causeText"]),
			};
			if let Some(id) = id {
				let clear_fields =
					explicit_null_model_fields(cause, DM_CAUSE_PATCH_FIELDS);
				lib_core::model::update_uuid_patch::<ReportedCauseOfDeathBmc, _>(
					ctx,
					mm,
					id,
					update,
					&clear_fields,
				)
				.await?;
			} else {
				ReportedCauseOfDeathBmc::create(
					ctx,
					mm,
					ReportedCauseOfDeathForCreate {
						death_info_id,
						sequence_number: i32_field(cause, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"reported_causes_of_death",
									"death_info_id",
									death_info_id,
									true,
								)
								.await?,
							),
						meddra_version: update.meddra_version,
						meddra_code: update.meddra_code,
						comments: update.comments,
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("autopsyCauses") {
		let Some(causes) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.autopsyCauses must be an array"),
			});
		};
		for value in causes {
			let cause = as_object(page_id, "autopsyCauses", value)?;
			let id = uuid_field(cause, &["id"]);
			if bool_field(cause, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					AutopsyCauseOfDeathBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let update = AutopsyCauseOfDeathForUpdate {
				meddra_version: string_field(cause, &["meddraVersion"]),
				meddra_code: string_field(cause, &["meddraCode"]),
				comments: string_field(cause, &["causeText"]),
			};
			if let Some(id) = id {
				let clear_fields =
					explicit_null_model_fields(cause, DM_CAUSE_PATCH_FIELDS);
				lib_core::model::update_uuid_patch::<AutopsyCauseOfDeathBmc, _>(
					ctx,
					mm,
					id,
					update,
					&clear_fields,
				)
				.await?;
			} else {
				AutopsyCauseOfDeathBmc::create(
					ctx,
					mm,
					AutopsyCauseOfDeathForCreate {
						death_info_id,
						sequence_number: i32_field(cause, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"autopsy_causes_of_death",
									"death_info_id",
									death_info_id,
									true,
								)
								.await?,
							),
						meddra_version: update.meddra_version,
						meddra_code: update.meddra_code,
						comments: update.comments,
					},
				)
				.await?;
			}
		}
	}

	let empty_parent = Map::new();
	let has_parent_children = rows.contains_key("parentMedicalHistory")
		|| rows.contains_key("parentPastDrugs");
	let parent_patch = optional_row_object(page_id, rows, "parentInfo")?
		.or_else(|| has_parent_children.then_some(&empty_parent));
	if let Some(parent) = parent_patch {
		let existing = ParentInformationBmc::list(
			ctx,
			mm,
			Some(vec![ParentInformationFilter {
				patient_id: Some(uuid_eq(patient_id)),
				..Default::default()
			}]),
			Some(ListOptions::from_limit(1)),
		)
		.await?
		.into_iter()
		.next();
		if bool_field(parent, &["deleted"]) == Some(true) {
			if let Some(id) = uuid_field(parent, &["id"])
				.or_else(|| existing.as_ref().map(|row| row.id))
			{
				ParentInformationBmc::delete(ctx, mm, id).await?;
			}
		} else {
			let (parent_identification, parent_identification_null_flavor) =
				canonical_string_field(
					parent,
					&["parentIdentification"],
					&["parentIdentificationNullFlavor"],
				);
			let (sex, sex_null_flavor) = canonical_string_field(
				parent,
				&["parentSex"],
				&["parentSexNullFlavor"],
			);
			let update = ParentInformationForUpdate {
				parent_identification,
				parent_identification_null_flavor,
				parent_birth_date: date_field(
					page_id,
					parent,
					"parentInformation.parentBirthDate",
					&["parentBirthDate"],
				)?,
				parent_birth_date_null_flavor: null_flavor_field(
					parent,
					&["parentBirthDateNullFlavor"],
				),
				parent_age: patch_decimal_field(
					page_id,
					parent,
					"parentInformation.parentAge.value",
					&["parentAge.value"],
				)?,
				parent_age_unit: nested_string_field(parent, &["parentAge.unit"]),
				last_menstrual_period_date: date_field(
					page_id,
					parent,
					"parentInformation.parentLastMenstrualPeriodDate",
					&["parentLastMenstrualPeriodDate"],
				)?,
				last_menstrual_period_date_null_flavor: null_flavor_field(
					parent,
					&["parentLastMenstrualPeriodDateNullFlavor"],
				),
				weight_kg: decimal_field(
					page_id,
					parent,
					"parentInformation.parentWeight.value",
					&["parentWeight.value"],
				)?,
				height_cm: decimal_field(
					page_id,
					parent,
					"parentInformation.parentHeight.value",
					&["parentHeight.value"],
				)?,
				sex,
				sex_null_flavor,
				medical_history_text: string_field(parent, &["medicalHistoryText"]),
			};
			if let Some(existing) = existing {
				let clear_fields =
					explicit_null_model_fields(parent, DM_PARENT_PATCH_FIELDS);
				ParentInformationBmc::update_patch(
					ctx,
					mm,
					existing.id,
					update,
					&clear_fields,
				)
				.await?;
			} else {
				ParentInformationBmc::create(
					ctx,
					mm,
					ParentInformationForCreate {
						patient_id,
						parent_identification: update.parent_identification,
						parent_identification_null_flavor: update
							.parent_identification_null_flavor,
						parent_birth_date: update.parent_birth_date,
						parent_birth_date_null_flavor: update
							.parent_birth_date_null_flavor,
						parent_age: update.parent_age.flatten(),
						parent_age_unit: update.parent_age_unit,
						last_menstrual_period_date: update
							.last_menstrual_period_date,
						last_menstrual_period_date_null_flavor: update
							.last_menstrual_period_date_null_flavor,
						weight_kg: update.weight_kg,
						height_cm: update.height_cm,
						sex: update.sex,
						sex_null_flavor: update.sex_null_flavor,
						medical_history_text: update.medical_history_text,
					},
				)
				.await?;
			}
		}
	}

	if let Some(value) = rows.get("parentMedicalHistory") {
		let Some(history_rows) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.parentMedicalHistory must be an array"),
			});
		};
		let parent_id = ParentInformationBmc::list(
			ctx,
			mm,
			Some(vec![ParentInformationFilter {
				patient_id: Some(uuid_eq(patient_id)),
				..Default::default()
			}]),
			Some(ListOptions::from_limit(1)),
		)
		.await?
		.into_iter()
		.next()
		.map(|row| row.id)
		.ok_or_else(|| Error::BadRequest {
			message: format!(
				"{page_id}.parentInfo is required before parent medical history"
			),
		})?;
		for value in history_rows {
			let history = as_object(page_id, "parentMedicalHistory", value)?;
			let id = uuid_field(history, &["id"]);
			if bool_field(history, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					ParentMedicalHistoryBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let meddra_code = string_field(history, &["meddraCode"]);
			let start_date_null_flavor =
				null_flavor_field(history, &["startDateNullFlavor"]);
			let continuing_null_flavor =
				null_flavor_field(history, &["continuingNullFlavor"]);
			let end_date_null_flavor =
				null_flavor_field(history, &["endDateNullFlavor"]);
			let update = ParentMedicalHistoryForUpdate {
				meddra_version: string_field(history, &["meddraVersion"]),
				meddra_code: meddra_code.clone(),
				start_date: date_field(
					page_id,
					history,
					"parentInformation.medicalHistoryEpisodes[].startDate",
					&["startDate"],
				)?,
				start_date_null_flavor: start_date_null_flavor.clone(),
				continuing: bool_field(history, &["continuing"]),
				continuing_null_flavor: continuing_null_flavor.clone(),
				end_date: date_field(
					page_id,
					history,
					"parentInformation.medicalHistoryEpisodes[].endDate",
					&["endDate"],
				)?,
				end_date_null_flavor: end_date_null_flavor.clone(),
				comments: string_field(history, &["comments"]),
			};
			let id = if let Some(id) = id {
				id
			} else {
				ParentMedicalHistoryBmc::create(
					ctx,
					mm,
					ParentMedicalHistoryForCreate {
						parent_id,
						sequence_number: i32_field(history, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"parent_medical_history",
									"parent_id",
									parent_id,
									true,
								)
								.await?,
							),
						meddra_code,
						start_date_null_flavor,
						continuing_null_flavor,
						end_date_null_flavor,
					},
				)
				.await?
			};
			let clear_fields = explicit_null_model_fields(
				history,
				DM_PARENT_MEDICAL_HISTORY_PATCH_FIELDS,
			);
			lib_core::model::update_uuid_patch::<ParentMedicalHistoryBmc, _>(
				ctx,
				mm,
				id,
				update,
				&clear_fields,
			)
			.await?;
		}
	}

	if let Some(value) = rows.get("parentPastDrugs") {
		let Some(drug_rows) = value.as_array() else {
			return Err(Error::BadRequest {
				message: format!("{page_id}.parentPastDrugs must be an array"),
			});
		};
		let parent_id = ParentInformationBmc::list(
			ctx,
			mm,
			Some(vec![ParentInformationFilter {
				patient_id: Some(uuid_eq(patient_id)),
				..Default::default()
			}]),
			Some(ListOptions::from_limit(1)),
		)
		.await?
		.into_iter()
		.next()
		.map(|row| row.id)
		.ok_or_else(|| Error::BadRequest {
			message: format!(
				"{page_id}.parentInfo is required before parent past drug history"
			),
		})?;
		for value in drug_rows {
			let drug = as_object(page_id, "parentPastDrugs", value)?;
			let id = uuid_field(drug, &["id"]);
			if bool_field(drug, &["deleted"]) == Some(true) {
				if let Some(id) = id {
					ParentPastDrugHistoryBmc::delete(ctx, mm, id).await?;
				}
				continue;
			}
			let update = ParentPastDrugHistoryForUpdate {
				drug_name: string_field(drug, &["drugName"]),
				mpid: string_field(drug, &["mpid"]),
				mpid_version: string_field(drug, &["mpidVersion"]),
				mfds_medicinal_product_version: string_field(
					drug,
					&["mfdsMedicinalProductVersion"],
				),
				mfds_medicinal_product_id: string_field(
					drug,
					&["mfdsMedicinalProductId"],
				),
				phpid: string_field(drug, &["phpid"]),
				phpid_version: string_field(drug, &["phpidVersion"]),
				start_date: date_field(
					page_id,
					drug,
					"parentInformation.pastDrugHistory[].startDate",
					&["startDate"],
				)?,
				start_date_null_flavor: null_flavor_field(
					drug,
					&["startDateNullFlavor"],
				),
				end_date: date_field(
					page_id,
					drug,
					"parentInformation.pastDrugHistory[].endDate",
					&["endDate"],
				)?,
				end_date_null_flavor: null_flavor_field(
					drug,
					&["endDateNullFlavor"],
				),
				indication_meddra_version: string_field(
					drug,
					&["indicationMeddraVersion"],
				),
				indication_meddra_code: string_field(
					drug,
					&["indicationMeddraCode"],
				),
				reaction_meddra_version: string_field(
					drug,
					&["reactionMeddraVersion"],
				),
				reaction_meddra_code: string_field(drug, &["reactionMeddraCode"]),
			};
			if let Some(id) = id {
				let clear_fields = explicit_null_model_fields(
					drug,
					DM_PARENT_PAST_DRUG_PATCH_FIELDS,
				);
				ParentPastDrugHistoryBmc::update_patch(
					ctx,
					mm,
					id,
					update,
					&clear_fields,
				)
				.await?;
			} else {
				ParentPastDrugHistoryBmc::create(
					ctx,
					mm,
					ParentPastDrugHistoryForCreate {
						parent_id,
						sequence_number: i32_field(drug, &["sequenceNumber"])
							.unwrap_or(
								next_child_sequence(
									ctx,
									mm,
									"parent_past_drug_history",
									"parent_id",
									parent_id,
									true,
								)
								.await?,
							),
						drug_name: update.drug_name,
						mpid: update.mpid,
						mpid_version: update.mpid_version,
						mfds_medicinal_product_version: update
							.mfds_medicinal_product_version,
						mfds_medicinal_product_id: update.mfds_medicinal_product_id,
						phpid: update.phpid,
						phpid_version: update.phpid_version,
						start_date: update.start_date,
						start_date_null_flavor: update.start_date_null_flavor,
						end_date: update.end_date,
						end_date_null_flavor: update.end_date_null_flavor,
						indication_meddra_version: update.indication_meddra_version,
						indication_meddra_code: update.indication_meddra_code,
						reaction_meddra_version: update.reaction_meddra_version,
						reaction_meddra_code: update.reaction_meddra_code,
					},
				)
				.await?;
			}
		}
	}
	Ok(())
}

pub(super) async fn load_editor_dm_data(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
) -> Result<Value> {
	let Some(patient) =
		(match PatientInformationBmc::get_by_case(ctx, mm, case_id).await {
			Ok(entity) => Some(entity),
			Err(lib_core::model::Error::EntityUuidNotFound { .. }) => None,
			Err(err) => return Err(err.into()),
		})
	else {
		return Ok(json!({
			"patientInformation": null,
			"patientIdentifiers": [],
			"medicalHistoryEpisodes": [],
			"deathInfo": null,
			"reportedCauses": [],
			"autopsyCauses": [],
			"parentInfo": null,
			"parentMedicalHistory": [],
			"parentPastDrugs": [],
		}));
	};

	let patient_id = patient.id;
	let patient_identifiers = PatientIdentifierBmc::list(
		ctx,
		mm,
		Some(vec![PatientIdentifierFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let medical_history_episodes = MedicalHistoryEpisodeBmc::list(
		ctx,
		mm,
		Some(vec![MedicalHistoryEpisodeFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
	)
	.await?;
	let medical_history_episodes = medical_history_episodes
		.into_iter()
		.map(|episode| {
			let mut value = json!(episode);
			if let Value::Object(ref mut map) = value {
				map.insert("start_date".to_string(), json!(episode.start_date));
				map.insert("end_date".to_string(), json!(episode.end_date));
			}
			value
		})
		.collect::<Vec<_>>();
	let parent_information_rows = ParentInformationBmc::list(
		ctx,
		mm,
		Some(vec![ParentInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
			..Default::default()
		}]),
		Some(ListOptions::from_order_bys(vec!["created_at", "id"])),
	)
	.await?;
	let parent_ids = parent_information_rows
		.iter()
		.map(|parent| parent.id)
		.collect::<Vec<_>>();
	let parent_medical_history_rows = if parent_ids.is_empty() {
		Vec::new()
	} else {
		ParentMedicalHistoryBmc::list(
			ctx,
			mm,
			Some(
				parent_ids
					.iter()
					.copied()
					.map(|parent_id| ParentMedicalHistoryFilter {
						parent_id: Some(uuid_eq(parent_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let parent_past_drug_rows = if parent_ids.is_empty() {
		Vec::new()
	} else {
		ParentPastDrugHistoryBmc::list(
			ctx,
			mm,
			Some(
				parent_ids
					.iter()
					.copied()
					.map(|parent_id| ParentPastDrugHistoryFilter {
						parent_id: Some(uuid_eq(parent_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let mut medical_history_by_parent = BTreeMap::new();
	for history in parent_medical_history_rows {
		medical_history_by_parent
			.entry(history.parent_id)
			.or_insert_with(Vec::new)
			.push(history);
	}
	let mut past_drugs_by_parent = BTreeMap::new();
	for drug in parent_past_drug_rows {
		past_drugs_by_parent
			.entry(drug.parent_id)
			.or_insert_with(Vec::new)
			.push(drug);
	}
	let mut parents = Vec::new();
	let mut parent_medical_history = Vec::new();
	let mut parent_past_drugs = Vec::new();
	for parent in &parent_information_rows {
		let medical_history = medical_history_by_parent
			.remove(&parent.id)
			.unwrap_or_default();
		let medical_history = medical_history
			.into_iter()
			.map(|history| {
				let mut value = json!(history);
				if let Value::Object(ref mut map) = value {
					map.insert(
						"start_date".to_string(),
						json!(ci_date(history.start_date)),
					);
					map.insert(
						"end_date".to_string(),
						json!(ci_date(history.end_date)),
					);
				}
				value
			})
			.collect::<Vec<_>>();
		let past_drug_history =
			past_drugs_by_parent.remove(&parent.id).unwrap_or_default();
		let past_drug_history = past_drug_history
			.into_iter()
			.map(|drug| {
				let mut value = json!(drug);
				if let Value::Object(ref mut map) = value {
					map.insert(
						"start_date".to_string(),
						json!(ci_date(drug.start_date)),
					);
					map.insert(
						"end_date".to_string(),
						json!(ci_date(drug.end_date)),
					);
				}
				value
			})
			.collect::<Vec<_>>();
		let mut parent_with_children = json!(parent);
		if let Value::Object(ref mut map) = parent_with_children {
			map.insert("medicalHistory".to_string(), json!(medical_history));
			map.insert("pastDrugHistory".to_string(), json!(past_drug_history));
			map.insert("pastDrugs".to_string(), json!(past_drug_history));
		}
		parent_medical_history.extend(medical_history);
		parent_past_drugs.extend(past_drug_history);
		parents.push(parent_with_children);
	}
	let death_information = PatientDeathInformationBmc::list(
		ctx,
		mm,
		Some(vec![PatientDeathInformationFilter {
			patient_id: Some(uuid_eq(patient_id)),
		}]),
		Some(ListOptions::from_order_bys(vec!["created_at", "id"])),
	)
	.await?;
	let death_info_ids = death_information
		.iter()
		.map(|death_info| death_info.id)
		.collect::<Vec<_>>();
	let reported_cause_rows = if death_info_ids.is_empty() {
		Vec::new()
	} else {
		ReportedCauseOfDeathBmc::list(
			ctx,
			mm,
			Some(
				death_info_ids
					.iter()
					.copied()
					.map(|death_info_id| ReportedCauseOfDeathFilter {
						death_info_id: Some(uuid_eq(death_info_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let autopsy_cause_rows = if death_info_ids.is_empty() {
		Vec::new()
	} else {
		AutopsyCauseOfDeathBmc::list(
			ctx,
			mm,
			Some(
				death_info_ids
					.iter()
					.copied()
					.map(|death_info_id| AutopsyCauseOfDeathFilter {
						death_info_id: Some(uuid_eq(death_info_id)),
						..Default::default()
					})
					.collect(),
			),
			Some(ListOptions::from_order_bys(vec!["sequence_number", "id"])),
		)
		.await?
	};
	let mut reported_causes_by_death = BTreeMap::new();
	for cause in reported_cause_rows {
		reported_causes_by_death
			.entry(cause.death_info_id)
			.or_insert_with(Vec::new)
			.push(cause);
	}
	let mut autopsy_causes_by_death = BTreeMap::new();
	for cause in autopsy_cause_rows {
		autopsy_causes_by_death
			.entry(cause.death_info_id)
			.or_insert_with(Vec::new)
			.push(cause);
	}
	let mut reported_causes = Vec::new();
	let mut autopsy_causes = Vec::new();
	for death_info in &death_information {
		reported_causes.extend(
			reported_causes_by_death
				.remove(&death_info.id)
				.unwrap_or_default(),
		);
		autopsy_causes.extend(
			autopsy_causes_by_death
				.remove(&death_info.id)
				.unwrap_or_default(),
		);
	}
	let death_info = death_information.into_iter().next().map(|death_info| {
		let mut value = json!(death_info);
		if let Value::Object(ref mut map) = value {
			map.insert(
				"date_of_death".to_string(),
				json!(ci_date(death_info.date_of_death)),
			);
		}
		value
	});
	let parent_info = parent_information_rows.into_iter().next().map(|parent| {
		let mut value = json!(parent);
		if let Value::Object(ref mut map) = value {
			map.insert(
				"parent_birth_date".to_string(),
				json!(ci_date(parent.parent_birth_date)),
			);
			map.insert(
				"last_menstrual_period_date".to_string(),
				json!(ci_date(parent.last_menstrual_period_date)),
			);
		}
		value
	});
	let mut patient_projection = json!(patient);
	if let Value::Object(ref mut map) = patient_projection {
		map.insert("birth_date".to_string(), json!(ci_date(patient.birth_date)));
		map.insert(
			"last_menstrual_period_date".to_string(),
			json!(ci_date(patient.last_menstrual_period_date)),
		);
	}

	Ok(json!({
		"patientInformation": patient_projection,
		"patientIdentifiers": patient_identifiers,
		"medicalHistoryEpisodes": medical_history_episodes,
		"deathInfo": death_info,
		"reportedCauses": reported_causes,
		"autopsyCauses": autopsy_causes,
		"parentInfo": parent_info,
		"parentMedicalHistory": parent_medical_history,
		"parentPastDrugs": parent_past_drugs,
		"parents": parents,
	}))
}

pub async fn get_editor_dm(
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
		"editor/DM",
		move |ctx, mm| {
			Box::pin(async move {
				Ok(direct_section_response(
					case_id,
					load_editor_dm_data(ctx, mm, case_id).await?,
				))
			})
		},
	)
	.await
}

direct_page_projection_handler!(
	get_editor_dm_page_projection,
	"DM",
	load_editor_dm_data,
);
