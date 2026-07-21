use super::common::*;
use lib_rest_core::ConstraintViolation;
use std::collections::BTreeSet;
use validator::{
	bindings_for_section, portable_constraints, validate_portable_value,
	PortableConstraintKind, PortableFieldBinding, PortableInputValue,
	PortableValueType,
};

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

fn input_value<'a>(
	value: &'a Value,
	value_type: PortableValueType,
) -> PortableInputValue<'a> {
	if value.is_null() {
		return PortableInputValue::Missing;
	}
	match (value_type, value) {
		(PortableValueType::String, Value::String(value)) => {
			PortableInputValue::String(value)
		}
		(PortableValueType::Boolean, Value::Bool(value)) => {
			PortableInputValue::Boolean(*value)
		}
		(PortableValueType::Number, Value::Number(value)) => {
			PortableInputValue::Number(value)
		}
		_ => PortableInputValue::InvalidType,
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

fn companion_binding(
	section: &str,
	binding: &PortableFieldBinding,
) -> Option<&'static PortableFieldBinding> {
	let path = binding.null_flavor_path?;
	bindings_for_section(section).find(|candidate| candidate.frontend_path == path)
}

fn in_band_null_flavor<'a>(
	binding: &PortableFieldBinding,
	value: &'a Value,
) -> Option<&'a str> {
	let same_path = binding.null_flavor_path == Some(binding.frontend_path);
	if binding.null_flavor_path.is_some() && !same_path {
		return None;
	}
	let constraints = portable_constraints();
	let has_value_rule = binding.rule_codes.iter().any(|code| {
		constraints.iter().any(|rule| {
			rule.code == *code && rule.kind != PortableConstraintKind::NullFlavor
		})
	});
	if !has_value_rule && !same_path {
		return None;
	}
	let candidate = value.as_str()?.trim();
	binding.rule_codes.iter().find_map(|code| {
		constraints
			.iter()
			.find(|rule| {
				rule.code == *code && rule.kind == PortableConstraintKind::NullFlavor
			})
			.filter(|rule| rule.values.iter().any(|allowed| allowed == candidate))
			.map(|_| candidate)
	})
}

fn binding_has_value_rule(binding: &PortableFieldBinding) -> bool {
	let constraints = portable_constraints();
	binding.rule_codes.iter().any(|code| {
		constraints.iter().any(|rule| {
			rule.code == *code && rule.kind != PortableConstraintKind::NullFlavor
		})
	})
}

fn validate_binding_value(
	binding: &PortableFieldBinding,
	value: &Value,
	null_flavor: Option<&str>,
	path: &str,
) -> Result<()> {
	let constraints = portable_constraints();
	let in_band = in_band_null_flavor(binding, value);
	let has_value_rule = binding_has_value_rule(binding);
	let same_path = binding.null_flavor_path == Some(binding.frontend_path);
	for rule_code in binding.rule_codes {
		let kind = constraints
			.iter()
			.find(|rule| rule.code == *rule_code)
			.map(|rule| rule.kind);
		if in_band.is_some() && kind != Some(PortableConstraintKind::NullFlavor) {
			continue;
		}
		if in_band.is_none()
			&& (has_value_rule || same_path)
			&& kind == Some(PortableConstraintKind::NullFlavor)
		{
			continue;
		}
		let input = in_band
			.map(PortableInputValue::String)
			.unwrap_or_else(|| input_value(value, binding.value_type));
		if let Err(error) =
			validate_portable_value(rule_code, input, in_band.or(null_flavor))
		{
			return Err(violation(&error.code, path, &error.message));
		}
	}
	Ok(())
}

fn violation(rule_code: &str, path: &str, message: &str) -> Error {
	Error::ConstraintViolation(ConstraintViolation {
		rule_code: rule_code.to_owned(),
		path: path.to_owned(),
		message: message.to_owned(),
	})
}

pub(super) fn validate_direct_changes(
	section: &str,
	changes: &BTreeMap<String, CaseEditorFieldPatch>,
) -> Result<()> {
	for binding in bindings_for_section(section) {
		let Some(patch) = changes.get(binding.request_path) else {
			continue;
		};
		let missing = Value::Null;
		let value = patch.value.as_ref().unwrap_or(&missing);
		let null_flavor = patch
			.null_flavor
			.as_ref()
			.and_then(Option::as_deref)
			.or_else(|| {
				companion_binding(section, binding)
					.and_then(|companion| changes.get(companion.request_path))
					.and_then(|patch| patch.value.as_ref())
					.and_then(Value::as_str)
			});
		validate_binding_value(binding, value, null_flavor, binding.frontend_path)?;
	}
	Ok(())
}

fn normalized_direct_object(
	source: &Map<String, Value>,
	aliases: &[(&str, &[&str])],
) -> Map<String, Value> {
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
		if let Some(value) = candidates
			.iter()
			.find_map(|key| source.get(*key).filter(|value| !value.is_null()))
		{
			insert_path(&mut normalized, target, value.clone());
		}
	}
	normalized
}

