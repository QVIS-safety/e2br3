use super::common::*;
use std::collections::BTreeSet;
use validator::{
	bindings_for_section, validate_portable_value, PortableFieldBinding,
	PortableInputValue, PortableValueType,
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

fn violation(rule_code: &str, path: &str, message: &str) -> Error {
	Error::BadRequest {
		message: format!("{rule_code} at {path}: {message}"),
	}
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
		for rule_code in binding.rule_codes {
			if let Err(error) = validate_portable_value(
				rule_code,
				input_value(value, binding.value_type),
				null_flavor,
			) {
				return Err(violation(
					&error.code,
					binding.frontend_path,
					&error.message,
				));
			}
		}
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
			for rule_code in binding.rule_codes {
				if let Err(error) = validate_portable_value(
					rule_code,
					input_value(matched.value, binding.value_type),
					null_flavor,
				) {
					return Err(violation(&error.code, &path, &error.message));
				}
			}
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
			Error::BadRequest { message } => message,
			other => panic!("expected bad request, got {other:?}"),
		}
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
		assert!(error_message(error).contains(
			"ICH.E.i.1.1a.LENGTH.MAX at reactions.0.primarySourceReaction"
		));

		let test_result =
			Map::from_iter([("resultValue".to_string(), json!("not-a-number"))]);
		let error = validate_row_payload("LB", "testResult", &test_result, None)
			.unwrap_err();
		assert!(error_message(error)
			.contains("ICH.F.r.3.2.ALLOWED.VALUE at testResults.0.testResultValue"));
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
}
