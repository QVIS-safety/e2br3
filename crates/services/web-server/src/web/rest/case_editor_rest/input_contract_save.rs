#[cfg(test)]
use super::common::json;
use super::common::{
	as_object, bool_field, optional_row_object, string_field, BTreeMap, Error, Map,
	Result, Value,
};
use input_contracts::{FieldInput, InputIssue, InputValue};
use lib_rest_core::ConstraintViolation;
use std::collections::BTreeSet;

#[path = "input_contract_fields.rs"]
mod input_contract_fields;
use input_contract_fields::validate_section_fields;

#[derive(Clone, Copy)]
enum InputType {
	String,
	Boolean,
	Number,
}

struct RequestMatch<'a> {
	value: &'a Value,
	indexes: Vec<usize>,
}

enum JsonNode<'a> {
	Object(&'a Map<String, Value>),
	Value(&'a Value),
}

fn request_matches<'a>(
	row: &'a Map<String, Value>,
	template: &str,
) -> Vec<RequestMatch<'a>> {
	fn visit<'a>(
		current: JsonNode<'a>,
		segments: &[&str],
		indexes: &[usize],
		matches: &mut Vec<RequestMatch<'a>>,
	) {
		if segments.is_empty() {
			if let JsonNode::Value(value) = current {
				matches.push(RequestMatch {
					value,
					indexes: indexes.to_vec(),
				});
			}
			return;
		}
		let object = match current {
			JsonNode::Object(object) => object,
			JsonNode::Value(value) => match value.as_object() {
				Some(object) => object,
				None => return,
			},
		};
		let segment = segments[0];
		let repeated = segment.ends_with("[]");
		let key = segment.strip_suffix("[]").unwrap_or(segment);
		let Some(value) = object.get(key) else {
			return;
		};
		if !repeated {
			visit(JsonNode::Value(value), &segments[1..], indexes, matches);
			return;
		}
		let Some(values) = value.as_array() else {
			return;
		};
		for (index, value) in values.iter().enumerate() {
			let mut concrete_indexes = indexes.to_vec();
			concrete_indexes.push(index);
			visit(
				JsonNode::Value(value),
				&segments[1..],
				&concrete_indexes,
				matches,
			);
		}
	}

	let segments = template.split('.').collect::<Vec<_>>();
	let mut matches = Vec::new();
	visit(JsonNode::Object(row), &segments, &[], &mut matches);
	matches
}

fn value_at_request_path<'a>(
	row: &'a Map<String, Value>,
	template: &str,
	indexes: &[usize],
) -> Option<&'a Value> {
	request_matches(row, template)
		.into_iter()
		.find(|matched| matched.indexes == indexes)
		.map(|matched| matched.value)
}

fn input_value<'a>(value: &'a Value, value_type: InputType) -> InputValue<'a> {
	if value.is_null() {
		return InputValue::Null;
	}
	match (value_type, value) {
		(InputType::String, Value::String(value)) => InputValue::String(value),
		(InputType::Boolean, Value::Bool(value)) => InputValue::Boolean(*value),
		(InputType::Number, Value::Number(value)) => InputValue::Number(value),
		_ => InputValue::InvalidType,
	}
}

fn concrete_frontend_path(template: &str, request_indexes: &[usize]) -> String {
	let repeated_count = template
		.split('.')
		.filter(|part| part.ends_with("[]"))
		.count();
	let mut indexes = vec![0; repeated_count.saturating_sub(request_indexes.len())];
	indexes.extend_from_slice(request_indexes);
	let mut index = indexes.into_iter();
	template
		.split('.')
		.map(|part| {
			part.strip_suffix("[]")
				.map(|part| format!("{part}.{}", index.next().unwrap_or(0)))
				.unwrap_or_else(|| part.to_string())
		})
		.collect::<Vec<_>>()
		.join(".")
}

fn violation(rule_code: &str, path: &str, message: &str) -> Error {
	Error::ConstraintViolation(ConstraintViolation {
		rule_code: rule_code.to_owned(),
		path: path.to_owned(),
		message: message.to_owned(),
	})
}

fn normalized_direct_object(
	source: &Map<String, Value>,
	aliases: &[(&str, &[&str])],
) -> Map<String, Value> {
	fn source_value<'a>(
		source: &'a Map<String, Value>,
		path: &str,
	) -> Option<&'a Value> {
		let mut segments = path.split('.');
		let first = segments.next()?;
		let mut value = source.get(first)?;
		for segment in segments {
			value = value.as_object()?.get(segment)?;
		}
		Some(value)
	}

	fn insert_path(target: &mut Map<String, Value>, path: &str, value: Value) {
		let mut current = target;
		let mut segments = path.split('.').peekable();
		while let Some(segment) = segments.next() {
			if segments.peek().is_none() {
				current.insert(segment.to_string(), value);
				return;
			}
			current = current
				.entry(segment.to_string())
				.or_insert_with(|| Value::Object(Map::new()))
				.as_object_mut()
				.expect("direct normalization path must remain an object");
		}
	}

	let mut normalized = Map::new();
	for (target, candidates) in aliases {
		if let Some(value) =
			candidates.iter().find_map(|key| source_value(source, key))
		{
			insert_path(&mut normalized, target, value.clone());
		}
	}
	normalized
}