pub(super) fn validate_direct_rows(
	section: &str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	let normalized = match section {
		"RP" => {
			optional_first_row_object(section, rows, "primarySources")?.map(|row| {
				normalized_direct_object(
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
						(
							"reporterCountryNullFlavor",
							&[
								"reporterCountryNullFlavor",
								"country_code_null_flavor",
							],
						),
						("reporterEmail", &["reporterEmail", "email"]),
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
				)
			})
		}
		"SD" => {
			optional_row_object(section, rows, "senderInformation")?.map(|row| {
				normalized_direct_object(
					row,
					&[
						("senderType", &["senderType", "sender_type"]),
						(
							"senderHealthProfessionalTypeKr1",
							&[
								"healthProfessionalTypeKr1",
								"health_professional_type_kr1",
							],
						),
						(
							"senderOrganization",
							&["organizationName", "organization_name"],
						),
						("senderDepartment", &["department"]),
						("senderPersonTitle", &["personTitle", "person_title"]),
						(
							"senderPersonGivenName",
							&["personGivenName", "person_given_name"],
						),
						(
							"senderPersonMiddleName",
							&["personMiddleName", "person_middle_name"],
						),
						(
							"senderPersonFamilyName",
							&["personFamilyName", "person_family_name"],
						),
						(
							"senderStreetAddress",
							&["streetAddress", "street_address"],
						),
						("senderCity", &["city"]),
						("senderState", &["state"]),
						("senderPostcode", &["postcode"]),
						("senderCountryCode", &["countryCode", "country_code"]),
						("senderTelephone", &["telephone"]),
						("senderFax", &["fax"]),
						("senderEmail", &["email"]),
					],
				)
			})
		}
		"LR" => optional_first_row_object(section, rows, "literatureReferences")?
			.map(|row| {
				normalized_direct_object(
					row,
					&[
						(
							"literatureReference",
							&["referenceText", "reference_text"],
						),
						(
							"referenceTextNullFlavor",
							&[
								"referenceTextNullFlavor",
								"reference_text_null_flavor",
							],
						),
						("documentBase64", &["documentBase64", "document_base64"]),
					],
				)
			}),
		"SI" => optional_row_object(section, rows, "studyInformation")?.map(|row| {
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
		}),
		"DM" => {
			optional_row_object(section, rows, "patientInformation")?.map(|row| {
				normalized_direct_object(
					row,
					&[
						(
							"patientInitials",
							&["patientInitials", "patient_initials"],
						),
						(
							"patientBirthDate",
							&["birthDateNullFlavor", "birth_date_null_flavor"],
						),
						("patientAge.unit", &["ageUnit", "age_unit"]),
						(
							"gestationPeriod.unit",
							&["gestationPeriodUnit", "gestation_period_unit"],
						),
						("patientAgeGroup", &["ageGroup", "age_group"]),
						("patientSex", &["sex", "sexNullFlavor", "sex_null_flavor"]),
						("raceCode", &["raceCode", "race_code"]),
						("ethnicityCode", &["ethnicityCode", "ethnicity_code"]),
						(
							"lastMenstrualPeriodDate",
							&[
								"lastMenstrualPeriodDateNullFlavor",
								"last_menstrual_period_date_null_flavor",
							],
						),
						(
							"medicalHistoryText",
							&[
								"medicalHistoryText",
								"medical_history_text",
								"medicalHistoryTextNullFlavor",
								"medical_history_text_null_flavor",
							],
						),
					],
				)
			})
		}
		"NR" => optional_row_object(section, rows, "narrative")?.map(|row| {
			normalized_direct_object(
				row,
				&[
					("caseNarrative", &["caseNarrative", "case_narrative"]),
					(
						"reporterComments",
						&["reporterComments", "reporter_comments"],
					),
					("senderComments", &["senderComments", "sender_comments"]),
				],
			)
		}),
		_ => None,
	};

	if let Some(row) = normalized {
		validate_row_payload(section, section, &row, None)?;
	}
	Ok(())
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
	binding: &PortableFieldBinding,
	changed_paths: Option<&BTreeSet<String>>,
) -> bool {
	changed_paths.is_none_or(|paths| {
		paths.iter().any(|path| {
			path == binding.request_path
				|| normalized_changed_path(path) == binding.request_path
		})
	})
}

pub(super) fn validate_row_payload(
	section: &str,
	_row_key: &str,
	row: &Map<String, Value>,
	changed_paths: Option<&BTreeSet<String>>,
) -> Result<()> {
	for binding in bindings_for_section(section) {
		if !binding_was_changed(binding, changed_paths) {
			continue;
		}
		for matched in request_matches(row, binding.request_path) {
			let null_flavor = companion_binding(section, binding)
				.and_then(|companion| {
					value_at_request_path(
						row,
						companion.request_path,
						&matched.indexes,
					)
				})
				.and_then(Value::as_str);
			let path =
				concrete_frontend_path(binding.frontend_path, &matched.indexes);
			validate_binding_value(binding, matched.value, null_flavor, &path)?;
		}
	}
	Ok(())
}