pub(super) fn validate_direct_rows(
	section: &str,
	rows: &BTreeMap<String, Value>,
	fda: bool,
) -> Result<()> {
	for (key, value) in rows {
		reject_control_characters(value, &format!("{section}.{key}"))?;
	}
	if section == "CI" {
		if let Some(row) = optional_row_object(section, rows, "case")? {
			validate_field(
				row,
				"reportYear",
				"case.reportYear",
				InputType::String,
				None,
				None,
				&[],
				|input| match input.value {
					InputValue::Missing | InputValue::Null => Vec::new(),
					InputValue::String(value)
						if value.trim().is_empty()
							|| (value.trim().len() == 4
								&& value
									.trim()
									.bytes()
									.all(|b| b.is_ascii_digit())) =>
					{
						Vec::new()
					}
					_ => vec![InputIssue {
						code: "LOCAL.REPORT_YEAR.FORMAT",
						message: "must be a four-digit year".into(),
					}],
				},
			)?;
		}
	}
	if section == "NR" {
		if let Some(row) = optional_row_object(section, rows, "narrative")? {
			validate_local_text(
				row,
				"additionalInformation",
				"narrative.additionalInformation",
				20000,
			)?;
		}
	}
	let normalized = match section {
		"CI" => {
			let mut normalized =
				optional_row_object(section, rows, "safetyReportIdentification")?
					.map(|row| {
						normalized_direct_object(
					row,
					&[
						("safetyReportId", &["safetyReportId", "safety_report_id"]),
						(
							"transmissionDate",
							&["transmissionDate", "transmission_date"],
						),
						("reportType", &["reportType", "report_type"]),
						(
							"dateFirstReceivedFromSource",
							&[
								"dateFirstReceivedFromSource",
								"date_first_received_from_source",
							],
						),
						(
							"dateOfMostRecentInformation",
							&[
								"dateOfMostRecentInformation",
								"date_of_most_recent_information",
							],
						),
						(
							"fulfilExpeditedCriteria",
							&[
								"fulfilExpeditedCriteria",
								"fulfil_expedited_criteria",
							],
						),
						(
							"localCriteriaReportType",
							&[
								"localCriteriaReportType",
								"local_criteria_report_type",
							],
						),
						(
							"combinationProductReportIndicator",
							&[
								"combinationProductReportIndicator",
								"combination_product_report_indicator",
							],
						),
						(
							"combinationProductReportIndicatorNullFlavor",
							&[
								"combinationProductReportIndicatorNullFlavor",
								"combination_product_report_indicator_null_flavor",
							],
						),
						(
							"worldwideUniqueId",
							&["worldwideUniqueId", "worldwide_unique_id"],
						),
						(
							"firstSenderType",
							&["firstSenderType", "first_sender_type"],
						),
						(
							"additionalDocumentsAvailable",
							&[
								"additionalDocumentsAvailable",
								"additional_documents_available",
							],
						),
						(
							"otherCaseIdentifiersExist",
							&[
								"otherCaseIdentifiersExist",
								"other_case_identifiers_exist",
							],
						),
						(
							"otherCaseIdentifiersExistNullFlavor",
							&[
								"otherCaseIdentifiersExistNullFlavor",
								"other_case_identifiers_exist_null_flavor",
							],
						),
						(
							"nullificationAmendmentCode",
							&[
								"nullificationAmendmentCode",
								"nullificationCode",
								"nullification_code",
							],
						),
						(
							"nullificationReason",
							&["nullificationReason", "nullification_reason"],
						),
					],
				)
					})
					.unwrap_or_default();
			for (owner, aliases) in [
				(
					"documentsHeldBySender",
					&[
						(
							"documentDescription",
							&["documentDescription", "document_description"][..],
						),
						(
							"includedDocument",
							&["includedDocument", "included_document"][..],
						),
					][..],
				),
				(
					"otherCaseIdentifiers",
					&[
						("source", &["source"][..]),
						(
							"caseIdentifier",
							&["caseIdentifier", "case_identifier"][..],
						),
					][..],
				),
				(
					"linkedReports",
					&[(
						"linkedReportNumber",
						&["linkedReportNumber", "linked_report_number"][..],
					)][..],
				),
			] {
				let Some(value) = rows.get(owner) else {
					continue;
				};
				let Some(items) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!("{section}.{owner} must be an array"),
					});
				};
				let mut normalized_items = Vec::with_capacity(items.len());
				for value in items {
					let row = as_object(section, owner, value)?;
					if bool_field(row, &["deleted", "_delete"]) == Some(true) {
						continue;
					}
					normalized_items
						.push(Value::Object(normalized_direct_object(row, aliases)));
				}
				normalized.insert(owner.to_string(), Value::Array(normalized_items));
			}
			(!normalized.is_empty()).then_some(normalized)
		}
		"RP" => {
			let Some(value) = rows.get("primarySources") else {
				return Ok(());
			};
			let Some(items) = value.as_array() else {
				return Err(Error::BadRequest {
					message: format!("{section}.primarySources must be an array"),
				});
			};
			for (row_index, value) in items.iter().enumerate() {
				let row = as_object(section, "primarySources", value)?;
				let normalized = normalized_direct_object(
					row,
					&[
						("reporterTitle", &["reporterTitle", "reporter_title"]),
						(
							"reporterTitleNullFlavor",
							&[
								"reporterTitleNullFlavor",
								"reporter_title_null_flavor",
							],
						),
						(
							"reporterGivenName",
							&["reporterGivenName", "reporter_given_name"],
						),
						(
							"reporterGivenNameNullFlavor",
							&[
								"reporterGivenNameNullFlavor",
								"reporter_given_name_null_flavor",
							],
						),
						(
							"reporterMiddleName",
							&["reporterMiddleName", "reporter_middle_name"],
						),
						(
							"reporterMiddleNameNullFlavor",
							&[
								"reporterMiddleNameNullFlavor",
								"reporter_middle_name_null_flavor",
							],
						),
						(
							"reporterFamilyName",
							&["reporterFamilyName", "reporter_family_name"],
						),
						(
							"reporterFamilyNameNullFlavor",
							&[
								"reporterFamilyNameNullFlavor",
								"reporter_family_name_null_flavor",
							],
						),
						(
							"reporterOrganization",
							&["reporterOrganization", "organization"],
						),
						(
							"reporterOrganizationNullFlavor",
							&[
								"reporterOrganizationNullFlavor",
								"organization_null_flavor",
							],
						),
						(
							"reporterDepartment",
							&["reporterDepartment", "department"],
						),
						(
							"reporterDepartmentNullFlavor",
							&[
								"reporterDepartmentNullFlavor",
								"department_null_flavor",
							],
						),
						("reporterStreet", &["reporterStreet", "street"]),
						(
							"reporterStreetNullFlavor",
							&["reporterStreetNullFlavor", "street_null_flavor"],
						),
						("reporterCity", &["reporterCity", "city"]),
						(
							"reporterCityNullFlavor",
							&["reporterCityNullFlavor", "city_null_flavor"],
						),
						("reporterState", &["reporterState", "state"]),
						(
							"reporterStateNullFlavor",
							&["reporterStateNullFlavor", "state_null_flavor"],
						),
						("reporterPostcode", &["reporterPostcode", "postcode"]),
						(
							"reporterPostcodeNullFlavor",
							&["reporterPostcodeNullFlavor", "postcode_null_flavor"],
						),
						("reporterTelephone", &["reporterTelephone", "telephone"]),
						(
							"reporterTelephoneNullFlavor",
							&[
								"reporterTelephoneNullFlavor",
								"telephone_null_flavor",
							],
						),
						("reporterCountry", &["reporterCountry", "country_code"]),
						("reporterEmail", &["reporterEmail", "email"]),
						(
							"reporterEmailNullFlavor",
							&["reporterEmailNullFlavor", "email_null_flavor"],
						),
						("qualification", &["qualification"]),
						(
							"qualificationNullFlavor",
							&[
								"qualificationNullFlavor",
								"qualification_null_flavor",
							],
						),
						(
							"qualificationKr1",
							&["qualificationKr1", "qualification_kr1"],
						),
						(
							"primarySourceForRegulatoryPurposes",
							&[
								"primarySourceForRegulatoryPurposes",
								"primary_source_regulatory",
							],
						),
					],
				);
				validate_row_payload_with_indexes(
					section,
					section,
					&normalized,
					None,
					&[row_index],
				)?;
			}
			return Ok(());
		}
		"SD" => optional_row_object(section, rows, "senderInformation")?.cloned(),
		"SI" => {
			let study = optional_row_object(section, rows, "studyInformation")?;
			let mut normalized = study
				.map(|row| {
					normalized_direct_object(
						row,
						&[
							("studyName", &["studyName", "study_name"]),
							(
								"studyNameNullFlavor",
								&["studyNameNullFlavor", "study_name_null_flavor"],
							),
							(
								"sponsorStudyNumber",
								&["sponsorStudyNumber", "sponsor_study_number"],
							),
							(
								"sponsorStudyNumberNullFlavor",
								&[
									"sponsorStudyNumberNullFlavor",
									"sponsor_study_number_null_flavor",
								],
							),
							(
								"studyTypeReaction",
								&["studyTypeReaction", "study_type_reaction"],
							),
							(
								"studyTypeReactionKr1",
								&["studyTypeReactionKr1", "study_type_reaction_kr1"],
							),
							(
								"fdaIndNumberOccurred",
								&["fdaIndNumberOccurred", "fda_ind_number_occurred"],
							),
							(
								"fdaPreAndaNumberOccurred",
								&[
									"fdaPreAndaNumberOccurred",
									"fda_pre_anda_number_occurred",
								],
							),
						],
					)
				})
				.unwrap_or_default();

			if let Some(study) = study {
				if let Some(value) = study
					.get("fdaCrossReportedIndNumbers")
					.or_else(|| study.get("fda_cross_reported_ind_numbers"))
				{
					let Some(items) = value.as_array() else {
						return Err(Error::BadRequest {
							message: format!(
								"{section}.studyInformation.fdaCrossReportedIndNumbers must be an array"
							),
						});
					};
					let mut normalized_items = Vec::with_capacity(items.len());
					for value in items {
						let row =
							as_object(section, "fdaCrossReportedIndNumbers", value)?;
						normalized_items.push(Value::Object(
							normalized_direct_object(
								row,
								&[
									("indNumber", &["indNumber", "ind_number"]),
									(
										"indNumberNullFlavor",
										&[
											"indNumberNullFlavor",
											"ind_number_null_flavor",
										],
									),
								],
							),
						));
					}
					normalized.insert(
						"fdaCrossReportedIndNumbers".to_string(),
						Value::Array(normalized_items),
					);
				}
			}

			if let Some(value) = rows.get("studyRegistrationNumbers") {
				let Some(items) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.studyRegistrationNumbers must be an array"
						),
					});
				};
				let mut normalized_items = Vec::with_capacity(items.len());
				for value in items {
					let row = as_object(section, "studyRegistrationNumbers", value)?;
					normalized_items.push(Value::Object(normalized_direct_object(
						row,
						&[
							(
								"registrationNumber",
								&["registrationNumber", "registration_number"],
							),
							(
								"registrationNumberNullFlavor",
								&[
									"registrationNumberNullFlavor",
									"registration_number_null_flavor",
								],
							),
							("countryCode", &["countryCode", "country_code"]),
							(
								"countryCodeNullFlavor",
								&[
									"countryCodeNullFlavor",
									"country_code_null_flavor",
								],
							),
						],
					)));
				}
				normalized.insert(
					"studyRegistrationNumbers".to_string(),
					Value::Array(normalized_items),
				);
			}

			(!normalized.is_empty()).then_some(normalized)
		}
		"DM" => {
			let mut normalized =
				optional_row_object(section, rows, "patientInformation")?
					.map(|row| {
						let mut normalized = normalized_direct_object(
							row,
							&[
								(
									"patientInitials",
									&["patientInitials", "patient_initials"],
								),
								(
									"patientInitialsNullFlavor",
									&[
										"patientInitialsNullFlavor",
										"patient_initials_null_flavor",
									],
								),
								(
									"patientBirthDate",
									&["patientBirthDate", "birth_date"],
								),
								(
									"patientBirthDateNullFlavor",
									&[
										"patientBirthDateNullFlavor",
										"birth_date_null_flavor",
									],
								),
								(
									"patientAge.value",
									&[
										"patientAge.value",
										"ageAtTimeOfOnset",
										"age_at_time_of_onset",
									],
								),
								(
									"patientAge.unit",
									&["patientAge.unit", "ageUnit", "age_unit"],
								),
								(
									"gestationPeriod.value",
									&["gestationPeriod.value", "gestation_period"],
								),
								(
									"gestationPeriod.unit",
									&[
										"gestationPeriod.unit",
										"gestationPeriodUnit",
										"gestation_period_unit",
									],
								),
								(
									"patientAgeGroup",
									&["patientAgeGroup", "ageGroup", "age_group"],
								),
								(
									"patientWeight.value",
									&[
										"patientWeight.value",
										"weightKg",
										"weight_kg",
									],
								),
								(
									"patientHeight.value",
									&[
										"patientHeight.value",
										"heightCm",
										"height_cm",
									],
								),
								("patientSex", &["patientSex", "sex"]),
								(
									"patientSexNullFlavor",
									&["patientSexNullFlavor", "sex_null_flavor"],
								),
								(
									"raceCodes",
									&[
										"raceCodes",
										"race_codes",
										"raceCode",
										"race_code",
									],
								),
								(
									"raceCodeNullFlavor",
									&["raceCodeNullFlavor", "race_code_null_flavor"],
								),
								(
									"ethnicityCode",
									&["ethnicityCode", "ethnicity_code"],
								),
								(
									"ethnicityCodeNullFlavor",
									&[
										"ethnicityCodeNullFlavor",
										"ethnicity_code_null_flavor",
									],
								),
								(
									"lastMenstrualPeriodDate",
									&[
										"lastMenstrualPeriodDate",
										"last_menstrual_period_date",
									],
								),
								(
									"lastMenstrualPeriodDateNullFlavor",
									&[
										"lastMenstrualPeriodDateNullFlavor",
										"last_menstrual_period_date_null_flavor",
									],
								),
								(
									"medicalHistoryText",
									&["medicalHistoryText", "medical_history_text"],
								),
								(
									"medicalHistoryTextNullFlavor",
									&[
										"medicalHistoryTextNullFlavor",
										"medical_history_text_null_flavor",
									],
								),
								(
									"concomitantTherapies",
									&["concomitantTherapies", "concomitant_therapy"],
								),
							],
						);
						if let Some(Value::String(value)) =
							normalized.get("raceCodes").cloned()
						{
							normalized.insert(
								"raceCodes".to_string(),
								Value::Array(vec![Value::String(value)]),
							);
						}
						normalized
					})
					.unwrap_or_default();
			if let Some(value) = rows.get("patientIdentifiers") {
				let identifiers =
					value.as_array().ok_or_else(|| Error::BadRequest {
						message: format!(
							"{section}.patientIdentifiers must be an array"
						),
					})?;
				for value in identifiers {
					let row = as_object(section, "patientIdentifiers", value)?;
					if bool_field(row, &["deleted", "_delete"]) == Some(true) {
						continue;
					}
					let Some(target) = string_field(
						row,
						&["identifierTypeCode", "identifier_type_code"],
					)
					.and_then(|code| match code.as_str() {
						"1" => Some("gpMedicalRecordNumber"),
						"2" => Some("specialistRecordNumber"),
						"3" => Some("hospitalRecordNumber"),
						"4" => Some("investigationNumber"),
						_ => None,
					}) else {
						continue;
					};
					if let Some(value) = row
						.get("identifierValue")
						.or_else(|| row.get("identifier_value"))
						.filter(|value| !value.is_null())
					{
						normalized.insert(target.to_string(), value.clone());
					}
					if let Some(value) = row
						.get("identifierValueNullFlavor")
						.or_else(|| row.get("identifier_value_null_flavor"))
						.filter(|value| !value.is_null())
					{
						normalized
							.insert(format!("{target}NullFlavor"), value.clone());
					}
				}
			}
			if let Some(value) = rows.get("medicalHistoryEpisodes") {
				let Some(episodes) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.medicalHistoryEpisodes must be an array"
						),
					});
				};
				let mut normalized_episodes = Vec::new();
				for value in episodes {
					let row = as_object(section, "medicalHistoryEpisodes", value)?;
					if bool_field(row, &["deleted", "_delete"]) == Some(true) {
						continue;
					}
					normalized_episodes.push(Value::Object(
						normalized_direct_object(
							row,
							&[
								(
									"meddraVersion",
									&["meddraVersion", "meddra_version"],
								),
								("meddraCode", &["meddraCode", "meddra_code"]),
								("startDate", &["startDate", "start_date"]),
								(
									"startDateNullFlavor",
									&[
										"startDateNullFlavor",
										"start_date_null_flavor",
									],
								),
								("continuing", &["continuing"]),
								(
									"continuingNullFlavor",
									&[
										"continuingNullFlavor",
										"continuing_null_flavor",
									],
								),
								("endDate", &["endDate", "end_date"]),
								(
									"endDateNullFlavor",
									&["endDateNullFlavor", "end_date_null_flavor"],
								),
								("comments", &["comments"]),
								(
									"familyHistory",
									&["familyHistory", "family_history"],
								),
							],
						),
					));
				}
				normalized.insert(
					"medicalHistoryEpisodes".to_string(),
					Value::Array(normalized_episodes),
				);
			}
			let mut patient_death = optional_row_object(section, rows, "deathInfo")?
				.map(|row| {
					normalized_direct_object(
						row,
						&[
							("dateOfDeath", &["dateOfDeath", "date_of_death"]),
							(
								"dateOfDeathNullFlavor",
								&[
									"dateOfDeathNullFlavor",
									"date_of_death_null_flavor",
								],
							),
							(
								"autopsyPerformed",
								&["autopsyPerformed", "autopsy_performed"],
							),
							(
								"autopsyPerformedNullFlavor",
								&[
									"autopsyPerformedNullFlavor",
									"autopsy_performed_null_flavor",
								],
							),
						],
					)
				})
				.unwrap_or_default();
			for (row_key, target) in [
				("reportedCauses", "reportedCausesOfDeath"),
				("autopsyCauses", "autopsyCausesOfDeath"),
			] {
				if let Some(value) = rows.get(row_key) {
					let Some(causes) = value.as_array() else {
						return Err(Error::BadRequest {
							message: format!("{section}.{row_key} must be an array"),
						});
					};
					let mut normalized_causes = Vec::new();
					for value in causes {
						let row = as_object(section, row_key, value)?;
						if bool_field(row, &["deleted", "_delete"]) == Some(true) {
							continue;
						}
						normalized_causes.push(Value::Object(
							normalized_direct_object(
								row,
								&[
									(
										"meddraVersion",
										&["meddraVersion", "meddra_version"],
									),
									("meddraCode", &["meddraCode", "meddra_code"]),
									("causeText", &["causeText", "comments"]),
								],
							),
						));
					}
					patient_death
						.insert(target.to_string(), Value::Array(normalized_causes));
				}
			}
			if !patient_death.is_empty() {
				normalized.insert(
					"patientDeath".to_string(),
					Value::Object(patient_death),
				);
			}
			let mut parent_information =
				optional_row_object(section, rows, "parentInfo")?
					.map(|parent| {
						normalized_direct_object(
							parent,
							&[
								(
									"parentIdentification",
									&[
										"parentIdentification",
										"parent_identification",
									],
								),
								(
									"parentIdentificationNullFlavor",
									&[
										"parentIdentificationNullFlavor",
										"parent_identification_null_flavor",
									],
								),
								(
									"parentBirthDate",
									&["parentBirthDate", "parent_birth_date"],
								),
								(
									"parentBirthDateNullFlavor",
									&[
										"parentBirthDateNullFlavor",
										"parent_birth_date_null_flavor",
									],
								),
								(
									"parentAge.value",
									&["parentAge.value", "parent_age"],
								),
								(
									"parentAge.unit",
									&[
										"parentAge.unit",
										"parentAgeUnit",
										"parent_age_unit",
									],
								),
								(
									"parentLastMenstrualPeriodDate",
									&[
										"parentLastMenstrualPeriodDate",
										"last_menstrual_period_date",
									],
								),
								(
									"parentLastMenstrualPeriodDateNullFlavor",
									&[
										"parentLastMenstrualPeriodDateNullFlavor",
										"last_menstrual_period_date_null_flavor",
									],
								),
								(
									"parentWeight.value",
									&["parentWeight.value", "weight_kg"],
								),
								(
									"parentHeight.value",
									&["parentHeight.value", "height_cm"],
								),
								("parentSex", &["parentSex", "sex"]),
								(
									"parentSexNullFlavor",
									&["parentSexNullFlavor", "sex_null_flavor"],
								),
								(
									"medicalHistoryText",
									&["medicalHistoryText", "medical_history_text"],
								),
							],
						)
					})
					.unwrap_or_default();
			if let Some(value) = rows.get("parentMedicalHistory") {
				let Some(history_rows) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.parentMedicalHistory must be an array"
						),
					});
				};
				let mut normalized_history = Vec::new();
				for value in history_rows {
					let row = as_object(section, "parentMedicalHistory", value)?;
					if bool_field(row, &["deleted", "_delete"]) == Some(true) {
						continue;
					}
					normalized_history.push(Value::Object(
						normalized_direct_object(
							row,
							&[
								(
									"meddraVersion",
									&["meddraVersion", "meddra_version"],
								),
								("meddraCode", &["meddraCode", "meddra_code"]),
								("startDate", &["startDate", "start_date"]),
								(
									"startDateNullFlavor",
									&[
										"startDateNullFlavor",
										"start_date_null_flavor",
									],
								),
								("continuing", &["continuing"]),
								(
									"continuingNullFlavor",
									&[
										"continuingNullFlavor",
										"continuing_null_flavor",
									],
								),
								("endDate", &["endDate", "end_date"]),
								(
									"endDateNullFlavor",
									&["endDateNullFlavor", "end_date_null_flavor"],
								),
								("comments", &["comments"]),
							],
						),
					));
				}
				parent_information.insert(
					"medicalHistoryEpisodes".to_string(),
					Value::Array(normalized_history),
				);
			}
			if let Some(value) = rows.get("parentPastDrugs") {
				let Some(drug_rows) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.parentPastDrugs must be an array"
						),
					});
				};
				let mut normalized_drugs = Vec::new();
				for value in drug_rows {
					let row = as_object(section, "parentPastDrugs", value)?;
					if bool_field(row, &["deleted", "_delete"]) == Some(true) {
						continue;
					}
					normalized_drugs.push(Value::Object(normalized_direct_object(
						row,
						&[
							("drugName", &["drugName", "drug_name"]),
							(
								"mfdsMedicinalProductVersion",
								&[
									"mfdsMedicinalProductVersion",
									"mfds_medicinal_product_version",
								],
							),
							(
								"mfdsMedicinalProductId",
								&[
									"mfdsMedicinalProductId",
									"mfds_medicinal_product_id",
								],
							),
							("mpidVersion", &["mpidVersion", "mpid_version"]),
							("mpid", &["mpid"]),
							("phpidVersion", &["phpidVersion", "phpid_version"]),
							("phpid", &["phpid"]),
							("startDate", &["startDate", "start_date"]),
							(
								"startDateNullFlavor",
								&["startDateNullFlavor", "start_date_null_flavor"],
							),
							("endDate", &["endDate", "end_date"]),
							(
								"endDateNullFlavor",
								&["endDateNullFlavor", "end_date_null_flavor"],
							),
							(
								"indicationMeddraVersion",
								&[
									"indicationMeddraVersion",
									"indication_meddra_version",
								],
							),
							(
								"indicationMeddraCode",
								&["indicationMeddraCode", "indication_meddra_code"],
							),
							(
								"reactionMeddraVersion",
								&[
									"reactionMeddraVersion",
									"reaction_meddra_version",
								],
							),
							(
								"reactionMeddraCode",
								&["reactionMeddraCode", "reaction_meddra_code"],
							),
						],
					)));
				}
				parent_information.insert(
					"pastDrugHistory".to_string(),
					Value::Array(normalized_drugs),
				);
			}
			if !parent_information.is_empty() {
				normalized.insert(
					"parentInformation".to_string(),
					Value::Object(parent_information),
				);
			}
			Some(normalized)
		}
		"NR" => {
			let narrative = optional_row_object(section, rows, "narrative")?;
			let mut normalized = narrative
				.map(|row| {
					normalized_direct_object(
						row,
						&[
							("caseNarrative", &["caseNarrative", "case_narrative"]),
							(
								"reporterComments",
								&["reporterComments", "reporter_comments"],
							),
							(
								"senderComments",
								&["senderComments", "sender_comments"],
							),
						],
					)
				})
				.unwrap_or_default();

			let sender_diagnoses = rows
				.get("senderDiagnoses")
				.or_else(|| narrative.and_then(|row| row.get("senderDiagnoses")));
			if let Some(value) = sender_diagnoses {
				let Some(items) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.senderDiagnoses must be an array"
						),
					});
				};
				let mut normalized_items = Vec::with_capacity(items.len());
				for value in items {
					let row = as_object(section, "senderDiagnoses", value)?;
					normalized_items.push(Value::Object(normalized_direct_object(
						row,
						&[
							(
								"diagnosisMeddraVersion",
								&[
									"diagnosisMeddraVersion",
									"diagnosis_meddra_version",
								],
							),
							(
								"diagnosisMeddraCode",
								&["diagnosisMeddraCode", "diagnosis_meddra_code"],
							),
						],
					)));
				}
				normalized.insert(
					"senderDiagnoses".to_string(),
					Value::Array(normalized_items),
				);
			}

			if let Some(value) = rows.get("caseSummaryInformation") {
				let Some(items) = value.as_array() else {
					return Err(Error::BadRequest {
						message: format!(
							"{section}.caseSummaryInformation must be an array"
						),
					});
				};
				let mut normalized_items = Vec::with_capacity(items.len());
				for value in items {
					let row = as_object(section, "caseSummaryInformation", value)?;
					normalized_items.push(Value::Object(normalized_direct_object(
						row,
						&[
							("summaryText", &["summaryText", "summary_text"]),
							("languageCode", &["languageCode", "language_code"]),
						],
					)));
				}
				normalized.insert(
					"caseSummaryInformation".to_string(),
					Value::Array(normalized_items),
				);
			}

			(!normalized.is_empty()).then_some(normalized)
		}
		_ => None,
	};

	if let Some(row) = normalized {
		validate_section_fields(section, &row, None, &[], fda)?;
	}
	Ok(())
}

pub(super) fn validate_local_text(
	row: &Map<String, Value>,
	request: &str,
	path: &str,
	limit: usize,
) -> Result<()> {
	validate_field(
		row,
		request,
		path,
		InputType::String,
		None,
		None,
		&[],
		|input| match input.value {
			InputValue::Missing | InputValue::Null => vec![],
			InputValue::String(value) if value.trim().chars().count() <= limit => {
				vec![]
			}
			_ => vec![InputIssue {
				code: "LOCAL.LENGTH.MAX",
				message: format!("must contain at most {limit} characters"),
			}],
		},
	)
}

fn normalized_changed_path(path: &str) -> String {
	path.split('.')
		.map(|part| {
			if part.parse::<usize>().is_ok() {
				"[]"
			} else {
				part
			}
		})
		.collect::<Vec<_>>()
		.join(".")
		.replace(".[]", "[]")
}

fn binding_was_changed(
	request_path: &str,
	changed_paths: Option<&BTreeSet<String>>,
) -> bool {
	changed_paths.is_none_or(|paths| {
		paths.iter().any(|path| {
			path == request_path || normalized_changed_path(path) == request_path
		})
	})
}

fn validate_field<F>(
	row: &Map<String, Value>,
	request_path: &str,
	frontend_path: &str,
	value_type: InputType,
	null_flavor: Option<(&str, &str, &'static str)>,
	changed_paths: Option<&BTreeSet<String>>,
	outer_indexes: &[usize],
	check: F,
) -> Result<()>
where
	F: for<'a> Fn(FieldInput<'a>) -> Vec<InputIssue>,
{
	if !binding_was_changed(request_path, changed_paths)
		&& null_flavor
			.is_none_or(|(path, _, _)| !binding_was_changed(path, changed_paths))
	{
		return Ok(());
	}
	let mut matched_indexes = request_matches(row, request_path)
		.into_iter()
		.map(|matched| matched.indexes)
		.collect::<Vec<_>>();
	if let Some((null_flavor_path, _, _)) = null_flavor {
		for matched in request_matches(row, null_flavor_path) {
			if !matched_indexes.contains(&matched.indexes) {
				matched_indexes.push(matched.indexes);
			}
		}
	}
	for indexes in matched_indexes {
		let mut concrete_indexes = outer_indexes.to_vec();
		concrete_indexes.extend_from_slice(&indexes);
		let value = value_at_request_path(row, request_path, &indexes)
			.map(|value| input_value(value, value_type))
			.unwrap_or(InputValue::Missing);
		let companion =
			null_flavor.and_then(|(request_path, frontend_path, code)| {
				value_at_request_path(row, request_path, &indexes)
					.map(|value| (value, frontend_path, code))
			});
		let null_flavor_value = companion
			.as_ref()
			.and_then(|(value, _, _)| {
				(!value.is_null()).then(|| {
					value.as_str().unwrap_or("__invalid_null_flavor_type__")
				})
			})
			.map(str::trim)
			.filter(|value| !value.is_empty());
		if !matches!(value, InputValue::Missing | InputValue::Null)
			&& null_flavor_value.is_some()
		{
			let (_, null_flavor_path, code) =
				companion.expect("companion is present");
			let path = concrete_frontend_path(null_flavor_path, &concrete_indexes);
			return Err(violation(
				code,
				&path,
				"value and NullFlavor cannot both be set",
			));
		}
		if let Some(issue) = check(FieldInput {
			value,
			null_flavor: null_flavor_value,
		})
		.into_iter()
		.next()
		{
			let issue_path = null_flavor
				.filter(|(_, _, code)| *code == issue.code)
				.map_or(frontend_path, |(_, path, _)| path);
			let path = concrete_frontend_path(issue_path, &concrete_indexes);
			return Err(violation(issue.code, &path, &issue.message));
		}
	}
	Ok(())
}