#[cfg(test)]
mod portable_save_tests {
	use super::*;

	fn changes(field: &str, value: Value) -> BTreeMap<String, CaseEditorFieldPatch> {
		BTreeMap::from([(
			field.to_string(),
			CaseEditorFieldPatch {
				value: Some(value),
				null_flavor: None,
			},
		)])
	}

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

	fn portable_constraint_message(code: &str) -> String {
		portable_constraints()
			.into_iter()
			.find(|constraint| constraint.code == code)
			.expect("portable Catalog constraint exists")
			.message
	}

	#[test]
	fn portable_save_rejects_direct_inline_and_null_flavor_values() {
		let inline =
			validate_direct_changes("CI", &changes("reportType", json!("9")))
				.unwrap_err();
		assert!(error_message(inline).contains(
			"ICH.C.1.3.ALLOWED.VALUE at safetyReportIdentification.reportType"
		));

		let null_flavor = validate_direct_changes(
			"CI",
			&changes("fulfilExpeditedCriteriaNullFlavor", json!("BAD")),
		)
		.unwrap_err();
		assert!(error_message(null_flavor).contains(
			"ICH.C.1.7.NULLFLAVOR.ALLOWED at safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor"
		));
	}

	#[test]
	fn portable_save_rejects_direct_overlength_values() {
		let error = validate_direct_changes(
			"SD",
			&changes("senderOrganization", json!("X".repeat(101))),
		)
		.unwrap_err();
		assert!(error_message(error).contains(
			"ICH.C.3.2.LENGTH.MAX at safetyReportIdentification.senderOrganization"
		));
	}

	#[test]
	fn portable_save_rejects_repeatable_row_values() {
		let reaction = Map::from_iter([(
			"reactionPrimarySourceNative".to_string(),
			json!("X".repeat(251)),
		)]);
		let error =
			validate_row_payload("AE", "reaction", &reaction, None).unwrap_err();
		let detail = constraint_violation(error);
		assert_eq!(detail.rule_code, "ICH.E.i.1.1a.LENGTH.MAX");
		assert_eq!(detail.path, "reactions.0.primarySourceReaction");
		assert_eq!(
			detail.message,
			portable_constraint_message("ICH.E.i.1.1a.LENGTH.MAX")
		);

		let test_result =
			Map::from_iter([("resultValue".to_string(), json!("not-a-number"))]);
		let error = validate_row_payload("LB", "testResult", &test_result, None)
			.unwrap_err();
		assert!(error_message(error)
			.contains("ICH.F.r.3.2.ALLOWED.VALUE at testResults.0.testResult"));
	}

	#[test]
	fn portable_save_preserves_nested_concrete_indexes() {
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
	fn portable_save_accepts_in_band_null_flavor_and_rejects_bad_date() {
		let allowed =
			Map::from_iter([("reactionStartDate".to_string(), json!("MSK"))]);
		validate_row_payload("AE", "reaction", &allowed, None).unwrap();

		let invalid =
			Map::from_iter([("reactionStartDate".to_string(), json!("2026-07-15"))]);
		let error =
			validate_row_payload("AE", "reaction", &invalid, None).unwrap_err();
		assert!(error_message(error)
			.contains("ICH.E.i.4.ALLOWED.VALUE at reactions.0.reactionStartDate"));
	}

	#[test]
	fn portable_save_accepts_normal_or_in_band_null_flavor_only_values() {
		let drug = Map::from_iter([(
			"dosageInformation".to_string(),
			json!([{
				"firstAdministrationDate": "20260715",
				"lastAdministrationDate": "MSK"
			}]),
		)]);
		validate_row_payload("DG", "drug", &drug, None).unwrap();
	}

	#[test]
	fn portable_save_rejects_direct_page_rows_before_mutation() {
		let narrative_rows = BTreeMap::from([(
			"narrative".to_string(),
			json!({ "caseNarrative": "X".repeat(100_001) }),
		)]);
		let error = validate_direct_rows("NR", &narrative_rows).unwrap_err();
		assert!(error_message(error)
			.contains("ICH.H.1.LENGTH.MAX at narrative.caseNarrative"));
		let snake_case_rows = BTreeMap::from([(
			"narrative".to_string(),
			json!({ "case_narrative": "X".repeat(100_001) }),
		)]);
		validate_direct_rows("NR", &snake_case_rows).unwrap_err();

		let sender_rows = BTreeMap::from([(
			"senderInformation".to_string(),
			json!({ "organizationName": "X".repeat(101) }),
		)]);
		let error = validate_direct_rows("SD", &sender_rows).unwrap_err();
		assert!(error_message(error).contains(
			"ICH.C.3.2.LENGTH.MAX at safetyReportIdentification.senderOrganization"
		));
	}
}