pub(crate) fn validate_row_payload(
	section: &str,
	row_key: &str,
	row: &Map<String, Value>,
	changed_paths: Option<&BTreeSet<String>>,
) -> Result<()> {
	validate_row_payload_with_indexes(section, row_key, row, changed_paths, &[])
}

fn validate_row_payload_with_indexes(
	section: &str,
	row_key: &str,
	row: &Map<String, Value>,
	changed_paths: Option<&BTreeSet<String>>,
	outer_indexes: &[usize],
) -> Result<()> {
	reject_control_characters(&Value::Object(row.clone()), row_key)?;
	validate_section_fields(section, row, changed_paths, outer_indexes, false)
}

fn reject_control_characters(value: &Value, path: &str) -> Result<()> {
	match value {
		Value::String(text)
			if text.chars().any(|character| {
				character.is_control() && !matches!(character, '\t' | '\n' | '\r')
			}) =>
		{
			return Err(violation(
				"INPUT.CONTROL_CHAR.REJECTED",
				path,
				"control characters are not allowed",
			));
		}
		Value::Array(values) => {
			for (index, value) in values.iter().enumerate() {
				reject_control_characters(value, &format!("{path}.{index}"))?;
			}
		}
		Value::Object(fields) => {
			for (key, value) in fields {
				reject_control_characters(value, &format!("{path}.{key}"))?;
			}
		}
		_ => {}
	}
	Ok(())
}

#[cfg(test)]
mod input_contract_save_tests {
	use super::*;

	fn error_message(error: Error) -> String {
		match error {
			Error::ConstraintViolation(detail) => format!(
				"{} at {}: {}",
				detail.rule_code, detail.path, detail.message
			),
			other => panic!("expected constraint violation, got {other:?}"),
		}
	}

	fn constraint_violation(error: Error) -> ConstraintViolation {
		match error {
			Error::ConstraintViolation(detail) => detail,
			other => panic!("expected constraint violation, got {other:?}"),
		}
	}

	#[test]
	fn input_contract_save_rejects_repeatable_row_values() {
		let reaction = Map::from_iter([(
			"primarySourceReaction".to_string(),
			json!("X".repeat(251)),
		)]);
		let error =
			validate_row_payload("AE", "reaction", &reaction, None).unwrap_err();
		let detail = constraint_violation(error);
		assert_eq!(detail.rule_code, "ICH.E.i.1.1a.LENGTH.MAX");
		assert_eq!(detail.path, "reactions.0.primarySourceReaction");
		assert_eq!(detail.message, "must contain at most 250 characters");

		let test_result =
			Map::from_iter([("testResult".to_string(), json!("not-a-number"))]);
		let error = validate_row_payload("LB", "testResult", &test_result, None)
			.unwrap_err();
		assert!(error_message(error)
			.contains("ICH.F.r.3.2.ALLOWED.VALUE at testResults.0.testResult"));

		let duration = Map::from_iter([(
			"reactionDuration".to_string(),
			json!({ "value": "1.00" }),
		)]);
		assert!(validate_row_payload("AE", "reaction", &duration, None).is_ok());
		let numeric_duration = Map::from_iter([(
			"reactionDuration".to_string(),
			json!({ "value": 1 }),
		)]);
		let detail = constraint_violation(
			validate_row_payload("AE", "reaction", &numeric_duration, None)
				.unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.E.i.6a.LENGTH.MAX");
		assert_eq!(detail.path, "reactions.0.reactionDuration.value");
	}

	#[test]
	fn input_contract_save_rejects_nul_before_persistence() {
		let row = Map::from_iter([("reporterEmail".to_string(), json!("bad\u{0}"))]);
		let detail = constraint_violation(
			validate_row_payload("RP", "primarySources", &row, None).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "INPUT.CONTROL_CHAR.REJECTED");

		let direct = BTreeMap::from([(
			"linkedReports".to_string(),
			json!([{ "linkedReportNumber": "bad\u{0}" }]),
		)]);
		let detail = constraint_violation(
			validate_direct_rows("CI", &direct, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "INPUT.CONTROL_CHAR.REJECTED");
	}

	#[test]
	fn direct_companion_null_flavors_are_contract_validated() {
		let ci = BTreeMap::from([(
			"safetyReportIdentification".to_string(),
			json!({ "combinationProductReportIndicatorNullFlavor": "ZZZ" }),
		)]);
		assert!(matches!(
			validate_direct_rows("CI", &ci, true),
			Err(Error::ConstraintViolation(_))
		));

		let rp = BTreeMap::from([(
			"primarySources".to_string(),
			json!([{ "reporterEmailNullFlavor": "ZZZ" }]),
		)]);
		assert!(matches!(
			validate_direct_rows("RP", &rp, true),
			Err(Error::ConstraintViolation(_))
		));
	}

	#[test]
	fn input_contract_save_preserves_nested_concrete_indexes() {
		let drug = Map::from_iter([(
			"dosageInformation".to_string(),
			json!([
				{ "doseValue": 1 },
				{ "doseValue": "not-a-number" }
			]),
		)]);
		let error = validate_row_payload("DG", "drug", &drug, None).unwrap_err();
		assert!(error_message(error)
			.contains("at drugs.0.dosageInformation.1.doseValue"));
	}

	#[test]
	fn input_contract_save_accepts_split_null_flavor_and_rejects_in_band_token() {
		let allowed = Map::from_iter([
			("reactionStartDate".to_string(), Value::Null),
			("reactionStartDateNullFlavor".to_string(), json!("MSK")),
		]);
		validate_row_payload("AE", "reaction", &allowed, None).unwrap();

		let invalid =
			Map::from_iter([("reactionStartDate".to_string(), json!("MSK"))]);
		let error =
			validate_row_payload("AE", "reaction", &invalid, None).unwrap_err();
		let detail = constraint_violation(error);
		assert_eq!(detail.rule_code, "ICH.E.i.4.ALLOWED.VALUE");
		assert_eq!(detail.path, "reactions.0.reactionStartDate");
	}

	#[test]
	fn input_contract_save_accepts_split_value_and_null_flavor_only_values() {
		let drug = Map::from_iter([(
			"dosageInformation".to_string(),
			json!([{
				"firstAdministrationDate": "20260715",
				"lastAdministrationDate": null,
				"lastAdministrationDateNullFlavor": "MSK"
			}]),
		)]);
		validate_row_payload("DG", "drug", &drug, None).unwrap();

		let invalid_token = Map::from_iter([(
			"dosageInformation".to_string(),
			json!([{ "firstAdministrationDate": "NI" }]),
		)]);
		let detail = constraint_violation(
			validate_row_payload("DG", "drug", &invalid_token, None).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.G.k.4.r.4.DATE.FORMAT");
	}

	#[test]
	fn input_contract_save_rejects_value_and_null_flavor_together() {
		let reaction = Map::from_iter([
			("reactionStartDate".to_string(), json!("20260715")),
			("reactionStartDateNullFlavor".to_string(), json!("MSK")),
		]);

		let detail = constraint_violation(
			validate_row_payload("AE", "reaction", &reaction, None).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.E.i.4.NULLFLAVOR.ALLOWED");
		assert_eq!(detail.path, "reactions.0.reactionStartDateNullFlavor");
		assert_eq!(detail.message, "value and NullFlavor cannot both be set");
	}

	#[test]
	fn input_contract_save_rejects_invalid_batch_transmission_date() {
		let message_header = Map::from_iter([(
			"batchTransmissionDate".to_string(),
			json!("not-a-date"),
		)]);
		let error =
			validate_row_payload("N", "messageHeader", &message_header, None)
				.unwrap_err();
		let detail = constraint_violation(error);
		assert_eq!(detail.rule_code, "ICH.N.1.5.ALLOWED.VALUE");
		assert_eq!(detail.path, "messageHeader.batchTransmissionDate");
	}

	#[test]
	fn input_contract_save_rejects_direct_page_rows_before_mutation() {
		let narrative_rows = BTreeMap::from([(
			"narrative".to_string(),
			json!({ "caseNarrative": "X".repeat(100_001) }),
		)]);
		let error = validate_direct_rows("NR", &narrative_rows, false).unwrap_err();
		assert!(error_message(error)
			.contains("ICH.H.1.LENGTH.MAX at narrative.caseNarrative"));
		let snake_case_rows = BTreeMap::from([(
			"narrative".to_string(),
			json!({ "case_narrative": "X".repeat(100_001) }),
		)]);
		validate_direct_rows("NR", &snake_case_rows, false).unwrap_err();
		let null_rows = BTreeMap::from([(
			"narrative".to_string(),
			json!({ "caseNarrative": null }),
		)]);
		let detail = constraint_violation(
			validate_direct_rows("NR", &null_rows, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.H.1.REQUIRED");
		assert_eq!(detail.path, "narrative.caseNarrative");

		let sender_rows = BTreeMap::from([(
			"senderInformation".to_string(),
			json!({ "organizationName": "X".repeat(101) }),
		)]);
		let error = validate_direct_rows("SD", &sender_rows, false).unwrap_err();
		assert!(error_message(error)
			.contains("ICH.C.3.2.LENGTH.MAX at senderInformation.organizationName"));
	}

	#[test]
	fn dm_patient_initials_na_requires_fda() {
		let rows = BTreeMap::from([(
			"patientInformation".to_string(),
			json!({
				"patientInitials": null,
				"patientInitialsNullFlavor": "NA"
			}),
		)]);

		validate_direct_rows("DM", &rows, true).unwrap();
		let detail = constraint_violation(
			validate_direct_rows("DM", &rows, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.D.1.NULLFLAVOR.ALLOWED");
		assert_eq!(detail.path, "patientInformation.patientInitialsNullFlavor");
	}

	#[test]
	fn dm_identifier_rows_project_to_flattened_input_contracts() {
		let rows = BTreeMap::from([
			("patientInformation".to_string(), json!({})),
			(
				"patientIdentifiers".to_string(),
				json!([{
					"identifierTypeCode": "1",
					"identifierValue": "X".repeat(101),
					"identifierValueNullFlavor": null
				}]),
			),
		]);
		let detail = constraint_violation(
			validate_direct_rows("DM", &rows, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.D.1.1.1.LENGTH.MAX");
		assert_eq!(detail.path, "patientInformation.gpMedicalRecordNumber");

		let rows = BTreeMap::from([
			("patientInformation".to_string(), json!({})),
			(
				"patientIdentifiers".to_string(),
				json!([{
					"identifierTypeCode": "4",
					"identifierValue": "INV-1",
					"identifierValueNullFlavor": "MSK"
				}]),
			),
		]);
		let detail = constraint_violation(
			validate_direct_rows("DM", &rows, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.D.1.1.4.NULLFLAVOR.ALLOWED");
		assert_eq!(
			detail.path,
			"patientInformation.investigationNumberNullFlavor"
		);
	}

	#[test]
	fn dm_parent_history_continuing_uses_split_pair() {
		let rows = BTreeMap::from([
			("patientInformation".to_string(), json!({})),
			(
				"parentMedicalHistory".to_string(),
				json!([{"continuing": null, "continuingNullFlavor": "NASK"}]),
			),
		]);
		validate_direct_rows("DM", &rows, false).unwrap();

		let rows = BTreeMap::from([
			("patientInformation".to_string(), json!({})),
			(
				"parentMedicalHistory".to_string(),
				json!([{"continuing": true, "continuingNullFlavor": "NASK"}]),
			),
		]);
		let detail = constraint_violation(
			validate_direct_rows("DM", &rows, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.D.10.7.1.r.3.NULLFLAVOR.ALLOWED");
		assert_eq!(
			detail.path,
			"patientInformation.parentInformation.medicalHistoryEpisodes.0.continuingNullFlavor"
		);
	}

	#[test]
	fn input_contract_save_rejects_si_nested_values_before_mutation() {
		let registrations = BTreeMap::from([
			("studyInformation".to_string(), json!({})),
			(
				"studyRegistrationNumbers".to_string(),
				json!([{"registrationNumber": "X".repeat(51)}]),
			),
		]);
		let detail = constraint_violation(
			validate_direct_rows("SI", &registrations, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "ICH.C.5.1.r.1.LENGTH.MAX");
		assert_eq!(
			detail.path,
			"studyInformation.studyRegistrationNumbers.0.registrationNumber"
		);

		let cross_report = BTreeMap::from([(
			"studyInformation".to_string(),
			json!({
				"fdaCrossReportedIndNumbers": [{"indNumberNullFlavor": "BAD"}]
			}),
		)]);
		let detail = constraint_violation(
			validate_direct_rows("SI", &cross_report, false).unwrap_err(),
		);
		assert_eq!(detail.rule_code, "FDA.C.5.6.r.NULLFLAVOR.ALLOWED");
		assert_eq!(
			detail.path,
			"studyInformation.fdaCrossReportedIndNumbers.0.indNumberNullFlavor"
		);
	}

	#[test]
	fn dg_relatedness_kr1_values_are_validated() {
		let row = json!({
			"drugReactionAssessments": [{
				"methodOfAssessmentKr1": "3",
				"resultOfAssessmentKr1": "7"
			}]
		});
		let detail = constraint_violation(
			validate_row_payload("DG", "drug", row.as_object().unwrap(), None)
				.unwrap_err(),
		);
		assert_eq!(detail.rule_code, "MFDS.G.k.9.i.2.r.2.KR.1.ALLOWED.VALUE");
		assert_eq!(
			detail.path,
			"drugs.0.drugReactionAssessments.0.methodOfAssessmentKr1"
		);
		let row = json!({
			"drugReactionAssessments": [{ "resultOfAssessmentKr1": "7" }]
		});
		let detail = constraint_violation(
			validate_row_payload("DG", "drug", row.as_object().unwrap(), None)
				.unwrap_err(),
		);
		assert_eq!(detail.rule_code, "MFDS.G.k.9.i.2.r.3.KR.1.ALLOWED.VALUE");

		let row = json!({
			"drugReactionAssessments": [{
				"resultOfAssessmentKr1": "1",
				"resultOfAssessmentKr1NullFlavor": "NA"
			}]
		});
		let detail = constraint_violation(
			validate_row_payload("DG", "drug", row.as_object().unwrap(), None)
				.unwrap_err(),
		);
		assert_eq!(
			detail.path,
			"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr1NullFlavor"
		);

		let row = json!({
			"drugReactionAssessments": [{
				"resultOfAssessmentKr1NullFlavor": "NA"
			}]
		});
		validate_row_payload("DG", "drug", row.as_object().unwrap(), None).unwrap();
	}

	#[test]
	fn dg_assessment_expectedness_is_validated() {
		let invalid = json!({
			"drugReactionAssessments": [{ "expectedness": "3" }]
		});
		let detail = constraint_violation(
			validate_row_payload("DG", "drug", invalid.as_object().unwrap(), None)
				.unwrap_err(),
		);
		assert_eq!(detail.rule_code, "LOCAL.G.k.9.i.expectedness.ALLOWED.VALUE");
		assert_eq!(
			detail.path,
			"drugs.0.drugReactionAssessments.0.expectedness"
		);

		let valid = json!({
			"drugReactionAssessments": [{ "expectedness": "2" }]
		});
		validate_row_payload("DG", "drug", valid.as_object().unwrap(), None)
			.unwrap();
	}
}
